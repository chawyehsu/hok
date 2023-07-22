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

    #[error("error")]
    HashMismatch,

    /// Invalid cache file error
    #[error("error")]
    InvalidCacheFile { path: PathBuf },

    #[error("invalid config key '{0}'")]
    InvalidConfigKey(String),
    #[error("invalid config value '{0}'")]
    InvalidConfigValue(String),

    #[error("error")]
    InvalidHashValue(String),

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

    /// Cycle dependency error
    #[error(transparent)]
    CyclicDependency(#[from] CyclicError),

    /// Git error
    #[error(transparent)]
    Git(#[from] git2::Error),

    /// I/O error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Network error
    #[error(transparent)]
    Network(#[from] ureq::Error),

    /// Regular expression error
    #[error(transparent)]
    Regex(#[from] regex::Error),

    /// Serde error
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}
