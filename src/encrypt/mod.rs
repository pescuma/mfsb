use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use argon2::Argon2;

mod ring_crypto;
mod rust_crypto;

pub struct Encryptor {
    name: &'static str,
    et: EncryptorType,
    inner: Box<dyn EncryptorImpl>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
pub enum EncryptorType {
    ChaCha20Poly1305_sw,
    ChaCha20Poly1305,
    AES_256_GCM,
    AES_128_GCM,
}

trait EncryptorImpl: Send + Sync {
    fn get_extra_space_needed(&self) -> u32;
    fn encrypt(&self, data: Vec<u8>) -> Result<Vec<u8>>;
    fn decrypt(&self, data: Vec<u8>) -> Result<Vec<u8>>;
}

impl Encryptor {
    pub fn list_available_names() -> Vec<&'static str> {
        return REGISTERED.keys().map(|k| *k).collect();
    }

    pub fn build_by_name(name: &str, password: &str) -> Result<Arc<Encryptor>> {
        let factory = REGISTERED
            .get(name)
            .with_context(|| format!("unknown encryptor: '{}'", name))?;

        factory(password)
    }

    fn build<T>(
        name: &'static str,
        et: EncryptorType,
        password: &str,
        factory: impl Fn([u8; 32]) -> T,
    ) -> Result<Arc<Self>>
    where
        T: EncryptorImpl + 'static,
    {
        let salt = b"mfsb salt";

        let mut key = [0u8; 32];
        Argon2::default().hash_password_into(password.as_bytes(), salt, &mut key)?;

        Ok(Arc::new(Self {
            name,
            et,
            inner: Box::new(factory(key)),
        }))
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_type(&self) -> EncryptorType {
        self.et
    }

    pub fn get_extra_space_needed(&self) -> u32 {
        self.inner.get_extra_space_needed()
    }

    pub fn encrypt(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        self.inner.encrypt(data)
    }

    pub fn decrypt(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        self.inner.decrypt(data)
    }
}

type Factory = Box<dyn Fn(&str) -> Result<Arc<Encryptor>> + Send + Sync>;

lazy_static! {
    static ref REGISTERED: HashMap<&'static str, Factory> = create_encryptors();
}

fn create_encryptors() -> HashMap<&'static str, Factory> {
    let mut by_name = HashMap::new();

    macro_rules! register {
        ($n:expr, $t:expr,  $f:expr) => {
            let factory: Factory = Box::new(|password| Encryptor::build($n, $t, password, $f));
            by_name.insert($n, factory);
        };
    }

    use EncryptorType::*;

    register!("ChaCha20Poly1305 (sw)", ChaCha20Poly1305_sw, |key| rust_crypto::ChaCha20Poly1305Encryptor::new(key));
    register!("ChaCha20Poly1305", ChaCha20Poly1305, |key| ring_crypto::RingEncryptor::new_chacha20_poly1305(key));
    register!("AES 256 GCM", AES_256_GCM, |key| ring_crypto::RingEncryptor::new_aes_256_gcm(key));
    register!("AES 128 GCM", AES_128_GCM, |key| ring_crypto::RingEncryptor::new_aes_128_gcm(key));

    by_name
}
