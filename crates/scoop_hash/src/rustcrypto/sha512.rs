use sha2::Digest;
use sha2::Sha512;
use std::cell::OnceCell;

pub struct WrappedSha512 {
    pub inner: OnceCell<Sha512>,
}

impl WrappedSha512 {
    pub fn new() -> Self {
        let inner = OnceCell::new();
        inner.set(Sha512::new()).unwrap();

        Self { inner }
    }
}
