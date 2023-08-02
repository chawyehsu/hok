use sha1::Digest;
use sha1::Sha1;

pub struct WrappedSha1 {
    pub inner: Sha1,
}

impl WrappedSha1 {
    pub fn new() -> Self {
        Self { inner: Sha1::new() }
    }
}
