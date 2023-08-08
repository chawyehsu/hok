use crate::{error::Fallible, internal, package::Package, Event, Session};

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
            internal::env::set(key, "")?;
        }

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageEnvVarRemoveDone);
        }
    }

    // Remove environment path
    if let Some(env_add_path) = package.manifest().env_add_path() {
        let mut env_path_list = internal::env::get_env_path_list()?;
        let config = session.config();
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

        let paths = env_add_path
            .into_iter()
            .map(|p| {
                internal::path::normalize_path(app_path.join(version).join(p))
                    .to_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();

        env_path_list.retain(|p| !paths.contains(p));

        internal::env::set("PATH", &env_path_list.join(";"))?;

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageEnvPathRemoveDone);
        }
    }

    Ok(())
}
