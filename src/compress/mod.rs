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

pub struct Compressor {
    name: &'static str,
    ct: CompressionType,
    inner: Box<dyn CompressorImpl>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompressionType {
    SNAPPY,
    ZSTD,
    DEFLATE,
    ZLIB,
    GZIP,
    BZIP2,
    LZMA,
    BROTLI,
    LZ4,
}

/// Trait that must be implemented by new compressors
trait CompressorImpl: Send + Sync {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;
}

impl Compressor {
    pub fn list_available_names() -> Vec<&'static str> {
        return REGISTERED.0.keys().map(|k| *k).collect();
    }

    pub fn build_by_name(name: &str) -> Result<Arc<Compressor>> {
        let compressor = REGISTERED.0
            .get(name)
            .with_context(|| format!("unknown compressor: '{}'", name))?;

        Ok(compressor.clone())
    }

    pub fn build_by_type(ct: CompressionType) -> Result<Arc<Compressor>> {
        let compressor = REGISTERED.1
            .get(&ct)
            .with_context(|| format!("unknown compressor: {:?}", ct))?;

        Ok(compressor.clone())
    }

    fn new(name: &'static str, ct: CompressionType, inner: Box<dyn CompressorImpl>) -> Self {
        Self { name, ct, inner }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_type(&self) -> CompressionType {
        self.ct
    }

    pub fn compress(&self, data: Vec<u8>) -> Result<(Option<CompressionType>, Vec<u8>)> {
        let result = self.inner.compress(&data)?;

        if result.len() < data.len() {
            Ok((Some(self.ct), result))
        } else {
            Ok((None, data))
        }
    }
}

lazy_static! {
    static ref REGISTERED: (HashMap<&'static str, Arc<Compressor>>, HashMap<CompressionType, Arc<Compressor>>) = create_compressors();
}

fn create_compressors() -> (HashMap<&'static str, Arc<Compressor>>, HashMap<CompressionType, Arc<Compressor>>) {
    let mut by_name = HashMap::new();
    let mut by_type = HashMap::new();

    macro_rules! register {
        ($n:expr, $t:expr,  $f:expr) => {
            let c = Arc::new(Compressor::new($n, $t, Box::new($f)));
            by_name.insert($n, c.clone());
            if !by_type.contains_key(&$t) {
                by_type.insert($t, c);
            }
        };
    }

    register!("Snappy", CompressionType::SNAPPY, SnappyCompressor::new());
    register!("zstd-default", CompressionType::ZSTD, ZstdCompressor::new(3));
    register!("zstd-fastest", CompressionType::ZSTD, ZstdCompressor::new(1));
    register!("zstd-better-compression", CompressionType::ZSTD, ZstdCompressor::new(8));
    register!("deflate-default", CompressionType::DEFLATE, DeflateCompressor::new(6));
    register!("deflate-fastest", CompressionType::DEFLATE, DeflateCompressor::new(1));
    register!("deflate-better-compression", CompressionType::DEFLATE, DeflateCompressor::new(9));
    register!("zlib-default", CompressionType::ZLIB, ZlibCompressor::new(6));
    register!("zlib-fastest", CompressionType::ZLIB, ZlibCompressor::new(1));
    register!("zlib-better-compression", CompressionType::ZLIB, ZlibCompressor::new(9));
    register!("gzip-default", CompressionType::GZIP, GzipCompressor::new(6));
    register!("gzip-fastest", CompressionType::GZIP, GzipCompressor::new(1));
    register!("gzip-better-compression", CompressionType::GZIP, GzipCompressor::new(9));
    register!("bzip2-default", CompressionType::BZIP2, Bzip2Compressor::new(6));
    register!("bzip2-fastest", CompressionType::BZIP2, Bzip2Compressor::new(1));
    register!("bzip2-better-compression", CompressionType::BZIP2, Bzip2Compressor::new(9));
    register!("lzma-default", CompressionType::LZMA, LzmaCompressor::new(6));
    register!("lzma-fastest", CompressionType::LZMA, LzmaCompressor::new(1));
    register!("lzma-better-compression", CompressionType::LZMA, LzmaCompressor::new(9));
    register!("brotli-default", CompressionType::BROTLI, BrotliCompressor::new(4));
    register!("brotli-fastest", CompressionType::BROTLI, BrotliCompressor::new(0));
    register!("brotli-better-compression", CompressionType::BROTLI, BrotliCompressor::new(8));
    register!("LZ4", CompressionType::LZ4, LZ4Compressor::new());

    (by_name, by_type)
}
