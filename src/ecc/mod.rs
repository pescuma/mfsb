use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};

mod secded;

pub struct ECC {
    name: &'static str,
    et: ECCType,
    inner: Box<dyn ECCImpl>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
pub enum ECCType {
    NONE,
    SECDED,
}

trait ECCImpl: Send + Sync {
    fn write(&self, data: Vec<u8>) -> Result<Vec<u8>>;
    fn read(&self, data: Vec<u8>) -> Result<Vec<u8>>;
}

impl ECC {
    pub fn list_available_names(include_none: bool) -> Vec<&'static str> {
        return REGISTERED
            .keys()
            .map(|k| *k)
            .filter(|k| include_none || *k != "None")
            .collect();
    }

    pub fn build_by_name(name: &str) -> Result<Arc<ECC>> {
        let factory = REGISTERED
            .get(name)
            .with_context(|| format!("unknown encryptor: '{}'", name))?;

        Ok(factory())
    }

    fn new(name: &'static str, et: ECCType, inner: Box<dyn ECCImpl>) -> Self {
        Self { name, et, inner }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_type(&self) -> ECCType {
        self.et
    }

    pub fn write(&self, data: Vec<u8>) -> Result<(ECCType, Vec<u8>)> {
        let result = self.inner.write(data)?;
        Ok((self.et, result))
    }

    pub fn read(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        self.inner.read(data)
    }
}

type Factory = Box<dyn Fn() -> Arc<ECC> + Send + Sync>;

lazy_static! {
    static ref REGISTERED: HashMap<&'static str, Factory> = create_encryptors();
}

fn create_encryptors() -> HashMap<&'static str, Factory> {
    let mut by_name = HashMap::new();

    macro_rules! register {
        ($n:expr, $t:expr,  $f:expr) => {
            let factory: Factory = Box::new(|| Arc::new(ECC::new($n, $t, Box::new($f))));
            by_name.insert($n, factory);
        };
    }

    use ECCType::*;

    register!("None", NONE, NoneECC::new());
    register!("SECDED", SECDED, secded::Impl::new());

    by_name
}

struct NoneECC {}

impl NoneECC {
    fn new() -> Self {
        Self {}
    }
}

impl ECCImpl for NoneECC {
    fn write(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        Ok(data)
    }

    fn read(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        Ok(data)
    }
}
