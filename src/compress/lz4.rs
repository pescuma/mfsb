use lz4_flex::{compress_prepend_size, decompress_size_prepended};

use super::*;

pub struct LZ4Compressor {}

impl LZ4Compressor {
    pub fn new() -> Self {
        LZ4Compressor {}
    }
}

impl CompressorImpl for LZ4Compressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let result = compress_prepend_size(data);
        Ok(result)
    }
}