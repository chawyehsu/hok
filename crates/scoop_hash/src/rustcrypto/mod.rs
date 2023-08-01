mod md5;
mod sha1;
mod sha256;
mod sha512;

pub use self::md5::WrappedMd5 as Md5;
pub use self::sha1::WrappedSha1 as Sha1;
pub use self::sha256::WrappedSha256 as Sha256;
pub use self::sha512::WrappedSha512 as Sha512;
pub use ::md5::Digest;
