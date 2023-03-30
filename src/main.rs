use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Error;
use anyhow::Result;
use flume::{Receiver, Sender};

use mfsb::ecc::ECC;
use mfsb::pack::builder::PackBuilder;
use mfsb::path_walk::path_walk;
use mfsb::snapshot::builder::SnapshotBuilder;
use mfsb::*;
use mfsb::{compress, encrypt, hash};

fn main() {
    let root = "C:\\Users\\rdomenecci\\Books";
    let snapshot = SnapshotBuilder::new(PathBuf::from(root));
    create_snapshot(snapshot);
}

fn create_snapshot(snapshot: Arc<SnapshotBuilder>) {
    let config = pipeline::Config::new();
    let monitor = pipeline::monitor::PipelineMonitor::new();

    let (tx, rx) = create_threads(&config, &monitor);

    tx.send(snapshot.clone()).unwrap();
    drop(tx);

    for _ in rx {}

    monitor.join_threads();

    assert_eq!(snapshot.is_complete(), true);
}

fn create_threads(
    config: &pipeline::Config,
    monitor: &pipeline::monitor::PipelineMonitor,
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

    monitor.create_step("Chunk", &chunk_rx, &pack_tx).spawn_thread({
        let chunker = config.chunker.clone();

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

    monitor.create_step("Pack", &pack_rx, &pack_prepare_tx).spawn_thread({
        let pack_size = config.pack_size;
        let pack_capacity = pack_size + config.chunker.get_max_block_size() + config.encryptor.get_extra_space_needed();
        let hasher = config.hasher.clone();

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

        for _ in 1..=config.prepare_threads {
            step.spawn_thread({
                let hasher = config.hasher.clone();
                let compressor = config.compressor.clone();
                let encryptor = config.encryptor.clone();
                let ecc = config.ecc.clone();

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
    hasher: &hash::Hasher,
    compressor: &compress::Compressor,
    encryptor: &encrypt::Encryptor,
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
