use std::path::PathBuf;

use crate::internal::dag::CyclicError;

pub type Fallible<T> = Result<T, Error>;

/// Error that may occur during performing operations.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Thrown when trying to add a bucket that already exists.
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

    /// Thrown when trying to mutate config while it is in use.
    #[error("Could not alter config because it is in use.")]
    ConfigInUse,

    /// Invalid config key error
    #[error("invalid config key '{0}'")]
    ConfigKeyInvalid(String),

    /// Invalid config value error
    #[error("invalid config value '{0}'")]
    ConfigValueInvalid(String),

    /// Thrown when trying to set the user agent twice.
    #[error("User agent already set")]
    UserAgentAlreadySet,

    #[error("error")]
    HashMismatch,

    /// Invalid cache file error
    #[error("error")]
    InvalidCacheFile { path: PathBuf },

    #[error("error")]
    InvalidHashValue(String),

    /// Throw when receiving an invalid answer from the frontend.
    #[error("invalid answer")]
    InvalidAnswer,

    /// Package not found error, this may occur when doing an explicit lookup
    /// for a package and no record with the given query was found.
    #[error("Could not find package named '{0}'")]
    PackageNotFound(String),

    /// Thrown when there are multiple candidates for a package name.
    #[error("Found multiple candidates for package named '{0}'")]
    PackageMultipleCandidates(String),

    /// Thrown when trying to perform (un)hold operation on a package that is
    /// not installed.
    #[error("package '{0}' is not installed")]
    PackageHoldNotInstalled(String),

    /// Thrown when trying to perform (un)hold operation on a package of which
    /// the installation is broken.
    #[error("package '{0}' is broken")]
    PackageHoldBrokenInstall(String),

    /// A custom error.
    #[error("{0}")]
    Custom(String),

    /// Cycle dependency error
    #[error(transparent)]
    CyclicDependency(#[from] CyclicError),

    /// Git error
    #[error(transparent)]
    Git(#[from] git2::Error),

    /// I/O error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Regular expression error
    #[error(transparent)]
    Regex(#[from] regex::Error),

    /// Serde error
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}
