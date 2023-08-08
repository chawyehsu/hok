use once_cell::sync::Lazy;
use std::path::PathBuf;

use crate::{error::Fallible, internal, package::Package, Event, Session};

static SCOOP_SHORTCUT_DIR: Lazy<PathBuf> = Lazy::new(shortcut_dir);

/// Return the path to the shortcut directory.
///
/// `~\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Scoop Apps`
fn shortcut_dir() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap();
    dir.push("Microsoft/Windows/Start Menu/Programs/Scoop Apps");
    internal::path::normalize_path(dir)
}

/// Remove shortcut(s) for a given package.
pub fn remove(session: &Session, package: &Package) -> Fallible<()> {
    assert!(package.is_installed());

    if let Some(shortcuts) = package.manifest().shortcuts() {
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShortcutRemoveStart);
        }

        for shortcut in shortcuts {
            let length = shortcut.len();
            assert!(length > 1);

            let mut path = SCOOP_SHORTCUT_DIR.join(shortcut[1]);
            path.set_extension("lnk");

            if let Some(tx) = session.emitter() {
                let shortcut_name = path.file_name().unwrap().to_str().unwrap().to_owned();
                let _ = tx.send(Event::PackageShortcutRemoveProgress(shortcut_name));
            }

            let _ = std::fs::remove_file(&path);
        }

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShortcutRemoveDone);
        }
    }
    Ok(())
}
