use std::fs;
use std::io::Read;

use anyhow::Result;
use cdc::RollingHash64;

use super::*;

pub struct Rabin {
    block_min: usize,
    block_max: usize,
    mmap: bool,
}

impl Rabin {
    pub fn new(block_size: u32, mmap: bool) -> Self {
        Rabin {
            block_min: (block_size * 9 / 10) as usize,
            block_max: (block_size * 2) as usize,
            mmap,
        }
    }

    fn split_mem(&self, mut file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let mut buffer = Vec::with_capacity(self.block_max);

        let mut rabin = cdc::Rabin64::new(6);

        loop {
            let read_len = (&mut file)
                .take((self.block_max - buffer.len()) as u64)
                .read_to_end(&mut buffer)?;

            if read_len == 0 {
                break;
            }
            if buffer.len() <= self.block_min {
                break;
            }

            let mut chunk_len = self.block_min;
            rabin.reset_and_prefill_window(&mut buffer[chunk_len - 64..chunk_len].iter().copied());
            while !predicate(rabin.hash) && chunk_len < buffer.len() {
                rabin.slide(&buffer[chunk_len]);
                chunk_len += 1;
            }

            cb(Vec::from(&buffer[..chunk_len]));

            if chunk_len < buffer.len() {
                buffer.copy_within(chunk_len.., 0);
            }
            buffer.truncate(buffer.len() - chunk_len);
        }

        if buffer.len() > 0 {
            cb(buffer);
        }

        Ok(())
    }

    fn split_mmap(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let mmap = unsafe { memmap2::Mmap::map(&file).context("failed to mmap")? };

        let mut rabin = cdc::Rabin64::new(6);

        let mut pos: usize = 0;
        while mmap.len() - pos > self.block_min {
            let mut chunk_len = self.block_min;

            rabin.reset_and_prefill_window(&mut mmap[pos + chunk_len - 64..pos + chunk_len].iter().copied());

            while !predicate(rabin.hash) && chunk_len < self.block_max && pos + chunk_len < mmap.len() {
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

impl ChunkerImpl for Rabin {
    fn get_max_block_size(&self) -> u32 {
        self.block_max as u32
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        if self.mmap {
            self.split_mmap(file, cb)
        } else {
            self.split_mem(file, cb)
        }
    }
}

#[inline]
fn predicate(x: u64) -> bool {
    const BITMASK: u64 = (1u64 << 13) - 1;
    x & BITMASK == BITMASK
}
