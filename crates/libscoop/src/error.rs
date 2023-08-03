use std::path::PathBuf;

use crate::{internal::dag::CyclicError, package::HashMismatchContext};

pub type Fallible<T> = Result<T, Error>;

/// Error that may occur during the lifetime of a [`Session`][1].
///
/// [1]: crate::Session
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

    /// Bucket not found error
    #[error("bucket '{0}' does not exist")]
    BucketNotFound(String),

    /// Bare bucket error
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

    /// Hash mismatch error
    #[error("{0}")]
    HashMismatch(HashMismatchContext),

    /// Invalid cache file error
    #[error("error")]
    InvalidCacheFile { path: PathBuf },

    /// Throw when receiving an invalid answer from the frontend.
    #[error("invalid answer")]
    InvalidAnswer,

    /// Package not found error, this may occur when doing an explicit lookup
    /// for a package and no record with the given query was found.
    #[error("Could not find package named '{0}'")]
    PackageNotFound(String),

    /// Thrown when trying to do a cascading uninstall of a package that has
    /// a held dependency.
    #[error("Trying to cascade uninstall held package '{0}'")]
    PackageCascadeRemoveHold(String),

    /// Package dependent found error
    #[error("Found dependent(s):\n{}", .0.iter().map(|(d, p)| format!("'{}' requires '{}'", d, p)).collect::<Vec<_>>().join("\n"))]
    PackageDependentFound(Vec<(String, String)>),

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

    /// Scoop hash error
    #[error(transparent)]
    Hash(#[from] scoop_hash::Error),

    /// Curl error
    #[error(transparent)]
    Curl(#[from] curl::Error),

    /// Curl Multi error
    #[error(transparent)]
    CurlMulti(#[from] curl::MultiError),

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
