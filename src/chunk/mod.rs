mod fastcdc_2020_mmap;
mod hash_roll_mem;
mod hash_roll_mmap;
mod rabin_mem;
mod rabin_mmap;
mod zpaq;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path;
use std::sync::Arc;

pub trait ChunkerFactory {
    type Type;
    fn name() -> &'static str;
    fn new(block_size: u32) -> Self::Type;
}

pub trait Chunker: Send + Sync {
    fn get_max_block_size(&self) -> u32;
    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()>;
}

pub type ChunkerFactoryMap = HashMap<&'static str, Box<dyn Fn(u32) -> Arc<dyn Chunker>>>;

pub fn list_available() -> ChunkerFactoryMap {
    let mut result: ChunkerFactoryMap = HashMap::new();
    macro_rules! add {
        ($F:ty) => {
            result.insert(
                <$F>::name(),
                Box::new(|block_size| Arc::new(<$F>::new(block_size))),
            );
        };
    }

    add!(fastcdc_2020_mmap::FastCDC2020Mmap);
    add!(hash_roll_mem::FastCdcMem);
    add!(hash_roll_mem::RollSumMem);
    add!(hash_roll_mem::ZpaqMem);
    add!(hash_roll_mem::RamMem);
    add!(hash_roll_mmap::FastCdcMmap);
    add!(hash_roll_mmap::RollSumMmap);
    add!(hash_roll_mmap::ZpaqMmap);
    add!(hash_roll_mmap::RamMmap);
    add!(rabin_mem::RabinMen);
    add!(rabin_mmap::RabinMmap);
    add!(zpaq::ZPAQ);

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
    if metadata.len() < chunker.get_max_block_size() as u64 {
        let chunk = fs::read(path)?;
        cb(chunk);
        return Ok(());
    }

    let file = fs::File::open(path)?;

    chunker.split(file, cb)
}
