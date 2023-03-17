use super::*;
use ::ring::aead;
use rand::RngCore;

pub struct RingEncryptor {
    algo: &'static aead::Algorithm,
    key: aead::LessSafeKey,
}

impl RingEncryptor {
    pub fn new_chacha20_poly1305(key: [u8; 32]) -> Self {
        Self::new(&aead::CHACHA20_POLY1305, key)
    }

    pub fn new_aes_256_gcm(key: [u8; 32]) -> Self {
        Self::new(&aead::AES_256_GCM, key)
    }

    pub fn new_aes_128_gcm(key: [u8; 32]) -> Self {
        Self::new(&aead::AES_128_GCM, key)
    }

    fn new(algo: &'static aead::Algorithm, key: [u8; 32]) -> Self {
        let key = aead::UnboundKey::new(algo, &key[..algo.key_len()]).unwrap();
        let key = aead::LessSafeKey::new(key);

        RingEncryptor { algo, key }
    }

    fn create_nonce() -> aead::Nonce {
        let mut rand_generator = rand::rngs::OsRng::default();

        let mut nonce = [0u8; 96 / 8];
        rand_generator.fill_bytes(&mut nonce);

        aead::Nonce::assume_unique_for_key(nonce)
    }

    fn create_ad() -> aead::Aad<[u8; 0]> {
        let ad = [0u8; 0];
        let ring_ad = aead::Aad::from(ad);
        ring_ad
    }
}

impl Encryptor for RingEncryptor {
    fn get_extra_space_needed(&self) -> u32 {
        self.algo.tag_len() as u32
    }

    fn encrypt(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
        let nonce = Self::create_nonce();
        let ad = Self::create_ad();

        self.key.seal_in_place_append_tag(nonce, ad, &mut data)?;

        Ok(data)
    }

    fn decrypt(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
        let nonce = Self::create_nonce();
        let ad = Self::create_ad();

        self.key.open_in_place(nonce, ad, &mut data)?;

        Ok(data)
    }
}
