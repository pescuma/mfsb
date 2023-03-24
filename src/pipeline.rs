use std::sync::Arc;

use crate::{chunk, compress, encrypt, hash};

pub struct Config {
    pub hasher: Arc<hash::Hasher>,
    pub chunker: Arc<chunk::Chunker>,
    pub compressor: Arc<compress::Compressor>,
    pub encryptor: Arc<dyn encrypt::Encryptor>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            hasher: hash::Hasher::build_by_name("Blake3").unwrap(),
            chunker: chunk::Chunker::build_by_name("Rabin64 (mmap)", 1 * 1024 * 1024).unwrap(),
            compressor: compress::Compressor::build_by_name("Snappy").unwrap(),
            encryptor: encrypt::new("ChaCha20Poly1305", "1234").unwrap(),
        }
    }
}
