//! Tests for the download module that need a real manifest and package.
//!
//! A manifest is only produced by parsing JSON, so these build one on disk and
//! let the package parser do the rest.

use std::path::PathBuf;

use crate::package::{Package, manifest::Manifest};

use super::{Download, cache_file_name};

/// One valid sha256 for every test: the manifest schema rejects a placeholder.
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// A package whose manifest declares `urls`.
fn package_with(urls: &[&str]) -> Package {
    let url = urls
        .iter()
        .map(|url| format!("\"{url}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let json = format!(
        r#"{{"version":"1.0.0","description":"test","homepage":"https://example.com","license":"MIT","url":[{url}],"hash":["{HASH}"]}}"#
    );

    let path = temp_manifest(&json);
    let manifest = Manifest::parse(&path).unwrap();
    let _ = std::fs::remove_file(&path);

    Package::from("tool", "bucket", manifest)
}

/// Write `json` to a unique temporary manifest path.
fn temp_manifest(json: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "hok-download-test-{}-{}.json",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::write(&path, json).unwrap();
    path
}

/// The manifests a package is built from, so a test reads as its URLs.
fn downloads_of(urls: &[&str]) -> Vec<String> {
    let package = package_with(urls);
    package
        .downloads()
        .iter()
        .map(|download| download.filename.clone())
        .collect()
}

#[test]
fn rename_fragment_names_the_download() {
    let package = package_with(&["https://example.com/get?id=7#/tool.exe"]);
    let downloads = package.downloads();
    assert_eq!(downloads.len(), 1);
    assert_eq!(downloads[0].url, "https://example.com/get?id=7");
    assert_eq!(downloads[0].filename, "tool.exe");
    assert_eq!(
        downloads[0].manifest_url,
        "https://example.com/get?id=7#/tool.exe"
    );
}

#[test]
fn rename_fragment_may_carry_a_path() {
    assert_eq!(downloads_of(&["https://example.com/get#/dl.7z"]), ["dl.7z"]);
    assert_eq!(
        downloads_of(&["https://example.com/get#dir/../tool.zip"]),
        ["tool.zip"]
    );
}

#[test]
fn a_rename_fragment_changes_the_extension_not_the_url() {
    let package = package_with(&["https://example.com/api/v1/download#/tool.exe"]);
    let download = &package.downloads()[0];
    assert!(
        !download.url.ends_with(".exe"),
        "the fragment must not reach the server"
    );
}

#[test]
fn without_a_fragment_the_url_leaf_names_the_download() {
    assert_eq!(
        downloads_of(&["https://example.com/releases/tool.zip"]),
        ["tool.zip"]
    );
}

#[test]
fn a_query_string_never_reaches_the_download_name() {
    assert_eq!(
        downloads_of(&["https://example.com/dl?file=tool.zip&raw=1"]),
        ["dl"]
    );
    assert_eq!(
        downloads_of(&["https://example.com/tool.zip?raw=1#/other.7z"]),
        ["other.7z"]
    );
}

#[test]
fn an_empty_fragment_is_ignored() {
    assert_eq!(
        downloads_of(&["https://example.com/releases/tool.zip#"]),
        ["tool.zip"]
    );
}

#[test]
fn several_downloads_keep_manifest_order() {
    let package = package_with(&["https://example.com/a.zip", "https://example.com/get#/b.7z"]);
    let downloads = package.downloads();
    assert_eq!(downloads.len(), 2);
    assert_eq!(downloads[0].manifest_url, "https://example.com/a.zip");
    assert_eq!(downloads[1].filename, "b.7z");
}

#[test]
fn cache_name_uses_the_package_identity_and_the_manifest_url() {
    let url = "https://example.com/get?id=7#/tool.exe";
    let package = package_with(&[url]);
    let download: &Download<'_> = &package.downloads()[0];

    let name = cache_file_name("tool", "1.0.0", download.manifest_url);
    assert!(name.starts_with("tool#1.0.0#"), "{name}");
    assert!(name.ends_with(".exe"), "{name}");
}
