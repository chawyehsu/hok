use std::path::PathBuf;
use crate::config::Config;


pub struct PersistManager {
  pub working_dir: PathBuf,
}

impl PersistManager {
  pub fn new(config: &Config) -> PersistManager {
    let working_dir = PathBuf::from(
      config.get("root_path").unwrap().as_str().unwrap()
    ).join("apps");

    PersistManager { working_dir }
  }
}
