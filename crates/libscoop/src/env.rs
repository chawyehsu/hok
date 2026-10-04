use crate::{config, error::Fallible, internal, package::Package, Error, Event, Session};

/// Unset all environment variables defined by a given package.
pub fn remove(session: &Session, package: &Package) -> Fallible<()> {
    assert!(package.is_installed());

    // Unset environment variables
    if let Some(env_set) = package.manifest().env_set() {
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageEnvVarRemoveStart);
        }

        let keys = env_set.keys();
        for key in keys {
            internal::env::set(key, None)?;
        }

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageEnvVarRemoveDone);
        }
    }

    // Remove environment path
    if let Some(env_add_path) = package.manifest().env_add_path() {
        let config = session.config();
        let env_path_name = match config.use_isolated_path() {
            Some(config::IsolatedPath::Named(name)) => name.to_owned(),
            Some(config::IsolatedPath::Boolean(true)) => "SCOOP_PATH".to_owned(),
            _ => "PATH".to_owned(),
        };
        let mut paths = internal::env::get_path_like_env(&env_path_name)?;
        let mut app_path = config.root_path().join("apps");
        app_path.push(package.name());

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageEnvPathRemoveStart);
        }

        let version = if config.no_junction() {
            package.installed_version().unwrap()
        } else {
            "current"
        };

        let env_add_path = env_add_path
            .into_iter()
            .map(|p| internal::path::normalize_path(app_path.join(version).join(p)))
            .collect::<Vec<_>>();

        paths.retain(|p| !env_add_path.contains(p));

        let updated = std::env::join_paths(paths).map_err(|e| Error::Custom(e.to_string()))?;

        internal::env::set(&env_path_name, Some(&updated))?;

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageEnvPathRemoveDone);
        }
    }

    Ok(())
}

/// Set environment variables and path entries for a given package during installation.
pub fn install(session: &Session, package: &Package) -> Fallible<()> {
    let config = session.config();

    // Set environment variables
    if let Some(env_set) = package.manifest().env_set() {
        for (key, value) in env_set {
            let resolved = resolve_env_value(session, package, value);
            let os_value = std::ffi::OsString::from(resolved);
            internal::env::set(key, Some(&os_value))?;
        }
    }

    // Add to PATH
    if let Some(env_add_path) = package.manifest().env_add_path() {
        let env_path_name = match config.use_isolated_path() {
            Some(config::IsolatedPath::Named(name)) => name.to_owned(),
            Some(config::IsolatedPath::Boolean(true)) => "SCOOP_PATH".to_owned(),
            _ => "PATH".to_owned(),
        };

        let mut paths = internal::env::get_path_like_env(&env_path_name)?;
        let mut app_path = config.root_path().join("apps");
        app_path.push(package.name());

        let version = if config.no_junction() {
            package.version()
        } else {
            "current"
        };

        for p in env_add_path {
            let path_entry = internal::path::normalize_path(app_path.join(version).join(p));
            if !paths.contains(&path_entry) {
                paths.push(path_entry);
            }
        }

        let updated = std::env::join_paths(paths).map_err(|e| Error::Custom(e.to_string()))?;
        internal::env::set(&env_path_name, Some(&updated))?;
    }

    Ok(())
}

/// Resolve environment variable values that contain placeholders.
///
/// Scoop supports the following placeholders in env_set values:
/// - `$env:SCOOP` - replaced with the Scoop root path
/// - `$env:HOME` - replaced with the user's home directory
/// - `$env:USERPROFILE` - replaced with the user's profile directory
fn resolve_env_value(session: &Session, package: &Package, value: &str) -> String {
    let config = session.config();
    let root_path = config.root_path().to_string_lossy();

    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    value
        .replace("$env:SCOOP", &root_path)
        .replace("$env:HOME", &home)
        .replace("$env:USERPROFILE", &home)
}
