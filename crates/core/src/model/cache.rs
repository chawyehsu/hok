use std::{borrow::Borrow, path::{Path, PathBuf}};
use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};
use crate::ScoopResult;

/// This is the representation of a Scoop's cache file.
#[derive(Clone, Debug)]
pub struct CacheFile {
    /// The absolute path of this cache file.
    path: PathBuf,
}

impl CacheFile {
    /// Create a Scoop [`CacheFile`] with the given PathBuf.
    ///
    /// This constructor is marked as private, since we don't want any caller
    /// outside the [`CacheManager`] to create new CacheFile directly.
    #[inline]
    pub fn new(path: PathBuf) -> ScoopResult<CacheFile> {
        // regex to match valid Scoop cache filename:
        // "app#version#filenamified_url"
        static RE: Lazy<Regex> = Lazy::new(|| {
            let r = r"(?P<app>[0-9a-zA-Z-_.]+)#(?P<version>[0-9a-zA-Z-.]+)#(?P<url>.*)";
            RegexBuilder::new(r).build().unwrap()
        });
        // check filename
        let name = path.file_name().map(|s| s.to_string_lossy()).unwrap();
        match RE.is_match(name.borrow()) {
            true => Ok(CacheFile { path }),
            false => anyhow::bail!("invalid cache filename of {}", path.display()),
        }
    }

    /// Get the `app` name of this `CacheFile`.
    #[inline]
    pub fn app_name(&self) -> String {
        self.file_name()
            .split_once("#")
            .map(|s| s.0.to_owned())
            .unwrap()
    }

    /// Get the filename of this `CacheFile`.
    #[inline]
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap()
    }

    /// Get the filepath of this `CacheFile`.
    #[inline]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// Get the tmp download path of this `CacheFile`.
    #[inline]
    pub fn tmp_path(&self) -> PathBuf {
        let mut temp = self.path.clone().into_os_string();
        temp.push(".download");
        PathBuf::from(temp)
    }

    /// Get the file size of this `CacheFile`.
    #[inline]
    pub fn size(&self) -> u64 {
        self.path.metadata().map(|m| m.len()).unwrap()
    }

    #[inline]
    pub fn size_as_bytes(&self, unit: bool) -> String {
        crate::util::filesize(self.size(), unit)
    }

    /// Get the app `version` of this `CacheFile`.
    #[inline]
    pub fn version(&self) -> String {
        self.file_name().splitn(3, "#").collect::<Vec<_>>()[1].to_owned()
    }

    #[inline]
    pub fn filename(&self) -> String {
        self.file_name().splitn(3, "#").collect::<Vec<_>>()[2].to_owned()
    }
}
