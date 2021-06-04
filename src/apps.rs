use anyhow::Result;
use std::fs::DirEntry;
use std::path::PathBuf;
use serde::Deserialize;
use crate::fs;
use crate::utils::compare_versions;

#[derive(Debug)]
pub struct App {
  pub name: String,
  path: PathBuf,
}

#[derive(Debug)]
pub struct AppManager {
  working_dir: PathBuf
}

#[derive(Debug, Deserialize)]
pub struct InstallInfo {
  pub architecture: String,
  pub bucket: Option<String>,
  pub url: Option<String>,
  pub hold: Option<bool>
}

impl App {
  pub fn new(path: PathBuf) -> App {
    let name = fs::leaf(path.as_path()).to_string();
    App { name, path, }
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
    let mut versions = self.installed_versions()
      .into_iter().map(|v| self.path.join(v.as_str()))
      .collect::<Vec<_>>();
    if versions.len() > 0 {
      versions.truncate(versions.len() - 1);
    }

    versions
  }

  pub fn current_install_info(&self) -> Result<InstallInfo> {
    let file = std::fs::File::open(
      self.path.join(self.current_version()).join("install.json")
    )?;

    Ok(serde_json::from_reader(file)?)
  }

  pub fn install_info_of<S: AsRef<str>>(&self, version: S) -> Result<InstallInfo> {
    let file = std::fs::File::open(
      self.path.join(version.as_ref()).join("install.json")
    )?;

    Ok(serde_json::from_reader(file)?)
  }

  fn installed_versions(&self) -> Vec<String> {
    let err = format!("failed to read directory '{}'",
      self.path.as_path().display());
    let entries = self.path.as_path().read_dir()
      .expect(err.as_str());

    let mut versions: Vec<String> = entries.filter_map(Result::ok)
      .filter(|x| x.metadata().unwrap().is_dir() &&
        x.file_name().to_str().unwrap() != "current")
      .map(|y| y.file_name().into_string().unwrap())
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
    let name = name.as_ref().trim_end_matches(".json")
      .split(&['/', '\\'][..]).last().unwrap();

    // Here we simply consider the app is installed by checking the app dir
    // exists.
    self.working_dir.as_path().join(name).exists()
  }

  pub fn get_app<S: AsRef<str>>(&self, name: S) -> App {
    let path = self.working_dir.as_path().join(name.as_ref());
    let name = fs::leaf(path.as_path());
    App { path, name, }
  }

  pub fn installed_apps(&self) -> Vec<App> {
    self.entries().into_iter().map(|entry| {
      let path = entry.path();
      let name = fs::leaf(path.as_path());
      App { path, name, }
    }).collect::<Vec<_>>()
  }

  pub fn outdated_app<S: AsRef<str>>(&self, name: S) -> Option<Vec<PathBuf>> {
    if self.is_app_installed(name.as_ref()) {
      let path = self.working_dir.as_path().join(name.as_ref());
      let name = fs::leaf(path.as_path());
      let app = App { path, name, };
      return Some(app.outdated_versions());
    }

    None
  }

  pub fn outdated_apps(&self) -> Vec<(String, Vec<PathBuf>)> {
    self.installed_apps().into_iter().map(|a| (
      a.name.to_string(), a.outdated_versions()
    )).collect::<Vec<_>>()
  }

  // fn uninstall_app<S: AsRef<str>>(&self, name: S) {
  //   if self.is_app_installed(name) {
  //     unimplemented!()
  //   }
  // }

  fn entries(&self) -> Vec<DirEntry> {
    let err = format!("failed to read directory '{}'",
      self.working_dir.as_path().display());
    let entries = self.working_dir.as_path().read_dir()
      .expect(err.as_str());

    entries.filter_map(Result::ok)
      .filter(|x| x.metadata().unwrap().is_dir() &&
        x.file_name().to_str().unwrap() != "scoop")
      .collect::<Vec<_>>()
  }
}
