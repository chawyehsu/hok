//! Shim configuration schema shared by the `hok-shim` launcher and the `libscoop`
//! shim management module.
//!
//! A shim launcher carries its configuration inside itself as a Windows string
//! table resource. This crate defines that layout and the encoding of a
//! configuration record into it, so the writer in `libscoop` and the reader in
//! `hok-shim` cannot drift apart.
//!
//! The same record is also encoded as Scoop's `.shim` text file, which exists
//! for interoperability only: the launcher never reads it.

#![no_std]

extern crate alloc;

mod config;
mod resource;
mod text;
mod utf16;

pub use config::Config;
pub use resource::{DecodeError, EncodeError, MAGIC, SCHEMA_VERSION, decode, encode};
pub use text::{TextError, from_text, to_text};
pub use utf16::{Utf16Str, Utf16String};
