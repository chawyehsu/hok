use sha2::Digest;
use sha2::Sha256;

pub struct WrappedSha256 {
    pub inner: Sha256,
}

impl WrappedSha256 {
    pub fn new() -> Self {
        Self {
            inner: Sha256::new(),
        }
    }
}
