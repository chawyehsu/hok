use crate::{error::Fallible, internal, package::Package, Event, Session};

/// Import PowerShell module for a given package.
///
/// This creates a symlink from the Scoop `modules` directory to the package's
/// psmodule directory.
pub fn add(session: &Session, package: &Package) -> Fallible<()> {
    if let Some(psmodule) = package.manifest().psmodule() {
        internal::fs::ensure_dir(&session.config().root_path().join("modules"))?;

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackagePsModuleRemoveStart(
                psmodule.name().to_owned(),
            ));
        }

        let config = session.config();
        let version = if config.no_junction() {
            package.version()
        } else {
            "current"
        };

        let module_path = config.root_path().join("modules").join(psmodule.name());

        let app_path = config
            .root_path()
            .join("apps")
            .join(package.name())
            .join(version);

        // Remove existing module symlink/directory if it exists
        let _ = std::fs::remove_dir_all(&module_path);
        let _ = std::fs::remove_dir(&module_path);

        internal::fs::symlink_dir(&app_path, &module_path)?;

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackagePsModuleRemoveDone);
        }
    }
    Ok(())
}

/// Remove PowerShell module imported by a given package.
pub fn remove(session: &Session, package: &Package) -> Fallible<()> {
    assert!(package.is_installed());

    if let Some(psmodule) = package.manifest().psmodule() {
        let config = session.config();
        let mut psmodule_path = config.root_path().join("modules");

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackagePsModuleRemoveStart(
                psmodule.name().to_owned(),
            ));
        }

        psmodule_path.push(psmodule.name());
        let _ = std::fs::remove_dir(psmodule_path);

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackagePsModuleRemoveDone);
        }
    }
    Ok(())
}
