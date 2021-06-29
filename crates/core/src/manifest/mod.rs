mod license;

use once_cell::sync::Lazy;
use regex::Regex;
use regex::RegexBuilder;
use serde::de;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Map;
use std::convert::Infallible;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::result::Result as StdResult;
use std::str::FromStr;

use crate::error::{Error, ErrorKind, Result};
use crate::fs::leaf_base;
use crate::utils;

////////////////////////////////////////////////////////////////////////////////
//  Manifest Custom Enums
////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ShortcutsType {
    TwoElement([String; 2]),
    ThreeElement([String; 3]),
    FourElement([String; 4]),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum HashExtractionMode {
    #[serde(rename = "download")]
    Download,
    #[serde(rename = "extract")]
    Extract,
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "xpath")]
    Xpath,
    #[serde(rename = "rdf")]
    Rdf,
    #[serde(rename = "metalink")]
    Metalink,
    #[serde(rename = "fosshub")]
    Fosshub,
    #[serde(rename = "sourceforge")]
    Sourceforge,
}

////////////////////////////////////////////////////////////////////////////////
//  Manifest Custom Structs
////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Installer {
    pub args: Option<VecItem>,
    pub file: Option<String>,
    pub script: Option<VecItem>,
    pub keep: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Uninstaller {
    pub args: Option<VecItem>,
    pub file: Option<String>,
    pub script: Option<VecItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchitectureInner {
    pub bin: Option<Bins>,
    pub checkver: Option<Checkver>,
    pub env_add_path: Option<VecItem>,
    pub env_set: Option<Map<String, serde_json::Value>>,
    pub extract_dir: Option<VecItem>,
    pub hash: Option<Hashes>,
    pub installer: Option<Installer>,
    pub post_install: Option<VecItem>,
    pub pre_install: Option<VecItem>,
    pub shortcuts: Option<Vec<ShortcutsType>>,
    pub uninstaller: Option<Uninstaller>,
    pub url: Option<Urls>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Architecture {
    #[serde(rename = "32bit")]
    ia32: Option<ArchitectureInner>,
    #[serde(rename = "64bit")]
    amd64: Option<ArchitectureInner>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HashExtraction {
    pub find: Option<String>,
    pub regex: Option<String>,
    pub jp: Option<String>,
    pub jsonpath: Option<String>,
    pub xpath: Option<String>,
    pub mode: Option<HashExtractionMode>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AutoupdateArchitectureInner {
    pub extract_dir: Option<VecItem>,
    pub url: Option<VecItem>,
    pub hash: Option<HashExtraction>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AutoupdateArchitecture {
    #[serde(rename = "32bit")]
    ia32: Option<AutoupdateArchitectureInner>,
    #[serde(rename = "64bit")]
    amd64: Option<AutoupdateArchitectureInner>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Autoupdate {
    pub architecture: Option<AutoupdateArchitecture>,
    pub extract_dir: Option<VecItem>,
    pub hash: Option<HashExtraction>,
    pub note: Option<VecItem>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Psmodule {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ManifestInner {
    pub architecture: Option<Architecture>,
    pub autoupdate: Option<Autoupdate>,
    bin: Option<Bins>,
    pub persist: Option<Persist>,
    pub checkver: Option<Checkver>,
    pub cookie: Option<serde_json::Value>,
    pub depends: Option<VecItem>,
    description: Option<String>,
    pub env_add_path: Option<VecItem>,
    pub env_set: Option<Map<String, serde_json::Value>>,
    pub extract_dir: Option<VecItem>,
    pub extract_to: Option<VecItem>,
    pub hash: Option<Hashes>,
    homepage: Option<String>,
    innosetup: Option<bool>,
    pub installer: Option<Installer>,
    license: Option<License>,
    pub notes: Option<VecItem>,
    pub post_install: Option<VecItem>,
    pub pre_install: Option<VecItem>,
    pub psmodule: Option<Psmodule>,
    pub shortcuts: Option<Vec<ShortcutsType>>,
    pub suggest: Option<serde_json::Value>,
    pub uninstaller: Option<Uninstaller>,
    url: Option<Urls>,
    version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    name: String,
    path: PathBuf,
    bucket: Option<String>,
    inner: ManifestInner,
    _private: (),
}

#[derive(Clone, Debug, Serialize)]
pub struct Checkver {
    // pub github: Option<String>,
    // pub re: Option<String>,
    pub regex: Option<String>,
    pub url: Option<String>,
    // pub jp: Option<String>,
    pub jsonpath: Option<String>,
    pub xpath: Option<String>,
    pub reverse: Option<bool>,
    pub replace: Option<String>,
    pub useragent: Option<String>,
    pub script: Option<VecItem>,
}

impl<'de> Deserialize<'de> for Checkver {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CheckverVisitor;
        impl<'de> Visitor<'de> for CheckverVisitor {
            type Value = Checkver;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("license string or map")
            }

            fn visit_str<E>(self, s: &str) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                let regex = match s {
                    "github" => Some("/releases/tag/(?:v|V)?([\\d.]+)".to_owned()),
                    _ => Some(s.to_owned()),
                };

                Ok(Checkver {
                    regex,
                    url: None,
                    jsonpath: None,
                    xpath: None,
                    reverse: None,
                    replace: None,
                    useragent: None,
                    script: None,
                })
            }

            fn visit_map<A>(self, mut map: A) -> StdResult<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut regex = None;
                let mut url = None;
                let mut jsonpath = None;
                let mut xpath = None;
                let mut reverse = None;
                let mut replace = None;
                let mut useragent = None;
                let mut script = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "re" | "regex" => regex = Some(map.next_value()?),
                        "url" => url = Some(map.next_value()?),
                        "jp" | "jsonpath" => jsonpath = Some(map.next_value()?),
                        "xpath" => xpath = Some(map.next_value()?),
                        "reverse" => {
                            let val =
                                bool::from_str(map.next_value()?).map_err(de::Error::custom)?;
                            reverse = Some(val);
                        }
                        "replace" => replace = Some(map.next_value()?),
                        "useragent" => useragent = Some(map.next_value()?),
                        "script" => {
                            // Can we avoid using `serde_json::Value` here?
                            let value: serde_json::Value = map.next_value()?;
                            let vi = value
                                .deserialize_any(OneOrVecVisitor(PhantomData))
                                .map_err(de::Error::custom)?;
                            script = Some(VecItem(vi))
                        }
                        _ => continue,
                    }
                }

                Ok(Checkver {
                    regex,
                    url,
                    jsonpath,
                    xpath,
                    reverse,
                    replace,
                    useragent,
                    script,
                })
            }
        }

        Ok(deserializer.deserialize_any(CheckverVisitor)?)
    }
}

/// A custom [`Visitor`] to visit a single `T` item or a vec of `T` items.
struct OneOrVecVisitor<T>(PhantomData<T>);
impl<'de, T> Visitor<'de> for OneOrVecVisitor<T>
where
    T: Deserialize<'de> + FromStr,
{
    type Value = Vec<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("one item or list of items")
    }

    #[inline]
    fn visit_str<E>(self, s: &str) -> StdResult<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(vec![T::from_str(s).ok().unwrap()])
    }

    fn visit_seq<S>(self, mut seq: S) -> StdResult<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let mut vec: Vec<T> = vec![];
        while let Some(item) = seq.next_element()? {
            vec.push(item)
        }

        Ok(vec)
    }
}

/// A [`VecItem`] represents an special item which might be deserialized from
/// a single `String` or a list or `String`s. There are different fields in a
/// Scoop [`Manifest`] using this data type:
///
/// - **bin**: A `bin` can be represented with a single `String` as its name,
/// or a `String` array containing 3 `String`s, *name*, *shim name* and *shim
/// args* respectively. \[`name`, `Option<shim_name>`, `Option<shim_args>`]
/// - **persist**: A `persist` item can be represented with a single `String`
/// as its name or a `String` array containing 2 `String`s, *source name*,
/// *target name* respectively. \[`source_name`, `Option<target_name>`]
/// - **notes**: A `notes` can be represented with a single `String` as the
/// only one note or a `String` vector containing more notes. \[`note_string`, ...]
#[derive(Clone, Debug, Serialize)]
pub struct VecItem(Vec<String>);

impl Deref for VecItem {
    type Target = Vec<String>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for VecItem {
    type Err = Infallible;

    #[inline]
    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        Ok(VecItem(vec![s.to_owned()]))
    }
}

impl<'de> Deserialize<'de> for VecItem {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(VecItem(
            deserializer.deserialize_any(OneOrVecVisitor(PhantomData))?,
        ))
    }
}

/// A representation of binaries to be shimmed of a Scoop app manifest.
#[derive(Clone, Debug, Serialize)]
pub struct Bins(Vec<VecItem>);

impl Deref for Bins {
    type Target = Vec<VecItem>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Bins {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Bins(
            deserializer.deserialize_any(OneOrVecVisitor(PhantomData))?,
        ))
    }
}

/// A representation of a list of entry to be persisted a Scoop app manifest.
#[derive(Clone, Debug, Serialize)]
pub struct Persist(Vec<VecItem>);

impl Deref for Persist {
    type Target = Vec<VecItem>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Persist {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Persist(
            deserializer.deserialize_any(OneOrVecVisitor(PhantomData))?,
        ))
    }
}

/// A representation of the download urls of a Scoop app manifest.
#[derive(Clone, Debug, Serialize)]
pub struct Urls(Vec<String>);

impl Deref for Urls {
    type Target = Vec<String>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Urls {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Urls(
            deserializer.deserialize_any(OneOrVecVisitor(PhantomData))?,
        ))
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct License {
    identifier: String,
    url: Option<String>,
}

impl License {
    fn new(identifier: String, mut url: Option<String>) -> License {
        // SPDX identifier detection
        let id = identifier.as_str();
        let is_spdx = self::license::SPDX.contains(id);
        if url.is_none() && is_spdx {
            url = Some(format!("https://spdx.org/licenses/{}.html", id));
        }

        License { identifier, url }
    }

    #[inline]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    #[inline]
    pub fn url(&self) -> Option<&String> {
        self.url.as_ref()
    }
}

impl<'de> Deserialize<'de> for License {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LicenseVisitor;
        impl<'de> Visitor<'de> for LicenseVisitor {
            type Value = License;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("license string or map")
            }

            #[inline]
            fn visit_str<E>(self, s: &str) -> StdResult<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(License::new(s.to_owned(), None))
            }

            fn visit_map<A>(self, mut map: A) -> StdResult<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut identifier = Err(de::Error::missing_field("identifier"));
                let mut url = None;

                while let Some((key, value)) = map.next_entry()? {
                    match key {
                        "identifier" => identifier = Ok(value),
                        "url" => url = Some(value),
                        _ => continue,
                    }
                }

                Ok(License::new(identifier?, url))
            }
        }

        Ok(deserializer.deserialize_any(LicenseVisitor)?)
    }
}

/// [`Hash(String)`] represents a valid hash provided in a Scoop app manifest.
/// Currently, it could be one of the following formats:
///
/// - **md5**: `^md5:[a-fA-F0-9]{32}$`
/// - **sha1**: `^sha1:[a-fA-F0-9]{40}$`
/// - **sha256**: `^(sha256:)?[a-fA-F0-9]{64}$`
/// - **sha512**: `^sha512:[a-fA-F0-9]{128}$`
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hash(String);

impl FromStr for Hash {
    type Err = Error;
    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        match Self::validate(s) {
            false => Err(Error(ErrorKind::Custom(format!(
                "{} is not a valid hash string",
                s
            )))),
            true => Ok(Self(String::from(s))),
        }
    }
}

impl Hash {
    fn validate(s: &str) -> bool {
        static REGEX_HASH: Lazy<Regex> = Lazy::new(|| {
            RegexBuilder::new(r"^md5:[a-fA-F0-9]{32}|sha1:[a-fA-F0-9]{40}|(sha256:)?[a-fA-F0-9]{64}|sha512:[a-fA-F0-9]{128}$")
                .build()
                .unwrap()
        });
        REGEX_HASH.is_match(s)
    }
}

impl Deref for Hash {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A representation of the download files' hashes of a Scoop app manifest.
#[derive(Clone, Debug, Serialize)]
pub struct Hashes(Vec<Hash>);

impl Deref for Hashes {
    type Target = Vec<Hash>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Hashes {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Hashes(
            deserializer.deserialize_any(OneOrVecVisitor(PhantomData))?,
        ))
    }
}

////////////////////////////////////////////////////////////////////////////////
//  Manifest impls
////////////////////////////////////////////////////////////////////////////////

impl Manifest {
    /// Create a [`Manifest`] representation of a json file with the given path.
    ///
    /// ## Errors
    ///
    /// This method will return a `std::io::Error` when the given path can't be
    /// read.
    ///
    /// It will return a `serde_json::Error` when json deserializing fail.
    pub fn from_path<P: AsRef<Path> + ?Sized>(path: &P) -> Result<Manifest> {
        // We read the entire manifest json file into memory first and then
        // deserialize it, as this is *a lot* faster than reading via the
        // `serde_json::from_reader`. See https://github.com/serde-rs/json/issues/160
        let mut bytes = Vec::new();
        File::open(path)?.read_to_end(&mut bytes)?;

        // Reading manifest json file is a bottleneck of the whole scoop-rs
        // project. We use `serde_json` because it's well documented and easy
        // to integrate. But I believe there should be an alternative to
        // `serde_json` which can parse json file much *faster*, perhaps
        // `simd_json` can be. See https://github.com/serde-rs/json-benchmark
        let data = serde_json::from_slice(&bytes)?;
        let name = leaf_base(path);
        let bucket = utils::extract_bucket_from(path);
        let path = path.as_ref().to_path_buf();
        log::debug!("{:?}", data);

        Ok(Manifest {
            name,
            path,
            bucket,
            inner: data,
            _private: (),
        })
    }

    #[inline]
    pub fn get_name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn get_manifest_bucket(&self) -> Option<&String> {
        self.bucket.as_ref()
    }

    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[inline]
    pub fn get_version(&self) -> &str {
        &self.inner.version
    }

    #[inline]
    pub fn get_description(&self) -> Option<&String> {
        self.inner.description.as_ref()
    }

    #[inline]
    pub fn get_homepage(&self) -> Option<&String> {
        self.inner.homepage.as_ref()
    }

    #[inline]
    pub fn get_license(&self) -> Option<&License> {
        self.inner.license.as_ref()
    }

    #[inline]
    pub fn get_checkver(&self) -> Option<&Checkver> {
        self.inner.checkver.as_ref()
    }

    #[inline]
    pub fn get_bins(&self) -> Option<Bins> {
        let manifest = &self.inner;

        // TODO

        manifest.bin.clone()
    }

    /// Extract download urls from this manifest, in following order:
    ///
    /// 1. if "64bit" urls are available, return;
    /// 2. then if "32bit" urls are available, return;
    /// 3. fallback to return common urls.
    pub fn get_download_urls(&self) -> Urls {
        let manifest = &self.inner;

        match manifest.architecture.clone() {
            Some(arch) => {
                // Find amd64 urls first
                if arch.amd64.is_some() && utils::os_is_arch64() {
                    match arch.amd64.unwrap().url {
                        Some(url) => return url,
                        None => {}
                    }
                }

                // Find ia32 urls if amd64 is not available
                if arch.ia32.is_some() {
                    match arch.ia32.unwrap().url {
                        Some(url) => return url,
                        None => {}
                    }
                }
            }
            None => {}
        }

        // Finally fallback to common urls.
        //
        // SAFETY: this is safe because a valid manifest must have at least
        // one download url.
        manifest.url.clone().unwrap()
    }

    /// Extract file hashes from this manifest, in following order:
    ///
    /// 1. if "64bit" hashes are available, return;
    /// 2. then if "32bit" hashes are available, return;
    /// 3. fallback to return common hashes.
    pub fn get_hashes(&self) -> Option<Hashes> {
        let manifest = &self.inner;

        // `nightly` version does not have hashes.
        if manifest.version == "nightly" {
            return None;
        }

        match manifest.architecture.clone() {
            Some(arch) => {
                // Find amd64 hashes first
                if arch.amd64.is_some() && utils::os_is_arch64() {
                    let hashes = arch.amd64.unwrap().hash;
                    if hashes.is_some() {
                        return hashes.clone();
                    }
                }

                // Find ia32 hashes if amd64 is not available
                if arch.ia32.is_some() {
                    let hashes = arch.ia32.unwrap().hash;
                    if hashes.is_some() {
                        return hashes.clone();
                    }
                }
            }
            None => {}
        }

        // Finally fallback to common hashes.
        manifest.hash.clone()
    }
}
