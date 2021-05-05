pub mod app;
pub mod bucket;
pub mod cache;
pub mod config;
pub mod git;
pub mod manifest;
pub mod utils;

use dirs;
use std::env;
use std::path::PathBuf;
use serde_json::Value;

#[derive(Debug)]
pub struct Scoop {
  pub config: Value,
  pub root_dir: PathBuf,
  pub cache_dir: PathBuf,
  pub global_dir: PathBuf,
  pub apps_dir: PathBuf,
  pub buckets_dir: PathBuf,
  pub modules_dir: PathBuf,
  pub persist_dir: PathBuf,
  pub shims_dir: PathBuf
}

impl Scoop {
  pub fn from_cfg(config: Value) -> Scoop {
    let root_dir: PathBuf = config["rootPath"]
      .as_str()
      .map(PathBuf::from)
      .unwrap_or_else(
        || env::var_os("SCOOP")
          .map(PathBuf::from).filter(|p| p.is_absolute())
          .unwrap_or_else(
            || dirs::home_dir().map(|p| p.join("scoop")).unwrap()
        )
      );

    let cache_dir: PathBuf = config["cachePath"]
      .as_str()
      .map(PathBuf::from)
      .unwrap_or_else(
        || env::var_os("SCOOP_CACHE")
          .map(PathBuf::from).filter(|p| p.is_absolute())
          .unwrap_or_else(
            || dirs::home_dir().map(|p| p.join("scoop\\cache")).unwrap()
        )
      );

    let global_dir: PathBuf = config["cachePath"]
      .as_str()
      .map(PathBuf::from)
      .unwrap_or_else(
        || env::var_os("SCOOP_GLOBAL")
          .map(PathBuf::from).filter(|p| p.is_absolute())
          .unwrap_or_else(
            || env::var_os("ProgramData")
              .map(PathBuf::from).map(|p| p.join("scoop")).unwrap()
        )
      );

    let apps_dir: PathBuf = root_dir.join("apps");
    let buckets_dir: PathBuf = root_dir.join("buckets");
    let modules_dir: PathBuf = root_dir.join("modules");
    let persist_dir: PathBuf = root_dir.join("persist");
    let shims_dir: PathBuf = root_dir.join("shims");

    Scoop {
      config, root_dir, cache_dir, global_dir, apps_dir,
      buckets_dir, modules_dir, persist_dir, shims_dir
    }
  }
}
