use std::cmp::min;

use ::secded::SecDed64;
use ::secded::SecDedCodec;
pub use anyhow::{Error, Result};
use bitbuffer::{BigEndian, BitReadBuffer, BitReadStream, BitWriteStream, LittleEndian};
use divrem::*;

use super::*;

pub struct Impl {
    sd: SecDed64,
    input_block_bits: usize,
    code_block_bits: usize,
    output_block_bits: usize,
}

impl Impl {
    pub fn new() -> Self {
        let sd = SecDed64::new(57);
        let input_block_bits = sd.encodable_size();
        let code_block_bits = sd.code_size();
        let output_block_bits = input_block_bits + code_block_bits;
        Self {
            sd,
            input_block_bits,
            code_block_bits,
            output_block_bits,
        }
    }

    fn get_output_size_bytes(&self, input_bytes: usize) -> usize {
        let input_bits = 4 + input_bytes * 8;
        let output_bits = input_bits.div_ceil(self.input_block_bits) * self.output_block_bits;
        return output_bits.div_ceil(8);
    }

    fn get_input_size_bytes(&self, output_bytes: usize) -> usize {
        let output_bits = output_bytes * 8;
        let input_bits = output_bits.div_ceil(self.output_block_bits) * self.input_block_bits;
        return (input_bits - 4).div_ceil(8);
    }

    fn encode(&self, mut data: u64) -> u64 {
        data = data << self.code_block_bits;

        let mut buffer = data.to_be_bytes();
        self.sd.encode(&mut buffer);

        let result = u64::from_be_bytes(buffer) << 1;

        result
    }

    fn decode(&self, mut data: u64) -> Result<u64> {
        data = data >> 1;

        let mut buffer = data.to_be_bytes();
        self.sd
            .decode(&mut buffer)
            .map_err(|_| Error::msg("Can't read data: Too many errors detected (SECDED)"))?;

        let result = u64::from_be_bytes(buffer) >> self.code_block_bits;

        Ok(result)
    }
}

impl ECCImpl for Impl {
    fn write(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        let mut data_reader = BitReader::new(&data);

        let mut result = Vec::with_capacity(self.get_output_size_bytes(data.len()));
        let mut result_writer = BitWriter::new(&mut result);

        let mut v = (data.len() as u64) << (32 - self.code_block_bits);
        v = v | data_reader.read_u64(self.input_block_bits - 32)?.0;
        result_writer.write_u64(self.encode(v), self.output_block_bits)?;

        loop {
            let (mut v, s) = data_reader.read_u64(self.input_block_bits)?;
            if s == 0 {
                break;
            }

            result_writer.write_u64(self.encode(v), self.output_block_bits)?;
        }

        Ok(result)
    }

    fn read(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        let mut data_reader = BitReader::new(&data);

        let mut result = Vec::with_capacity(self.get_input_size_bytes(data.len()));
        let mut result_writer = BitWriter::new(&mut result);

        let (mut v, size) = data_reader.read_u64(self.output_block_bits)?;
        anyhow::ensure!(size > 32, "input too small");

        v = self.decode(v)?;
        let result_len = v >> (32 - self.code_block_bits);
        result_writer.write_u64(v, self.input_block_bits - 32)?;

        loop {
            let (mut v, s) = data_reader.read_u64(self.output_block_bits)?;
            if s == 0 {
                break;
            }

            result_writer.write_u64(self.decode(v)?, self.input_block_bits)?;
        }

        result.resize(result_len as usize, 0);

        Ok(result)
    }
}

struct BitReader<'a> {
    available_bits: usize,
    stream: BitReadStream<'a, LittleEndian>,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        let stream = BitReadStream::new(BitReadBuffer::new(data, LittleEndian));

        Self {
            available_bits: data.len() * 8,
            stream,
        }
    }

    pub fn read_u64(&mut self, read_bits: usize) -> Result<(u64, usize)> {
        let to_read = min(read_bits, self.available_bits);

        if to_read == 0 {
            return Ok((0, 0));
        }

        let result = self.stream.read_int::<u64>(to_read)?;
        self.available_bits -= to_read;

        Ok((result, to_read))
    }
}

struct BitWriter<'a> {
    stream: BitWriteStream<'a, LittleEndian>,
}

impl<'a> BitWriter<'a> {
    pub fn new(data: &'a mut Vec<u8>) -> Self {
        let stream = BitWriteStream::new(data, LittleEndian);

        Self { stream }
    }

    pub fn write_u64(&mut self, data: u64, write_bits: usize) -> Result<()> {
        self.stream.write_int(data, write_bits)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::ecc::secded::Impl;
    use crate::ecc::ECCImpl;

    #[test]
    fn round_trips_255() {
        let mut v = Vec::new();
        for i in 1..=100 {
            v.push(255);
            round_trip_test(v.clone());
        }
    }

    #[test]
    fn round_trips_growing() {
        let mut v = Vec::new();
        for i in 1..=100 {
            v.push((i % 256) as u8);
            round_trip_test(v.clone());
        }
    }

    fn round_trip_test(orig: Vec<u8>) {
        let i = Impl::new();

        let ecc = i.write(orig.clone());
        assert_eq!(false, ecc.is_err());

        let back = i.read(ecc.unwrap());
        assert_eq!(false, back.is_err());

        assert_eq!(orig, back.unwrap());
    }
}
