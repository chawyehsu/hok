// #![allow(unused)]
pub(super) mod download;
pub(super) mod manifest;
pub(super) mod query;
pub(super) mod resolve;

use lazycell::LazyCell;
use std::path::Path;

use manifest::{License, Manifest};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Package {
    pub bucket: String,
    pub name: String,
    pub manifest_hash: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<String>>,
    pub manifest: Manifest,
    #[serde(skip_serializing_if = "Option::is_none")]
    upgradable_version: Option<String>,

    #[serde(skip)]
    origin: LazyCell<OriginateFrom>,

    #[serde(skip)]
    install_state: LazyCell<InstallState>,

    /// The upgradable package, if any.
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
        self.bucket.as_ref().map(|s| s.as_str())
    }

    #[inline]
    pub fn held(&self) -> bool {
        self.held
    }

    #[inline]
    pub fn url(&self) -> Option<&str> {
        self.url.as_ref().map(|s| s.as_str())
    }

    #[inline]
    pub fn version(&self) -> &str {
        self.version.as_str()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum InstallOption {
    AssumeNo,
    AssumeYes,
    DownloadOnly,
    IgnoreFailure,
    IgnoreHold,
    IgnoreCache,
    NoHashCheck,
    NoUpgrade,
    OnlyUpgrade,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum QueryOption {
    /// With Binaries. Enable query on package binaries.
    Binary,

    /// With Descriptions. Enable query on package descriptions.
    Description,

    /// Explicit mode. With this flag, an exact match query will be performed
    /// through the package name. Regex will be disabled, `Description` and
    /// `Binary` flags will be ignored.
    Explicit,

    Upgradable,
}

impl Package {
    pub fn from(name: &str, bucket: &str, manifest: Manifest) -> Package {
        let manifest_hash = 0u64;
        let dependencies = manifest
            .raw_dependencies()
            .map(|v| v.into_iter().map(|s| s.to_owned()).collect());

        Package {
            bucket: bucket.to_owned(),
            dependencies,
            // shims,
            manifest_hash,
            name: name.to_owned(),
            // state,
            manifest,
            upgradable_version: None,
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

    #[inline]
    pub fn description(&self) -> Option<&str> {
        self.manifest.description()
    }

    #[inline]
    pub fn homepage(&self) -> &str {
        self.manifest.homepage()
    }

    #[inline]
    pub fn license(&self) -> &License {
        self.manifest.license()
    }

    #[inline]
    pub fn installed(&self) -> bool {
        match self.install_state.borrow() {
            None => false,
            Some(state) => match state {
                InstallState::NotInstalled => false,
                InstallState::Installed(_) => true,
            },
        }
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

    #[inline]
    pub fn manfest_path(&self) -> &Path {
        self.manifest.path()
    }

    /// Check if the package is upgradable. Return the upgradable version when
    /// it is.
    #[inline]
    pub fn upgradable(&self) -> Option<&str> {
        let origin_pkg = self.upgradable.borrow();

        match origin_pkg {
            None => None,
            Some(inner) => match inner {
                None => None,
                Some(pkg) => Some(pkg.version()),
            },
        }
    }

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
