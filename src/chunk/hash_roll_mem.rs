use super::*;
use anyhow::Result;
use hash_roll::{ChunkIncr, ToChunkIncr};
use std::io::Read;
use std::{fs, io};

pub struct FastCdcMem {
    pub block_min: u64,
    pub block_normal: u64,
    pub block_max: u64,
}

impl ChunkerFactory for FastCdcMem {
    type Type = FastCdcMem;

    fn name() -> &'static str {
        "FastCDC (hr)"
    }

    fn new(block_size: u32) -> Self::Type {
        FastCdcMem {
            block_min: (block_size * 8 / 10) as u64,
            block_normal: block_size as u64,
            block_max: (block_size * 12 / 10) as u64,
        }
    }
}

impl Chunker for FastCdcMem {
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

pub struct ZpaqMem {
    pub bits: u8,
    pub block_max: u32,
}

impl ChunkerFactory for ZpaqMem {
    type Type = ZpaqMem;

    fn name() -> &'static str {
        "ZPAQ (hr)"
    }

    fn new(block_size: u32) -> Self::Type {
        ZpaqMem {
            bits: (block_size as f32).log2().ceil() as u8,
            block_max: (block_size * 2), // TODO
        }
    }
}

impl Chunker for ZpaqMem {
    fn get_max_block_size(&self) -> u32 {
        self.block_max
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let cfg = hash_roll::zpaq::Zpaq::with_average_size_pow_2(self.bits);

        split(cfg, file, cb)
    }
}

pub struct RollSumMem {
    pub block_size: u32,
    pub block_max: u32,
}

impl ChunkerFactory for RollSumMem {
    type Type = RollSumMem;

    fn name() -> &'static str {
        "Roll Sum (hr)"
    }

    fn new(block_size: u32) -> Self::Type {
        RollSumMem {
            block_size,
            block_max: (block_size * 2), // TODO
        }
    }
}

impl Chunker for RollSumMem {
    fn get_max_block_size(&self) -> u32 {
        self.block_max
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let cfg = hash_roll::bup::RollSum::with_window(self.block_size as usize);
        // let cfg = hash_roll::ram::Ram::with_w(1 << 20);

        split(cfg, file, cb)
    }
}

pub struct RamMem {
    pub block_size: u32,
    pub block_max: u32,
}

impl ChunkerFactory for RamMem {
    type Type = RamMem;

    fn name() -> &'static str {
        "RAM (hr)"
    }

    fn new(block_size: u32) -> Self::Type {
        RamMem {
            block_size,
            block_max: (block_size * 2), // TODO
        }
    }
}

impl Chunker for RamMem {
    fn get_max_block_size(&self) -> u32 {
        self.block_max
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let cfg = hash_roll::ram::Ram::with_w(self.block_size as u64);

        split(cfg, file, cb)
    }
}

fn split(cfg: impl ToChunkIncr, mut file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
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
