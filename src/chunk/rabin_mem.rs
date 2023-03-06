use super::*;
use anyhow::Result;
use cdc::RollingHash64;
use std::fs;
use std::io::Read;

pub struct RabinMen {
    block_min: usize,
    block_max: usize,
}

impl ChunkerFactory for RabinMen {
    type Type = RabinMen;

    fn name() -> &'static str {
        "Rabin64"
    }

    fn new(block_size: u32) -> Self::Type {
        RabinMen {
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

impl Chunker for RabinMen {
    fn get_max_block_size(&self) -> u32 {
        self.block_max as u32
    }

    fn split(&self, mut file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
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
}
