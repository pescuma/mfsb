use anyhow::Error;
use mfsb::pack::builder::PackBuilder;
use mfsb::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn main() {
    let root = "C:\\Users\\rdomenecci\\Books";
    let snapshot = snapshot::builder::SnapshotBuilder::new(PathBuf::from(root));
    create_snapshot(snapshot);
}

fn create_snapshot(snapshot: Arc<snapshot::builder::SnapshotBuilder>) {
    let mut threads = Vec::new();

    create_threads(&mut threads, snapshot.clone());

    for thread in threads {
        thread.join().unwrap();
    }

    assert_eq!(snapshot.is_complete(), true);
}

fn create_threads(
    threads: &mut Vec<thread::JoinHandle<()>>,
    snapshot: Arc<snapshot::builder::SnapshotBuilder>,
) {
    let config = pipeline::Config::new();
    let pack_size: u32 = 20 * 1024 * 1024;
    let chunks_per_pack = pack_size / config.chunker.get_max_block_size();

    let (walk_tx, walk_rx) = flume::unbounded();
    let (chunk_tx, chunk_rx) = flume::unbounded();
    let (hash_chunk_tx, hash_chunk_rx) = flume::bounded(chunks_per_pack as usize);
    let (pack_tx, pack_rx) = flume::bounded(0);
    let (store_pack_tx, store_pack_rx) = flume::bounded(0);

    walk_tx.send(snapshot).unwrap();

    macro_rules! recv {
        ($e:expr) => {
            match $e.recv() {
                Err(_) => break,
                Ok(o) => o,
            }
        };
    }

    threads.push(
        thread::Builder::new()
            .name(String::from("walk"))
            .spawn({
                let walk_rx = walk_rx.clone();
                let chunk_tx = chunk_tx.clone();

                move || loop {
                    let snapshot = recv!(walk_rx);

                    let now = Instant::now();

                    let mut count = 0;

                    let result = path_walk::path_walk(
                        snapshot.get_root().to_path_buf(),
                        |path, relative_path, metadata| {
                            match metadata {
                                Err(err) => {
                                    let path = snapshot.add_path(path, relative_path, None);
                                    path.set_error(err.into());
                                }
                                Ok(metadata) => {
                                    let len = if metadata.is_file() {
                                        metadata.len()
                                    } else {
                                        0
                                    };

                                    let path =
                                        snapshot.add_path(path, relative_path, Some(metadata));

                                    if len == 0 {
                                        path.set_finished_adding_chunks(0);
                                    } else {
                                        chunk_tx.send((snapshot.clone(), path)).unwrap();
                                    }
                                }
                            }

                            count += 1;
                        },
                    );

                    println!("path walk of {} items in: {:.0?}", count, now.elapsed());

                    match result {
                        Err(e) => snapshot.set_error(e),
                        Ok(_) => snapshot.set_finished_adding_paths(count),
                    }
                }
            })
            .unwrap(),
    );

    threads.push(
        thread::Builder::new()
            .name(String::from("chunk"))
            .spawn({
                let chunker = config.chunker.clone();
                let chunk_rx = chunk_rx.clone();
                let hash_chunk_tx = hash_chunk_tx.clone();

                move || loop {
                    let (snapshot, file) = recv!(chunk_rx);

                    let mut chunks = 0;

                    let result = chunk::split(
                        chunker.as_ref(),
                        file.get_path(),
                        file.get_metadata().unwrap(),
                        &mut |data| {
                            let chunk = file.add_chunk(data.len() as u32);
                            hash_chunk_tx
                                .send((snapshot.clone(), file.clone(), chunk, data))
                                .unwrap();
                            chunks += 1;
                        },
                    );
                    match result {
                        Err(e) => file.set_error(e),
                        Ok(_) => file.set_finished_adding_chunks(chunks),
                    }
                }
            })
            .unwrap(),
    );

    threads.push(
        thread::Builder::new()
            .name(String::from("hash chunk"))
            .spawn({
                let hasher = config.hasher.clone();
                let hash_chunk_rx = hash_chunk_rx.clone();
                let pack_tx = pack_tx.clone();

                move || loop {
                    let (snapshot, path, chunk, data) = recv!(hash_chunk_rx);

                    let hash = hasher.hash(&data);
                    chunk.set_hash(hash);

                    pack_tx.send((snapshot, path, chunk, data)).unwrap();
                }
            })
            .unwrap(),
    );

    threads.push(
        thread::Builder::new()
            .name(String::from("pack"))
            .spawn({
                let pack_capacity = pack_size
                    + config.chunker.get_max_block_size()
                    + config.encryptor.get_extra_space_needed();
                let hasher = config.hasher.clone();
                let compressor = config.compressor.clone();
                let encryptor = config.encryptor.clone();

                let pack_rx = pack_rx.clone();
                let store_pack_tx = store_pack_tx.clone();

                move || {
                    let mut pack = PackBuilder::new(pack_capacity);

                    loop {
                        let (snapshot, file, chunk, data) = recv!(pack_rx);

                        pack.add_chunk(snapshot, file, chunk, data);

                        if pack.get_size_chunks() > pack_size {
                            println!("pack finished: {}", pack.get_size_chunks());
                            prepare(
                                &mut pack,
                                hasher.as_ref(),
                                compressor.as_ref(),
                                encryptor.as_ref(),
                            );
                            store_pack_tx.send(pack).unwrap();
                            pack = PackBuilder::new(pack_capacity);
                        }
                    }

                    if pack.get_size_chunks() > 0 {
                        println!("pack finished: {}", pack.get_size_chunks());
                        prepare(
                            &mut pack,
                            hasher.as_ref(),
                            compressor.as_ref(),
                            encryptor.as_ref(),
                        );
                        store_pack_tx.send(pack).unwrap();
                    }
                }
            })
            .unwrap(),
    );

    threads.push(
        thread::Builder::new()
            .name(String::from("store pack"))
            .spawn({
                let store_pack_rx = store_pack_rx.clone();

                move || loop {
                    let mut pack = recv!(store_pack_rx);

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
                }
            })
            .unwrap(),
    );
}

fn prepare(
    pack: &mut PackBuilder,
    hasher: &dyn hash::Hasher,
    compressor: &dyn compress::Compressor,
    encryptor: &dyn encrypt::Encryptor,
) {
    let hash = hash::hash(hasher, pack.get_data());
    pack.set_hash(hash);

    let now = Instant::now();
    let compressed = match compress::compress(compressor, pack.take_data()) {
        Err(e) => {
            pack.set_error(e);
            return;
        }
        Ok(o) => o,
    };
    pack.set_compressed_data(compressed.0, compressed.1);
    println!(
        "pack compressed? {} {}% in: {:.0?}",
        compressed.1,
        pack.get_size_compress() * 100 / pack.get_size_chunks(),
        now.elapsed()
    );

    let now = Instant::now();
    let encrypted = match encrypt::encrypt(encryptor, pack.take_data()) {
        Err(e) => {
            pack.set_error(e);
            return;
        }
        Ok(o) => o,
    };
    pack.set_encrypted_data(encrypted);
    println!(
        "pack encrypted {}% in: {:.0?}",
        pack.get_size_encrypt() * 100 / pack.get_size_compress(),
        now.elapsed()
    );
}
