mod md5;
mod sha1;
mod sha256;
mod sha512;

use std::fmt;

pub use md5::Md5;
pub use sha1::Sha1;
pub use sha256::Sha256;
pub use sha512::Sha512;

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
    pub fn new<S: AsRef<str>>(hash: S) -> Checksum {
        let input_hash = hash.as_ref().to_lowercase();
        let method = input_hash.split_once(":").unwrap_or(("sha256", "")).0;
        let hasher: Box<dyn Hasher> = match method {
            "md5" => Box::new(Md5::new()),
            "sha1" => Box::new(Sha1::new()),
            "sha256" => Box::new(Sha256::new()),
            "sha512" => Box::new(Sha512::new()),
            _ => unreachable!(),
        };

        Checksum { hasher, input_hash }
    }

    pub fn consume(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    pub fn result(&mut self) -> String {
        self.hasher.sum()
    }

    pub fn checksum(&mut self) -> bool {
        self.result() == self.input_hash
    }
}
