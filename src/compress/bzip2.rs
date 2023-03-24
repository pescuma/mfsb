use std::io::prelude::*;

use ::bzip2::read::{BzDecoder, BzEncoder};
use ::bzip2::Compression;

use super::*;

pub struct Bzip2Compressor {
    level: u32,
}

impl Bzip2Compressor {
    pub fn new(level: u32) -> Self {
        Bzip2Compressor { level }
    }
}

impl CompressorImpl for Bzip2Compressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut compressor = BzEncoder::new(data, Compression::new(self.level));
        let mut result = Vec::new();
        compressor.read_to_end(&mut result)?;
        Ok(result)
    }
}
