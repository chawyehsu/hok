#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

mod apps;
mod bucket;
mod cache;
mod config;
mod dependency;
mod error;
pub mod fs;
mod git;
mod http;
mod license;
mod manifest;
mod persist;
mod scoop_impl;
mod search;
pub mod sys;
pub mod utils;

use error::ScoopResult;

pub use apps::AppManager;
pub use bucket::BucketManager;
pub use cache::{CacheEntry, CacheManager};
pub use config::Config;
pub use dependency::DepGraph;
pub use git::Git;
pub use http::Client as HttpClient;
pub use manifest::{License, Manifest};
pub use persist::PersistManager;
pub use scoop_impl::{find_manifest, search};
