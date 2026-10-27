//! The configuration record itself.

use alloc::vec::Vec;

use crate::utf16::Utf16String;

/// Everything a shim launcher needs to run its target.
///
/// The record is intentionally a plain value with no caching or path logic:
/// every field holds exactly what the writer put in, already resolved to
/// absolute form where a path is expected.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    /// Absolute path of the program to launch.
    pub target: Utf16String,

    /// Fixed arguments prepended to whatever the caller passes.
    pub args: Vec<Utf16String>,

    /// Working directory for the child, or empty to inherit.
    pub cwd: Utf16String,

    /// Whether to request elevation.
    pub elevate: bool,

    /// Environment variables to set in the child.
    pub env: Vec<(Utf16String, Utf16String)>,
}

impl Config {
    /// Whether this record has enough to launch anything.
    pub fn is_launchable(&self) -> bool {
        !self.target.is_empty()
    }
}
