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

    fn decompress(&self, data: &[u8], _result_size: u32) -> Result<Vec<u8>> {
        let result = ::zstd::stream::decode_all(data)?;
        Ok(result)
    }
}
