use super::*;
use ::ring::aead;
use ::ring::aead::*;
use argon2::Argon2;
use rand::RngCore;

pub struct RingEncryptor {
    algo: &'static aead::Algorithm,
    key: LessSafeKey,
}

pub struct ChaCha20Poly1305RingEncryptor {}

impl EncryptorFactory for ChaCha20Poly1305RingEncryptor {
    type Type = RingEncryptor;

    fn name() -> &'static str {
        "ChaCha20Poly1305"
    }

    fn new(password: &str) -> Result<Self::Type> {
        RingEncryptor::new(&CHACHA20_POLY1305, password)
    }
}

pub struct Aes256GcmRingEncryptor {}

impl EncryptorFactory for Aes256GcmRingEncryptor {
    type Type = RingEncryptor;

    fn name() -> &'static str {
        "AES 256 GCM"
    }

    fn new(password: &str) -> Result<Self::Type> {
        RingEncryptor::new(&AES_256_GCM, password)
    }
}

pub struct Aes128GcmRingEncryptor {}

impl EncryptorFactory for Aes128GcmRingEncryptor {
    type Type = RingEncryptor;

    fn name() -> &'static str {
        "AES 128 GCM"
    }

    fn new(password: &str) -> Result<Self::Type> {
        RingEncryptor::new(&AES_128_GCM, password)
    }
}

impl RingEncryptor {
    fn new(algo: &'static aead::Algorithm, password: &str) -> Result<Self> {
        let salt = b"mfsb salt";

        let mut key = vec![0u8; algo.key_len()];
        Argon2::default().hash_password_into(password.as_bytes(), salt, &mut key)?;

        let key = UnboundKey::new(algo, &key)?;
        let key = LessSafeKey::new(key);

        Ok(RingEncryptor { algo, key })
    }

    fn create_nonce() -> Nonce {
        let mut rand_generator = rand::rngs::OsRng::default();

        let mut nonce = [0u8; 96 / 8];
        rand_generator.fill_bytes(&mut nonce);

        Nonce::assume_unique_for_key(nonce)
    }

    fn create_ad() -> Aad<[u8; 0]> {
        let ad = [0u8; 0];
        let ring_ad = Aad::from(ad);
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
