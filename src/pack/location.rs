pub struct PackLocation {
    pub hash: Vec<u8>,
    pub start: u64,
    pub size: u64,
}

impl PackLocation {
    pub fn new(hash: Vec<u8>, start: u64, size: u64) -> PackLocation {
        PackLocation { hash, start, size }
    }
}
