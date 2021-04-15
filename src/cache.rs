use std::fs::DirEntry;
use crate::{Scoop, utils};

impl Scoop {
  pub fn cache_show(&self, f: DirEntry) {
    let fmeta = std::fs::metadata(f.path()).unwrap();
    let ff = f.file_name().into_string().unwrap();
    let fname: Vec<&str> = ff.split("#").collect();

    println!("{: >6} {} ({}) {}",
      utils::filesize(fmeta.len(), true),
      fname[0],
      fname[1],
      ff
    );
  }
}
