mod snappy;
mod zstd;
mod deflate;
mod zlib;
mod lz4;
mod gzip;
mod bzip2;
mod lzma;
mod brotli;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use crate::compress::brotli::BrotliCompressor;
use crate::compress::snappy::SnappyCompressor;
use crate::compress::zstd::ZstdCompressor;
use crate::compress::bzip2::Bzip2Compressor;
use crate::compress::deflate::DeflateCompressor;
use crate::compress::gzip::GzipCompressor;
use crate::compress::lz4::LZ4Compressor;
use crate::compress::lzma::LzmaCompressor;
use crate::compress::zlib::ZlibCompressor;

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
    result.insert("deflate-fastest", lazy!(DeflateCompressor::new(1)));
    result.insert("deflate-default", lazy!(DeflateCompressor::new(6)));
    result.insert("deflate-better-compression", lazy!(DeflateCompressor::new(9)));
    result.insert("zlib-fastest", lazy!(ZlibCompressor::new(1)));
    result.insert("zlib-default", lazy!(ZlibCompressor::new(6)));
    result.insert("zlib-better-compression", lazy!(ZlibCompressor::new(9)));
    result.insert("gzip-fastest", lazy!(GzipCompressor::new(1)));
    result.insert("gzip-default", lazy!(GzipCompressor::new(6)));
    result.insert("gzip-better-compression", lazy!(GzipCompressor::new(9)));
    result.insert("bzip2-fastest", lazy!(Bzip2Compressor::new(1)));
    result.insert("bzip2-default", lazy!(Bzip2Compressor::new(6)));
    result.insert("bzip2-better-compression", lazy!(Bzip2Compressor::new(9)));
    result.insert("lzma-fastest", lazy!(LzmaCompressor::new(1)));
    result.insert("lzma-default", lazy!(LzmaCompressor::new(6)));
    result.insert("lzma-better-compression", lazy!(LzmaCompressor::new(9)));
    result.insert("brotli-fastest", lazy!(BrotliCompressor::new(0)));
    result.insert("brotli-default", lazy!(BrotliCompressor::new(4)));
    result.insert("brotli-better-compression", lazy!(BrotliCompressor::new(8)));
    result.insert("LZ4", lazy!(LZ4Compressor::new()));

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
