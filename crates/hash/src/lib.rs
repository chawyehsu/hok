mod md5;
mod sha1;
mod sha256;
mod sha512;

use core::fmt;

pub use crate::md5::Md5;
pub use crate::sha1::Sha1;
pub use crate::sha256::Sha256;
pub use crate::sha512::Sha512;

trait Hasher {
    fn hash_type(&self) -> String;
    fn update(&mut self, data: &[u8]);
    fn sum(&mut self) -> String;
}

impl Hasher for Md5 {
    fn hash_type(&self) -> String {
        "md5".to_string()
    }

    fn update(&mut self, data: &[u8]) {
        self.consume(data);
    }

    fn sum(&mut self) -> String {
        self.result_string()
    }
}

impl Hasher for Sha1 {
    fn hash_type(&self) -> String {
        "sha1".to_string()
    }

    fn update(&mut self, data: &[u8]) {
        self.consume(data);
    }

    fn sum(&mut self) -> String {
        self.result_string()
    }
}

impl Hasher for Sha256 {
    fn hash_type(&self) -> String {
        "sha256".to_string()
    }

    fn update(&mut self, data: &[u8]) {
        self.consume(data);
    }

    fn sum(&mut self) -> String {
        self.result_string()
    }
}

impl Hasher for Sha512 {
    fn hash_type(&self) -> String {
        "sha512".to_string()
    }

    fn update(&mut self, data: &[u8]) {
        self.consume(data);
    }

    fn sum(&mut self) -> String {
        self.result_string()
    }
}

pub struct Error;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported hash type")
    }
}

#[derive(Debug)]
pub struct Checksum {
    hasher: Box<dyn Hasher>,
    input_hash: String,
}

impl fmt::Debug for dyn Hasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hasher {{ hash_type: {} }}", self.hash_type())
    }
}

impl Checksum {
    pub fn new<S: AsRef<str>>(hash: S) -> Result<Checksum, Error> {
        let hash = hash.as_ref().to_lowercase();
        let (method, input_hash) = hash.split_once(":").unwrap_or(("sha256", ""));
        let input_hash = input_hash.to_string();
        let hasher: Box<dyn Hasher> = match method {
            "md5" => Box::new(Md5::new()),
            "sha1" => Box::new(Sha1::new()),
            "sha256" => Box::new(Sha256::new()),
            "sha512" => Box::new(Sha512::new()),
            _ => return Err(Error),
        };

        Ok(Checksum { hasher, input_hash })
    }

    #[inline]
    pub fn consume(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    #[inline]
    pub fn result(&mut self) -> String {
        self.hasher.sum()
    }

    #[inline]
    pub fn checksum(&mut self) -> bool {
        self.result() == self.input_hash
    }
}
