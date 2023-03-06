use super::*;
use anyhow::{Context, Result};
use cdc::RollingHash64;
use std::fs;

pub struct RabinMmap {
    block_min: usize,
    block_max: usize,
}

impl ChunkerFactory for RabinMmap {
    type Type = RabinMmap;

    fn name() -> &'static str {
        "Rabin64 (mmap)"
    }

    fn new(block_size: u32) -> Self::Type {
        RabinMmap {
            block_min: (block_size * 9 / 10) as usize,
            block_max: (block_size * 2) as usize,
        }
    }
}

#[inline]
fn predicate(x: u64) -> bool {
    const BITMASK: u64 = (1u64 << 13) - 1;
    x & BITMASK == BITMASK
}

impl Chunker for RabinMmap {
    fn get_max_block_size(&self) -> u32 {
        self.block_max as u32
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let mmap = unsafe { memmap2::Mmap::map(&file).context("failed to mmap")? };

        let mut rabin = cdc::Rabin64::new(6);

        let mut pos: usize = 0;
        while mmap.len() - pos > self.block_min {
            let mut chunk_len = self.block_min;

            rabin.reset_and_prefill_window(
                &mut mmap[pos + chunk_len - 64..pos + chunk_len].iter().copied(),
            );

            while !predicate(rabin.hash)
                && chunk_len < self.block_max
                && pos + chunk_len < mmap.len()
            {
                rabin.slide(&mmap[pos + chunk_len]);
                chunk_len += 1;
            }

            cb(Vec::from(&mmap[pos..pos + chunk_len]));
            pos += chunk_len;
        }

        if pos < mmap.len() {
            cb(Vec::from(&mmap[pos..]));
        }

        Ok(())
    }
}
