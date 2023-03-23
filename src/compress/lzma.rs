use super::*;
use std::io::prelude::*;
use xz2::read::{XzEncoder, XzDecoder};

pub struct LzmaCompressor {
    level: u32,
}

impl LzmaCompressor {
    pub fn new(level: u32) -> Self {
        LzmaCompressor {
            level
        }
    }
}

impl CompressorImpl for LzmaCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut compressor = XzEncoder::new(data, self.level);
        let mut result = Vec::new();
        compressor.read_to_end(&mut result)?;
        Ok(result)
    }
}
