use std::path::PathBuf;
use crate::config::Config;


pub struct PersistManager {
  pub working_dir: PathBuf,
}

impl PersistManager {
  pub fn new(config: &Config) -> PersistManager {
    let working_dir = config.root_path.join("apps");

    PersistManager { working_dir }
  }
}
