use super::*;

pub struct SnappyCompressor {}

impl CompressorFactory for SnappyCompressor {
    type Type = SnappyCompressor;

    fn name() -> &'static str {
        "Snappy"
    }

    fn new() -> Self::Type {
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
