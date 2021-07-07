use crate::config::Config;
use crate::error::ScoopResult;
use std::path::PathBuf;

pub struct AppPersist {
    path: PathBuf,
}

impl AppPersist {
    fn new(path: PathBuf) -> AppPersist {
        AppPersist { path }
    }

    pub fn contains(&self, file: String) -> bool {
        self.path.join(file).exists()
    }
}

pub struct PersistManager {
    pub working_dir: PathBuf,
}

impl PersistManager {
    pub fn new(config: &Config) -> PersistManager {
        let working_dir = config.get_root_path().join("apps");

        PersistManager { working_dir }
    }

    pub fn add(&self, app_name: &str) -> AppPersist {
        let path = self.working_dir.join(app_name);
        AppPersist::new(path)
    }

    /// Purge persistent data of the given `app_name`. All data will be removed.
    pub fn purge(&self, app_name: &str) -> ScoopResult<()> {
        let path = self.working_dir.join(app_name);
        if path.exists() {
            std::fs::remove_dir(path)?;
        }

        Ok(())
    }
}
