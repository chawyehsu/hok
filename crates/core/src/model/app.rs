use lazycell::LazyCell;

use super::Manifest;
use crate::{
    util::{compare_versions, extract_name_and_bucket},
    Config, ScoopResult,
};
use std::{fs::File, io::Read, path::Path};

/// A strcut that represents a single application that is already installed.
#[derive(Debug)]
pub struct InstalledApp<'cfg> {
    /// The global Scoop config.
    config: &'cfg Config,
    /// See [`AvailableApp`] for a description of this field.
    name: String,
    /// See [`AvailableApp`] for a description of this field.
    bucket: String,
    /// Installed versions of the app.
    versions: Vec<String>,
    // Normally an `App` should alwasy have its `manifest` unless the app is
    // installed and its manifest got renamed or removed from the bucket.
    //
    // The original Scoop implementation does not handle manifest renames and
    // deletions, we will implement it later, now we just keep this behaivor for
    // compatibility.
    manifest: LazyCell<Option<Manifest>>,
}

impl<'cfg> InstalledApp<'cfg> {
    pub fn new<S>(config: &Config, name: S, bucket: S) -> ScoopResult<InstalledApp>
    where
        S: AsRef<str>,
    {
        let name = name.as_ref().to_owned();
        let bucket = bucket.as_ref().to_owned();
        let mut versions = config
            .apps_path()
            .join(name.as_str())
            .read_dir()?
            .filter_map(|de| {
                if de.is_err() {
                    return None;
                }
                let de = de.unwrap();
                if de.file_type().unwrap().is_file() {
                    return None;
                }
                let version = de.file_name().into_string().unwrap();
                if version == "current" {
                    return None;
                }
                Some(version)
            })
            .collect::<Vec<_>>();
        if versions.len() == 0 {
            anyhow::bail!("broken installation of app {}", name.as_str());
        }
        log::trace!("{} versions {:?}", name.as_str(), versions);
        if versions.len() > 1 {
            versions.sort_by(compare_versions);
        }
        let manifest = LazyCell::new();
        Ok(InstalledApp {
            config,
            name,
            bucket,
            versions,
            manifest,
        })
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    #[inline]
    pub fn version(&self) -> &str {
        self.versions.last().unwrap()
    }
}

/// A strcut that represents a single application that is available to install.
#[derive(Debug)]
pub struct AvailableApp<'cfg> {
    /// The global Scoop config.
    config: &'cfg Config,
    /// This is the unique identifier of an `App`. The name is the same as the
    /// manifest's name `<name>.json`. Thus a bucket can only contains a `App`
    /// one time. Different buckets may have the same `App`, to install the app
    /// from a specific bucket, a `<bucket>/` prefix needs to be specified.
    name: String,
    /// This is the bucket of an `App`.
    ///
    /// The original Scoop implementation allows to install apps from direct URLs,
    /// which makes an orphan app without its bucket. We have no plans to support
    /// this since it's a bad use case not only it brings more complexity but
    /// also the orphan app can not be updated. Thus, this field is required for
    /// Every app.
    bucket: String,
    /// This is the manifest of an `App`. An `AvailableApp` always has a manifest.
    manifest: Manifest,
}

impl<'cfg> AvailableApp<'cfg> {
    pub fn new<P>(config: &Config, path: P) -> ScoopResult<AvailableApp>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let (name, bucket) = extract_name_and_bucket(path)?;
        let manifest = Manifest::new(path)?;
        Ok(AvailableApp {
            config,
            name,
            bucket,
            manifest,
        })
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    #[inline]
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
}

/// A struct that represents the install metadata of an `InstalledApp`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstallInfo {
    architecture: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hold: Option<bool>,
}

impl InstallInfo {
    pub fn new<P: AsRef<Path>>(path: P) -> ScoopResult<InstallInfo> {
        let path = path.as_ref();
        let mut bytes = Vec::new();
        File::open(path)?.read_to_end(&mut bytes)?;
        let info: InstallInfo = serde_json::from_slice(&bytes)?;
        Ok(info)
    }

    #[inline]
    pub fn bucket(&self) -> Option<&str> {
        self.bucket.as_deref()
    }
}
