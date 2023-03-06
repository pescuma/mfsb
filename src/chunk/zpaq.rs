use super::*;
use anyhow::Result;
use std::fs;

pub struct ZPAQ {
    nbits: usize,
    block_max: usize,
}

impl ChunkerFactory for ZPAQ {
    type Type = ZPAQ;

    fn name() -> &'static str {
        "ZPAQ"
    }

    fn new(block_size: u32) -> Self::Type {
        ZPAQ {
            nbits: (block_size as f32).log2().ceil() as usize,
            block_max: (block_size * 2) as usize,
        }
    }
}

impl Chunker for ZPAQ {
    fn get_max_block_size(&self) -> u32 {
        self.block_max as u32
    }

    fn split(&self, file: fs::File, cb: &mut dyn FnMut(Vec<u8>)) -> Result<()> {
        let chunker = cdchunking::Chunker::new(cdchunking::ZPAQ::new(self.nbits)) //
            .max_size(self.block_max);

        for chunk in chunker.whole_chunks(file) {
            let chunk = chunk?;

            cb(chunk);
        }

        Ok(())
    }
}
