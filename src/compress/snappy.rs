use super::*;

pub struct SnappyCompressor {}

impl SnappyCompressor {
    pub fn new() -> Self {
        SnappyCompressor {}
    }
}

impl Compressor for SnappyCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut enc = snap::raw::Encoder::new();
        let result = enc.compress_vec(data)?;
        Ok(result)
    }
}
