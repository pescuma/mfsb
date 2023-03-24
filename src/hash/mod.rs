use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use anyhow::{Context, Result};

mod blake3;
mod digest;

pub struct Hasher {
    name: &'static str,
    ht: HasherType,
    inner: Box<dyn HasherImpl>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
pub enum HasherType {
    Blake2s_256,
    Blake2b_512,
    Blake3,
    Sha2_256,
    Sha2_512,
    Sha3_256,
    Sha3_512,
    Tiger,
    Whirlpool,
}

trait HasherImpl: Send + Sync {
    fn hash(&self, data: &[u8]) -> Vec<u8>;
}

impl Hasher {
    pub fn list_available_names() -> Vec<&'static str> {
        return REGISTERED.keys().map(|k| *k).collect();
    }

    pub fn build_by_name(name: &str) -> Result<Arc<Hasher>> {
        let factory = REGISTERED
            .get(name)
            .with_context(|| format!("unknown hasher: '{}'", name))?;

        Ok(factory())
    }

    fn new(name: &'static str, ct: HasherType, inner: Box<dyn HasherImpl>) -> Self {
        Self {
            name,
            ht: ct,
            inner,
        }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_type(&self) -> HasherType {
        self.ht
    }

    pub fn hash(&self, data: &[u8]) -> Vec<u8> {
        self.inner.hash(data)
    }
}

type Factory = Box<dyn Fn() -> Arc<Hasher> + Send + Sync>;

lazy_static! {
    static ref REGISTERED: HashMap<&'static str, Factory> = create_hashers();
}

fn create_hashers() -> HashMap<&'static str, Factory> {
    let mut by_name = HashMap::new();

    macro_rules! register {
        ($n:expr, $t:expr,  $f:expr) => {
            let factory: Factory = Box::new(|| Arc::new(Hasher::new($n, $t, Box::new($f))));
            by_name.insert($n, factory);
        };
    }

    use HasherType::*;

    register!(
        "Blake2s-256",
        Blake2s_256,
        digest::Blake2s_256_Hasher::new()
    );
    register!(
        "Blake2d-512",
        Blake2b_512,
        digest::Blake2b_512_Hasher::new()
    );
    register!("Blake3", Blake3, blake3::Blake3Hasher::new());
    register!("SHA-2-256", Sha2_256, digest::Sha2_256_Hasher::new());
    register!("SHA-2-512", Sha2_512, digest::Sha2_512_Hasher::new());
    register!("SHA-3-256", Sha3_256, digest::Sha3_256_Hasher::new());
    register!("SHA-3-512", Sha3_512, digest::Sha3_512_Hasher::new());
    register!("Tiger", Tiger, digest::TigerHasher::new());
    register!("Whirlpool", Whirlpool, digest::WhirlpoolHasher::new());

    by_name
}
