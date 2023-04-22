use std::io::prelude::*;

use xz2::read::{XzDecoder, XzEncoder};

use super::*;

pub struct LzmaCompressor {
    level: u32,
}

impl LzmaCompressor {
    pub fn new(level: u32) -> Self {
        LzmaCompressor { level }
    }
}

impl CompressorImpl for LzmaCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut compressor = XzEncoder::new(data, self.level);

        let mut result = Vec::new();
        compressor.read_to_end(&mut result)?;
        Ok(result)
    }

    fn decompress(&self, data: &[u8], result_size: u32) -> Result<Vec<u8>> {
        let mut compressor = XzDecoder::new(data);

        let mut result = Vec::with_capacity(result_size as usize);
        compressor.read_to_end(&mut result)?;
        Ok(result)
    }
}
