use sha2::Digest;
use sha2::Sha256;
use std::cell::OnceCell;

pub struct WrappedSha256 {
    pub inner: OnceCell<Sha256>,
}

impl WrappedSha256 {
    pub fn new() -> Self {
        let inner = OnceCell::new();
        inner.set(Sha256::new()).unwrap();

        Self { inner }
    }
}
