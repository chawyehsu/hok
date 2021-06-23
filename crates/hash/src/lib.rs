mod md5;
mod sha1;
mod sha256;
mod sha512;

pub use md5::Md5;
pub use sha1::Sha1;
pub use sha256::Sha256;
pub use sha512::Sha512;

pub fn checksum(hash: &str, data: &[u8]) -> bool {
    let hash = hash.to_lowercase();
    let method = hash.split_once(":").unwrap().0;
    match method {
        "md5" => md5sum(hash, data),
        "sha1" => sha1sum(hash, data),
        "sha256" => sha256sum(hash, data),
        "sha512" => sha512sum(hash, data),
        _ => unreachable!(),
    }
}

pub fn md5sum(hash: String, data: &[u8]) -> bool {
    hash == Md5::new().consume(data).result_string()
}

pub fn sha1sum(hash: String, data: &[u8]) -> bool {
    hash == Sha1::new().consume(data).result_string()
}

pub fn sha256sum(hash: String, data: &[u8]) -> bool {
    hash == Sha256::new().consume(data).result_string()
}

pub fn sha512sum(hash: String, data: &[u8]) -> bool {
    hash == Sha512::new().consume(data).result_string()
}
