use crate::util::is_admin;
use crate::Result;
use clap::{ArgAction, Parser};
use libscoop::{operation, Session};
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

/// Cleanup apps by removing old versions
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// Given named app(s) to be cleaned up
    #[arg(action = ArgAction::Append)]
    app: Vec<String>,
    /// Clean up all installed apps
    #[arg(short = 'a', long, action = ArgAction::SetTrue)]
    all: bool,
    /// Clean up globally installed apps
    #[arg(short = 'g', long, action = ArgAction::SetTrue)]
    global: bool,
    /// Remove download cache simultaneously
    #[arg(short = 'k', long, action = ArgAction::SetTrue)]
    cache: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    // `arg_required_else_help` only checks that a CLI argument was supplied. Validate that the
    // request names apps or explicitly asks for all apps.
    if args.app.is_empty() && !args.all {
        return Err(anyhow::anyhow!(
            "no app names given. Either give apps to clean, use --all or * wildcard."
        ));
    }

    if args.global && !is_admin() {
        return Err(anyhow::anyhow!(
            "you need admin rights to cleanup global apps"
        ));
    }

    let all_apps = args.all || args.app.iter().any(|app| app == "*");
    let config = session.config();
    let user_apps = config.root_path().join("apps");
    let global_apps = config.global_path().join("apps");
    drop(config);

    // Scoop's `--global --all` includes both scopes; named --global requests target only the
    // global installation. The user root is the only scope for non-global requests.
    let mut app_roots = vec![(user_apps, false)];
    if args.global && all_apps {
        app_roots.push((global_apps, true));
    } else if args.global {
        app_roots.clear();
        app_roots.push((global_apps, true));
    }

    let mut selected = Vec::<(String, PathBuf, bool)>::new();
    let mut seen = HashSet::new();
    if all_apps {
        for (apps_path, global) in &app_roots {
            for name in installed_app_names(apps_path)? {
                if seen.insert((apps_path.clone(), name.clone())) {
                    selected.push((name, apps_path.clone(), *global));
                }
            }
        }
    } else {
        for requested in &args.app {
            let name = match requested.split_once('/') {
                Some((bucket, name)) if is_safe_app_name(bucket) && is_safe_app_name(name) => name,
                None if is_safe_app_name(requested) => requested,
                _ => return Err(anyhow::anyhow!("invalid app name: {requested}")),
            };

            let mut found = false;
            for (apps_path, global) in &app_roots {
                let app_path = apps_path.join(name);
                if app_path.is_dir() {
                    found = true;
                    if seen.insert((apps_path.clone(), name.to_owned())) {
                        selected.push((name.to_owned(), apps_path.clone(), *global));
                    }
                }
            }
            if !found {
                println!("{} was not installed.", requested);
            }
        }
    }

    for (name, apps_path, global) in &selected {
        cleanup(
            name,
            &apps_path.join(name),
            *global,
            !all_apps,
            args.cache,
            session,
        )?;
    }

    if args.cache {
        remove_download_temporary_files(session)?;
    }

    if all_apps {
        println!("Installed app versions are clean.");
        if args.cache {
            println!("Pruned obsolete caches for installed apps and removed partial downloads.");
        } else {
            println!("Download caches were left untouched; use --cache to prune obsolete caches.");
        }
    }

    Ok(())
}

fn installed_app_names(apps_path: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let entries = match fs::read_dir(apps_path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(names),
        Err(error) => return Err(error.into()),
    };

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "scoop" {
            continue;
        }
        // Ignore broken/non-installed app directories when listing all apps. Named requests
        // still report an error from cleanup if their installation metadata cannot be resolved.
        if active_version(&entry.path())?.is_some() {
            names.push(name);
        }
    }

    names.sort_unstable();
    Ok(names)
}

fn cleanup(
    app: &str,
    app_path: &Path,
    global: bool,
    verbose: bool,
    cache: bool,
    session: &Session,
) -> Result<()> {
    if !app_path.is_dir() {
        if verbose {
            println!("{} is not installed.", app);
        }
        return Ok(());
    }

    let active_version = active_version(app_path)?
        .ok_or_else(|| anyhow::anyhow!("could not determine the active version for {app}"))?;
    let mut old_versions = Vec::new();
    for entry in fs::read_dir(app_path)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let version_name = entry.file_name().to_string_lossy().into_owned();
        if version_name == "current" || version_name == active_version.directory_name {
            continue;
        }
        old_versions.push((version_name, entry.path()));
    }

    if old_versions.is_empty() {
        if verbose {
            println!("{} is already clean", app);
        }
    } else {
        print!("Removing {}{}:", app, if global { " (global)" } else { "" });
        for (version, version_path) in old_versions {
            unlink_persist_links(&version_path).map_err(|error| {
                anyhow::anyhow!("failed to unlink persist paths for {app} {version}: {error}")
            })?;
            remove_dir_all::remove_dir_all(&version_path).map_err(|error| {
                anyhow::anyhow!(
                    "failed to remove old version {}: {error}",
                    version_path.display()
                )
            })?;
            print!(" {}", version);
        }
        println!();
    }

    if cache {
        let files = operation::cache_list(session, "*")?;
        for file in files {
            if file.package_name().eq_ignore_ascii_case(app)
                && file.version() != active_version.manifest_version
            {
                fs::remove_file(file.path())?;
            }
        }
    }

    Ok(())
}

struct ActiveVersion {
    directory_name: String,
    manifest_version: String,
}

fn active_version(app_path: &Path) -> Result<Option<ActiveVersion>> {
    let current_path = app_path.join("current");
    let current_manifest = current_path.join("manifest.json");
    if current_manifest.is_file() {
        let active_path = fs::canonicalize(&current_path)?;
        for entry in fs::read_dir(app_path)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if !file_type.is_dir() || file_type.is_symlink() {
                continue;
            }
            if fs::canonicalize(entry.path())? == active_path {
                return Ok(Some(ActiveVersion {
                    directory_name: entry.file_name().to_string_lossy().into_owned(),
                    manifest_version: read_manifest_version(&current_manifest)?,
                }));
            }
        }
        return Err(anyhow::anyhow!(
            "`current` for {} does not resolve to a version directory",
            app_path.display()
        ));
    }

    // If there is no `current` alias (as with no-junction installations), a sole installed
    // version is safe to preserve. With multiple candidates the active version is ambiguous, so
    // refuse to delete anything rather than risk removing it.
    let mut candidates = Vec::new();
    for entry in fs::read_dir(app_path)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let version_path = entry.path();
        let manifest = version_path.join("manifest.json");
        let install_info = version_path.join("install.json");
        if manifest.is_file() && install_info.is_file() {
            candidates.push(ActiveVersion {
                directory_name: entry.file_name().to_string_lossy().into_owned(),
                manifest_version: read_manifest_version(&manifest)?,
            });
        }
    }

    if candidates.len() == 1 {
        return Ok(candidates.pop());
    }
    if candidates.is_empty() {
        return Ok(None);
    }

    Err(anyhow::anyhow!(
        "multiple versions found for {} but no `current` link identifies the active one",
        app_path.display()
    ))
}

fn read_manifest_version(manifest_path: &Path) -> Result<String> {
    let manifest: serde_json::Value = serde_json::from_reader(fs::File::open(manifest_path)?)?;
    manifest
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| anyhow::anyhow!("manifest {} has no version", manifest_path.display()))
}

fn unlink_persist_links(version_path: &Path) -> Result<()> {
    let manifest_path = version_path.join("manifest.json");
    if !manifest_path.is_file() {
        return Ok(());
    }
    let manifest: serde_json::Value = serde_json::from_reader(fs::File::open(manifest_path)?)?;
    let Some(entries) = manifest
        .get("persist")
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(());
    };

    for entry in entries {
        let source = match entry {
            serde_json::Value::String(path) => Some(path.as_str()),
            serde_json::Value::Array(paths) => paths.first().and_then(serde_json::Value::as_str),
            _ => None,
        };
        let Some(source) = source else { continue };
        let relative = Path::new(source);
        if relative.is_absolute()
            || relative.components().any(|part| {
                matches!(
                    part,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(anyhow::anyhow!(
                "unsafe persist path in {}: {source}",
                version_path.display()
            ));
        }

        let link_path = version_path.join(relative);
        let metadata = match fs::symlink_metadata(&link_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if !metadata.file_type().is_symlink() {
            continue;
        }

        #[cfg(windows)]
        {
            if link_path
                .metadata()
                .map(|target| target.is_dir())
                .unwrap_or(false)
            {
                fs::remove_dir(link_path)?;
            } else {
                fs::remove_file(link_path)?;
            }
        }
        #[cfg(not(windows))]
        fs::remove_file(link_path)?;
    }

    Ok(())
}

fn remove_download_temporary_files(session: &Session) -> Result<()> {
    let cache_path = session.config().cache_path().to_owned();
    let entries = match fs::read_dir(cache_path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };

    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".download"))
        {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn is_safe_app_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('\\')
        && !name.contains('/')
        && !name.contains(':')
        && !name.contains("..")
}
