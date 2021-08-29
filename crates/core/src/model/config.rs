use crate::ScoopResult;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::result::Result;

/// Configuration information for Scoop.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(skip)]
    #[serde(default = "default::config_path")]
    config_path: PathBuf,
    #[serde(rename = "7zipextract_use_external")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub szipextract_use_external: Option<bool>,
    #[serde(alias = "aria2_enabled")]
    #[serde(rename = "aria2-enabled")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aria2_enabled: Option<bool>,
    /// The cache path
    #[serde(alias = "cache_path")]
    #[serde(rename = "cachePath")]
    #[serde(default = "default::cache_path")]
    #[serde(skip_serializing_if = "default::is_default_cache_path")]
    cache_path: PathBuf,
    /// The global path
    #[serde(alias = "global_path")]
    #[serde(rename = "globalPath")]
    #[serde(default = "default::global_path")]
    #[serde(skip_serializing_if = "default::is_default_global_path")]
    pub global_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lastupdate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msiextract_use_lessmsi: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scoop_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scoop_repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shim: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_update_log: Option<bool>,
    /// This is the root directory of a Scoop installation, by default the value
    /// is `$HOME/scoop`. If this field has a default value, then it will not be
    /// written into Scoop's config file.
    #[serde(alias = "root_path")]
    #[serde(rename = "rootPath")]
    #[serde(default = "default::root_path")]
    #[serde(skip_serializing_if = "default::is_default_root_path")]
    root_path: PathBuf,
}

/// This private module contains functions of constructing default paths used
/// to create the default Scoop `Config`, with system's environment variables.
mod default {
    use std::path::{Path, PathBuf};

    /// Join the given `path` to `$HOME` and return a new [`PathBuf`].
    #[inline]
    fn home_join<P: AsRef<Path>>(path: P) -> PathBuf {
        dirs::home_dir().map(|p| p.join(path.as_ref())).unwrap()
    }

    /// Check if the given `path` is equal to the `default` one.
    #[inline]
    fn is_default(default: &Path, path: &Path) -> bool {
        path.eq(default)
    }

    /// Get the default Scoop config path.
    #[inline]
    pub(super) fn config_path() -> PathBuf {
        home_join(".config\\scoop\\config.json")
    }

    /// Get the default Scoop root path.
    #[inline]
    pub(super) fn root_path() -> PathBuf {
        home_join("scoop")
    }

    /// Get the default Scoop cache path.
    #[inline]
    pub(super) fn cache_path() -> PathBuf {
        root_path().join("cache")
    }

    /// Get the default Scoop global path.
    #[inline]
    pub(super) fn global_path() -> PathBuf {
        std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .map(|p| p.join("scoop"))
            .unwrap()
    }

    /// Check if the given `path` is equal to the `default` Scoop root path.
    #[inline]
    pub(super) fn is_default_root_path<P: AsRef<Path>>(path: &P) -> bool {
        is_default(root_path().as_path(), path.as_ref())
    }

    /// Check if the given `path` is equal to the `default` Scoop cache path.
    #[inline]
    pub(super) fn is_default_cache_path<P: AsRef<Path>>(path: &P) -> bool {
        is_default(cache_path().as_path(), path.as_ref())
    }

    /// Check if the given `path` is equal to the `default` Scoop global path.
    #[inline]
    pub(super) fn is_default_global_path<P: AsRef<Path>>(path: &P) -> bool {
        is_default(global_path().as_path(), path.as_ref())
    }
}

impl Default for Config {
    /// Create a new Scoop [`Config`] with default values.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// Initialize a Scoop [`Config`]. It will try to read and load the default
    /// config file, which is placed at `$HOME/.config/scoop/config.json`, or
    /// fallback to create a new one with default values when it failed to read
    /// the config file.
    pub fn init() -> Config {
        let default = default::config_path();
        log::debug!("loading '{}'", default.display());
        Self::from_path(default.as_path()).unwrap_or_else(|_| {
            log::warn!("failed to read config file {}", default.display());
            Self::new()
        })
    }

    /// Create a new Scoop [`Config`] with default values.
    pub fn new() -> Config {
        serde_json::from_str("{}").unwrap()
    }

    /// Create a new Scoop [`Config`] with default values.
    pub fn from_path<P: AsRef<Path>>(path: P) -> ScoopResult<Config> {
        let buf = io::BufReader::new(File::open(path.as_ref())?);
        Ok(serde_json::from_reader(buf)?)
    }

    /// Get Scoop's cache path from the [`Config`], by default the value is
    /// `$HOME/scoop/cache`.
    #[inline]
    pub fn cache_path(&self) -> &Path {
        &self.cache_path
    }

    /// Get Scoop's root path from the [`Config`], by default the value is
    /// `$HOME/scoop`.
    #[inline]
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Get Scoop's `apps` directory path from the [`Config`], by default the
    /// value is `$HOME/scoop/apps`.
    #[inline]
    pub fn apps_path(&self) -> PathBuf {
        self.root_path().join("apps")
    }

    /// Get Scoop's buckets path from the [`Config`], by default the value is
    /// `$HOME/scoop/buckets`.
    #[inline]
    pub fn buckets_path(&self) -> PathBuf {
        self.root_path().join("buckets")
    }

    pub fn set<S>(&mut self, key: S, value: S) -> Result<&Config, &'static str>
    where
        S: AsRef<str>,
    {
        let value = value.as_ref();
        match key.as_ref() {
            "7zipextract_use_external" => match value.parse::<bool>() {
                Ok(value) => self.szipextract_use_external = Some(value),
                Err(_) => return Err("invalid config value."),
            },
            "aria2_enabled" => match value.parse::<bool>() {
                Ok(value) => self.aria2_enabled = Some(value),
                Err(_) => return Err("invalid config value."),
            },
            "lastupdate" => self.lastupdate = Some(value.into()),
            "msiextract_use_lessmsi" => match value.parse::<bool>() {
                Ok(value) => self.msiextract_use_lessmsi = Some(value),
                Err(_) => return Err("invalid config value."),
            },
            "proxy" => match value {
                "none" | "null" => self.proxy = None,
                _ => self.proxy = Some(value.to_string()),
            },
            _ => return Err("invalid config key name."),
        }

        Ok(self)
    }

    pub fn unset<S: AsRef<str>>(&mut self, key: S) -> Result<&Config, &'static str> {
        match key.as_ref() {
            "7zipextract_use_external" => self.szipextract_use_external = None,
            "aria2_enabled" => self.aria2_enabled = None,
            "msiextract_use_lessmsi" => self.msiextract_use_lessmsi = None,
            "proxy" => self.proxy = None,
            _ => return Err("invalid config key name."),
        }

        Ok(self)
    }

    pub fn save(&self) {
        // Ensure config directory exists
        crate::util::ensure_dir(self.config_path.parent().unwrap()).unwrap();
        // Then read or create the config file
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.config_path.as_path())
            .unwrap();

        match serde_json::to_writer_pretty(file, self) {
            Ok(_) => trace!("successfully update config to {:?}", self),
            Err(e) => error!("failed to update config. (error {})", e),
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // szipextract_use_external
        if self.szipextract_use_external.is_some() {
            writeln!(
                f,
                "7zipextract_use_external = {}",
                self.szipextract_use_external.unwrap()
            )?;
        }
        // aria2_enabled
        if self.aria2_enabled.is_some() {
            writeln!(f, "aria2-enabled = {}", self.aria2_enabled.unwrap())?;
        }
        // cache_path
        if !default::is_default_cache_path(&self.cache_path) {
            writeln!(f, "cachePath = {}", self.cache_path.display())?;
        }
        // global_path
        if !default::is_default_global_path(&self.global_path) {
            writeln!(f, "globalPath = {}", self.global_path.display())?;
        }
        // lastupdate
        if self.lastupdate.is_some() {
            writeln!(f, "lastupdate = {}", self.lastupdate.as_ref().unwrap())?;
        }
        // msiextract_use_lessmsi
        if self.msiextract_use_lessmsi.is_some() {
            writeln!(
                f,
                "msiextract_use_lessmsi = {}",
                self.msiextract_use_lessmsi.unwrap()
            )?;
        }
        // proxy
        if self.proxy.is_some() {
            writeln!(f, "proxy = {}", self.proxy.as_ref().unwrap())?;
        }
        // scoop_branch
        if self.scoop_branch.is_some() {
            writeln!(f, "scoop_branch = {}", self.scoop_branch.as_ref().unwrap())?;
        }
        // scoop_repo
        if self.scoop_repo.is_some() {
            writeln!(f, "scoop_repo = {}", self.scoop_repo.as_ref().unwrap())?;
        }
        // shim
        if self.shim.is_some() {
            writeln!(f, "shim = {}", self.shim.as_ref().unwrap())?;
        }
        // show_update_log
        if self.show_update_log.is_some() {
            writeln!(f, "show_update_log = {}", self.show_update_log.unwrap())?;
        }
        // root_path
        if !default::is_default_root_path(&self.root_path) {
            writeln!(f, "rootPath = {}", self.root_path.display())?;
        }

        Ok(())
    }
}
