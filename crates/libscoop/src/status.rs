use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::bucket::Bucket;
use crate::error::Fallible;
use crate::package::{InstallInfo, Manifest};
use crate::Session;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReport {
    pub scoop_outdated: bool,
    pub buckets_outdated: bool,
    pub network_failure: bool,
    pub entries: Vec<StatusEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEntry {
    pub name: String,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub missing_dependencies: Vec<String>,
    pub flags: Vec<StatusInfoFlag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StatusInfoFlag {
    Outdated,
    InstallFailed,
    Held,
    Deprecated,
    ManifestRemoved,
    MissingDependencies,
}

impl StatusInfoFlag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Outdated => "Outdated",
            Self::InstallFailed => "Install failed",
            Self::Held => "Held package",
            Self::Deprecated => "Deprecated",
            Self::ManifestRemoved => "Manifest removed",
            Self::MissingDependencies => "Missing dependencies",
        }
    }
}

fn normalize_pkg_name(s: &str) -> String {
    s.split_once('/')
        .map(|(_, name)| name)
        .unwrap_or(s)
        .to_ascii_lowercase()
}

pub fn collect_status(session: &Session, local_only: bool) -> Fallible<StatusReport> {
    let mut report = StatusReport {
        scoop_outdated: false,
        buckets_outdated: false,
        network_failure: false,
        entries: vec![],
    };

    if !local_only {
        let config = session.config();
        match check_repo_outdated(config.root_path()) {
            Ok(outdated) => report.scoop_outdated = outdated,
            Err(_) => report.network_failure = true,
        }

        if let Ok(buckets) = crate::bucket::bucket_added(session) {
            for bucket in buckets {
                match check_repo_outdated(bucket.path()) {
                    Ok(outdated) => {
                        if outdated {
                            report.buckets_outdated = true;
                            break;
                        }
                    }
                    Err(_) => report.network_failure = true,
                }
            }
        }
    }

    report.entries = collect_status_entries(session)?;
    Ok(report)
}

fn collect_status_entries(session: &Session) -> Fallible<Vec<StatusEntry>> {
    let mut entries = vec![];
    let mut installed_packages = HashSet::new();

    let config = session.config();
    let apps_dirs = vec![
        config.root_path().join("apps"),
        crate::config::global_path().join("apps"),
    ];

    // First pass
    for apps_dir in &apps_dirs {
        if !apps_dir.exists() { continue; }
        if let Ok(dir_entries) = std::fs::read_dir(apps_dir) {
            for entry in dir_entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let name = normalize_pkg_name(&entry.file_name().to_string_lossy());
                    if name != "scoop" {
                        installed_packages.insert(name);
                    }
                }
            }
        }
    }

    // Second pass
    for apps_dir in &apps_dirs {
        if !apps_dir.exists() { continue; }
        if let Ok(dir_entries) = std::fs::read_dir(apps_dir) {
            for entry in dir_entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let app_name_buf = entry.file_name();
                    let app_name = app_name_buf.to_string_lossy();
                    if app_name.to_ascii_lowercase() == "scoop" { continue; }

                    if let Ok(Some(status_entry)) = collect_status_entry(session, apps_dir, &app_name, &installed_packages) {
                        entries.push(status_entry);
                    }
                }
            }
        }
    }

    Ok(entries)
}

fn collect_status_entry(
    session: &Session,
    apps_dir: &Path,
    app_name: &str,
    installed_packages: &HashSet<String>,
) -> Fallible<Option<StatusEntry>> {
    let app_dir = apps_dir.join(app_name);
    let current_dir = app_dir.join("current");
    let manifest_path = current_dir.join("manifest.json");
    let install_path = current_dir.join("install.json");

    let mut entry = StatusEntry {
        name: app_name.to_owned(),
        installed_version: None,
        latest_version: None,
        missing_dependencies: vec![],
        flags: vec![],
    };

    if !manifest_path.exists() || !install_path.exists() {
        entry.flags.push(StatusInfoFlag::InstallFailed);
        return Ok(Some(entry));
    }

    let manifest = Manifest::parse(&manifest_path)?;
    let install_info = InstallInfo::parse(&install_path).ok();
    
    entry.installed_version = Some(manifest.version().to_owned());
    
    if let Some(info) = &install_info {
        if info.is_held() { entry.flags.push(StatusInfoFlag::Held); }
        
        if let Some(bucket_name) = info.bucket() {
            let bucket_dir = session.config().root_path().join("buckets").join(bucket_name);
            if let Ok(bucket) = Bucket::from(&bucket_dir) {
                if let Some(origin_path) = bucket.path_of_manifest(app_name) {
                    if let Ok(origin_manifest) = Manifest::parse(&origin_path) {
                        if crate::internal::compare_versions(&origin_manifest.version(), &manifest.version()) == std::cmp::Ordering::Greater {
                            entry.latest_version = Some(origin_manifest.version().to_owned());
                            entry.flags.push(StatusInfoFlag::Outdated);
                        }
                    }
                } else {
                    entry.flags.push(StatusInfoFlag::ManifestRemoved);
                }
            }
        }
    }

    entry.missing_dependencies = detect_missing_dependencies(&manifest, installed_packages);
    if !entry.missing_dependencies.is_empty() {
        entry.flags.push(StatusInfoFlag::MissingDependencies);
    }

    if entry.flags.is_empty() { Ok(None) } else { Ok(Some(entry)) }
}

fn check_repo_outdated(repo_path: &Path) -> Fallible<bool> {
    if !repo_path.join(".git").exists() { return Ok(false); }
    let repo = git2::Repository::open(repo_path)?;
    let mut remote = repo.find_remote("origin")?;
    let head = repo.head()?;
    let branch = head.shorthand().unwrap_or("main");
    remote.fetch(&[branch], None, None)?;
    let head_id = head.target().unwrap();
    let origin_id = repo.revparse_single(&format!("origin/{}", branch))?.id();
    Ok(head_id != origin_id)
}
fn detect_missing_dependencies(
    manifest: &Manifest,
    installed: &HashSet<String>,
) -> Vec<String> {
    let deps: Vec<String> = manifest.dependencies().iter()
        .map(|d| normalize_pkg_name(d))
        .filter(|dep_name| {
            // Filter out hook-based dependencies that Scoop status ignores
            let ignored = ["lessmsi", "innounp", "7zip", "dark"];
            !ignored.contains(&dep_name.as_str()) && !installed.contains(dep_name)
        })
        .collect();

    deps
}


