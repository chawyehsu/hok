#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

mod apps;
mod bucket;
mod cache;
mod config;
mod error;
pub mod fs;
mod git;
mod http;
mod manifest;
mod persist;
mod scoop_impl;
mod search;
pub mod sys;
pub mod utils;

use error::Result;

pub use apps::AppManager;
pub use bucket::{is_known_bucket, known_bucket_url, known_buckets, BucketManager};
pub use cache::{CacheEntry, CacheManager};
pub use config::Config;
pub use manifest::{License, Manifest};
pub use persist::PersistManager;
pub use scoop_impl::Scoop;
