use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::compress::brotli::BrotliCompressor;
use crate::compress::bzip2::Bzip2Compressor;
use crate::compress::deflate::DeflateCompressor;
use crate::compress::gzip::GzipCompressor;
use crate::compress::lz4::LZ4Compressor;
use crate::compress::lzma::LzmaCompressor;
use crate::compress::snappy::SnappyCompressor;
use crate::compress::zlib::ZlibCompressor;
use crate::compress::zstd::ZstdCompressor;

mod brotli;
mod bzip2;
mod deflate;
mod gzip;
mod lz4;
mod lzma;
mod snappy;
mod zlib;
mod zstd;

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

trait CompressorImpl: Send + Sync {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;
}

impl Compressor {
    pub fn list_available_names() -> Vec<&'static str> {
        return REGISTERED.0.keys().map(|k| *k).collect();
    }

    pub fn build_by_name(name: &str) -> Result<Arc<Compressor>> {
        let compressor = REGISTERED
            .0
            .get(name)
            .with_context(|| format!("unknown compressor: '{}'", name))?;

        Ok(compressor.clone())
    }

    pub fn build_by_type(ct: CompressionType) -> Result<Arc<Compressor>> {
        let compressor = REGISTERED
            .1
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
    static ref REGISTERED: (HashMap<&'static str, Arc<Compressor>>, HashMap<CompressionType, Arc<Compressor>>) =
        create_compressors();
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

    use CompressionType::*;

    register!("Snappy", SNAPPY, SnappyCompressor::new());
    register!("zstd-default", ZSTD, ZstdCompressor::new(3));
    register!("zstd-fastest", ZSTD, ZstdCompressor::new(1));
    register!("zstd-better-compression", ZSTD, ZstdCompressor::new(8));
    register!("deflate-default", DEFLATE, DeflateCompressor::new(6));
    register!("deflate-fastest", DEFLATE, DeflateCompressor::new(1));
    register!("deflate-better-compression", DEFLATE, DeflateCompressor::new(9));
    register!("zlib-default", ZLIB, ZlibCompressor::new(6));
    register!("zlib-fastest", ZLIB, ZlibCompressor::new(1));
    register!("zlib-better-compression", ZLIB, ZlibCompressor::new(9));
    register!("gzip-default", GZIP, GzipCompressor::new(6));
    register!("gzip-fastest", GZIP, GzipCompressor::new(1));
    register!("gzip-better-compression", GZIP, GzipCompressor::new(9));
    register!("bzip2-default", BZIP2, Bzip2Compressor::new(6));
    register!("bzip2-fastest", BZIP2, Bzip2Compressor::new(1));
    register!("bzip2-better-compression", BZIP2, Bzip2Compressor::new(9));
    register!("lzma-default", LZMA, LzmaCompressor::new(6));
    register!("lzma-fastest", LZMA, LzmaCompressor::new(1));
    register!("lzma-better-compression", LZMA, LzmaCompressor::new(9));
    register!("brotli-default", BROTLI, BrotliCompressor::new(4));
    register!("brotli-fastest", BROTLI, BrotliCompressor::new(0));
    register!("brotli-better-compression", BROTLI, BrotliCompressor::new(8));
    register!("LZ4", LZ4, LZ4Compressor::new());

    (by_name, by_type)
}
