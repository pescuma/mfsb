use super::*;
use anyhow::{Context, Result};
use hash_roll::{ChunkIncr, ToChunkIncr};
use std::fs;

pub struct FastCdcMmap {
    pub block_min: u64,
    pub block_normal: u64,
    pub block_max: u64,
}

impl ChunkerFactory for FastCdcMmap {
    type Type = FastCdcMmap;

    fn name() -> &'static str {
        "FastCDC (hr mmap)"
    }

    fn new(block_size: u32) -> Self::Type {
        FastCdcMmap {
            block_min: (block_size * 8 / 10) as u64,
            block_normal: block_size as u64,
            block_max: (block_size * 12 / 10) as u64,
        }
    }
}

impl Chunker for FastCdcMmap {
    fn get_max_block_size(&self) -> u32 {
        self.block_max as u32
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let cfg = hash_roll::fastcdc::FastCdc::new(
            &hash_roll::gear_table::GEAR_64,
            self.block_min,
            self.block_normal,
            self.block_max,
        );

        split(cfg, file, cb)
    }
}

pub struct ZpaqMmap {
    pub bits: u8,
    pub block_max: u32,
}

impl ChunkerFactory for ZpaqMmap {
    type Type = ZpaqMmap;

    fn name() -> &'static str {
        "ZPAQ (hr mmap)"
    }

    fn new(block_size: u32) -> Self::Type {
        ZpaqMmap {
            bits: (block_size as f32).log2().ceil() as u8,
            block_max: (block_size * 2), // TODO
        }
    }
}

impl Chunker for ZpaqMmap {
    fn get_max_block_size(&self) -> u32 {
        self.block_max
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let cfg = hash_roll::zpaq::Zpaq::with_average_size_pow_2(self.bits);

        split(cfg, file, cb)
    }
}

pub struct RollSumMmap {
    pub block_size: u32,
    pub block_max: u32,
}

impl ChunkerFactory for RollSumMmap {
    type Type = RollSumMmap;

    fn name() -> &'static str {
        "Roll Sum (hr mmap)"
    }

    fn new(block_size: u32) -> Self::Type {
        RollSumMmap {
            block_size,
            block_max: (block_size * 2), // TODO
        }
    }
}

impl Chunker for RollSumMmap {
    fn get_max_block_size(&self) -> u32 {
        self.block_max
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let cfg = hash_roll::bup::RollSum::with_window(self.block_size as usize);

        split(cfg, file, cb)
    }
}

pub struct RamMmap {
    pub block_size: u32,
    pub block_max: u32,
}

impl ChunkerFactory for RamMmap {
    type Type = RamMmap;

    fn name() -> &'static str {
        "RAM (hr mmap)"
    }

    fn new(block_size: u32) -> Self::Type {
        RamMmap {
            block_size,
            block_max: (block_size * 2), // TODO
        }
    }
}

impl Chunker for RamMmap {
    fn get_max_block_size(&self) -> u32 {
        self.block_max
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let cfg = hash_roll::ram::Ram::with_w(self.block_size as u64);

        split(cfg, file, cb)
    }
}

fn split(cfg: impl ToChunkIncr, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
    let mut chunker = cfg.to_chunk_incr();

    let mmap = unsafe { memmap2::Mmap::map(&file).context("failed to mmap")? };

    let mut start = 0;

    while let Some(separator) = chunker.push(&mmap[start..]) {
        cb(Vec::from(&mmap[start..start + separator]));
        start += separator;
    }

    if start < mmap.len() {
        cb(Vec::from(&mmap[start..]));
    }

    Ok(())
}
