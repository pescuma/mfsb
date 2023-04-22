use super::*;

pub struct SnappyCompressor {}

impl SnappyCompressor {
    pub fn new() -> Self {
        SnappyCompressor {}
    }
}

impl CompressorImpl for SnappyCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut enc = snap::raw::Encoder::new();
        let result = enc.compress_vec(data)?;
        Ok(result)
    }

    fn decompress(&self, data: &[u8], _result_size: u32) -> Result<Vec<u8>> {
        let mut enc = snap::raw::Decoder::new();
        let result = enc.decompress_vec(data)?;
        Ok(result)
    }
}
