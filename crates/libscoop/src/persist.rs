use crate::{error::Fallible, internal, package::Package, Session};

/// Remove persisted files/directories symlinks for a package.
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

/// Create persisted files/directories symlinks for a package.
///
/// This moves data that should persist across updates to a `persist` directory
/// and creates symlinks from the app's install directory to the persist directory.
pub fn link(session: &Session, package: &Package) -> Fallible<()> {
    if let Some(persists) = package.manifest().persist() {
        let config = session.config();
        let mut app_path = config.root_path().join("apps");
        app_path.push(package.name());

        let version = if config.no_junction() {
            package.version()
        } else {
            "current"
        };

        let persist_root = config.root_path().join("persist").join(package.name());
        let app_version_path = app_path.join(version);

        for persist in persists {
            assert!(!persist.is_empty());

            // The persist entry can be either a file or directory path
            let rel_path = &persist[0];
            let app_path_item = app_version_path.join(rel_path);
            let persist_path_item = persist_root.join(rel_path);

            if app_path_item.is_dir() {
                // Move the directory to persist and create a symlink
                internal::fs::ensure_dir(persist_path_item.parent().unwrap())?;
                std::fs::rename(&app_path_item, &persist_path_item)?;
                internal::fs::symlink_dir(&persist_path_item, &app_path_item)?;
            } else if app_path_item.is_file() {
                // Move the file to persist and create a symlink
                internal::fs::ensure_dir(persist_path_item.parent().unwrap())?;
                std::fs::rename(&app_path_item, &persist_path_item)?;
                internal::fs::symlink_file(&persist_path_item, &app_path_item)?;
            } else {
                // The file/directory doesn't exist yet, create parents
                internal::fs::ensure_dir(persist_path_item.parent().unwrap())?;
                // Create a symlink from app dir to persist dir (may be created later)
                if persist_path_item.is_dir() {
                    internal::fs::remove_symlink(&app_path_item)?;
                    internal::fs::symlink_dir(&persist_path_item, &app_path_item)?;
                } else if persist_path_item.is_file() {
                    internal::fs::remove_symlink(&app_path_item)?;
                    internal::fs::symlink_file(&persist_path_item, &app_path_item)?;
                }
            }
        }
    }
    Ok(())
}
