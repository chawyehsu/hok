use sha1::Digest;
use sha1::Sha1;
use std::cell::OnceCell;

pub struct WrappedSha1 {
    pub inner: OnceCell<Sha1>,
}

impl WrappedSha1 {
    pub fn new() -> Self {
        let inner = OnceCell::new();
        inner.set(Sha1::new()).unwrap();

        Self { inner }
    }
}
