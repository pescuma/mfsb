use ::digest::Digest;

use super::*;

pub struct Hasher<T>
where
    T: Digest + Send + Sync,
{
    _marker: std::marker::PhantomData<T>,
}

impl<T> Hasher<T>
where
    T: Digest + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> HasherImpl for Hasher<T>
where
    T: Digest + Send + Sync,
{
    fn hash(&self, data: &[u8]) -> Vec<u8> {
        let hash = T::digest(data);
        hash.to_vec()
    }
}

#[allow(non_camel_case_types)]
pub type Blake2s_256_Hasher = Hasher<::blake2::Blake2s256>;

#[allow(non_camel_case_types)]
pub type Blake2b_512_Hasher = Hasher<::blake2::Blake2b512>;

#[allow(non_camel_case_types)]
pub type Sha2_256_Hasher = Hasher<::sha2::Sha256>;

#[allow(non_camel_case_types)]
pub type Sha2_512_Hasher = Hasher<::sha2::Sha512>;

#[allow(non_camel_case_types)]
pub type Sha3_256_Hasher = Hasher<::sha3::Sha3_256>;

#[allow(non_camel_case_types)]
pub type Sha3_512_Hasher = Hasher<::sha3::Sha3_512>;

pub type TigerHasher = Hasher<::tiger::Tiger>;

pub type WhirlpoolHasher = Hasher<::whirlpool::Whirlpool>;
