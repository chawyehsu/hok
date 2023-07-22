use std::path::PathBuf;

use crate::internal::dag::CyclicError;

pub type Fallible<T> = Result<T, Error>;

/// Error that may occur during performing operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("bucket '{0}' already exists")]
    BucketAlreadyExists(String),

    /// Thrown when trying to add a bucket that is not a known bucket without
    /// specifying a remote url.
    #[error("'{0}' is not a known bucket, <repo> is required")]
    BucketAddRemoteRequired(String),

    #[error("bucket '{0}' does not exist")]
    BucketNotFound(String),

    #[error("bare bucket '{0}' is no longer supported")]
    BareBucketFound(String),

    #[error("{0}")]
    Custom(String),

    #[error(transparent)]
    CyclicDependencies(#[from] CyclicError),

    #[error("error")]
    Db,
    /// Thrown when there was an [I/O error][1] opening the config file
    ///
    /// [1]: std::io::Error
    #[error("error")]
    ReadConfigError(PathBuf),
    /// Thrown when there was an [serde_json error][1] parsing the config file
    ///
    /// [1]: serde_json::Error
    #[error("error")]
    ParseConfigError(PathBuf),

    #[error(transparent)]
    Git(#[from] git2::Error),

    #[error("error")]
    HashMismatch,

    /// Thrown when constructing a [`CacheFile`][1] from a given path with a
    /// filename that does not match the [`REGEX_CACHE_FILE`][2] format.
    ///
    /// [1]: crate::types::CacheFile
    /// [2]: crate::constants::REGEX_CACHE_FILE
    #[error("error")]
    InvalidCacheFile { path: PathBuf },

    #[error("invalid config key '{0}'")]
    InvalidConfigKey(String),
    #[error("invalid config value '{0}'")]
    InvalidConfigValue(String),

    #[error("error")]
    InvalidHashValue(String),

    #[error("http {message}")]
    Http {
        message: String,
        source: Option<ureq::Error>,
    },

    /// Wrapped [std I/O error][1]. Throw when doing I/O operations, such as
    /// reading or writing files or directories.
    ///
    /// [1]: std::io::Error
    #[error("{message}")]
    Io {
        message: String,
        source: std::io::Error,
    },

    // /// Thrown when multiple package records are found for a given query.
    // /// This is useful when a single record for a query is needed.
    // #[error("multiple records found for queries: {0}", records.iter().map(|r| r.0.as_str()).collect::<Vec<_>>().join(" "))]
    // PackageMultipleRecordsFound {
    //     records: Vec<(String, Vec<Package>)>,
    // },
    /// Thrown when no package is found for a given query.
    #[error("could not find package(s): {0}", queries.join(" "))]
    PackageNotFound { queries: Vec<String> },

    /// Thrown when trying to perform (un)hold operation on a package that is
    /// not installed.
    #[error("package '{0}' is not installed")]
    PackageHoldNotInstalled(String),

    /// Thrown when trying to perform (un)hold operation on a package of which
    /// the installation is broken.
    #[error("package '{0}' is broken")]
    PackageHoldBrokenInstall(String),

    /// Wrapped possible [serde_json Error][1]. Throw when (de)serializing JSON
    /// files.
    ///
    /// [1]: https://docs.serde.rs/serde_json/struct.Error.html
    #[error("bad regular expression, {0}")]
    Regex(regex::Error),

    /// Wrapped possible [serde_json Error][1]. Throw when (de)serializing JSON
    /// files.
    ///
    /// [1]: https://docs.serde.rs/serde_json/struct.Error.html
    #[error("{message}")]
    Serde {
        message: String,
        source: serde_json::Error,
    },
}

pub(super) trait Context<T> {
    fn with_context<F>(self, f: F) -> Fallible<T>
    where
        F: FnOnce() -> String;
}

impl<T> Context<T> for std::io::Result<T> {
    fn with_context<F>(self, f: F) -> Fallible<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|source| Error::Io {
            message: f(),
            source,
        })
    }
}

impl<T> Context<T> for Result<T, ureq::Error> {
    fn with_context<F>(self, f: F) -> Fallible<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|source| Error::Http {
            message: f(),
            source: Some(source),
        })
    }
}

impl<T> Context<T> for serde_json::Result<T> {
    fn with_context<F>(self, f: F) -> Fallible<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|source| Error::Serde {
            message: f(),
            source,
        })
    }
}

impl From<regex::Error> for Error {
    fn from(source: regex::Error) -> Self {
        Error::Regex(source)
    }
}

// impl<T> Context<T> for Result<T, regex::Error> {
//     fn with_context<F>(self, f: F) -> Fallible<T>
//     where
//         F: FnOnce() -> String,
//     {
//         self.map_err(|source| Error::Serde {
//             message: f(),
//             source,
//         })
//     }
// }
