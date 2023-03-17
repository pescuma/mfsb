mod fastcdc;
mod hash_roll;
mod rabin;
mod cdchunking;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path;
use std::sync::Arc;

pub trait Chunker: Send + Sync {
    fn get_block_size(&self) -> u32;
    fn get_max_block_size(&self) -> u32;
    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()>;
}

pub type ChunkerFactoryMap = HashMap<&'static str, Box<dyn Fn(u32) -> Arc<dyn Chunker>>>;

pub fn list_available() -> ChunkerFactoryMap {
    let mut result: ChunkerFactoryMap = HashMap::new();
    macro_rules! lazy {
        ($f:expr) => {
            Box::new(|block_size| Arc::new(($f)(block_size)))
        };
    }

    result.insert("FastCDC v2020 (mmap)", lazy!(|block_size| fastcdc::FastCDC2020Mmap::new(block_size)));
    result.insert("FastCDC", lazy!(|block_size| hash_roll::FastCdc::new(block_size, false)));
    result.insert("FastCDC (mmap)", lazy!(|block_size| hash_roll::FastCdc::new(block_size, true)));
    result.insert("Roll Sum", lazy!(|block_size| hash_roll::RollSum::new(block_size, false)));
    result.insert("Roll Sum (mmap)", lazy!(|block_size| hash_roll::RollSum::new(block_size, true)));
    result.insert("ZPAQ", lazy!(|block_size| hash_roll::ZPAQ::new(block_size, false)));
    result.insert("ZPAQ (mmap)", lazy!(|block_size| hash_roll::ZPAQ::new(block_size, true)));
    result.insert("RAM", lazy!(|block_size| hash_roll::RAM::new(block_size, false)));
    result.insert("RAM (mmap)", lazy!(|block_size| hash_roll::RAM::new(block_size, true)));
    result.insert("Rabin64", lazy!(|block_size| rabin::Rabin::new(block_size, false)));
    result.insert("Rabin64 (mmap)", lazy!(|block_size| rabin::Rabin::new(block_size, true)));
    result.insert("ZPAQ (cc)", lazy!(|block_size| cdchunking::ZPAQ::new(block_size)));

    return result;
}

pub fn new(name: &str, block_size: u32) -> Result<Arc<dyn Chunker>> {
    let available = list_available();

    let factory = available
        .get(name)
        .with_context(|| format!("unknown chunk: '{}'", name))?;

    Ok(factory(block_size))
}

pub fn split(
    chunker: &dyn Chunker,
    path: &path::Path,
    metadata: &fs::Metadata,
    cb: &mut dyn FnMut(Vec<u8>),
) -> Result<()> {
    if metadata.len() < chunker.get_block_size() as u64 {
        let chunk = fs::read(path)?;
        cb(chunk);
        return Ok(());
    }

    let file = fs::File::open(path)?;

    chunker.split(file, cb)
}
