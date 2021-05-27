use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

use log::{warn, trace, error};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

// config file of the original scoop
static OLD_CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
  dirs::home_dir().map(|p|
    p.join(".config\\scoop\\config.json")).unwrap()
});
// config file of scoop-rs
static RS_CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
  dirs::home_dir().map(|p|
    p.join(".config\\scoop\\rs.config.json")).unwrap()
});
// default paths of scoop
static ROOT_PATH: Lazy<PathBuf> = Lazy::new(|| {
  dirs::home_dir().map(|p| p.join("scoop")).unwrap()
});
static CACHE_PATH: Lazy<PathBuf> = Lazy::new(|| {
  dirs::home_dir().map(|p| p.join("scoop\\cache")).unwrap()
});
static GLOBAL_PATH: Lazy<PathBuf> = Lazy::new(|| {
  std::env::var_os("ProgramData").map(PathBuf::from)
    .map(|p| p.join("scoop")).unwrap()
});
static ALLOWED_CONFIG_KEYS: &[&str] = &[
  "proxy", "root_path", "cache_path", "global_path"
];

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
  inner: Value,
}

impl Default for Config {
  fn default() -> Self {
    // @deprecated, scoop-rs
    let root_path = std::env::var_os("SCOOP")
      .map(PathBuf::from)
      .filter(|p| p.is_absolute())
      .unwrap_or(ROOT_PATH.to_path_buf())
      .to_str().unwrap().to_string();
    // @deprecated, scoop-rs
    let cache_path = std::env::var_os("SCOOP_CACHE")
      .map(PathBuf::from)
      .filter(|p| p.is_absolute())
      .unwrap_or(CACHE_PATH.to_path_buf())
      .to_str().unwrap().to_string();
    // @deprecated, scoop-rs
    let global_path = std::env::var_os("SCOOP_GLOBAL")
      .map(PathBuf::from)
      .filter(|p| p.is_absolute())
      .unwrap_or(GLOBAL_PATH.to_owned())
      .to_str().unwrap().to_string();

    let mut default: Map<String, Value> = Map::new();
    default.insert("proxy".to_string(), Value::Null);
    default.insert("root_path".to_string(), Value::String(root_path));
    default.insert("cache_path".to_string(), Value::String(cache_path));
    default.insert("global_path".to_string(), Value::String(global_path));

    Config { inner: Value::Object(default) }
  }
}

impl Config {
  pub fn new() -> Config {
    if !RS_CONFIG_PATH.exists() {
      return Self::from_old();
    }

    Self::from_path_or_default(RS_CONFIG_PATH.as_path())
  }

  fn from_old() -> Config {
    Self::from_path_or_default(OLD_CONFIG_PATH.as_path())
  }

  fn from_path_or_default<P: AsRef<Path>>(path: P) -> Config {
    let path_str = path.as_ref().to_str().unwrap();
    let file = File::open(path.as_ref());
    let mut config = Self::default();

    if file.is_err() {
      warn!("could not read file {}", path_str);
      return config;
    }

    match serde_json::from_reader(file.unwrap()) {
      Ok(data) => {
        let data = match data {
          Value::Object(value) => {
            value.into_iter()
              .map(|(k, v)| (k.to_ascii_lowercase(), v))
              .filter(|(k, _)| Self::validate(k))
              .collect::<Map<String, Value>>()
          },
          _ => {
            error!("invalid format of config file {}", path_str);
            Map::new()
          }
        };

        for (k, v) in data {
          config.set(k, v.as_str().unwrap().to_string());
        }
      },
      Err(_) => {
        error!("failed to parse config file {}", path_str);
      }
    }

    return config;
  }

  fn validate<S: AsRef<str>>(key: S) -> bool {
    ALLOWED_CONFIG_KEYS.contains(&key.as_ref())
  }

  pub fn get<S: AsRef<str>>(&self, key: S) -> Option<&Value> {
    self.inner.get(key.as_ref())
  }

  pub fn get_all(&self) -> &Value {
    &self.inner
  }

  pub fn set<S: AsRef<str>>(&mut self, key: S, value: S) -> &Config {
    if Self::validate(key.as_ref()) {
      let value = match value.as_ref() {
        v if v == "true" || v == "false" => Value::Bool(v.parse::<bool>().unwrap()),
        v if v == "null" || v == "none" => Value::Null,
        v => Value::String(v.to_string()),
      };

      self.inner.as_object_mut().unwrap().insert(
        key.as_ref().to_string(), value
      );
    } else {
      error!("invalid config key name '{}'", key.as_ref());
      std::process::exit(1);
    }

    self
  }

  pub fn remove<S: AsRef<str>>(&mut self, key: S) -> &Config {
    self.set(key.as_ref(), "null");
    self
  }

  pub fn save(&self) {
    // Ensure config directory exists
    crate::fs::ensure_dir(RS_CONFIG_PATH.parent().unwrap()).unwrap();
    // Then read or create the config file
    let file = OpenOptions::new()
      .write(true).create(true).truncate(true).open(RS_CONFIG_PATH.as_path());

    match file {
      Ok(file) => {
        let mut data = self.inner.clone();

        if data["proxy"].is_null() {
          data.as_object_mut().unwrap().remove("proxy");
        }

        if data["root_path"].is_string() {
          let val = data["root_path"].as_str().unwrap();
          if val == ROOT_PATH.to_str().unwrap().to_string() {
            data.as_object_mut().unwrap().remove("root_path");
          }
        }

        if data["cache_path"].is_string() {
          let val = data["cache_path"].as_str().unwrap();
          if val == CACHE_PATH.to_str().unwrap().to_string() {
            data.as_object_mut().unwrap().remove("cache_path");
          }
        }

        if data["global_path"].is_string() {
          let val = data["global_path"].as_str().unwrap();
          if val == GLOBAL_PATH.to_str().unwrap().to_string() {
            data.as_object_mut().unwrap().remove("global_path");
          }
        }

        match serde_json::to_writer_pretty(file, &data) {
          Ok(_) => trace!("successfully update config to {}", data),
          Err(e) => error!("failed to update config. (error {})", e)
        };
      },
      Err(_) => error!("failed to open config file {}", RS_CONFIG_PATH.to_string_lossy())
    }
  }
}
