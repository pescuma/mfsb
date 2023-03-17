use super::*;
use anyhow::Result;
use std::io::Read;
use std::{fs, io};
use ::hash_roll as hr;
use ::hash_roll::ChunkIncr;

pub struct FastCdc {
    block_min: u64,
    block_normal: u64,
    block_max: u64,
    mmap: bool,
}

impl FastCdc {
    pub fn new(block_size: u32, mmap: bool) -> Self {
        FastCdc {
            block_min: (block_size * 8 / 10) as u64,
            block_normal: block_size as u64,
            block_max: (block_size * 12 / 10) as u64,
            mmap,
        }
    }
}

impl Chunker for FastCdc {
    fn get_block_size(&self) -> u32 {
        self.block_normal as u32
    }

    fn get_max_block_size(&self) -> u32 {
        self.block_max as u32
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let cfg = hr::fastcdc::FastCdc::new(
            &hr::gear_table::GEAR_64,
            self.block_min,
            self.block_normal,
            self.block_max,
        );

        if self.mmap {
            split_mmap(cfg, file, cb)
        } else {
            split_mem(cfg, file, cb)
        }
    }
}

pub struct ZPAQ {
    block_size: u32,
    block_max: u32,
    bits: u8,
    mmap: bool,
}

impl ZPAQ {
    pub fn new(block_size: u32, mmap: bool) -> Self {
        ZPAQ {
            block_size,
            block_max: (block_size * 2), // TODO
            bits: (block_size as f32).log2().ceil() as u8,
            mmap,
        }
    }
}

impl Chunker for ZPAQ {
    fn get_block_size(&self) -> u32 {
        self.block_size
    }

    fn get_max_block_size(&self) -> u32 {
        self.block_max
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let cfg = hr::zpaq::Zpaq::with_average_size_pow_2(self.bits);

        if self.mmap {
            split_mmap(cfg, file, cb)
        } else {
            split_mem(cfg, file, cb)
        }
    }
}

pub struct RollSum {
    block_size: u32,
    block_max: u32,
    mmap: bool,
}

impl RollSum {
    pub fn new(block_size: u32, mmap: bool) -> Self {
        RollSum {
            block_size,
            block_max: (block_size * 2), // TODO
            mmap,
        }
    }
}

impl Chunker for RollSum {
    fn get_block_size(&self) -> u32 {
        self.block_size
    }

    fn get_max_block_size(&self) -> u32 {
        self.block_max
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let cfg = hr::bup::RollSum::with_window(self.block_size as usize);

        if self.mmap {
            split_mmap(cfg, file, cb)
        } else {
            split_mem(cfg, file, cb)
        }
    }
}

pub struct RAM {
    block_size: u32,
    block_max: u32,
    mmap: bool,
}

impl RAM {
    pub fn new(block_size: u32, mmap: bool) -> Self {
        RAM {
            block_size,
            block_max: (block_size * 2), // TODO
            mmap,
        }
    }
}

impl Chunker for RAM {
    fn get_block_size(&self) -> u32 {
        self.block_size
    }

    fn get_max_block_size(&self) -> u32 {
        self.block_max
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let cfg = hr::ram::Ram::with_w(self.block_size as u64);

        if self.mmap {
            split_mmap(cfg, file, cb)
        } else {
            split_mem(cfg, file, cb)
        }
    }
}

fn split_mem(cfg: impl hr::ToChunkIncr, mut file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
    let mut chunker = cfg.to_chunk_incr();

    let mut chunk = Vec::new();

    let mut buffer = [0; 1024];
    loop {
        let size = match file.read(&mut buffer) {
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            o => o?,
        };

        if size == 0 {
            break;
        }

        let mut start = 0;
        while let Some(separator) = chunker.push(&buffer[start..size]) {
            chunk.extend_from_slice(&buffer[start..start + separator]);
            cb(chunk);
            chunk = Vec::new();

            start = start + separator;
        }
        if start < size {
            chunk.extend_from_slice(&buffer[start..size]);
        }
    }

    if !chunk.is_empty() {
        cb(chunk);
    }

    Ok(())
}

fn split_mmap(cfg: impl hr::ToChunkIncr, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
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
