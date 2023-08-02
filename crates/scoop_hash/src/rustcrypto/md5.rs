use md5::Digest;
use md5::Md5;

pub struct WrappedMd5 {
    pub inner: Md5,
}

impl WrappedMd5 {
    pub fn new() -> Self {
        Self { inner: Md5::new() }
    }
}
