use super::*;

pub struct ZlibCompressor {
    pub level: i32,
}

impl ZlibCompressor {
    pub fn new(level: i32) -> Self {
        ZlibCompressor { level }
    }
}

impl CompressorImpl for ZlibCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut compressor = libdeflater::Compressor::new(libdeflater::CompressionLvl::new(self.level).unwrap());

        let size = compressor.zlib_compress_bound(data.len());
        let mut result = vec![0; size];

        let size = compressor.zlib_compress(data, &mut result)?;
        result.resize(size, 0);

        Ok(result)
    }

    fn decompress(&self, data: &[u8], result_size: u32) -> Result<Vec<u8>> {
        let mut decompressor = libdeflater::Decompressor::new();

        let mut result = vec![0; result_size as usize];
        let size = decompressor.zlib_decompress(data, &mut result)?;
        result.resize(size, 0);

        Ok(result)
    }
}
