#![allow(dead_code)]
#[derive(Debug)]
pub enum Format {
    /// .bz2
    Bz2,
    /// .gz
    Gzip,
    /// .rar
    Rar,
    /// .7z, .xz
    XZip,
    /// .tar
    Tar,
    /// .zip
    Zip,
    /// .zstd
    Zst,
}
