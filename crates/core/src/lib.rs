#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

mod dependency;
mod git;
mod http;
mod license;
pub mod manager;
mod model;
pub mod ops;
pub mod sys;
pub mod util;

pub use dependency::DepGraph;
pub use git::Git;
pub use http::Client as HttpClient;
pub use model::Config;

/// A wrapped `Result` used in this crate.
pub(crate) type ScoopResult<T> = anyhow::Result<T>;
