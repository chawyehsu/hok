use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};

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

/// Create a Windows `.lnk` shortcut using PowerShell.
fn create_lnk(shortcut_path: &Path, target: &Path, args: Option<&str>) -> Fallible<()> {
    #[cfg(windows)]
    {
        let target_str = target.to_string_lossy().replace('\\', "\\\\");
        let args_str = args.unwrap_or("");
        let shortcut_str = shortcut_path.to_string_lossy().replace('\\', "\\\\");

        let ps_cmd = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
            $sc = $ws.CreateShortcut('{}'); \
            $sc.TargetPath = '{}'; \
            $sc.Arguments = '{}'; \
            $sc.Save()",
            shortcut_str, target_str, args_str
        );

        let output = std::process::Command::new("powershell.exe")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(&ps_cmd)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::Custom(format!(
                "failed to create shortcut '{}': {}",
                shortcut_path.display(),
                stderr
            )));
        }

        Ok(())
    }

    #[cfg(unix)]
    {
        // On Unix, create a symlink as a simple shortcut
        let _ = std::fs::remove_file(shortcut_path);
        std::os::unix::fs::symlink(target, shortcut_path)?;
        Ok(())
    }

    #[cfg(not(any(windows, unix)))]
    {
        Err(crate::Error::Custom(
            "unsupported platform for shortcut creation".to_string(),
        ))
    }
}

/// Add shortcut(s) for a given package.
///
/// Creates `.lnk` shortcuts in the user's Start Menu for each shortcut defined
/// in the package's `shortcuts` manifest field.
pub fn add(session: &Session, package: &Package) -> Fallible<()> {
    if let Some(shortcuts) = package.manifest().shortcuts() {
        internal::fs::ensure_dir(&*SCOOP_SHORTCUT_DIR)?;

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShortcutRemoveStart);
        }

        let config = session.config();
        let version = if config.no_junction() {
            package.installed_version().unwrap_or(package.version())
        } else {
            "current"
        };

        let app_path = config.root_path().join("apps").join(package.name());

        for shortcut in shortcuts {
            assert!(!shortcut.is_empty());

            let target = app_path.join(version).join(shortcut[0]);

            let name = if shortcut.len() > 1 {
                shortcut[1]
            } else {
                internal::path::leaf_base(&target).unwrap_or(package.name())
            };

            let mut path = SCOOP_SHORTCUT_DIR.join(name);
            path.set_extension("lnk");

            create_lnk(&path, &target, None)?;
        }

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShortcutRemoveDone);
        }
    }

    Ok(())
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
