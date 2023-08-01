use core::cell::OnceCell;
use std::error::Error as StdError;

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

impl core::fmt::Debug for dyn Hasher {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Hasher {{ hash_type: {} }}", self.hash_type())
    }
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

#[derive(Debug)]
pub struct Error;

impl StdError for Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unsupported hash algorithm")
    }
}

pub struct ChecksumBuilder {
    hasher: OnceCell<Box<dyn Hasher>>,
}

impl ChecksumBuilder {
    /// Creates a new ChecksumBuilder instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scoop_hash::ChecksumBuilder;
    /// let mut md5 = ChecksumBuilder::new().md5().build();
    /// md5.consume(b"hello world");
    /// assert!(md5.check("5eb63bbbe01eeed093cb22bb8f5acdc3"));
    /// ```
    pub fn new() -> ChecksumBuilder {
        let hasher = OnceCell::new();

        ChecksumBuilder { hasher }
    }

    /// Use the specified hash algorithm.
    ///
    /// # Errors
    ///
    /// Returns an error if the specified algorithm is not supported.
    pub fn algo(&mut self, algo: &str) -> Result<&mut ChecksumBuilder, Error> {
        match algo {
            "md5" => Ok(self.md5()),
            "sha1" => Ok(self.sha1()),
            "sha256" => Ok(self.sha256()),
            "sha512" => Ok(self.sha512()),
            _ => Err(Error),
        }
    }

    /// Use the md5 hash algorithm.
    pub fn md5(&mut self) -> &mut ChecksumBuilder {
        let algo: Box<dyn Hasher> = Box::new(Md5::new());
        self.set_algo(algo)
    }

    /// Use the sha1 hash algorithm.
    pub fn sha1(&mut self) -> &mut ChecksumBuilder {
        let algo: Box<dyn Hasher> = Box::new(Sha1::new());
        self.set_algo(algo)
    }

    /// Use the sha256 hash algorithm.
    pub fn sha256(&mut self) -> &mut ChecksumBuilder {
        let algo: Box<dyn Hasher> = Box::new(Sha256::new());
        self.set_algo(algo)
    }

    /// Use the sha512 hash algorithm.
    pub fn sha512(&mut self) -> &mut ChecksumBuilder {
        let algo: Box<dyn Hasher> = Box::new(Sha512::new());
        self.set_algo(algo)
    }

    fn set_algo(&mut self, algo: Box<dyn Hasher>) -> &mut ChecksumBuilder {
        let _ = self.hasher.take();
        self.hasher.set(algo).unwrap();
        self
    }

    /// Build the Checksum instance for use.
    ///
    /// If no hash algorithm is specified, sha256 will be used.
    pub fn build(&mut self) -> Checksum {
        let output_hash = OnceCell::new();
        let hasher = self.hasher.take().unwrap_or_else(|| {
            let algo: Box<dyn Hasher> = Box::new(Sha256::new());
            algo
        });

        Checksum {
            hasher,
            output_hash,
        }
    }
}

/// Checksum is a wrapper around a hash algorithm.
#[derive(Debug)]
pub struct Checksum {
    /// The hash algorithm.
    hasher: Box<dyn Hasher>,

    /// The computed hash.
    output_hash: OnceCell<String>,
}

impl Checksum {
    /// Consumes the provided data.
    ///
    /// Note that no data can be consumed after the result has been computed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scoop_hash::ChecksumBuilder;
    /// let mut sha256 = ChecksumBuilder::new().build();
    /// sha256.consume(b"hello world");
    /// let result = sha256.result();
    /// assert_eq!(result, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    /// ```
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
    pub fn check(&mut self, input: &str) -> bool {
        input == self.result()
    }
}
