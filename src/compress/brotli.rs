use std::io::{self, Read, Write};

use brotlic::{BrotliEncoderOptions, CompressorWriter, DecompressorReader, Quality};

use crate::pack;

use super::*;

pub struct BrotliCompressor {
    level: u8,
}

impl BrotliCompressor {
    pub fn new(level: u8) -> Self {
        BrotliCompressor { level }
    }
}

impl CompressorImpl for BrotliCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let encoder = BrotliEncoderOptions::new().quality(Quality::new(self.level)?).build()?;
        let mut compressor = CompressorWriter::with_encoder(encoder, Vec::new());
        compressor.write_all(data)?;
        let result = compressor.into_inner()?;
        Ok(result)
    }
}
