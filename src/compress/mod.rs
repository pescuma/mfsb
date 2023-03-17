mod snappy;
mod zstd;

use crate::compress::snappy::SnappyCompressor;
use crate::compress::zstd::ZstdCompressor;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;

pub trait Compressor: Send + Sync {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;
}

pub type CompressorFactoryMap = HashMap<&'static str, Box<dyn Fn() -> Arc<dyn Compressor>>>;

pub fn list_available() -> CompressorFactoryMap {
    let mut result: CompressorFactoryMap = HashMap::new();
    macro_rules! lazy {
        ($f:expr) => {
            Box::new(|| Arc::new($f))
        };
    }

    result.insert("Snappy", lazy!(SnappyCompressor::new()));
    result.insert("zstd-fastest", lazy!(ZstdCompressor::new(1)));
    result.insert("zstd-default", lazy!(ZstdCompressor::new(3)));
    result.insert("zstd-better-compression", lazy!(ZstdCompressor::new(8)));

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
