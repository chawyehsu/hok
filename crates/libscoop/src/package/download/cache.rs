//! The cache file name, and its inverse.
//!
//! A cached download is named `<name>#<version>#<sha><ext>` by [`cache_file_name`],
//! and [`CacheFileName::parse`] takes that name apart again. Both live here so
//! the grammar has one owner: the writer and the cache listing cannot drift.
//!
//! This mirrors Scoop's `cache_path`, so a cache shared with Scoop resolves.

use super::Download;
use crate::internal::hash::ChecksumBuilder;

/// The separator between the name, the version and the url part.
const SEP: char = '#';

/// The number of hex characters of the url hash kept in a name.
const HASH_LEN: usize = 7;

/// The cache file name of one download.
///
/// This mirrors Scoop's `cache_path`: `<name>#<version>#<7-char sha256 of the
/// url><extension>`. The hash covers the manifest url verbatim, rename fragment
/// included, so it agrees with Scoop and with an existing cache.
///
/// The extension is taken from the download name rather than from the url.
/// `GetExtension` on a url keeps whatever follows the last dot, so
/// `tool.zip?raw=1` would report `zip?raw=1`, and a `?` is not a legal name
/// character on Windows. The download name is already a file name: the query is
/// stripped and a rename fragment decides the suffix. The suffix is still the
/// last dot segment, so a `.tar.gz` download is cached as `.gz`, as Scoop does.
///
/// This is the cache name, not the name the download is installed under.
pub(crate) fn cache_file_name(name: &str, version: &str, download: &Download<'_>) -> String {
    let mut hasher = ChecksumBuilder::new().sha256().build();
    hasher.consume(download.manifest_url.as_bytes());
    let hash = hasher.finalize();

    let ext = std::path::Path::new(&download.filename)
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .filter(|ext| is_legal_name_part(ext))
        .map(|ext| format!(".{ext}"))
        .unwrap_or_default();

    format!(
        "{name}{SEP}{version}{SEP}{}{ext}",
        &hash[..HASH_LEN.min(hash.len())]
    )
}

/// Whether `part` can appear in a file name on any supported platform.
///
/// This is the narrow set the cache parser already accepts for a name, minus the
/// separator: a download name is not sanitized before it reaches the cache, so a
/// suffix that could not be written is dropped instead of failing the install.
fn is_legal_name_part(part: &str) -> bool {
    !part.is_empty()
        && part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
}

/// The parts a cache file name is made of.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CacheFileName<'a> {
    /// The package name.
    pub(crate) package: &'a str,
    /// The package version.
    pub(crate) version: &'a str,
    /// The url part: the hash, the extension, or a legacy underscored url.
    pub(crate) url: &'a str,
}

impl<'a> CacheFileName<'a> {
    /// Take a cache file name apart, or `None` if it is not one.
    ///
    /// The url part is everything after the second separator: a legacy Scoop
    /// name carries a whole underscored url there, which itself may contain a
    /// separator.
    pub(crate) fn parse(file_name: &'a str) -> Option<CacheFileName<'a>> {
        let mut parts = file_name.splitn(3, SEP);
        let package = parts.next()?;
        let version = parts.next()?;
        let url = parts.next()?;

        let valid = !package.is_empty()
            && package
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
            && !version.is_empty()
            && version
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-.".contains(c))
            && !url.is_empty();

        valid.then_some(CacheFileName {
            package,
            version,
            url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheFileName, cache_file_name};
    use crate::package::download::Download;

    /// The hash `cache_file_name` computes, so the tests read as the format.
    fn hash_of(url: &str) -> String {
        let mut hasher = crate::internal::hash::ChecksumBuilder::new()
            .sha256()
            .build();
        hasher.consume(url.as_bytes());
        hasher.finalize()[..7].to_owned()
    }

    #[test]
    fn name_hashes_the_manifest_url() {
        let url = "https://example.com/get?id=7#/tool.exe";
        let download = Download::new(url);
        assert_eq!(
            cache_file_name("tool", "1.0.0", &download),
            format!("tool#1.0.0#{}.exe", hash_of(url))
        );
    }

    /// The suffix is the last dot segment, as Scoop reports it, so a `.tar.gz`
    /// download is cached as `.gz`.
    #[test]
    fn extension_is_the_last_dot_segment() {
        for (url, expected) in [
            ("https://example.com/releases/tool.tar.gz", ".gz"),
            ("https://example.com/releases/tool.zip", ".zip"),
            ("https://example.com/get#/dl.7z", ".7z"),
            ("https://example.com/get?id=7#/tool.exe", ".exe"),
            ("https://example.com/download#/tool", ""),
        ] {
            let download = Download::new(url);
            let name = cache_file_name("tool", "1.0.0", &download);
            assert!(
                name.ends_with(expected),
                "{url} should cache with extension '{expected}', got '{name}'"
            );
        }
    }

    /// A query string must never reach the name: `?` is not legal on Windows,
    /// and `GetExtension` of the url would carry it into the suffix.
    #[test]
    fn a_query_string_never_reaches_the_name() {
        for url in [
            "https://example.com/tool.zip?raw=1",
            "https://example.com/dl?file=tool.tar.gz",
            "https://example.com/tool.zip?raw=1#/other.7z",
        ] {
            let download = Download::new(url);
            let name = cache_file_name("tool", "1.0.0", &download);
            assert!(!name.contains('?'), "{url} produced '{name}'");
            assert!(!name.contains('&'), "{url} produced '{name}'");
        }
    }

    #[test]
    fn a_query_url_keeps_the_extension_before_it() {
        let download = Download::new("https://example.com/tool.zip?raw=1");
        assert_eq!(
            cache_file_name("tool", "1.0.0", &download),
            format!(
                "tool#1.0.0#{}.zip",
                hash_of("https://example.com/tool.zip?raw=1")
            )
        );
    }

    #[test]
    fn parse_reverses_the_name() {
        let url = "https://example.com/get#/tool.exe";
        let download = Download::new(url);
        let name = cache_file_name("tool", "1.0.0", &download);
        let parsed = CacheFileName::parse(&name).unwrap();
        assert_eq!(parsed.package, "tool");
        assert_eq!(parsed.version, "1.0.0");
        assert_eq!(
            parsed.url,
            format!("{}.exe", hash_of("https://example.com/get#/tool.exe"))
        );
    }

    #[test]
    fn parse_accepts_a_legacy_underscored_url() {
        let parsed = CacheFileName::parse("tool#1.0.0#https_example.com_tool.zip").unwrap();
        assert_eq!(parsed.package, "tool");
        assert_eq!(parsed.version, "1.0.0");
        assert_eq!(parsed.url, "https_example.com_tool.zip");
    }

    #[test]
    fn parse_keeps_a_hash_in_a_legacy_url() {
        let parsed = CacheFileName::parse("tool#1.0.0#https_a.com_b_c.zip").unwrap();
        assert_eq!(parsed.url, "https_a.com_b_c.zip");
    }

    #[test]
    fn parse_rejects_a_name_without_three_parts() {
        assert!(CacheFileName::parse("tool#1.0.0").is_none());
        assert!(CacheFileName::parse("tool").is_none());
        assert!(CacheFileName::parse("tool#1.0.0#").is_none());
        assert!(CacheFileName::parse("#1.0.0#abc.zip").is_none());
        assert!(CacheFileName::parse("tool##abc.zip").is_none());
    }

    #[test]
    fn parse_rejects_characters_scoop_does_not_write() {
        assert!(CacheFileName::parse("to ol#1.0.0#abc.zip").is_none());
        assert!(CacheFileName::parse("tool#1 0 0#abc.zip").is_none());
    }
}
