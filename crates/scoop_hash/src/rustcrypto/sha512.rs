use sha2::Digest;
use sha2::Sha512;

pub struct WrappedSha512 {
    pub inner: Sha512,
}

impl WrappedSha512 {
    pub fn new() -> Self {
        Self {
            inner: Sha512::new(),
        }
    }
}
