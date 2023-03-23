use super::*;

pub struct ZstdCompressor {
    level: i32,
}

impl ZstdCompressor {
    pub fn new(level: i32) -> Self {
        ZstdCompressor { level }
    }
}

impl CompressorImpl for ZstdCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let result = ::zstd::stream::encode_all(data, self.level)?;
        Ok(result)
    }
}
