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
