use std::collections::HashMap;
use std::io::Read;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use crate::error::{Error, Fallible};
use crate::internal::fs::write_json;

/// Builder pattern for generating [`Config`].
pub struct ConfigBuilder {
    /// Path of the config file.
    ///
    /// default is [`default::config_path()`].
    path: PathBuf,
}

impl ConfigBuilder {
    pub fn new() -> ConfigBuilder {
        Self {
            path: default::config_path(),
        }
    }

    pub fn path<P: AsRef<Path>>(&mut self, path: P) -> ConfigBuilder {
        Self {
            path: path.as_ref().to_owned(),
        }
    }

    /// Load the config file from the config path.
    pub fn load(&self) -> Fallible<Config> {
        let mut buf = vec![];
        let path = self.path.clone();

        std::fs::File::open(&path)?.read_to_end(&mut buf)?;

        let inner = serde_json::from_slice(&buf)?;
        let config = Config { path, inner };
        Ok(config)
    }
}

/// Scoop Configuration representation.
///
/// **NOTE**: Not all fields are supported. For the purpose of not erasing unused
/// fields during serialization, they are implemented to be (de)serializable.
/// However, most of them are set to private and transparent during the whole
/// (de)serialization process.
#[derive(Clone, Debug)]
pub struct Config {
    /// The file path of this [`Config`].
    pub path: PathBuf,

    /// Inner config data.
    inner: ConfigInner,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConfigInner {
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
    cache_path: PathBuf,

    #[serde(skip_serializing_if = "Option::is_none")]
    cat_style: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    deafult_architecture: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    debug: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    force_update: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    gh_token: Option<String>,

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

    /// Disable `current` version junction creation.
    ///
    /// The 'current' version alias will not be used. Shims and shortcuts will
    /// point to specific version instead.
    ///
    /// This config was introduced in Jan, 2017 with the name `NO_JUNCTIONS`:
    /// https://github.com/ScoopInstaller/Scoop/commit/a14ffdb5
    ///
    /// It was renamed to `no_junction` in Aug, 2022 (later in release v0.3.0):
    /// https://github.com/ScoopInstaller/Scoop/pull/5116
    #[serde(alias = "no_junctions")]
    #[serde(skip_serializing_if = "Option::is_none")]
    no_junction: Option<bool>,

    /// A list of private hosts.
    ///
    /// # Note
    ///
    /// Array of private hosts that need additional authentication. For example,
    /// if you want to access a private GitHub repository, you need to add the
    /// host to this list with 'match' and 'headers' strings.
    ///
    /// This config was introduced in Feb, 2021:
    /// https://github.com/ScoopInstaller/Scoop/pull/4254
    #[serde(skip_serializing_if = "Option::is_none")]
    private_hosts: Option<Vec<PrivateHosts>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    proxy: Option<String>,

    #[serde(alias = "rootPath")]
    #[serde(default = "default::root_path")]
    #[serde(skip_serializing_if = "default::is_default_root_path")]
    root_path: PathBuf,

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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrivateHosts {
    /// A string defining the host to match.
    #[serde(rename = "match")]
    match_: String,

    /// A string defining HTTP headers.
    headers: String,
}

impl Config {
    /// Initialize the config with default values.
    ///
    /// This function will try to write the default config to the default path,
    /// located in the XDG_CONFIG_HOME directory.
    pub(crate) fn init() -> Config {
        let config = Config::default();
        // try to write the default config to the default path, error is ignored
        let _ = write_json(default::config_path(), &config.inner);
        config
    }

    /// Get the `cache` directory of Scoop.
    #[inline]
    pub fn cache_path(&self) -> &Path {
        self.cache_path.as_path()
    }

    /// Get the root directory of Scoop.
    ///
    /// This is the root directory of a Scoop installation, by default the value
    /// is `$HOME/scoop`. It may be changed by setting the `SCOOP` environment
    /// variable.
    #[inline]
    pub fn root_path(&self) -> &Path {
        self.root_path.as_path()
    }

    /// Get the `no_junction` config.
    #[inline]
    pub fn no_junction(&self) -> bool {
        self.no_junction.unwrap_or_default()
    }

    /// Get the `proxy` setting.
    #[inline]
    pub fn proxy(&self) -> Option<&str> {
        self.proxy.as_deref()
    }

    /// Get the `cat_style` setting.
    #[inline]
    pub fn cat_style(&self) -> &str {
        self.cat_style.as_deref().unwrap_or_default()
    }

    /// Update config key with new value.
    pub(crate) fn set(&mut self, key: &str, value: &str) -> Fallible<()> {
        let is_unset = value.is_empty();
        match key {
            "use_external_7zip" => match is_unset {
                true => self.inner.use_external_7zip = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.use_external_7zip = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "aria2_enabled" | "aria2-enabled" => match is_unset {
                true => self.inner.aria2_enabled = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.aria2_enabled = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "cat_style" => {
                self.inner.cat_style = match is_unset {
                    true => None,
                    false => Some(value.to_string()),
                }
            }
            "gh_token" => {
                self.inner.gh_token = match is_unset {
                    true => None,
                    false => Some(value.to_string()),
                }
            }
            "last_update" => {
                self.inner.last_update = match is_unset {
                    true => None,
                    false => Some(value.to_string()),
                }
            }
            "use_lessmsi" => match is_unset {
                true => self.inner.use_lessmsi = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.use_lessmsi = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "proxy" => match value {
                "" | "none" => self.inner.proxy = None,
                _ => self.inner.proxy = Some(value.to_string()),
            },
            key => return Err(Error::ConfigKeyInvalid(key.to_owned())),
        }

        self.commit()
    }

    /// Commit config changes and save to the config file
    pub(crate) fn commit(&self) -> Fallible<()> {
        write_json(&self.path, &self.inner)
    }

    /// Pretty print the config
    pub(crate) fn pretty(&self) -> Fallible<String> {
        Ok(serde_json::to_string_pretty(&self.inner)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        let inner = ConfigInner {
            alias: Default::default(),
            aria2_enabled: Default::default(),
            aria2_max_connection_per_server: Default::default(),
            aria2_min_split_size: Default::default(),
            aria2_options: Default::default(),
            aria2_retry_wait: Default::default(),
            aria2_split: Default::default(),
            aria2_warning_enabled: Default::default(),
            // default_cache_path: default::cache_path(),
            cache_path: default::cache_path(),
            cat_style: Default::default(),
            deafult_architecture: Default::default(),
            debug: Default::default(),
            force_update: Default::default(),
            gh_token: Default::default(),
            // default_global_path: default::global_path(),
            global_path: default::global_path(),
            ignore_running_processes: Default::default(),
            last_update: Default::default(),
            show_manifest: Default::default(),
            use_lessmsi: Default::default(),
            no_junction: Default::default(),
            private_hosts: Default::default(),
            proxy: Default::default(),
            // default_root_path: default::root_path(),
            root_path: default::root_path(),
            scoop_branch: Default::default(),
            scoop_repo: Default::default(),
            shim: Default::default(),
            show_update_log: Default::default(),
            use_external_7zip: Default::default(),
        };
        Config {
            path: default::config_path(),
            inner,
        }
    }
}

impl Deref for Config {
    type Target = ConfigInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Get a list of possible config paths.
///
/// There are 3 possible locations for the `config.json` file:
///   1) Side-by-side with the real executable (symlink resolved);
///   2) Located in the `root` directory of Scoop;
///   3) Located in the XDG_CONFIG_HOME directory.
pub(crate) fn possible_config_paths() -> Vec<PathBuf> {
    let mut ret = vec![];

    if let Ok(exe_path) = std::env::current_exe() {
        if let Ok(metadata) = std::fs::symlink_metadata(&exe_path) {
            let is_symlink = metadata.is_symlink();
            let mut path = exe_path.clone();

            if is_symlink {
                // since the executable is a symlink, we can use `read_link`
                // to get the real path of the executable
                if let Ok(real_path) = std::fs::read_link(&exe_path) {
                    path = real_path;
                }
            }

            path.pop();
            path.push("config.json");

            // 1) config.json side-by-side with the real executable
            ret.push(path.clone());

            // this pop is ok, it removes `config.json` we just pushed
            // <app>\<current>\<app_name>\apps\<root> (in theory)
            //       ^^^^^^^^^
            path.pop();
            // <app>\<current>\<app_name>\apps\<root> (in theory)
            //                 ^^^^^^^^^^
            if path.pop() {
                // <app>\<current>\<app_name>\apps\<root> (in theory)
                //                            ^^^^
                if path.pop() {
                    // <app>\<current>\<app_name>\apps\<root> (in theory)
                    //                                 ^^^^^
                    if path.pop() {
                        path.push("config.json");

                        // 2) config.json located in the `root` directory of
                        // Scoop, i.e., the portable config.json
                        ret.push(path);
                    }
                }
            }
        }

        // 3) config.json located in the XDG_CONFIG_HOME directory, i.e.,
        // `~/.config/scoop/config.json`
        ret.push(default::config_path());
    }

    ret
}

/// This private module contains functions of constructing default paths used
/// to create the default Scoop `Config`, with system's environment variables.
mod default {
    use std::path::{Path, PathBuf};

    use crate::internal::path::normalize_path;

    /// Join the given `path` to `$HOME` and return a new [`PathBuf`].
    #[inline]
    fn home_join<P: AsRef<Path>>(path: P) -> PathBuf {
        dirs::home_dir().map(|p| p.join(path.as_ref())).unwrap()
    }

    /// Get the default Scoop config path: `$HOME/.config/scoop/config.json`.
    #[inline]
    pub(super) fn config_path() -> PathBuf {
        normalize_path(home_join(".config/scoop/config.json"))
    }

    /// Get the default Scoop root path.
    #[inline]
    pub(super) fn root_path() -> PathBuf {
        let path = if let Some(path) = std::env::var_os("SCOOP") {
            PathBuf::from(path)
        } else {
            home_join("scoop")
        };

        normalize_path(path)
    }

    /// Get the default Scoop cache path.
    #[inline]
    pub(super) fn cache_path() -> PathBuf {
        let path = if let Some(path) = std::env::var_os("SCOOP_CACHE") {
            PathBuf::from(path)
        } else {
            root_path().join("cache")
        };

        normalize_path(path)
    }

    /// Get the default Scoop global path.
    #[inline]
    pub(super) fn global_path() -> PathBuf {
        let path = if let Some(path) = std::env::var_os("SCOOP_GLOBAL") {
            return PathBuf::from(path);
        } else {
            std::env::var_os("ProgramData")
                .map(PathBuf::from)
                .map(|p| p.join("scoop"))
                .unwrap_or(PathBuf::from("C:/ProgramData/scoop"))
        };

        normalize_path(path)
    }

    /// Check if the given `path` is equal to the `default` one.
    #[inline]
    fn is_default(default: &Path, path: &Path) -> bool {
        path.eq(default)
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
