use std::io;
use std::result;

#[derive(Debug)]
pub enum ErrorKind {
    Io(io::Error),
}
pub struct Error(ErrorKind);

/// A type alias for `Result<T, scoop::Error>`.
pub type Result<T> = result::Result<T, Error>;
