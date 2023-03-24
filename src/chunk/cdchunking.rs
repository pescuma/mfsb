use std::fs;

use anyhow::Result;

use super::*;

pub struct ZPAQ {
    block_max: u32,
    nbits: usize,
}

impl ZPAQ {
    pub fn new(block_size: u32) -> Self {
        ZPAQ {
            block_max: block_size * 2,
            nbits: (block_size as f32).log2().ceil() as usize,
        }
    }
}

impl ChunkerImpl for ZPAQ {
    fn get_max_block_size(&self) -> u32 {
        self.block_max
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let chunker = ::cdchunking::Chunker::new(::cdchunking::ZPAQ::new(self.nbits)) //
            .max_size(self.block_max as usize);

        for chunk in chunker.whole_chunks(file) {
            let chunk = chunk?;

            cb(chunk);
        }

        Ok(())
    }
}
