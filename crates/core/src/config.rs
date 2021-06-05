use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::PathBuf;
use std::result::Result;

use log::{error, trace, warn};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(skip)]
    #[serde(default = "default_config::config_path")]
    config_path: PathBuf,
    #[serde(rename = "7zipextract_use_external")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub szipextract_use_external: Option<bool>,
    #[serde(rename = "aria2-enabled")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aria2_enabled: Option<bool>,
    #[serde(rename = "cachePath")]
    #[serde(default = "default_config::cache_path")]
    #[serde(skip_serializing_if = "default_config::is_default_cache_path")]
    pub cache_path: PathBuf,
    #[serde(rename = "globalPath")]
    #[serde(default = "default_config::global_path")]
    #[serde(skip_serializing_if = "default_config::is_default_global_path")]
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
    #[serde(rename = "rootPath")]
    #[serde(default = "default_config::root_path")]
    #[serde(skip_serializing_if = "default_config::is_default_root_path")]
    pub root_path: PathBuf,
}

mod default_config {
    use once_cell::sync::Lazy;
    use std::path::{Path, PathBuf};

    pub(super) fn config_path() -> PathBuf {
        static CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
            dirs::home_dir()
                .map(|p| p.join(".config\\scoop\\config.json"))
                .unwrap()
        });
        CONFIG_PATH.to_path_buf()
    }

    pub(super) fn root_path() -> PathBuf {
        static ROOT_PATH: Lazy<PathBuf> =
            Lazy::new(|| dirs::home_dir().map(|p| p.join("scoop")).unwrap());
        ROOT_PATH.to_path_buf()
    }

    pub(super) fn is_default_root_path<T: AsRef<Path>>(t: &T) -> bool {
        root_path().to_str().unwrap() == t.as_ref().to_str().unwrap()
    }

    pub(super) fn cache_path() -> PathBuf {
        static CACHE_PATH: Lazy<PathBuf> =
            Lazy::new(|| dirs::home_dir().map(|p| p.join("scoop\\cache")).unwrap());
        CACHE_PATH.to_path_buf()
    }

    pub(super) fn is_default_cache_path<T: AsRef<Path>>(t: &T) -> bool {
        cache_path().to_str().unwrap() == t.as_ref().to_str().unwrap()
    }

    pub(super) fn global_path() -> PathBuf {
        static GLOBAL_PATH: Lazy<PathBuf> = Lazy::new(|| {
            std::env::var_os("ProgramData")
                .map(PathBuf::from)
                .map(|p| p.join("scoop"))
                .unwrap()
        });
        GLOBAL_PATH.to_path_buf()
    }

    pub(super) fn is_default_global_path<T: AsRef<Path>>(t: &T) -> bool {
        global_path().to_str().unwrap() == t.as_ref().to_str().unwrap()
    }
}

impl Config {
    pub fn new() -> Config {
        let default = default_config::config_path();
        match File::open(default.as_path()) {
            Ok(file) => {
                let buf = io::BufReader::new(file);
                serde_json::from_reader(buf).unwrap()
            }
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    warn!(
                        "Default Scoop config file {} not found, {}",
                        default.display(),
                        "trying to init new one."
                    );
                } else {
                    error!(
                        "Failed read default config file {} (err: {}), {}",
                        default.display(),
                        err,
                        "fallback to init new one."
                    );
                }
                serde_json::from_str("{}").unwrap()
            }
        }
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
        crate::fs::ensure_dir(self.config_path.parent().unwrap()).unwrap();
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
        if !default_config::is_default_cache_path(&self.cache_path) {
            writeln!(f, "cachePath = {}", self.cache_path.display())?;
        }
        // global_path
        if !default_config::is_default_global_path(&self.global_path) {
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
        if !default_config::is_default_root_path(&self.root_path) {
            writeln!(f, "rootPath = {}", self.root_path.display())?;
        }

        Ok(())
    }
}
