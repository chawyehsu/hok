use std::path::{Path, PathBuf};

use crate::error::{Error, Fallible};
use crate::package::download::CacheFileName;

/// Scoop cache file representation
#[derive(Clone, Debug)]
pub struct CacheFile {
    path: PathBuf,
}

impl CacheFile {
    pub fn from(path: PathBuf) -> Fallible<CacheFile> {
        match self::file_name(&path).and_then(CacheFileName::parse) {
            None => Err(Error::InvalidCacheFile { path }),
            Some(_) => Ok(CacheFile { path }),
        }
    }

    /// Get path of this cache file
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get file name of this cache file
    #[inline]
    pub fn file_name(&self) -> &str {
        file_name(&self.path).unwrap_or_default()
    }

    /// Get package name of this cache file
    #[inline]
    pub fn package_name(&self) -> &str {
        CacheFileName::parse(self.file_name())
            .map(|name| name.package)
            .unwrap_or_default()
    }

    /// Get version of this cache file
    #[inline]
    pub fn version(&self) -> &str {
        CacheFileName::parse(self.file_name())
            .map(|name| name.version)
            .unwrap_or_default()
    }
}

/// The file name component of `path`, if it is valid UTF-8.
fn file_name(path: &Path) -> Option<&str> {
    path.file_name()?.to_str()
}
