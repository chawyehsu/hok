use crate::{error::Fallible, package::Package, Event, Session};

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
