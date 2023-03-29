use std::cmp::max;
use std::sync::Arc;

use crate::{chunk, compress, encrypt, hash};

pub mod monitor;

pub struct Config {
    pub pack_size: u32,
    pub hasher: Arc<hash::Hasher>,
    pub chunker: Arc<chunk::Chunker>,
    pub prepare_threads: i8,
    pub compressor: Arc<compress::Compressor>,
    pub encryptor: Arc<encrypt::Encryptor>,
}

impl Config {
    pub fn new() -> Config {
        // let threads = max(std::thread::available_parallelism().unwrap().get() / 4, 1) as i8;
        let threads = 1;

        Config {
            pack_size: 20 * 1024 * 1024,
            hasher: hash::Hasher::build_by_name("Blake3").unwrap(),
            chunker: chunk::Chunker::build_by_name("Rabin64 (mmap)", 1 * 1024 * 1024).unwrap(),
            prepare_threads: threads,
            compressor: compress::Compressor::build_by_name("Snappy").unwrap(),
            encryptor: encrypt::Encryptor::build_by_name("ChaCha20Poly1305", "1234").unwrap(),
        }
    }
}
