use std::sync::Arc;

use crate::{chunk, compress, encrypt, hash};

pub struct Config {
    pub hasher: Arc<dyn hash::Hasher>,
    pub chunker: Arc<chunk::Chunker>,
    pub compressor: Arc<compress::Compressor>,
    pub encryptor: Arc<dyn encrypt::Encryptor>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            hasher: hash::new("Blake3").unwrap(),
            // chunk: Box::newChaCha20Poly1305(chunk::rabin_mmap::RabinMmap::newChaCha20Poly1305(1 * 1024 * 1024)),
            // chunk: Box::newChaCha20Poly1305(chunk::hash_roll_mmap::FastCDC::newChaCha20Poly1305(1 * 1024 * 1024)),
            chunker: chunk::Chunker::build_by_name("Rabin64 (mmap)", 1 * 1024 * 1024).unwrap(),
            compressor: compress::Compressor::build_by_name("Snappy").unwrap(),
            encryptor: encrypt::new("ChaCha20Poly1305", "1234").unwrap(),
        }
    }
}
