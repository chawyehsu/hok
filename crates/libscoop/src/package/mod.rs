pub(crate) mod download;
pub(crate) mod manifest;
pub(crate) mod metadata;
pub(crate) mod query;
pub(crate) mod resolve;
pub(crate) mod sync;

use once_cell::unsync::OnceCell;
use std::{fmt, path::PathBuf};

pub use manifest::{Algorithm, HashString, InstallInfo, License, Manifest};
pub use query::QueryOption;
pub use sync::SyncOption;

use crate::{constant::ISOLATED_PACKAGE_BUCKET, internal};

/// A Scoop package.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Package {
    /// The bucket name of this package.
    bucket: String,

    /// The name of this package.
    name: String,

    /// The manifest of this package.
    manifest: Manifest,

    #[serde(skip)]
    origin: OnceCell<OriginateFrom>,

    /// The install state of the package.
    #[serde(skip)]
    install_state: OnceCell<InstallState>,

    /// The upgradable package, if any.
    ///
    /// This field is never serialized.
    #[serde(skip)]
    upgradable: OnceCell<Option<Box<Package>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OriginateFrom {
    Bucket(String),
    File(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InstallState {
    NotInstalled,
    Installed(InstallStateInstalled),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstallStateInstalled {
    pub version: String,
    pub bucket: Option<String>,
    pub arch: String,
    pub held: bool,
    pub url: Option<String>,
}

impl InstallStateInstalled {
    #[inline]
    pub fn bucket(&self) -> Option<&str> {
        self.bucket.as_deref()
    }

    #[inline]
    pub fn held(&self) -> bool {
        self.held
    }

    #[inline]
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    #[inline]
    pub fn version(&self) -> &str {
        self.version.as_str()
    }
}

impl Package {
    pub fn from(name: &str, bucket: &str, manifest: Manifest) -> Package {
        Package {
            bucket: bucket.to_owned(),
            name: name.to_owned(),
            manifest,
            origin: OnceCell::new(),
            install_state: OnceCell::new(),
            upgradable: OnceCell::new(),
        }
    }

    /// The identity of this package.
    ///
    /// # Returns
    ///
    /// The package identity in the form of `bucket/name`, which is unique for
    /// each package across all buckets.
    #[inline]
    pub fn ident(&self) -> String {
        format!("{}/{}", self.bucket, self.name)
    }

    /// Get the name of this package.
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Get the bucket name of this package.
    ///
    /// # Note
    ///
    /// Although this method in some cases returns a bucket namer which can be
    /// the same as the bucket name from the install state of a package, it is
    /// not guaranteed to be.
    ///
    /// This method is not identical to `installed_bucket()`, which is designed
    /// to returns the precise installed bucket name if any.
    #[inline]
    pub fn bucket(&self) -> &str {
        self.bucket.as_str()
    }

    /// Get the version of this package.
    ///
    /// # Note
    ///
    /// Although this method in some cases returns a version number which can be
    /// the same as the version number from the installe state of a package, it
    /// is not guaranteed to be.
    ///
    /// This method is not identical to `installed_version()`, which is designed
    /// to returns the precise installed version number if any.
    #[inline]
    pub fn version(&self) -> &str {
        self.manifest.version()
    }

    /// Get the description of this package.
    #[inline]
    pub fn description(&self) -> Option<&str> {
        self.manifest.description()
    }

    /// Get the homepage of this package.
    #[inline]
    pub fn homepage(&self) -> &str {
        self.manifest.homepage()
    }

    /// Get the license of this package.
    pub fn license(&self) -> &License {
        self.manifest.license()
    }

    /// Get the cookie of this package.
    pub fn cookie(&self) -> Option<Vec<(&str, &str)>> {
        self.manifest.cookie().map(|c| {
            c.iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect::<Vec<_>>()
        })
    }

    /// Get the dependencies of this package.
    ///
    /// # Note
    ///
    /// There is no guarantee that whether a dependency is represented as a
    /// format of `bucket/name` or `name`.
    ///
    /// # Returns
    ///
    /// A list of dependencies of this package.
    pub fn dependencies(&self) -> Vec<String> {
        self.manifest.dependencies()
    }

    /// Get the downloads of this package, in manifest order.
    pub(crate) fn downloads(&self) -> Vec<Download<'_>> {
        self.manifest
            .url()
            .into_iter()
            .map(Download::new)
            .collect::<Vec<_>>()
    }

    pub(crate) fn download_hashes(&self) -> Vec<&HashString> {
        self.manifest.hash()
    }

    /// Get the installed bucket of this package.
    ///
    /// # Returns
    ///
    /// The installed bucket of this package, if any.
    pub fn installed_bucket(&self) -> Option<&str> {
        match self.install_state.get() {
            None => None,
            Some(state) => match state {
                InstallState::NotInstalled => None,
                InstallState::Installed(info) => {
                    Some(info.bucket().unwrap_or(ISOLATED_PACKAGE_BUCKET))
                }
            },
        }
    }

    /// Get the installed version of this package.
    ///
    /// # Returns
    ///
    /// The installed version of this package, if any.
    pub fn installed_version(&self) -> Option<&str> {
        match self.install_state.get() {
            None => None,
            Some(state) => match state {
                InstallState::NotInstalled => None,
                InstallState::Installed(info) => Some(info.version()),
            },
        }
    }

    /// Check if the package is held.
    ///
    /// # Note
    ///
    /// Only installed package can be held, therefore this method will always
    /// return `false` if the package is not installed.
    pub fn is_held(&self) -> bool {
        match self.install_state.get() {
            None => false,
            Some(state) => match state {
                InstallState::NotInstalled => false,
                InstallState::Installed(info) => info.held(),
            },
        }
    }

    /// Check if the package is installed.
    pub fn is_installed(&self) -> bool {
        self.installed_version().is_some()
    }

    #[inline]
    pub fn is_nightly(&self) -> bool {
        self.version() == "nightly"
    }

    /// Check if the package is strictly installed, which means the package is
    /// installed from the bucket it belongs to rather than from other buckets.
    pub fn is_strictly_installed(&self) -> bool {
        match self.install_state.get() {
            None => false,
            Some(state) => match state {
                InstallState::NotInstalled => false,
                InstallState::Installed(info) => match info.bucket() {
                    Some(bucket) => bucket == self.bucket(),
                    None => false,
                },
            },
        }
    }

    /// Get the manifest of this package.
    ///
    /// # Returns
    ///
    /// The manifest reference of this package.
    #[inline]
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Get the upgradable version of this package.
    ///
    /// # Returns
    ///
    /// The upgradable version when the package is upgradable, otherwise `None`.
    pub fn upgradable_version(&self) -> Option<&str> {
        let origin_pkg = self.upgradable.get();

        if let Some(Some(pkg)) = origin_pkg {
            return Some(pkg.version());
        } else if let Some(installed_version) = self.installed_version() {
            let this_version = self.version();
            let is_upgradable = internal::compare_versions(this_version, installed_version)
                == std::cmp::Ordering::Greater;
            if is_upgradable {
                return Some(this_version);
            }
        }

        None
    }

    /// Check if this package is upgradable.
    ///
    /// # Returns
    ///
    /// The reference to the upgradable package of this package when it is
    /// upgradable, otherwise `None`.
    pub fn upgradable(&self) -> Option<&Package> {
        if let Some(Some(pkg)) = self.upgradable.get() {
            return Some(pkg.as_ref());
        }
        None
    }

    /// Get shims defined in this package.
    ///
    /// # Returns
    ///
    /// A list of shims defined in this package.
    pub fn shims(&self) -> Option<Vec<&str>> {
        self.manifest.shims()
    }

    pub fn supported_arch(&self) -> Vec<String> {
        let mut ret = vec![];
        if let Some(arch) = self.manifest.architecture() {
            if arch.ia32.is_some() {
                ret.push("ia32".to_string());
            }
            if arch.amd64.is_some() {
                ret.push("amd64".to_string());
            }
            if arch.aarch64.is_some() {
                ret.push("aarch64".to_string());
            }
        }
        ret
    }

    /// Check if this package defines install hooks (powershell scripts) in its
    /// manifest.
    pub(crate) fn has_install_script(&self) -> bool {
        [
            self.manifest.pre_install(),
            self.manifest.post_install(),
            self.manifest
                .installer()
                .map(|i| i.script())
                .unwrap_or_default(),
        ]
        .into_iter()
        .any(|h| h.is_some())
    }

    /// Check if this package defines uninstall hooks (powershell scripts) in its
    /// manifest.
    pub(crate) fn has_uninstall_script(&self) -> bool {
        [
            self.manifest
                .uninstaller()
                .map(|u| u.script())
                .unwrap_or_default(),
            self.manifest.pre_uninstall(),
            self.manifest.post_uninstall(),
        ]
        .into_iter()
        .any(|h| h.is_some())
    }

    pub(crate) fn fill_install_state(&self, state: InstallState) {
        let origin = match &state {
            InstallState::NotInstalled => OriginateFrom::Bucket(self.bucket.clone()),
            InstallState::Installed(info) => match info.url() {
                Some(url) => OriginateFrom::File(url.to_owned()),
                None => OriginateFrom::Bucket(
                    info.bucket().unwrap_or(ISOLATED_PACKAGE_BUCKET).to_owned(),
                ),
            },
        };

        let _ = self.origin.set(origin);
        let _ = self.install_state.set(state);
    }

    pub(crate) fn fill_upgradable(&self, upgradable: Package) {
        let upgradable = Some(Box::new(upgradable));
        let _ = self.upgradable.set(upgradable);
    }
}

impl PartialEq for Package {
    fn eq(&self, other: &Package) -> bool {
        self.name() == other.name()
    }
}

/// One download of a package, as the manifest declares it.
///
/// A manifest URL may carry a `#/<name>` fragment that renames the download,
/// for example `https://example.com/dl?id=7#/tool.exe`. The fragment is not sent
/// to the server: it names the file that the download becomes on disk.
pub(crate) struct Download<'a> {
    /// The manifest URL as written, rename fragment included.
    pub(crate) manifest_url: &'a str,
    /// The URL to fetch, without the rename fragment.
    pub(crate) url: &'a str,
    /// The name the download takes once installed.
    ///
    /// The rename fragment when the URL has one, otherwise the last path
    /// segment of the URL.
    pub(crate) filename: String,
}

impl<'a> Download<'a> {
    fn new(manifest_url: &'a str) -> Download<'a> {
        let (url, rename) = match manifest_url.split_once('#') {
            Some((url, rename)) => (url, Some(rename)),
            None => (manifest_url, None),
        };

        let filename = rename
            .filter(|rename| !rename.is_empty())
            .and_then(|rename| {
                rename
                    .trim_start_matches(['/', '\\'])
                    .rsplit(['/', '\\'])
                    .next()
            })
            .filter(|rename| !rename.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| url_leaf(url));

        Download {
            manifest_url,
            url,
            filename,
        }
    }
}

/// The last path segment of `url`, or `download` when it has none.
fn url_leaf(url: &str) -> String {
    let url = url.split_once('#').map(|(head, _)| head).unwrap_or(url);
    let url = url.split_once('?').map(|(head, _)| head).unwrap_or(url);

    url.trim_end_matches('/')
        .rsplit(['/', '\\'])
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("download")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{Download, Package, manifest::Manifest, url_leaf};
    use std::path::PathBuf;

    /// A package whose manifest declares `urls` for this host.
    fn package_with(urls: &[&str]) -> Package {
        let url = urls
            .iter()
            .map(|url| format!("\"{url}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let json = format!(
            r#"{{"version":"1.0.0","description":"test","homepage":"https://example.com","license":"MIT","url":[{}],"hash":["0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"]}}"#,
            url
        );
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "hok-download-test-{}-{}.json",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::write(&path, json).unwrap();
        let manifest = Manifest::parse(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        Package::from("tool", "bucket", manifest)
    }

    #[test]
    fn rename_fragment_names_the_download() {
        let package = package_with(&["https://example.com/get?id=7#/tool.exe"]);
        let downloads = package.downloads();
        assert_eq!(downloads.len(), 1);
        assert_eq!(downloads[0].url, "https://example.com/get?id=7");
        assert_eq!(downloads[0].filename, "tool.exe");
    }

    #[test]
    fn rename_fragment_may_carry_a_path() {
        let package = package_with(&["https://example.com/get#/dl.7z"]);
        assert_eq!(package.downloads()[0].filename, "dl.7z");

        let package = package_with(&["https://example.com/get#dir/../tool.zip"]);
        assert_eq!(package.downloads()[0].filename, "tool.zip");
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
        let package = package_with(&["https://example.com/releases/tool.zip"]);
        assert_eq!(package.downloads()[0].filename, "tool.zip");
    }

    #[test]
    fn a_query_string_never_reaches_the_download_name() {
        let package = package_with(&["https://example.com/dl?file=tool.zip&raw=1"]);
        assert_eq!(package.downloads()[0].filename, "dl");

        let package = package_with(&["https://example.com/tool.zip?raw=1#/other.7z"]);
        assert_eq!(package.downloads()[0].filename, "other.7z");
    }

    #[test]
    fn an_empty_fragment_is_ignored() {
        let package = package_with(&["https://example.com/releases/tool.zip#"]);
        assert_eq!(package.downloads()[0].filename, "tool.zip");
    }

    #[test]
    fn cache_name_hashes_the_manifest_url_and_names_the_fragment_extension() {
        let package = package_with(&["https://example.com/get?id=7#/tool.exe"]);
        let download = &package.downloads()[0];
        assert_eq!(
            download.manifest_url,
            "https://example.com/get?id=7#/tool.exe"
        );
        assert_eq!(download.url, "https://example.com/get?id=7");
        assert_eq!(download.filename, "tool.exe");
    }

    #[test]
    fn cache_name_hash_is_the_manifest_url_hash() {
        use crate::internal::hash::ChecksumBuilder;

        let manifest_url = "https://example.com/get?id=7#/tool.exe";
        let package = package_with(&[manifest_url]);
        let download = &package.downloads()[0];

        let mut hasher = ChecksumBuilder::new().sha256().build();
        hasher.consume(manifest_url.as_bytes());
        let expected_hash = hasher.finalize();
        let expected_hash = &expected_hash[..7];

        let name = super::download::cache_filename(&package, download);
        assert_eq!(
            name,
            format!("tool#1.0.0#{expected_hash}.exe"),
            "the cache hash must stay the manifest URL hash"
        );
    }

    /// Scoop's `cache_path` appends `Path::GetExtension` of the whole url, which
    /// is the last dot segment. A `.tar.gz` download is cached as `.gz` there,
    /// and the cache is shared with Scoop, so this must match.
    #[test]
    fn cache_name_extension_matches_scoop() {
        for (manifest_url, expected) in [
            ("https://example.com/releases/tool.tar.gz", ".gz"),
            ("https://example.com/releases/tool.zip", ".zip"),
            ("https://example.com/get#/dl.7z", ".7z"),
            ("https://example.com/get?id=7#/tool.exe", ".exe"),
            ("https://example.com/download#/tool", ""),
        ] {
            let package = package_with(&[manifest_url]);
            let name = super::download::cache_filename(&package, &package.downloads()[0]);
            assert!(
                name.ends_with(expected),
                "{manifest_url} should cache with extension '{expected}', got '{name}'"
            );
        }
    }

    #[test]
    fn url_leaf_is_the_last_path_segment() {
        assert_eq!(
            url_leaf("https://example.com/releases/tool.zip"),
            "tool.zip"
        );
        assert_eq!(url_leaf("https://example.com/releases/"), "releases");
        assert_eq!(url_leaf("https://example.com"), "example.com");
        assert_eq!(url_leaf("https://example.com/dl?id=7"), "dl");
        assert_eq!(url_leaf("https://example.com/tool.zip?raw=1"), "tool.zip");
    }

    #[test]
    fn url_leaf_never_keeps_a_fragment() {
        assert_eq!(url_leaf("https://example.com/get#/dl.7z"), "get");
    }
}

/// Extact `name` from `bucket/name`.
pub(super) fn extract_name<S: AsRef<str>>(input: S) -> String {
    input
        .as_ref()
        .split_once('/')
        .map(|(_, n)| n)
        .unwrap_or(input.as_ref())
        .to_owned()
}

/// Hash mismatch context.
#[derive(Clone, Debug)]
pub struct HashMismatchContext {
    name: String,
    url: String,
    expected: String,
    actual: String,
}

impl fmt::Display for HashMismatchContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hash mismatch for package '{}':\n     Url: {}\nExpected: {}\n  Actual: {}",
            self.name(),
            self.url(),
            self.expected(),
            self.actual()
        )
    }
}

impl HashMismatchContext {
    /// Create a new hash mismatch context.
    pub fn new(name: String, url: String, expected: String, actual: String) -> HashMismatchContext {
        HashMismatchContext {
            name,
            url,
            expected,
            actual,
        }
    }

    /// name of the package.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// url of corresponding hash mismatched file.
    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    /// Expected hash.
    pub fn expected(&self) -> &str {
        self.expected.as_str()
    }

    /// Actual hash.
    pub fn actual(&self) -> &str {
        self.actual.as_str()
    }
}
