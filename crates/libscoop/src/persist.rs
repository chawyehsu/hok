use crate::{error::Fallible, internal, package::Package, Session};

/// Remove PowerShell module imported by a given package.
pub fn unlink(session: &Session, package: &Package) -> Fallible<()> {
    assert!(package.is_installed());

    if let Some(persists) = package.manifest().persist() {
        let config = session.config();
        let mut app_path = config.root_path().join("apps");
        app_path.push(package.name());

        let version = if config.no_junction() {
            package.installed_version().unwrap()
        } else {
            "current"
        };

        let persist_path = app_path.join(version);
        for persist in persists {
            assert!(!persist.is_empty());

            let src = internal::path::normalize_path(persist_path.join(persist[0]));
            internal::fs::remove_symlink(src)?;
        }
    }
    Ok(())
}
