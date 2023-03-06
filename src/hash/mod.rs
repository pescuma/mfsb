mod blake3;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;

pub trait HasherFactory {
    type Type;
    fn name() -> &'static str;
    fn new() -> Self::Type;
}

pub trait Hasher: Send + Sync {
    fn hash(&self, data: &[u8]) -> Vec<u8>;
}

pub type HasherFactoryMap = HashMap<&'static str, Box<dyn Fn() -> Arc<dyn Hasher>>>;

pub fn list_available() -> HasherFactoryMap {
    let mut result: HasherFactoryMap = HashMap::new();
    macro_rules! add {
        ($F:ty) => {
            result.insert(<$F>::name(), Box::new(|| Arc::new(<$F>::new())));
        };
    }

    add!(blake3::Blake3Hasher);

    return result;
}

pub fn new(name: &str) -> Result<Arc<dyn Hasher>> {
    let available = list_available();

    let factory = available
        .get(name)
        .with_context(|| format!("unknown hasher: '{}'", name))?;

    Ok(factory())
}

pub fn hash(hasher: &dyn Hasher, data: &[u8]) -> Vec<u8> {
    hasher.hash(data)
}
