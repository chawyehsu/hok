use std::cell::OnceCell;

#[cfg(feature = "rustcrypto")]
mod rustcrypto;
#[cfg(feature = "rustcrypto")]
use rustcrypto::{Digest, Md5, Sha1, Sha256, Sha512};

#[cfg(not(feature = "rustcrypto"))]
mod selfcontained;
#[cfg(not(feature = "rustcrypto"))]
use selfcontained::{Md5, Sha1, Sha256, Sha512};

trait Hasher {
    fn hash_type(&self) -> String;
    fn update(&mut self, data: &[u8]);
    fn sum(&mut self) -> String;
}

macro_rules! impl_hasher_for {
    ($hasher:ty) => {
        impl Hasher for $hasher {
            fn hash_type(&self) -> String {
                stringify!($hasher).to_string()
            }

            fn update(&mut self, data: &[u8]) {
                #[cfg(not(feature = "rustcrypto"))]
                self.consume(data);

                #[cfg(feature = "rustcrypto")]
                self.inner.get_mut().unwrap().update(data);
            }

            fn sum(&mut self) -> String {
                #[cfg(not(feature = "rustcrypto"))]
                let ret = self.result_string();

                #[cfg(feature = "rustcrypto")]
                let ret = format!("{:x}", self.inner.take().unwrap().finalize());

                ret
            }
        }
    };
}

impl_hasher_for!(Md5);
impl_hasher_for!(Sha1);
impl_hasher_for!(Sha256);
impl_hasher_for!(Sha512);

/// Error is returned when the hash type is not supported.
#[derive(Debug)]
pub struct Error;

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported hash type")
    }
}
/// Checksum is a wrapper around a hash algorithm.
#[derive(Debug)]
pub struct Checksum {
    /// The hash algorithm.
    hasher: Box<dyn Hasher>,

    /// The input hash.
    input_hash: String,

    /// The computed hash.
    output_hash: OnceCell<String>,
}

impl std::fmt::Debug for dyn Hasher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hasher {{ hash_type: {} }}", self.hash_type())
    }
}

impl Checksum {
    /// Creates a new Checksum instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scoop_hash::Checksum;
    /// let mut checksum = Checksum::new("sha256:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9").expect("invalid input hash");
    /// checksum.consume(b"hello world");
    /// assert!(checksum.check());
    /// assert_eq!(checksum.result(), "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    /// ```
    pub fn new<S: AsRef<str>>(hash: S) -> Result<Checksum, Error> {
        let hash = hash.as_ref().to_lowercase();
        let (method, input_hash) = hash.split_once(':').unwrap_or(("sha256", &hash));
        let input_hash = input_hash.to_string();
        let hasher: Box<dyn Hasher> = match method {
            "md5" => Box::new(Md5::new()),
            "sha1" => Box::new(Sha1::new()),
            "sha256" => Box::new(Sha256::new()),
            "sha512" => Box::new(Sha512::new()),
            _ => return Err(Error),
        };

        let output_hash = OnceCell::new();

        Ok(Checksum {
            hasher,
            input_hash,
            output_hash,
        })
    }

    /// Consumes the provided data.
    ///
    /// Note that no data can be consumed after the result has been computed.
    #[inline]
    pub fn consume(&mut self, data: &[u8]) {
        if self.output_hash.get().is_some() {
            return;
        }

        self.hasher.update(data);
    }

    /// Gets the result of the hash computation as a hex string.
    #[inline]
    pub fn result(&mut self) -> &str {
        self.output_hash.get_or_init(|| self.hasher.sum())
    }

    /// Checks if the result of the hash computation matches the input hash.
    #[inline]
    pub fn check(&mut self) -> bool {
        let left = self.input_hash.to_owned();
        let right = self.result();
        left == right
    }
}
