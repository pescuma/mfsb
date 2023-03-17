use super::*;

pub struct Blake3Hasher {}

impl Blake3Hasher {
    pub fn new() -> Self {
        Blake3Hasher {}
    }
}

impl Hasher for Blake3Hasher {
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        let hash = ::blake3::hash(data);
        Vec::from(*hash.as_bytes())
    }
}
