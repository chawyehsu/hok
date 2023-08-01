use md5::Digest;
use md5::Md5;
use std::cell::OnceCell;

pub struct WrappedMd5 {
    pub inner: OnceCell<Md5>,
}

impl WrappedMd5 {
    pub fn new() -> Self {
        let inner = OnceCell::new();
        inner.set(Md5::new()).unwrap();

        Self { inner }
    }
}
