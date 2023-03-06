mod snappy;
mod zstd;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;

pub trait CompressorFactory {
    type Type;
    fn name() -> &'static str;
    fn new() -> Self::Type;
}

pub trait Compressor: Send + Sync {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;
}

pub type CompressorFactoryMap = HashMap<&'static str, Box<dyn Fn() -> Arc<dyn Compressor>>>;

pub fn list_available() -> CompressorFactoryMap {
    let mut result: CompressorFactoryMap = HashMap::new();
    macro_rules! add {
        ($F:ty) => {
            result.insert(<$F>::name(), Box::new(|| Arc::new(<$F>::new())));
        };
    }

    add!(snappy::SnappyCompressor);
    add!(zstd::ZstdCompressor);
    add!(zstd::Zstd1Compressor);
    add!(zstd::Zstd2Compressor);
    add!(zstd::Zstd3Compressor);
    add!(zstd::Zstd4Compressor);
    add!(zstd::Zstd5Compressor);
    add!(zstd::Zstd6Compressor);
    add!(zstd::Zstd7Compressor);
    add!(zstd::Zstd8Compressor);
    add!(zstd::Zstd9Compressor);

    return result;
}

pub fn new(name: &str) -> Result<Arc<dyn Compressor>> {
    let available = list_available();

    let factory = available
        .get(name)
        .with_context(|| format!("unknown compressor: '{}'", name))?;

    Ok(factory())
}

pub fn compress(compressor: &dyn Compressor, data: Vec<u8>) -> Result<(Vec<u8>, bool)> {
    let result = compressor.compress(&data)?;

    Ok(if result.len() < data.len() {
        (result, true)
    } else {
        (data, false)
    })
}
