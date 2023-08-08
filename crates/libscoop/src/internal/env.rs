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
pub fn set(key: &str, value: &str) -> Fallible<()> {
    let path = Path::new("Environment");
    let (env, _) = HKCU.create_subkey(path)?;

    if value.is_empty() {
        env.delete_value(key)?;
    } else {
        env.set_value(key, &value)?;
    }
    Ok(())
}
