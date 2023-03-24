use std::collections::HashMap;
use std::fs;
use std::path;
use std::sync::Arc;

use anyhow::{Context, Result};

mod fastcdc;
mod hash_roll;
mod rabin;
mod cdchunking;

pub struct Chunker {
    name: &'static str,
    ct: ChunkerType,
    block_size: u32,
    inner: Box<dyn ChunkerImpl>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
pub enum ChunkerType {
    FastCDC_v2020_mmap,
    FastCDC,
    FastCDC_mmap,
    RollSum,
    RollSum_mmap,
    ZPAQ,
    ZPAQ_mmap,
    RAM,
    RAM_mmap,
    Rabin64,
    Rabin64_mmap,
    ZPAQ_cc,
}

trait ChunkerImpl: Send + Sync {
    fn get_max_block_size(&self) -> u32;
    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()>;
}

impl Chunker {
    pub fn list_available_names() -> Vec<&'static str> {
        return REGISTERED.keys().map(|k| *k).collect();
    }

    pub fn build_by_name(name: &str, block_size: u32) -> Result<Arc<Chunker>> {
        let factory = REGISTERED
            .get(name)
            .with_context(|| format!("unknown chunker: '{}'", name))?;

        Ok(factory(block_size))
    }

    fn new(name: &'static str, ct: ChunkerType, block_size: u32, inner: Box<dyn ChunkerImpl>) -> Self {
        Self { name, ct, block_size, inner }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_type(&self) -> ChunkerType {
        self.ct
    }

    pub fn get_block_size(&self) -> u32 {
        self.block_size
    }

    pub fn get_max_block_size(&self) -> u32 {
        self.inner.get_max_block_size()
    }

    pub fn split(&self, path: &path::Path, metadata: &fs::Metadata, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        if metadata.len() < self.get_block_size() as u64 {
            let chunk = fs::read(path)?;
            cb(chunk);
            return Ok(());
        }

        let file = fs::File::open(path)?;
        self.inner.split(file, cb)
    }
}

type Factory = Box<dyn Fn(u32) -> Arc<Chunker> + Send + Sync>;

lazy_static! {
    static ref REGISTERED: HashMap<&'static str, Factory> = create_chunkers();
}

fn create_chunkers() -> HashMap<&'static str, Factory> {
    let mut by_name = HashMap::new();

    macro_rules! register {
        ($n:expr, $t:expr,  $f:expr) => {
            let factory : Factory = Box::new(|block_size| Arc::new(Chunker::new($n, $t, block_size, Box::new($f(block_size)))));
            by_name.insert($n, factory);
        };
    }

    register!("FastCDC v2020 (mmap)", ChunkerType::FastCDC_v2020_mmap, |block_size| fastcdc::FastCDC2020Mmap::new(block_size));
    register!("FastCDC", ChunkerType::FastCDC, |block_size| hash_roll::FastCdc::new(block_size, false));
    register!("FastCDC (mmap)", ChunkerType::FastCDC_mmap, |block_size| hash_roll::FastCdc::new(block_size, true));
    register!("Roll Sum", ChunkerType::RollSum, |block_size| hash_roll::RollSum::new(block_size, false));
    register!("Roll Sum (mmap)", ChunkerType::RollSum_mmap, |block_size| hash_roll::RollSum::new(block_size, true));
    register!("ZPAQ", ChunkerType::ZPAQ, |block_size| hash_roll::ZPAQ::new(block_size, false));
    register!("ZPAQ (mmap)", ChunkerType::ZPAQ_mmap, |block_size| hash_roll::ZPAQ::new(block_size, true));
    register!("RAM", ChunkerType::RAM, |block_size| hash_roll::RAM::new(block_size, false));
    register!("RAM (mmap)", ChunkerType::RAM_mmap,  |block_size| hash_roll::RAM::new(block_size, true));
    register!("Rabin64", ChunkerType::Rabin64, |block_size| rabin::Rabin::new(block_size, false));
    register!("Rabin64 (mmap)", ChunkerType::Rabin64_mmap, |block_size| rabin::Rabin::new(block_size, true));
    register!("ZPAQ (cc)", ChunkerType::ZPAQ_cc, |block_size| cdchunking::ZPAQ::new(block_size));

    by_name
}
