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
pub mod manifest;
mod persist;
mod scoop_impl;
mod search;
pub mod sys;
pub mod utils;

pub use apps::AppManager;
pub use bucket::{is_known_bucket, known_bucket_url, known_buckets, BucketManager};
pub use cache::CacheManager;
pub use config::Config;
pub use error::{Error, Result};
pub use persist::PersistManager;
pub use scoop_impl::Scoop;
