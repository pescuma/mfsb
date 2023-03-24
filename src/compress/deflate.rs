use super::*;

pub struct DeflateCompressor {
    level: i32,
}

impl DeflateCompressor {
    pub fn new(level: i32) -> Self {
        DeflateCompressor { level }
    }
}

impl CompressorImpl for DeflateCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut compressor = libdeflater::Compressor::new(libdeflater::CompressionLvl::new(self.level).unwrap());

        let size = compressor.deflate_compress_bound(data.len());
        let mut result = vec![0; size];

        let size = compressor.deflate_compress(data, &mut result)?;
        result.resize(size, 0);

        Ok(result)
    }
}
