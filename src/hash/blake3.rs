use super::*;

pub struct Blake3Hasher {}

impl HasherFactory for Blake3Hasher {
    type Type = Blake3Hasher;

    fn name() -> &'static str {
        "Blake3"
    }

    fn new() -> Self::Type {
        Blake3Hasher {}
    }
}

impl Hasher for Blake3Hasher {
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        let hash = ::blake3::hash(data);
        Vec::from(*hash.as_bytes())
    }
}
