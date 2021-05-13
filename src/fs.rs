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
      entry.path().extension().unwrap().eq("json")
    })
    .collect();

  Ok(jsons)
}

/// Return the Leaf, i.e. filename with extension, of given path.
pub fn leaf<P: AsRef<Path>>(path: P) -> String {
  path.as_ref().file_name().unwrap().to_os_string().into_string().unwrap()
}

/// Return the LeafBase, i.e. filename without extension, of given path.
pub fn leaf_base<P: AsRef<Path>>(path: P) -> String {
  let leaf = leaf(path.as_ref());
  match leaf.contains(".") {
    false => leaf,
    true => leaf.split_once(".").unwrap().0.to_string()
  }
}
