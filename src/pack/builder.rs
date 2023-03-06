use crate::snapshot::builder::{ChunkBuilder, PathBuilder, SnapshotBuilder};
use anyhow::Error;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct PackBuilder {
    pub chunks: Vec<(
        Arc<SnapshotBuilder>,
        Arc<PathBuilder>,
        Arc<ChunkBuilder>,
        u32,
        u32,
    )>,
    data: Option<Vec<u8>>,
    hash: Vec<u8>,
    size_chunks: u32,
    size_compress: u32,
    size_encrypt: u32,
    size_ecc: u32,
    error: Mutex<Option<Error>>,
    start: Instant,
}

impl PackBuilder {
    pub fn new(pack_capacity: u32) -> PackBuilder {
        PackBuilder {
            chunks: Vec::new(),
            data: Some(Vec::with_capacity(pack_capacity as usize)),
            hash: Vec::new(),
            size_chunks: 0,
            size_compress: 0,
            size_encrypt: 0,
            size_ecc: 0,
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
        self.size_chunks = self_data.len() as u32;
    }

    pub fn get_size_chunks(&self) -> u32 {
        self.size_chunks
    }

    pub fn get_size_compress(&self) -> u32 {
        self.size_compress
    }

    pub fn get_size_encrypt(&self) -> u32 {
        self.size_encrypt
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

    pub fn set_compressed_data(&mut self, data: Vec<u8>, compressed: bool) {
        self.size_compress = data.len() as u32;
        self.data = Some(data);
    }

    pub fn set_encrypted_data(&mut self, data: Vec<u8>) {
        self.size_encrypt = data.len() as u32;
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
