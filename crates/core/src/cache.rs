use std::path::{Path, PathBuf};

use crate::constants::REGEX_CACHE_FILE;
use crate::error::{Context, Error, Fallible};
use crate::Session;

/// Scoop cache file representation
#[derive(Clone, Debug)]
pub struct CacheFile {
    path: PathBuf,
}

impl CacheFile {
    pub fn from(path: PathBuf) -> Fallible<CacheFile> {
        let text = path.file_name().unwrap().to_str().unwrap();
        match REGEX_CACHE_FILE.is_match(text) {
            false => Err(Error::InvalidCacheFile { path }.into()),
            true => Ok(CacheFile { path }),
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
        &self.path.file_name().unwrap().to_str().unwrap()
    }

    /// Get package name of this cache file
    #[inline]
    pub fn package_name(&self) -> &str {
        self.file_name().split_once("#").map(|s| s.0).unwrap()
    }

    /// Get version of this cache file
    #[inline]
    pub fn version(&self) -> &str {
        self.file_name().splitn(3, "#").collect::<Vec<_>>()[1]
    }

    /// Get the tmp path of this cache file
    #[inline]
    pub fn tmp_path(&self) -> PathBuf {
        let mut temp = self.path.clone().into_os_string();
        temp.push(".download");
        PathBuf::from(temp)
    }
}

pub fn cache_list(session: &Session, query: &str) -> Fallible<Vec<CacheFile>> {
    let mut entires = session
        .config
        .cache_path
        .read_dir()
        .with_context(|| {
            format!(
                "failed to read cache dir: {}",
                session.config.cache_path.display()
            )
        })?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().unwrap().is_file())
        .filter_map(|de| CacheFile::from(de.path()).ok())
        .collect::<Vec<_>>();
    match query {
        "" | "*" => {}
        query => {
            entires = entires
                .into_iter()
                .filter(|f| f.package_name().contains(query))
                .collect::<Vec<_>>();
        }
    }
    Ok(entires)
}

pub fn cache_remove(session: &Session, query: &str) -> Fallible<()> {
    match query {
        "*" => Ok(
            crate::util::empty_dir(&session.config.cache_path).with_context(|| {
                format!(
                    "failed to empty cache dir: {}",
                    session.config.cache_path.display()
                )
            })?,
        ),
        query => {
            let files = cache_list(session, query)?;
            for f in files.into_iter() {
                std::fs::remove_file(f.path()).with_context(|| {
                    format!("failed to remove cache file: {}", f.path().display())
                })?;
            }
            Ok(())
        }
    }
}
