use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Error;

use mfsb::pack::builder::PackBuilder;
use mfsb::*;

fn main() {
    let root = "C:\\Users\\rdomenecci\\Books";
    let snapshot = snapshot::builder::SnapshotBuilder::new(PathBuf::from(root));
    create_snapshot(snapshot);
}

fn create_snapshot(snapshot: Arc<snapshot::builder::SnapshotBuilder>) {
    create_threads(snapshot.clone());

    assert_eq!(snapshot.is_complete(), true);
}

fn create_threads(snapshot: Arc<snapshot::builder::SnapshotBuilder>) {
    let config = pipeline::Config::new();
    let chunks_per_pack = config.pack_size / config.chunker.get_block_size();

    let (walk_tx, walk_rx) = flume::unbounded();
    let (chunk_tx, chunk_rx) = flume::unbounded();
    let (hash_chunk_tx, hash_chunk_rx) = flume::bounded(chunks_per_pack as usize);
    let (pack_tx, pack_rx) = flume::bounded(0);
    let (pack_prepare_tx, pack_prepare_rx) = flume::bounded(0);
    let (store_pack_tx, store_pack_rx) = flume::bounded(0);
    let (index_tx, index_rx) = flume::bounded(0);

    walk_tx.send(snapshot).unwrap();

    let monitor = pipeline::monitor::PipelineMonitor::new();

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
        .spawn_thread(|ctx| loop {
            let snapshot = recv!(ctx);

            let now = Instant::now();

            let mut count = 0;

            let result = path_walk::path_walk(snapshot.get_root().to_path_buf(), |path, relative_path, metadata| {
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

            println!("path walk of {} items in: {:.0?}", count, now.elapsed());

            match result {
                Err(e) => snapshot.set_error(e),
                Ok(_) => snapshot.set_finished_adding_paths(count),
            }
        });

    monitor.create_step("Chunk", &chunk_rx, &hash_chunk_tx).spawn_thread({
        let chunker = config.chunker.clone();

        move |ctx| loop {
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
        }
    });

    monitor
        .create_step("Hash chunk", &hash_chunk_rx, &pack_tx)
        .spawn_thread({
            let hasher = config.hasher.clone();

            move |ctx| loop {
                let (snapshot, path, chunk, data) = recv!(ctx);

                let hash = hasher.hash(&data);
                chunk.set_hash(hash);

                ctx.send((snapshot, path, chunk, data));
            }
        });

    monitor.create_step("Pack", &pack_rx, &pack_prepare_tx).spawn_thread({
        let pack_size = config.pack_size;
        let pack_capacity = pack_size + config.chunker.get_max_block_size() + config.encryptor.get_extra_space_needed();

        move |ctx| {
            let mut pack = PackBuilder::new(pack_capacity);

            loop {
                let (snapshot, file, chunk, data) = recv!(ctx);

                pack.add_chunk(snapshot, file, chunk, data);

                if pack.get_size_chunks() > pack_size {
                    ctx.send(pack);
                    pack = PackBuilder::new(pack_capacity);
                }
            }

            if pack.get_size_chunks() > 0 {
                ctx.send(pack);
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

                move |ctx| loop {
                    let mut pack = recv!(ctx);

                    prepare(&mut pack, hasher.as_ref(), compressor.as_ref(), encryptor.as_ref());

                    ctx.send(pack);
                }
            });
        }
    }

    monitor
        .create_step("Store pack", &store_pack_rx, &index_tx)
        .spawn_thread({
            move |ctx| loop {
                let mut pack = recv!(ctx);

                if let Some(e) = pack.take_error() {
                    for (_, file, chunk, _, _) in pack.chunks {
                        file.set_error(Error::msg(format!(
                            "error creating pack with chunk {}: {}",
                            chunk.get_index(),
                            e
                        )));
                    }
                    continue;
                }

                ctx.send(pack);
            }
        });

    monitor.join_threads();
}

fn prepare(
    pack: &mut PackBuilder,
    hasher: &hash::Hasher,
    compressor: &compress::Compressor,
    encryptor: &encrypt::Encryptor,
) {
    let hash = hasher.hash(pack.get_data());
    pack.set_hash(hash);

    let now = Instant::now();
    let compressed = match compressor.compress(pack.take_data()) {
        Err(e) => {
            pack.set_error(e);
            return;
        }
        Ok(o) => o,
    };
    pack.set_compressed_data(compressed.0, compressed.1);
    println!(
        "pack compressed? {:?} {}% in: {:.0?}",
        compressed.0,
        pack.get_size_compress() * 100 / pack.get_size_chunks(),
        now.elapsed()
    );

    let now = Instant::now();
    let encrypted = match encryptor.encrypt(pack.take_data()) {
        Err(e) => {
            pack.set_error(e);
            return;
        }
        Ok(o) => o,
    };
    pack.set_encrypted_data(encrypted.0, encrypted.1);
    println!(
        "pack encrypted {:?} {}% in: {:.0?}",
        encrypted.0,
        pack.get_size_encrypt() * 100 / pack.get_size_compress(),
        now.elapsed()
    );
}
