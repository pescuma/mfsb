use std::cmp::max;
use std::sync::Arc;

use anyhow::Error;
use anyhow::Result;
use flume::{Receiver, Sender};

use crate::chunk::Chunker;
use crate::compress::Compressor;
use crate::ecc::ECC;
use crate::encrypt::Encryptor;
use crate::hash::Hasher;
use crate::pack::builder::PackBuilder;
use crate::path_walk::path_walk;
use crate::pipeline::monitor::PipelineMonitor;
use crate::snapshot::builder::SnapshotBuilder;

pub mod monitor;

pub struct Pipeline {
    monitor: PipelineMonitor,
}

impl Pipeline {
    pub fn new(mut threads: u8) -> (Pipeline, Sender<Arc<SnapshotBuilder>>, Receiver<Arc<SnapshotBuilder>>) {
        if threads == 0 {
            threads = max(std::thread::available_parallelism().unwrap().get() / 4, 1) as u8;
        }

        let pack_size = 20 * 1024 * 1024;
        let hasher = Hasher::build_by_name("Blake3").unwrap();
        let chunker = Chunker::build_by_name("Rabin64 (mmap)", 1 * 1024 * 1024).unwrap();
        let prepare_threads = threads;
        let compressor = Compressor::build_by_name("Snappy").unwrap();
        let encryptor = Encryptor::build_by_name("ChaCha20Poly1305", "1234").unwrap();
        let ecc = ECC::build_by_name("SECDED").unwrap();
        let mut monitor = PipelineMonitor::new();

        let (tx, rx) =
            create_threads(&mut monitor, pack_size, hasher, chunker, prepare_threads, compressor, encryptor, ecc);

        (Self { monitor }, tx, rx)
    }

    pub fn join_threads(&self) {
        self.monitor.join_threads();
    }
}

fn create_threads(
    monitor: &mut PipelineMonitor,
    pack_size: u32,
    hasher: Arc<Hasher>,
    chunker: Arc<Chunker>,
    prepare_threads: u8,
    compressor: Arc<Compressor>,
    encryptor: Arc<Encryptor>,
    ecc: Arc<ECC>,
) -> (Sender<Arc<SnapshotBuilder>>, Receiver<Arc<SnapshotBuilder>>) {
    let (walk_tx, walk_rx): (Sender<Arc<SnapshotBuilder>>, Receiver<Arc<SnapshotBuilder>>) = flume::unbounded();
    let (chunk_tx, chunk_rx) = flume::unbounded();
    let (pack_tx, pack_rx) = flume::bounded(0);
    let (pack_prepare_tx, pack_prepare_rx) = flume::bounded(0);
    let (store_pack_tx, store_pack_rx) = flume::bounded(0);
    let (index_tx, index_rx) = flume::bounded(0);

    macro_rules! recv {
        ($e:expr) => {
            match $e.recv() {
                Err(_) => break,
                Ok(o) => o,
            }
        };
    }

    monitor
        .create_step("Walk", &walk_rx, &chunk_tx)
        .spawn_thread(|mut ctx| loop {
            let snapshot = recv!(ctx);

            let mut count = 0;

            let result = path_walk(snapshot.get_root().to_path_buf(), |path, relative_path, metadata| {
                match metadata {
                    Err(err) => {
                        let path = snapshot.add_path(path, relative_path, None);
                        path.set_error(err.into());
                    }
                    Ok(metadata) => {
                        let len = if metadata.is_file() { metadata.len() } else { 0 };

                        let path = snapshot.add_path(path, relative_path, Some(metadata));

                        if len == 0 {
                            path.set_finished_adding_chunks(0);
                        } else {
                            ctx.send((snapshot.clone(), path));
                        }
                    }
                }

                count += 1;
            });
            match result {
                Err(e) => snapshot.set_error(e),
                Ok(_) => snapshot.set_finished_adding_paths(count),
            };

            ctx.on_completed();
        });

    monitor
        .create_step("Chunk", &chunk_rx, &pack_tx)
        .spawn_thread({
            let chunker = chunker.clone();

            move |mut ctx| loop {
                let (snapshot, file) = recv!(ctx);

                let mut chunks = 0;

                let result = chunker.split(file.get_path(), file.get_metadata().unwrap(), &mut |data| {
                    let chunk = file.add_chunk(data.len() as u32);
                    ctx.send((snapshot.clone(), file.clone(), chunk, data));
                    chunks += 1;
                });
                match result {
                    Err(e) => file.set_error(e),
                    Ok(_) => file.set_finished_adding_chunks(chunks),
                }

                ctx.on_completed();
            }
        });

    monitor
        .create_step("Pack", &pack_rx, &pack_prepare_tx)
        .spawn_thread({
            let pack_capacity = pack_size + chunker.get_max_block_size() + encryptor.get_extra_space_needed();
            let hasher = hasher.clone();

            move |mut ctx| {
                let mut pack = PackBuilder::new(pack_capacity);

                loop {
                    let (snapshot, file, chunk, data) = recv!(ctx);

                    let hash = hasher.hash(&data);
                    chunk.set_hash(hash);

                    pack.add_chunk(snapshot, file, chunk, data);

                    if pack.get_size_chunks() > pack_size {
                        ctx.send(pack);
                        ctx.on_completed();

                        pack = PackBuilder::new(pack_capacity);
                    }
                }

                if pack.get_size_chunks() > 0 {
                    ctx.send(pack);
                    ctx.on_completed();
                }
            }
        });

    {
        let mut step = monitor.create_step("Prepare pack", &pack_prepare_rx, &store_pack_tx);

        for _ in 1..=prepare_threads {
            step.spawn_thread({
                let hasher = hasher.clone();
                let compressor = compressor.clone();
                let encryptor = encryptor.clone();
                let ecc = ecc.clone();

                move |mut ctx| loop {
                    let mut pack = recv!(ctx);

                    let result =
                        prepare(&mut pack, hasher.as_ref(), compressor.as_ref(), encryptor.as_ref(), ecc.as_ref());
                    match result {
                        Err(e) => pack.set_error(e),
                        Ok(_) => ctx.send(pack),
                    }

                    ctx.on_completed();
                }
            });
        }
    }

    monitor
        .create_step("Store pack", &store_pack_rx, &index_tx)
        .spawn_thread(move |mut ctx| loop {
            let mut pack = recv!(ctx);

            if let Some(e) = pack.take_error() {
                for (_, file, chunk, _, _) in pack.chunks {
                    file.set_error(Error::msg(format!("error creating pack with chunk {}: {}", chunk.get_index(), e)));
                }
            }

            ctx.on_completed();
        });

    (walk_tx, index_rx)
}

fn prepare(
    pack: &mut PackBuilder,
    hasher: &Hasher,
    compressor: &Compressor,
    encryptor: &Encryptor,
    ecc: &ECC,
) -> Result<()> {
    let hash = hasher.hash(pack.get_data());
    pack.set_hash(hash);

    let compressed = compressor.compress(pack.take_data())?;
    pack.set_compressed_data(compressed.0, compressed.1);

    let encrypted = encryptor.encrypt(pack.take_data())?;
    pack.set_encrypted_data(encrypted.0, encrypted.1);

    let after_ecc = ecc.write(pack.take_data())?;
    pack.set_ecc_data(after_ecc.0, after_ecc.1);

    Ok(())
}
