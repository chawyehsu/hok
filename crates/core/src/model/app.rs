use super::Manifest;
use crate::{util::leaf, Config};
use std::path::PathBuf;

#[derive(Debug)]
pub struct App<'cfg> {
    config: &'cfg Config,
    name: String,
    bucket: String,
    manifest: Manifest,
}

#[derive(Debug)]
struct Installed {

}

impl<'cfg> App<'cfg> {
    #[inline]
    pub(crate) fn new(config: &Config, name: String, bucket: String, manifest: Manifest) -> App {
        App {
            config,
            name,
            bucket,
            manifest,
        }
    }

    #[inline]
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    #[inline]
    pub fn version(&self) -> &str {
        self.manifest().version()
    }

    /// Method for searching `bin` field of this app, used by `ops::search`.
    pub(crate) fn search_bin(&self, name: &str) -> Option<String> {
        match self.manifest().bin() {
            Some(bins) => {
                for bin in bins.iter() {
                    let length = bin.len();
                    if length > 0 {
                        // the first is the original name
                        let leaf_bin = leaf(&PathBuf::from(bin[0].clone()));
                        if leaf_bin.contains(name) {
                            return Some(leaf_bin);
                        }
                    }
                    if length > 1 {
                        // the second is the shim name
                        if bin[1].contains(name) {
                            return Some(bin[1].clone());
                        }
                    }
                }
            }
            None => {}
        }
        None
    }
}
