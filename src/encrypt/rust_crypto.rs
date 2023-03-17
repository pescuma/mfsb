use super::*;
use ::chacha20poly1305::aead::AeadCore;
use ::chacha20poly1305::aead::KeyInit;
use ::chacha20poly1305::aead::OsRng;
use ::chacha20poly1305::AeadInPlace;
use ::chacha20poly1305::ChaCha20Poly1305;
use generic_array::typenum::Unsigned;
use generic_array::GenericArray;

pub struct ChaCha20Poly1305Encryptor {
    pub cipher: ChaCha20Poly1305,
}

impl ChaCha20Poly1305Encryptor {
    pub fn new(key: [u8; 32]) -> ChaCha20Poly1305Encryptor {
        let cipher = ChaCha20Poly1305::new(&GenericArray::from(key));

        ChaCha20Poly1305Encryptor { cipher }
    }
}

impl Encryptor for ChaCha20Poly1305Encryptor {
    fn get_extra_space_needed(&self) -> u32 {
        <ChaCha20Poly1305 as AeadCore>::TagSize::to_u32()
    }

    fn encrypt(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ad = [0u8; 0];

        self.cipher.encrypt_in_place(&nonce, &ad, &mut data)?;

        Ok(data)
    }

    fn decrypt(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ad = [0u8; 0];

        self.cipher.decrypt_in_place(&nonce, &ad, &mut data)?;

        Ok(data)
    }
}
