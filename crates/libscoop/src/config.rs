use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::{Error, Fallible};
use crate::internal::fs::write_json;

pub struct ConfigBuilder {
    path: PathBuf,
}

/// Scoop Configuration representation.
///
/// **NOTE**: Not all fields are supported. For the purpose of not erasing unused
/// fields during serialization, they are implemented to be (de)serializable.
/// However, most of them are set to private and transparent during the whole
/// (de)serialization process.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    alias: Option<HashMap<String, String>>,
    #[serde(alias = "aria2_enabled")]
    #[serde(rename = "aria2-enabled")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_enabled: Option<bool>,
    #[serde(alias = "aria2_max_connection_per_server")]
    #[serde(rename = "aria2-max-connection-per-server")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_max_connection_per_server: Option<u32>,
    #[serde(alias = "aria2_min_split_size")]
    #[serde(rename = "aria2-min-split-size")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_min_split_size: Option<String>,
    #[serde(alias = "aria2_options")]
    #[serde(rename = "aria2-options")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_options: Option<String>,
    #[serde(alias = "aria2_retry_wait")]
    #[serde(rename = "aria2-retry-wait")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_retry_wait: Option<u32>,
    #[serde(alias = "aria2_split")]
    #[serde(rename = "aria2-split")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_split: Option<u32>,
    #[serde(alias = "aria2_warning_enabled")]
    #[serde(rename = "aria2-warning-enabled")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_warning_enabled: Option<bool>,
    #[serde(alias = "cachePath")]
    #[serde(default = "default::cache_path")]
    #[serde(skip_serializing_if = "default::is_default_cache_path")]
    pub cache_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    cat_style: Option<String>,
    /// Path of the config file. Default is `$HOME/.config/scoop/config.json`.
    #[serde(skip)]
    #[serde(default = "default::config_path")]
    pub config_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    deafult_architecture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    force_update: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gh_token: Option<String>,
    /// The global path
    #[serde(alias = "globalPath")]
    #[serde(default = "default::global_path")]
    #[serde(skip_serializing_if = "default::is_default_global_path")]
    global_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    ignore_running_processes: Option<bool>,
    #[serde(alias = "lastupdate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    last_update: Option<String>,
    #[serde(alias = "manifest_review")]
    #[serde(skip_serializing_if = "Option::is_none")]
    show_manifest: Option<bool>,
    #[serde(alias = "msiextract_use_lessmsi")]
    #[serde(skip_serializing_if = "Option::is_none")]
    use_lessmsi: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_junctions: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy: Option<String>,
    /// This is the root directory of a Scoop installation, by default the value
    /// is `$HOME/scoop`.
    #[serde(alias = "rootPath")]
    #[serde(default = "default::root_path")]
    #[serde(skip_serializing_if = "default::is_default_root_path")]
    pub root_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    scoop_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scoop_repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shim: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    show_update_log: Option<bool>,
    #[serde(alias = "7zipextract_use_external")]
    #[serde(skip_serializing_if = "Option::is_none")]
    use_external_7zip: Option<bool>,
}

impl ConfigBuilder {
    pub fn new<P: AsRef<Path>>(path: P) -> ConfigBuilder {
        let path = path.as_ref().to_owned();
        Self { path }
    }

    pub fn build(&self) -> Fallible<Config> {
        let mut buf = vec![];
        std::fs::File::open(self.path.as_path())?
            .read_to_end(&mut buf)
            .unwrap_or_else(|_| -> usize {
                buf = "{}".as_bytes().to_vec();
                0
            });
        Ok(serde_json::from_slice(&buf)?)
    }
}

impl Config {
    #[inline]
    pub fn root_path(&self) -> &Path {
        self.root_path.as_path()
    }

    #[inline]
    pub fn proxy(&self) -> Option<&str> {
        self.proxy.as_deref()
    }

    #[inline]
    pub fn cat_style(&self) -> &str {
        self.cat_style.as_deref().unwrap_or_default()
    }

    /// Update config key with new value.
    pub(crate) fn set(&mut self, key: &str, value: &str) -> Fallible<()> {
        let is_unset = value.is_empty();
        match key {
            "use_external_7zip" => match is_unset {
                true => self.use_external_7zip = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.use_external_7zip = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "aria2_enabled" | "aria2-enabled" => match is_unset {
                true => self.aria2_enabled = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.aria2_enabled = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "cat_style" => {
                self.cat_style = match is_unset {
                    true => None,
                    false => Some(value.to_string()),
                }
            }
            "gh_token" => {
                self.gh_token = match is_unset {
                    true => None,
                    false => Some(value.to_string()),
                }
            }
            "last_update" => {
                self.last_update = match is_unset {
                    true => None,
                    false => Some(value.to_string()),
                }
            }
            "use_lessmsi" => match is_unset {
                true => self.use_lessmsi = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.use_lessmsi = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "proxy" => match value {
                "" | "none" => self.proxy = None,
                _ => self.proxy = Some(value.to_string()),
            },
            key => return Err(Error::ConfigKeyInvalid(key.to_owned())),
        }

        self.commit()
    }

    /// Commit config changes and save to the config file
    pub(crate) fn commit(&self) -> Fallible<()> {
        write_json(&self.config_path, self)
    }

    /// Pretty print the config
    pub(crate) fn pretty(&self) -> Fallible<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

pub(crate) fn default_config_path() -> PathBuf {
    default::config_path()
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
        home_join(".config/scoop/config.json")
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
    pub(super) fn is_default_root_path<P: AsRef<Path>>(path: P) -> bool {
        is_default(root_path().as_path(), path.as_ref())
    }

    /// Check if the given `path` is equal to the `default` Scoop cache path.
    #[inline]
    pub(super) fn is_default_cache_path<P: AsRef<Path>>(path: P) -> bool {
        is_default(cache_path().as_path(), path.as_ref())
    }

    /// Check if the given `path` is equal to the `default` Scoop global path.
    #[inline]
    pub(super) fn is_default_global_path<P: AsRef<Path>>(path: P) -> bool {
        is_default(global_path().as_path(), path.as_ref())
    }
}
