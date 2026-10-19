//! Scoop's `.shim` text format.
//!
//! The sidecar file next to a shim launcher exists for interoperability with
//! Scoop. It is written from the same [`Config`] as the resource blocks, so the
//! two cannot disagree, but the launcher never reads it back.
//!
//! The grammar is one `key = value` per line, UTF-8, with an optional BOM.
//! Blank lines and lines starting with `#`, `;` or `//` are ignored. This
//! module only ever *writes* that format; reading it is not our concern.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use crate::config::Config;
use crate::utf16::{Utf16Str, Utf16String};

/// Render a configuration as the contents of a `.shim` file.
pub fn to_text(config: &Config) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "path = \"{}\"", escape(config.target.as_utf16()));
    for arg in &config.args {
        let _ = writeln!(out, "args = {}", escape(arg.as_utf16()));
    }
    if !config.cwd.is_empty() {
        let _ = writeln!(out, "cwd = \"{}\"", escape(config.cwd.as_utf16()));
    }
    if config.elevate {
        let _ = writeln!(out, "elevate = true");
    }
    for (name, value) in &config.env {
        let _ = writeln!(
            out,
            "env:{} = \"{}\"",
            escape(name.as_utf16()),
            escape(value.as_utf16())
        );
    }

    out
}

/// Parse the contents of a `.shim` file.
///
/// Only used by tests and by tooling that inspects what we wrote; the launcher
/// has no use for it.
pub fn from_text(text: &str) -> Result<Config, TextError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut config = Config::default();

    for line in text.split('\n') {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with(';')
            || line.starts_with("//")
        {
            continue;
        }

        let (key, value) = line.split_once('=').ok_or(TextError::MalformedLine)?;
        let key = key.trim();
        let value = unquote(value.trim());

        if key.eq_ignore_ascii_case("path") {
            if config.target.is_empty() {
                config.target = value;
            }
        } else if key.eq_ignore_ascii_case("args") {
            if !value.is_empty() {
                config.args.push(value);
            }
        } else if key.eq_ignore_ascii_case("cwd") || key.eq_ignore_ascii_case("workdir") {
            config.cwd = value;
        } else if key.eq_ignore_ascii_case("elevate") || key.eq_ignore_ascii_case("runas") {
            config.elevate = is_truthy(value.as_utf16());
        } else {
            let name = key.strip_prefix("env:").unwrap_or(key);
            if name.is_empty() {
                continue;
            }
            config.env.push((Utf16String::from(name), value));
        }
    }

    if config.target.is_empty() {
        return Err(TextError::MissingTarget);
    }

    Ok(config)
}

/// Failure while parsing `.shim` text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextError {
    /// A line was neither blank, a comment, nor `key = value`.
    MalformedLine,
    /// No `path` line was present.
    MissingTarget,
}

fn is_truthy(value: &Utf16Str) -> bool {
    ["true", "1", "yes"]
        .iter()
        .any(|word| value.eq_ignore_ascii_case(Utf16Str::from_units(&utf16_of(word))))
}

/// Escape a string so it can be written on a single line.
///
/// Backslashes are doubled so a Windows path survives a round trip through
/// [`from_text`], which unescapes. The launcher never reads this file, so the
/// escaping only has to be self-consistent, not CommandLineToArgvW compatible.
fn escape(value: &Utf16Str) -> String {
    let mut out = String::with_capacity(value.utf8_len());
    for c in value.to_display_string().chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\r' => {}
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out
}

/// Reverse [`escape`], dropping the surrounding quotes if present.
fn unquote(value: &str) -> Utf16String {
    let inner = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(value);

    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    Utf16String::from(out.as_str())
}

fn utf16_of(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}
