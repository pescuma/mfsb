use std::fs;

use anyhow::{Context, Result};

use super::*;

pub struct FastCDC2020Mmap {
    block_min: u32,
    block_avg: u32,
    block_max: u32,
}

impl FastCDC2020Mmap {
    pub fn new(block_size: u32) -> Self {
        FastCDC2020Mmap {
            block_min: (block_size * 9 / 10) as u32,
            block_avg: block_size as u32,
            block_max: (block_size * 2) as u32,
        }
    }
}

impl ChunkerImpl for FastCDC2020Mmap {
    fn get_max_block_size(&self) -> u32 {
        self.block_max
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let mmap = unsafe { memmap2::Mmap::map(&file).context("failed to mmap")? };

        let chunker = ::fastcdc::v2020::FastCDC::new(
            &mmap[..], //
            self.block_min,
            self.block_avg,
            self.block_max,
        );

        for chunk in chunker {
            cb(Vec::from(&mmap[chunk.offset..chunk.offset + chunk.length]));
        }

        Ok(())
    }
}
