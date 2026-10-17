use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::error::Error as StdError;
use std::fmt;

/// A hash algorithm supported by Scoop manifests.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Algorithm {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

impl Algorithm {
    /// Returns the canonical lowercase name of this algorithm.
    pub fn as_str(&self) -> &'static str {
        match self {
            Algorithm::Md5 => "md5",
            Algorithm::Sha1 => "sha1",
            Algorithm::Sha256 => "sha256",
            Algorithm::Sha512 => "sha512",
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Algorithm {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "md5" => Ok(Algorithm::Md5),
            "sha1" => Ok(Algorithm::Sha1),
            "sha256" => Ok(Algorithm::Sha256),
            "sha512" => Ok(Algorithm::Sha512),
            _ => Err(Error::UnsupportedAlgorithm(s.to_string())),
        }
    }
}

/// Error raised when an unsupported hash algorithm is requested.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    UnsupportedAlgorithm(String),
}

impl StdError for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnsupportedAlgorithm(algo) => {
                write!(f, "unsupported hash algorithm: {}", algo)
            }
        }
    }
}

enum Hasher {
    Md5(Md5),
    Sha1(Sha1),
    Sha256(Sha256),
    Sha512(Sha512),
}

impl Hasher {
    fn new(algo: Algorithm) -> Hasher {
        match algo {
            Algorithm::Md5 => Hasher::Md5(Md5::default()),
            Algorithm::Sha1 => Hasher::Sha1(Sha1::default()),
            Algorithm::Sha256 => Hasher::Sha256(Sha256::default()),
            Algorithm::Sha512 => Hasher::Sha512(Sha512::default()),
        }
    }

    #[inline]
    fn update(&mut self, data: &[u8]) {
        match self {
            Hasher::Md5(h) => md5::Digest::update(h, data),
            Hasher::Sha1(h) => sha1::Digest::update(h, data),
            Hasher::Sha256(h) => sha2::Digest::update(h, data),
            Hasher::Sha512(h) => sha2::Digest::update(h, data),
        }
    }

    #[inline]
    fn finalize(self) -> String {
        match self {
            Hasher::Md5(h) => format!("{:x}", md5::Digest::finalize(h)),
            Hasher::Sha1(h) => format!("{:x}", sha1::Digest::finalize(h)),
            Hasher::Sha256(h) => format!("{:x}", sha2::Digest::finalize(h)),
            Hasher::Sha512(h) => format!("{:x}", sha2::Digest::finalize(h)),
        }
    }
}

/// ChecksumBuilder is used to create a Checksum instance.
pub struct ChecksumBuilder {
    hasher: Hasher,
}

impl Default for ChecksumBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ChecksumBuilder {
    /// Creates a new ChecksumBuilder instance.
    ///
    /// Defaults to the sha256 hash algorithm.
    pub fn new() -> ChecksumBuilder {
        ChecksumBuilder {
            hasher: Hasher::new(Algorithm::Sha256),
        }
    }

    /// Use the specified hash algorithm.
    pub fn algo(self, algo: Algorithm) -> ChecksumBuilder {
        ChecksumBuilder {
            hasher: Hasher::new(algo),
        }
    }

    /// Use the md5 hash algorithm.
    pub fn md5(self) -> ChecksumBuilder {
        self.algo(Algorithm::Md5)
    }

    /// Use the sha1 hash algorithm.
    pub fn sha1(self) -> ChecksumBuilder {
        self.algo(Algorithm::Sha1)
    }

    /// Use the sha256 hash algorithm.
    pub fn sha256(self) -> ChecksumBuilder {
        self.algo(Algorithm::Sha256)
    }

    /// Use the sha512 hash algorithm.
    pub fn sha512(self) -> ChecksumBuilder {
        self.algo(Algorithm::Sha512)
    }

    /// Build the Checksum instance for use.
    ///
    /// If no hash algorithm is specified, sha256 will be used.
    pub fn build(self) -> Checksum {
        Checksum {
            hasher: self.hasher,
        }
    }
}

/// Checksum is a wrapper around a hash algorithm.
pub struct Checksum {
    hasher: Hasher,
}

impl fmt::Debug for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Checksum").finish_non_exhaustive()
    }
}

impl Checksum {
    /// Consumes the provided data.
    #[inline]
    pub fn consume(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    /// Gets the result of the hash computation as a hex string.
    ///
    /// Note that the Checksum instance is consumed after getting the result.
    #[inline]
    pub fn finalize(self) -> String {
        self.hasher.finalize()
    }

    /// Checks if the result of the hash computation matches the input hash.
    ///
    /// Note that the Checksum instance is consumed after calling this method,
    /// if you want to keep the checksum result, use the finalize method instead.
    #[inline]
    pub fn check(self, input: &str) -> bool {
        input == self.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &[u8] = b"hello world";

    #[test]
    fn known_vectors() {
        let cases = [
            (Algorithm::Md5, "5eb63bbbe01eeed093cb22bb8f5acdc3"),
            (Algorithm::Sha1, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed"),
            (
                Algorithm::Sha256,
                "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
            ),
            (
                Algorithm::Sha512,
                "309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f\
                 989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f",
            ),
        ];

        for (algo, expected) in cases {
            let mut hasher = ChecksumBuilder::new().algo(algo).build();
            hasher.consume(DATA);
            assert_eq!(hasher.finalize(), expected, "algorithm {}", algo);
        }
    }

    #[test]
    fn streaming_matches_oneshot() {
        let data = b"the quick brown fox jumps over the lazy dog";

        let mut oneshot = ChecksumBuilder::new().sha256().build();
        oneshot.consume(data);
        let oneshot = oneshot.finalize();

        let mut streamed = ChecksumBuilder::new().sha256().build();
        for byte in data {
            streamed.consume(std::slice::from_ref(byte));
        }

        assert_eq!(streamed.finalize(), oneshot);
    }

    #[test]
    fn algorithm_lookup() {
        assert_eq!("sha512".parse::<Algorithm>().unwrap(), Algorithm::Sha512);
        assert!("sha3".parse::<Algorithm>().is_err());
        assert!("SHA256".parse::<Algorithm>().is_err());
    }
}
