//! This crate aims to provide a full-featured, practical, and efficient Rust
//! reimplementation of [Scoop], the Windows command-line installer. It is a
//! library crate providing the core functionality of interacting with Scoop,
//! and is not intended to be used directly by end users. Developers who wish
//! to implement a Scoop frontend or make use of Scoop's functionality in their
//! own applications may use this crate. For end users, they may take a glance
//! at [Hok], a reference implementation built on top of this crate, which
//! provides a command-line interface similar to Scoop.
//!
//! # Overview
//!
//! The primary type in this crate is a [`Session`], which is an entry point to
//! this crate. A session instance is basically a handle to the global state of
//! libscoop. Most of the functions exposed by this crate take a session as
//! their first argument.
//!
//! ## Examples
//!
//! Initialize a Scoop session, get the configuration associated with the
//! session, and print the root path of Scoop to stdout:
//!
//! ```rust
//! use libscoop::Session;
//! let (session, _) = Session::init().expect("failed to create session");
//! let config = session.get_config();
//! println!("{}", config.root_path().display());
//! ```
//!
//! [Scoop]: https://scoop.sh/
//! [Hok]: https://github.com/chawyehsu/hok
extern crate log;
#[macro_use]
extern crate serde;

mod bucket;
mod cache;
mod config;
mod constant;
mod error;
mod event;
mod internal;
mod package;
mod session;

pub mod operation;

pub use error::Error;
pub use event::Event;
pub use package::QueryOption;
pub use session::Session;
pub use tokio;
