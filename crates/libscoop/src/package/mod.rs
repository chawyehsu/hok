mod manifest;
pub mod query;
pub mod resolve;
mod sync;

use lazycell::LazyCell;
use std::path::Path;

pub use manifest::{InstallInfo, License, Manifest};
pub use query::QueryOption;
pub use sync::SyncOption;

/// A Scoop package.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Package {
    /// The bucket name of this package.
    bucket: String,

    /// The name of this package.
    name: String,

    /// The manifest of this package.
    pub manifest: Manifest,

    #[serde(skip)]
    origin: LazyCell<OriginateFrom>,

    /// The install state of the package.
    #[serde(skip)]
    install_state: LazyCell<InstallState>,

    /// The upgradable package, if any.
    ///
    /// This field is never serialized.
    #[serde(skip)]
    upgradable: LazyCell<Option<Box<Package>>>,
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
            origin: LazyCell::new(),
            install_state: LazyCell::new(),
            upgradable: LazyCell::new(),
        }
    }

    /// Return the identity of this package, in the form of `bucket/name`, which
    /// is unique for each package
    #[inline]
    pub fn ident(&self) -> String {
        format!("{}/{}", self.bucket, self.name)
    }

    /// Get the bucket name of this package.
    #[inline]
    pub fn bucket(&self) -> &str {
        self.bucket.as_str()
    }

    /// Get the name of this package.
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
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
    #[inline]
    pub fn license(&self) -> &License {
        self.manifest.license()
    }

    /// Get the dependencies of this package.
    #[inline]
    pub fn dependencies(&self) -> Vec<String> {
        self.manifest.dependencies()
    }

    /// Get download urls of this package.
    #[inline]
    pub fn url(&self) -> Vec<&str> {
        self.manifest.url()
    }

    #[inline]
    pub fn installed_version(&self) -> Option<&str> {
        match self.install_state.borrow() {
            None => None,
            Some(state) => match state {
                InstallState::NotInstalled => None,
                InstallState::Installed(info) => Some(info.version()),
            },
        }
    }

    /// Check if the package is held.
    #[inline]
    pub fn is_held(&self) -> bool {
        match self.install_state.borrow() {
            None => false,
            Some(state) => match state {
                InstallState::NotInstalled => false,
                InstallState::Installed(info) => info.held(),
            },
        }
    }

    /// Check if the package is installed.
    #[inline]
    pub fn is_installed(&self) -> bool {
        self.installed_version().is_some()
    }

    /// Check if the package is strictly installed, which means the package is
    /// installed from the bucket it belongs to rather than from other buckets.
    #[inline]
    pub fn is_strictly_installed(&self) -> bool {
        match self.install_state.borrow() {
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

    /// Get the path of the manifest file of this package.
    #[inline]
    pub fn manfest_path(&self) -> &Path {
        self.manifest.path()
    }

    /// Check if the package is upgradable. Return the upgradable version when
    /// it is.
    #[inline]
    pub fn upgradable(&self) -> Option<&str> {
        let origin_pkg = self.upgradable.borrow();

        if let Some(Some(pkg)) = origin_pkg {
            return Some(pkg.version());
        }
        None
    }

    /// Get the version of this package.
    #[inline]
    pub fn version(&self) -> &str {
        self.manifest.version()
    }

    #[inline]
    pub fn shims(&self) -> Option<Vec<&str>> {
        self.manifest.executables()
    }

    #[inline]
    pub(crate) fn fill_install_state(&self, state: InstallState) {
        let origin = match &state {
            InstallState::NotInstalled => OriginateFrom::Bucket(self.bucket.clone()),
            InstallState::Installed(info) => match info.url() {
                Some(url) => OriginateFrom::File(url.to_owned()),
                None => OriginateFrom::Bucket(info.bucket().unwrap_or("__isolated").to_owned()),
            },
        };

        let _ = self.origin.fill(origin);
        let _ = self.install_state.fill(state);
    }

    #[inline]
    pub(crate) fn fill_upgradable(&self, upgradable: Package) {
        let upgradable = Some(Box::new(upgradable));
        let _ = self.upgradable.fill(upgradable);
    }
}

impl PartialEq for Package {
    fn eq(&self, other: &Package) -> bool {
        self.name() == other.name()
    }
}
