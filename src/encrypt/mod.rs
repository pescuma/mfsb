mod chacha20poly1305_rustcrypto;
mod ring;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;

pub trait EncryptorFactory {
    type Type;
    fn name() -> &'static str;
    fn new(password: &str) -> Result<Self::Type>;
}

pub trait Encryptor: Send + Sync {
    fn get_extra_space_needed(&self) -> u32;
    fn encrypt(&self, data: Vec<u8>) -> Result<Vec<u8>>;
    fn decrypt(&self, data: Vec<u8>) -> Result<Vec<u8>>;
}

pub type EncryptorFactoryMap =
    HashMap<&'static str, Box<dyn Fn(&str) -> Result<Arc<dyn Encryptor>>>>;

pub fn list_available() -> EncryptorFactoryMap {
    let mut result: EncryptorFactoryMap = HashMap::new();
    macro_rules! add {
        ($F:ty) => {
            result.insert(
                <$F>::name(),
                Box::new(|password| {
                    let result = <$F>::new(password)?;
                    Ok(Arc::new(result))
                }),
            );
        };
    }

    add!(chacha20poly1305_rustcrypto::ChaCha20Poly1305Encryptor);
    add!(ring::ChaCha20Poly1305RingEncryptor);
    add!(ring::Aes256GcmRingEncryptor);
    add!(ring::Aes128GcmRingEncryptor);

    return result;
}

pub fn new(name: &str, password: &str) -> Result<Arc<dyn Encryptor>> {
    let available = list_available();

    let factory = available
        .get(name)
        .with_context(|| format!("unknown encryptor: '{}'", name))?;

    factory(password)
}

pub fn encrypt(encryptor: &dyn Encryptor, data: Vec<u8>) -> Result<Vec<u8>> {
    encryptor.encrypt(data)
}

pub fn decrypt(encryptor: &dyn Encryptor, data: Vec<u8>) -> Result<Vec<u8>> {
    encryptor.decrypt(data)
}
