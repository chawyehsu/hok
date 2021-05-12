use std::io;
use std::path::Path;

/// Ensure given `path` exist.
///
/// Will call [`std::fs::create_dir_all`] if `path` doesn't exist.
pub fn ensure_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
  match path.as_ref().exists() {
    false => std::fs::create_dir_all(path.as_ref()),
    true => Ok(())
  }
}
