use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Error;

use crate::compress::CompressionType;
use crate::ecc::ECCType;
use crate::encrypt::EncryptorType;
use crate::snapshot::builder::{ChunkBuilder, PathBuilder, SnapshotBuilder};

pub struct PackBuilder {
    pub chunks: Vec<(Arc<SnapshotBuilder>, Arc<PathBuilder>, Arc<ChunkBuilder>, u32, u32)>,
    data: Option<Vec<u8>>,
    hash: Vec<u8>,
    chunks_size: u32,
    compress_type: Option<CompressionType>,
    compress_size: u32,
    encrypt_type: Option<EncryptorType>,
    encrypt_size: u32,
    ecc_type: Option<ECCType>,
    ecc_size: u32,
    error: Mutex<Option<Error>>,
    start: Instant,
}

impl PackBuilder {
    pub fn new(pack_capacity: u32) -> PackBuilder {
        PackBuilder {
            chunks: Vec::new(),
            data: Some(Vec::with_capacity(pack_capacity as usize)),
            hash: Vec::new(),
            chunks_size: 0,
            compress_type: None,
            compress_size: 0,
            encrypt_type: None,
            encrypt_size: 0,
            ecc_type: None,
            ecc_size: 0,
            error: Mutex::new(None),
            start: Instant::now(),
        }
    }

    pub fn add_chunk(
        &mut self,
        snapshot: Arc<SnapshotBuilder>,
        file: Arc<PathBuilder>,
        chunk: Arc<ChunkBuilder>,
        mut data: Vec<u8>,
    ) {
        let self_data = self.data.as_mut().unwrap();
        let start = self_data.len() as u32;
        let size = data.len() as u32;

        self.chunks.push((snapshot, file, chunk, start, size));
        self_data.append(&mut data);
        self.chunks_size = self_data.len() as u32;
    }

    pub fn get_size_chunks(&self) -> u32 {
        self.chunks_size
    }

    pub fn get_size_compress(&self) -> u32 {
        self.compress_size
    }

    pub fn get_size_encrypt(&self) -> u32 {
        self.encrypt_size
    }

    pub fn get_data(&self) -> &[u8] {
        self.data.as_ref().unwrap()
    }

    pub fn take_data(&mut self) -> Vec<u8> {
        self.data.take().unwrap()
    }

    pub fn set_hash(&mut self, hash: Vec<u8>) {
        self.hash = hash;
    }

    pub fn set_compressed_data(&mut self, ct: CompressionType, data: Vec<u8>) {
        self.compress_type = Some(ct);
        self.compress_size = data.len() as u32;
        self.data = Some(data);
    }

    pub fn set_encrypted_data(&mut self, et: EncryptorType, data: Vec<u8>) {
        self.encrypt_type = Some(et);
        self.encrypt_size = data.len() as u32;
        self.data = Some(data);
    }

    pub fn set_ecc_data(&mut self, et: ECCType, data: Vec<u8>) {
        self.ecc_type = Some(et);
        self.ecc_size = data.len() as u32;
        self.data = Some(data);
    }

    pub fn set_error(&mut self, error: Error) {
        *self.error.lock().unwrap() = Some(error);
    }

    pub fn has_error(&mut self) -> bool {
        self.error.lock().unwrap().is_some()
    }

    pub fn take_error(&mut self) -> Option<Error> {
        self.error.lock().unwrap().take()
    }
}
