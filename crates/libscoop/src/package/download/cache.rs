//! The cache file name, and its inverse.
//!
//! A cached download is named `<name>#<version>#<sha><ext>` by [`cache_file_name`],
//! and [`CacheFileName::parse`] takes that name apart again. Both live here so
//! the grammar has one owner: the writer and the cache listing cannot drift.
//!
//! This mirrors Scoop's `cache_path`, so a cache shared with Scoop resolves.

use crate::internal::hash::ChecksumBuilder;

/// The separator between the name, the version and the url part.
const SEP: char = '#';

/// The number of hex characters of the url hash kept in a name.
const HASH_LEN: usize = 7;

/// The cache file name of one download.
///
/// This mirrors Scoop's `cache_path`: `<name>#<version>#<7-char sha256 of the
/// url><extension>`, where the extension is `Path::GetExtension` of the whole
/// url. The hash therefore covers the manifest url verbatim, rename fragment
/// included, and the extension is the one `GetExtension` reports, which is the
/// last dot segment: a `.tar.gz` download is cached as `.gz`. That is deliberate,
/// because it is what Scoop writes and the cache is shared.
///
/// This is the cache name, not the name the download is installed under.
pub(crate) fn cache_file_name(name: &str, version: &str, manifest_url: &str) -> String {
    let mut hasher = ChecksumBuilder::new().sha256().build();
    hasher.consume(manifest_url.as_bytes());
    let hash = hasher.finalize();

    let ext = std::path::Path::new(manifest_url)
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .filter(|ext| !ext.is_empty())
        .map(|ext| format!(".{ext}"))
        .unwrap_or_default();

    format!(
        "{name}{SEP}{version}{SEP}{}{ext}",
        &hash[..HASH_LEN.min(hash.len())]
    )
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
        assert_eq!(
            cache_file_name("tool", "1.0.0", url),
            format!("tool#1.0.0#{}.exe", hash_of(url))
        );
    }

    /// Scoop appends `Path::GetExtension` of the whole url, which is the last
    /// dot segment, so a `.tar.gz` download is cached as `.gz`.
    #[test]
    fn extension_matches_scoop() {
        for (manifest_url, expected) in [
            ("https://example.com/releases/tool.tar.gz", ".gz"),
            ("https://example.com/releases/tool.zip", ".zip"),
            ("https://example.com/get#/dl.7z", ".7z"),
            ("https://example.com/get?id=7#/tool.exe", ".exe"),
            ("https://example.com/download#/tool", ""),
        ] {
            let name = cache_file_name("tool", "1.0.0", manifest_url);
            assert!(
                name.ends_with(expected),
                "{manifest_url} should cache with extension '{expected}', got '{name}'"
            );
        }
    }

    #[test]
    fn parse_reverses_the_name() {
        let name = cache_file_name("tool", "1.0.0", "https://example.com/get#/tool.exe");
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
