use crate::{chunk, compress, encrypt, hash};
use std::sync::Arc;

pub struct Config {
    pub hasher: Arc<dyn hash::Hasher>,
    pub chunker: Arc<dyn chunk::Chunker>,
    pub compressor: Arc<dyn compress::Compressor>,
    pub encryptor: Arc<dyn encrypt::Encryptor>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            hasher: hash::new("Blake3").unwrap(),
            // chunk: Box::new(chunk::rabin_mmap::RabinMmap::new(1 * 1024 * 1024)),
            // chunk: Box::new(chunk::hash_roll_mmap::FastCDC::new(1 * 1024 * 1024)),
            chunker: chunk::new("Rabin64 (mmap)", 1 * 1024 * 1024).unwrap(),
            compressor: compress::new("Snappy").unwrap(),
            encryptor: encrypt::new("ChaCha20Poly1305", "1234").unwrap(),
        }
    }
}
