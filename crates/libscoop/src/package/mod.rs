pub(crate) mod download;
pub(crate) mod manifest;
pub(crate) mod query;
pub(crate) mod resolve;
pub(crate) mod sync;

use once_cell::unsync::OnceCell;
use std::fmt;

pub use manifest::{InstallInfo, License, Manifest};
pub use query::QueryOption;
pub use sync::SyncOption;

use crate::{constant::ISOLATED_PACKAGE_BUCKET, internal};

/// A Scoop package.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Package {
    /// The bucket name of this package.
    bucket: String,

    /// The name of this package.
    name: String,

    /// The manifest of this package.
    manifest: Manifest,

    #[serde(skip)]
    origin: OnceCell<OriginateFrom>,

    /// The install state of the package.
    #[serde(skip)]
    install_state: OnceCell<InstallState>,

    /// The upgradable package, if any.
    ///
    /// This field is never serialized.
    #[serde(skip)]
    upgradable: OnceCell<Option<Box<Package>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OriginateFrom {
    Bucket(String),
    File(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InstallState {
    NotInstalled,
    Installed(InstallStateInstalled),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstallStateInstalled {
    pub version: String,
    pub bucket: Option<String>,
    pub arch: String,
    pub held: bool,
    pub url: Option<String>,
}

impl InstallStateInstalled {
    #[inline]
    pub fn bucket(&self) -> Option<&str> {
        self.bucket.as_deref()
    }

    #[inline]
    pub fn held(&self) -> bool {
        self.held
    }

    #[inline]
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    #[inline]
    pub fn version(&self) -> &str {
        self.version.as_str()
    }
}

impl Package {
    pub fn from(name: &str, bucket: &str, manifest: Manifest) -> Package {
        Package {
            bucket: bucket.to_owned(),
            name: name.to_owned(),
            manifest,
            origin: OnceCell::new(),
            install_state: OnceCell::new(),
            upgradable: OnceCell::new(),
        }
    }

    /// The identity of this package.
    ///
    /// # Returns
    ///
    /// The package identity in the form of `bucket/name`, which is unique for
    /// each package across all buckets.
    #[inline]
    pub fn ident(&self) -> String {
        format!("{}/{}", self.bucket, self.name)
    }

    /// Get the name of this package.
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Get the bucket name of this package.
    ///
    /// # Note
    ///
    /// Although this method in some cases returns a bucket namer which can be
    /// the same as the bucket name from the install state of a package, it is
    /// not guaranteed to be.
    ///
    /// This method is not identical to `installed_bucket()`, which is designed
    /// to returns the precise installed bucket name if any.
    #[inline]
    pub fn bucket(&self) -> &str {
        self.bucket.as_str()
    }

    /// Get the version of this package.
    ///
    /// # Note
    ///
    /// Although this method in some cases returns a version number which can be
    /// the same as the version number from the installe state of a package, it
    /// is not guaranteed to be.
    ///
    /// This method is not identical to `installed_version()`, which is designed
    /// to returns the precise installed version number if any.
    #[inline]
    pub fn version(&self) -> &str {
        self.manifest.version()
    }

    /// Get the description of this package.
    #[inline]
    pub fn description(&self) -> Option<&str> {
        self.manifest.description()
    }

    /// Get the homepage of this package.
    #[inline]
    pub fn homepage(&self) -> &str {
        self.manifest.homepage()
    }

    /// Get the license of this package.
    pub fn license(&self) -> &License {
        self.manifest.license()
    }

    /// Get the cookie of this package.
    pub fn cookie(&self) -> Option<Vec<(&str, &str)>> {
        self.manifest.cookie().map(|c| {
            c.iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect::<Vec<_>>()
        })
    }

    /// Get the dependencies of this package.
    ///
    /// # Note
    ///
    /// There is no guarantee that whether a dependency is represented as a
    /// format of `bucket/name` or `name`.
    ///
    /// # Returns
    ///
    /// A list of dependencies of this package.
    pub fn dependencies(&self) -> Vec<String> {
        self.manifest.dependencies()
    }

    /// Get download urls of this package.
    ///
    /// # Note
    ///
    /// This method will return the actual download urls without the `#/dl.7z`
    /// fragment which is used to fake the file extension of the download urls.
    pub(crate) fn download_urls(&self) -> Vec<&str> {
        self.manifest
            .url()
            .into_iter()
            .map(|u| u.split_once('#').map(|s| s.0).unwrap_or(u))
            .collect::<Vec<_>>()
    }

    /// Get download urls of this package.
    pub(crate) fn download_filenames(&self) -> Vec<String> {
        self.manifest
            .url()
            .into_iter()
            .map(|u| {
                format!(
                    "{}#{}#{}",
                    self.name(),
                    self.version(),
                    internal::fs::filenamify(u)
                )
            })
            .collect::<Vec<_>>()
    }

    pub(crate) fn download_hashes(&self) -> Vec<&str> {
        self.manifest.hash()
    }

    /// Get the installed bucket of this package.
    ///
    /// # Returns
    ///
    /// The installed bucket of this package, if any.
    pub fn installed_bucket(&self) -> Option<&str> {
        match self.install_state.get() {
            None => None,
            Some(state) => match state {
                InstallState::NotInstalled => None,
                InstallState::Installed(info) => {
                    Some(info.bucket().unwrap_or(ISOLATED_PACKAGE_BUCKET))
                }
            },
        }
    }

    /// Get the installed version of this package.
    ///
    /// # Returns
    ///
    /// The installed version of this package, if any.
    pub fn installed_version(&self) -> Option<&str> {
        match self.install_state.get() {
            None => None,
            Some(state) => match state {
                InstallState::NotInstalled => None,
                InstallState::Installed(info) => Some(info.version()),
            },
        }
    }

    /// Check if the package is held.
    ///
    /// # Note
    ///
    /// Only installed package can be held, therefore this method will always
    /// return `false` if the package is not installed.
    pub fn is_held(&self) -> bool {
        match self.install_state.get() {
            None => false,
            Some(state) => match state {
                InstallState::NotInstalled => false,
                InstallState::Installed(info) => info.held(),
            },
        }
    }

    /// Check if the package is installed.
    pub fn is_installed(&self) -> bool {
        self.installed_version().is_some()
    }

    #[inline]
    pub fn is_nightly(&self) -> bool {
        self.version() == "nightly"
    }

    /// Check if the package is strictly installed, which means the package is
    /// installed from the bucket it belongs to rather than from other buckets.
    pub fn is_strictly_installed(&self) -> bool {
        match self.install_state.get() {
            None => false,
            Some(state) => match state {
                InstallState::NotInstalled => false,
                InstallState::Installed(info) => match info.bucket() {
                    Some(bucket) => bucket == self.bucket(),
                    None => false,
                },
            },
        }
    }

    /// Get the manifest of this package.
    ///
    /// # Returns
    ///
    /// The manifest reference of this package.
    #[inline]
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Get the upgradable version of this package.
    ///
    /// # Returns
    ///
    /// The upgradable version when the package is upgradable, otherwise `None`.
    pub fn upgradable_version(&self) -> Option<&str> {
        let origin_pkg = self.upgradable.get();

        if let Some(Some(pkg)) = origin_pkg {
            return Some(pkg.version());
        } else if let Some(installed_version) = self.installed_version() {
            let this_version = self.version();
            let is_upgradable = internal::compare_versions(this_version, installed_version)
                == std::cmp::Ordering::Greater;
            if is_upgradable {
                return Some(this_version);
            }
        }

        None
    }

    /// Check if this package is upgradable.
    ///
    /// # Returns
    ///
    /// The reference to the upgradable package of this package when it is
    /// upgradable, otherwise `None`.
    pub fn upgradable(&self) -> Option<&Package> {
        if let Some(Some(pkg)) = self.upgradable.get() {
            return Some(pkg.as_ref());
        }
        None
    }

    /// Get shims defined in this package.
    ///
    /// # Returns
    ///
    /// A list of shims defined in this package.
    pub fn shims(&self) -> Option<Vec<&str>> {
        self.manifest.shims()
    }

    pub fn supported_arch(&self) -> Vec<String> {
        let mut ret = vec![];
        if let Some(arch) = self.manifest.architecture() {
            if arch.ia32.is_some() {
                ret.push("ia32".to_string());
            }
            if arch.amd64.is_some() {
                ret.push("amd64".to_string());
            }
            if arch.aarch64.is_some() {
                ret.push("aarch64".to_string());
            }
        }
        ret
    }

    /// Check if this package has used powershell script hooks in its manifest.
    pub(crate) fn has_ps_script(&self) -> bool {
        [
            self.manifest.pre_install(),
            self.manifest.post_install(),
            self.manifest
                .installer()
                .map(|i| i.script())
                .unwrap_or_default(),
            self.manifest
                .uninstaller()
                .map(|u| u.script())
                .unwrap_or_default(),
            self.manifest.pre_uninstall(),
            self.manifest.post_uninstall(),
        ]
        .into_iter()
        .any(|h| h.is_some())
    }

    pub(crate) fn fill_install_state(&self, state: InstallState) {
        let origin = match &state {
            InstallState::NotInstalled => OriginateFrom::Bucket(self.bucket.clone()),
            InstallState::Installed(info) => match info.url() {
                Some(url) => OriginateFrom::File(url.to_owned()),
                None => OriginateFrom::Bucket(
                    info.bucket().unwrap_or(ISOLATED_PACKAGE_BUCKET).to_owned(),
                ),
            },
        };

        let _ = self.origin.set(origin);
        let _ = self.install_state.set(state);
    }

    pub(crate) fn fill_upgradable(&self, upgradable: Package) {
        let upgradable = Some(Box::new(upgradable));
        let _ = self.upgradable.set(upgradable);
    }
}

impl PartialEq for Package {
    fn eq(&self, other: &Package) -> bool {
        self.name() == other.name()
    }
}

/// Extact `name` from `bucket/name`.
pub(super) fn extract_name<S: AsRef<str>>(input: S) -> String {
    input
        .as_ref()
        .split_once('/')
        .map(|(_, n)| n)
        .unwrap_or(input.as_ref())
        .to_owned()
}

/// Hash mismatch context.
#[derive(Clone, Debug)]
pub struct HashMismatchContext {
    name: String,
    url: String,
    expected: String,
    actual: String,
}

impl fmt::Display for HashMismatchContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hash mismatch for package '{}':\n     Url: {}\nExpected: {}\n  Actual: {}",
            self.name(),
            self.url(),
            self.expected(),
            self.actual()
        )
    }
}

impl HashMismatchContext {
    /// Create a new hash mismatch context.
    pub fn new(name: String, url: String, expected: String, actual: String) -> HashMismatchContext {
        HashMismatchContext {
            name,
            url,
            expected,
            actual,
        }
    }

    /// name of the package.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// url of corresponding hash mismatched file.
    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    /// Expected hash.
    pub fn expected(&self) -> &str {
        self.expected.as_str()
    }

    /// Actual hash.
    pub fn actual(&self) -> &str {
        self.actual.as_str()
    }
}
