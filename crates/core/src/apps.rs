use crate::fs::leaf;
use crate::utils::compare_versions;
use crate::Result;
use serde::Deserialize;
use std::fs::OpenOptions;
use std::fs::{DirEntry, File};
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug)]
pub struct App {
    pub name: String,
    path: PathBuf,
}

#[derive(Debug)]
pub struct AppManager {
    working_dir: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstallInfo {
    pub architecture: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hold: Option<bool>,
}

impl InstallInfo {
    pub fn hold(&mut self) -> &Self {
        self.hold = Some(true);
        self
    }

    pub fn unhold(&mut self) -> &Self {
        self.hold = None;
        self
    }

    pub fn is_hold(&self) -> bool {
        self.hold == Some(true)
    }
}

impl App {
    pub fn new(path: PathBuf) -> App {
        let name = leaf(path.as_path()).to_string();
        App { name, path }
    }

    pub fn current_version(&self) -> String {
        let versions = self.installed_versions();
        if versions.is_empty() {
            eprintln!("faild to find any version of app '{}'", self.name);
            std::process::exit(1);
        }

        versions.last().unwrap().to_owned()
    }

    pub fn outdated_versions(&self) -> Vec<PathBuf> {
        let mut versions = self
            .installed_versions()
            .into_iter()
            .map(|v| self.path.join(v.as_str()))
            .collect::<Vec<_>>();
        if versions.len() > 0 {
            versions.truncate(versions.len() - 1);
        }

        versions
    }

    pub fn current_install_info(&self) -> Result<InstallInfo> {
        self.install_info_of(self.current_version())
    }

    pub fn install_info_of<S: AsRef<str>>(&self, version: S) -> Result<InstallInfo> {
        let path = self.path.join(version.as_ref()).join("install.json");

        let mut bytes = Vec::new();
        File::open(path)?.read_to_end(&mut bytes)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn hold(&self) -> Result<()> {
        let version = self.current_version();
        let path = self.path.join(version.as_str()).join("install.json");
        let mut cur_info = self.install_info_of(version.as_str())?;
        cur_info.hold();
        self.update_install_info(&path, &cur_info);
        Ok(())
    }

    pub fn unhold(&self) -> Result<()> {
        let version = self.current_version();
        let path = self.path.join(version.as_str()).join("install.json");
        let mut cur_info = self.install_info_of(version.as_str())?;

        if cur_info.is_hold() {
            cur_info.unhold();
            self.update_install_info(&path, &cur_info);
        }

        Ok(())
    }

    fn update_install_info<P>(&self, path: &P, data: &InstallInfo)
    where
        P: AsRef<Path> + ?Sized,
    {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .unwrap();

        serde_json::to_writer_pretty(file, data).unwrap();
    }

    /// Get all installed versions of this app.
    fn installed_versions(&self) -> Vec<String> {
        let mut versions: Vec<String> = self
            .path
            .as_path()
            .read_dir()
            .unwrap()
            .map(|i| i.unwrap())
            .filter(|entry| {
                entry.file_type().unwrap().is_dir()
                    && entry.file_name().to_str().unwrap() != "current"
            })
            .map(|entry| entry.file_name().into_string().unwrap())
            .collect();

        if versions.len() > 1 {
            versions.sort_unstable_by(compare_versions);
        }

        versions
    }
}

impl AppManager {
    /// Create an [`AppsManager`] from the given Scoop [`Config`]
    pub fn new(working_dir: PathBuf) -> AppManager {
        AppManager { working_dir }
    }

    /// Create an [`AppsManager`] and set its working directory to the given
    /// [`PathBuf`].
    ///
    /// Caveats: the constructor does not validate the given PathBuf. Caller
    /// should ensure the path is a valid apps directory.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let working_dir = PathBuf::from(r"C:\Scoop\apps");
    /// let am = AppManager::from(working_dir);
    /// ```
    pub fn from(working_dir: PathBuf) -> AppManager {
        AppManager { working_dir }
    }

    /// Check if app of the given name is installed.
    pub fn is_app_installed<S: AsRef<str>>(&self, name: S) -> bool {
        // transform
        //   `app_name.json`, or
        //   `/path/to/app_name.json`, or
        //   `http(s)://example.com/raw/app_name.json`
        // to `app_name`
        let name = name
            .as_ref()
            .trim_end_matches(".json")
            .split(&['/', '\\'][..])
            .last()
            .unwrap();

        // Here we simply consider the app is installed by checking the app dir
        // exists.
        self.working_dir.as_path().join(name).exists()
    }

    pub fn get_app<S: AsRef<str>>(&self, name: S) -> App {
        let path = self.working_dir.as_path().join(name.as_ref());
        let name = leaf(path.as_path());
        App { path, name }
    }

    pub fn installed_apps(&self) -> Vec<App> {
        self.entries()
            .into_iter()
            .map(|entry| {
                let path = entry.path();
                let name = leaf(path.as_path());
                App { path, name }
            })
            .collect::<Vec<_>>()
    }

    pub fn outdated_app<S: AsRef<str>>(&self, name: S) -> Option<Vec<PathBuf>> {
        if self.is_app_installed(name.as_ref()) {
            let path = self.working_dir.as_path().join(name.as_ref());
            let name = leaf(path.as_path());
            let app = App { path, name };
            return Some(app.outdated_versions());
        }

        None
    }

    pub fn outdated_apps(&self) -> Vec<(String, Vec<PathBuf>)> {
        self.installed_apps()
            .into_iter()
            .map(|a| (a.name.to_string(), a.outdated_versions()))
            .collect::<Vec<_>>()
    }

    // fn uninstall_app<S: AsRef<str>>(&self, name: S) {
    //   if self.is_app_installed(name) {
    //     unimplemented!()
    //   }
    // }

    fn entries(&self) -> Vec<DirEntry> {
        match self.working_dir.as_path().exists() {
            false => vec![], // Return empty vec if `working_dir` is not created.
            true => self
                .working_dir
                .as_path()
                .read_dir()
                .unwrap()
                .map(|x| x.unwrap())
                .filter(|de| {
                    de.file_type().unwrap().is_dir() && de.file_name().to_str().unwrap() != "scoop"
                })
                .collect::<Vec<_>>(),
        }
    }
}
