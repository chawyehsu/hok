use std::{fs, io};
use std::path::Path;

/// Ensure given `path` exist.
///
/// Will call [`std::fs::create_dir_all`] if `path` doesn't exist.
pub fn ensure_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
  match path.as_ref().exists() {
    false => fs::create_dir_all(path.as_ref()),
    true => Ok(())
  }
}

pub fn read_dir_json<P: AsRef<Path>>(path: P) -> io::Result<Vec<fs::DirEntry>> {
  let jsons = fs::read_dir(path.as_ref())?
    .filter_map(Result::ok)
    .filter(|entry| {
      entry.file_name().into_string().unwrap().ends_with(".json")
    })
    .collect();

  Ok(jsons)
}
