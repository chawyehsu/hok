use std::path::PathBuf;

use crate::error::Fallible;

#[cfg(unix)]
pub use unix::{get, set};
#[cfg(windows)]
pub use windows::{get, set};

/// Get the value of a path-like environment variable.
pub fn get_path_like_env(name: &str) -> Fallible<Vec<PathBuf>> {
    let paths = get(name)?;
    Ok(std::env::split_paths(&paths).collect())
}

#[cfg(windows)]
mod windows {
    use once_cell::sync::Lazy;
    use std::ffi::OsString;
    use std::path::Path;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    use crate::error::Fallible;

    /// `HKEY_CURRENT_USER` registry key handle.
    static HKCU: Lazy<RegKey> = Lazy::new(|| RegKey::predef(HKEY_CURRENT_USER));

    /// Get the value of an environment variable.
    /// Returns an empty string if the variable is not set.
    pub fn get(key: &str) -> Fallible<OsString> {
        let path = Path::new("Environment");
        let env = HKCU.open_subkey(path)?;
        Ok(env.get_value(key)?)
    }

    /// Set the value of an environment variable.
    /// If the value is an empty string, the variable is deleted.
    pub fn set(key: &str, value: Option<&OsString>) -> Fallible<()> {
        let path = Path::new("Environment");
        let (env, _) = HKCU.create_subkey(path)?;

        if value.is_none() {
            // ignore error of deleting non-existent value
            let _ = env.delete_value(key);
        } else {
            env.set_value(key, value.unwrap())?;
        }
        Ok(())
    }
}

#[cfg(unix)]
mod unix {
    use std::ffi::OsString;

    use crate::error::Fallible;

    /// Get the value of an environment variable.
    /// Returns an empty string if the variable is not set.
    pub fn get(key: &str) -> Fallible<OsString> {
        Ok(std::env::var_os(key).unwrap_or_default())
    }

    /// Set the value of an environment variable.
    /// If the value is an empty string, the variable is deleted.
    pub fn set(key: &str, value: Option<&OsString>) -> Fallible<()> {
        // no-op
        Ok(())
    }
}
