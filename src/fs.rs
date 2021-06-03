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

pub fn empty_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
  match path.as_ref().exists() {
    true => remove_dir_all::remove_dir_contents(path.as_ref()),
    false => Ok(())
  }
}

/// Read all JSON files in given `path` directory.
pub fn read_dir_json<P: AsRef<Path>>(path: P) -> io::Result<Vec<fs::DirEntry>> {
  let jsons = fs::read_dir(path.as_ref())?
    .filter_map(Result::ok)
    .filter(|entry| {
      entry.file_name().into_string().unwrap().ends_with(".json")
    })
    .collect();

  Ok(jsons)
}

/// Return the Leaf, i.e. file name (with extension), or directory name
/// of given path.
pub fn leaf<P: AsRef<Path>>(path: P) -> String {
  path.as_ref().file_name().unwrap().to_str().unwrap().to_string()
}

/// Return the LeafBase, i.e. file name without extension, for given file path.
///
/// If the given path is a directory, it returns the [Leaf] of the path instead.
///
/// [Leaf]: self::leaf
pub fn leaf_base<P: AsRef<Path>>(path: P) -> String {
  if path.as_ref().is_file() {
    path.as_ref().file_stem().unwrap().to_str().unwrap().to_string()
  } else {
    self::leaf(path)
  }
}
