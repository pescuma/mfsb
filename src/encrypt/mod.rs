mod ring_crypto;
mod rust_crypto;

use anyhow::{Context, Result};
use argon2::Argon2;
use std::collections::HashMap;
use std::sync::Arc;

pub trait Encryptor: Send + Sync {
    fn get_extra_space_needed(&self) -> u32;
    fn encrypt(&self, data: Vec<u8>) -> Result<Vec<u8>>;
    fn decrypt(&self, data: Vec<u8>) -> Result<Vec<u8>>;
}

pub type EncryptorFactoryMap = HashMap<&'static str, Box<dyn Fn([u8; 32]) -> Arc<dyn Encryptor>>>;

pub fn list_available() -> EncryptorFactoryMap {
    let mut result: EncryptorFactoryMap = HashMap::new();
    macro_rules! lazy {
        ($f:expr) => {
            Box::new(|key| {
                let result = ($f)(key);
                Arc::new(result)
            })
        };
    }

    result.insert(
        "ChaCha20Poly1305 (sw)",
        lazy!(|key| rust_crypto::ChaCha20Poly1305Encryptor::new(key)),
    );
    result.insert(
        "ChaCha20Poly1305",
        lazy!(|key| ring_crypto::RingEncryptor::new_chacha20_poly1305(key)),
    );
    result.insert(
        "AES 256 GCM",
        lazy!(|key| ring_crypto::RingEncryptor::new_aes_256_gcm(key)),
    );
    result.insert(
        "AES 128 GCM",
        lazy!(|key| ring_crypto::RingEncryptor::new_aes_128_gcm(key)),
    );

    return result;
}

pub fn new(name: &str, password: &str) -> Result<Arc<dyn Encryptor>> {
    let available = list_available();

    let factory = available
        .get(name)
        .with_context(|| format!("unknown encryptor: '{}'", name))?;

    let salt = b"mfsb salt";

    let mut key = [0u8; 32];
    Argon2::default().hash_password_into(password.as_bytes(), salt, &mut key)?;

    Ok(factory(key))
}

pub fn encrypt(encryptor: &dyn Encryptor, data: Vec<u8>) -> Result<Vec<u8>> {
    encryptor.encrypt(data)
}

pub fn decrypt(encryptor: &dyn Encryptor, data: Vec<u8>) -> Result<Vec<u8>> {
    encryptor.decrypt(data)
}
