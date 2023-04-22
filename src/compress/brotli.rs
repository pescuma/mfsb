use std::io::{Read, Write};

use brotlic::{BrotliDecoderOptions, BrotliEncoderOptions, CompressorWriter, DecompressorReader, Quality};

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
        let encoder = BrotliEncoderOptions::new()
            .quality(Quality::new(self.level)?)
            .build()?;
        let mut compressor = CompressorWriter::with_encoder(encoder, Vec::new());

        compressor.write_all(data)?;
        let result = compressor.into_inner()?;
        Ok(result)
    }

    fn decompress(&self, data: &[u8], result_size: u32) -> Result<Vec<u8>> {
        let decoder = BrotliDecoderOptions::new().build()?;
        let mut decompressor = DecompressorReader::with_decoder(decoder, data);

        let mut result = Vec::with_capacity(result_size as usize);
        decompressor.read_to_end(&mut result)?;
        Ok(result)
    }
}
