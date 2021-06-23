mod license;

use once_cell::sync::Lazy;
use regex::Regex;
use regex::RegexBuilder;
use serde::de;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::Deserializer;
use serde_json::Map;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use serde_json::Value;

use crate::error::{Error, ErrorKind, Result};
use crate::fs::leaf_base;
use crate::utils;
////////////////////////////////////////////////////////////////////////////////
//  Manifest Custom Types
////////////////////////////////////////////////////////////////////////////////
type LicenseIdentifier = String;
type PersistType = BinType;
////////////////////////////////////////////////////////////////////////////////
//  Manifest Custom Enums
////////////////////////////////////////////////////////////////////////////////
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StringOrStringArray {
    String(String),
    Array(Vec<String>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BinType {
    String(String),
    Array(Vec<StringOrStringArray>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CheckverType {
    Simple(String),
    Complex(ComplexCheckver),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum License {
    Simple(LicenseIdentifier),
    Complex(LicensePair),
}

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
pub struct LicensePair {
    pub identifier: LicenseIdentifier,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Installer {
    pub args: Option<StringOrStringArray>,
    pub file: Option<String>,
    pub script: Option<StringOrStringArray>,
    pub keep: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Uninstaller {
    pub args: Option<StringOrStringArray>,
    pub file: Option<String>,
    pub script: Option<StringOrStringArray>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchitectureInner {
    pub bin: Option<BinType>,
    pub checkver: Option<CheckverType>,
    pub env_add_path: Option<StringOrStringArray>,
    pub env_set: Option<Map<String, Value>>,
    pub extract_dir: Option<StringOrStringArray>,
    #[serde(default, deserialize_with = "deserialize_option_hashes")]
    pub hash: Option<Hashes>,
    pub installer: Option<Installer>,
    pub post_install: Option<StringOrStringArray>,
    pub pre_install: Option<StringOrStringArray>,
    pub shortcuts: Option<Vec<ShortcutsType>>,
    pub uninstaller: Option<Uninstaller>,
    #[serde(default, deserialize_with = "deserialize_option_url")]
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
pub struct ComplexCheckver {
    pub github: Option<String>,
    pub re: Option<String>,
    pub regex: Option<String>,
    pub url: Option<String>,
    pub jp: Option<String>,
    pub jsonpath: Option<String>,
    pub xpath: Option<String>,
    pub reverse: Option<bool>,
    pub replace: Option<String>,
    pub useragent: Option<String>,
    pub script: Option<StringOrStringArray>,
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
    pub extract_dir: Option<StringOrStringArray>,
    pub url: Option<StringOrStringArray>,
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
    pub extract_dir: Option<StringOrStringArray>,
    pub hash: Option<HashExtraction>,
    pub note: Option<StringOrStringArray>,
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
    pub bin: Option<BinType>,
    pub persist: Option<PersistType>,
    pub checkver: Option<CheckverType>,
    pub cookie: Option<Value>,
    pub depends: Option<StringOrStringArray>,
    pub description: Option<String>,
    pub env_add_path: Option<StringOrStringArray>,
    pub env_set: Option<Map<String, Value>>,
    pub extract_dir: Option<StringOrStringArray>,
    pub extract_to: Option<StringOrStringArray>,
    #[serde(default, deserialize_with = "deserialize_option_hashes")]
    pub hash: Option<Hashes>,
    pub homepage: Option<String>,
    pub innosetup: Option<bool>,
    pub installer: Option<Installer>,
    pub license: Option<License>,
    pub notes: Option<StringOrStringArray>,
    pub post_install: Option<StringOrStringArray>,
    pub pre_install: Option<StringOrStringArray>,
    pub psmodule: Option<Psmodule>,
    pub shortcuts: Option<Vec<ShortcutsType>>,
    pub suggest: Option<Value>,
    pub uninstaller: Option<Uninstaller>,
    #[serde(default, deserialize_with = "deserialize_option_url")]
    pub url: Option<Urls>,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub name: String,
    pub path: PathBuf,
    pub bucket: Option<String>,
    pub data: ManifestInner,
    _private: (),
}

/// A representation of the download urls of a Scoop app manifest.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Urls(Vec<String>);

impl Deref for Urls {
    type Target = Vec<String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct UrlVisitor;
impl<'de> Visitor<'de> for UrlVisitor {
    type Value = Urls;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("url string or list of url strings")
    }

    fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Urls(vec![s.to_string()]))
    }

    fn visit_seq<S>(self, mut seq: S) -> std::result::Result<Self::Value, S::Error>
    where
        S: SeqAccess<'de>,
    {
        let mut v: Vec<String> = Vec::new();
        while let Some(item) = seq.next_element()? {
            v.push(item)
        }

        Ok(Urls(v))
    }
}

#[allow(unused)]
fn deserialize_url<'de, D>(deserializer: D) -> std::result::Result<Urls, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(UrlVisitor)
}

fn deserialize_option_url<'de, D>(deserializer: D) -> std::result::Result<Option<Urls>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionalUrlVisitor;
    impl<'de> Visitor<'de> for OptionalUrlVisitor {
        type Value = Option<Urls>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("null or string or list of strings")
        }

        fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, d: D) -> std::result::Result<Self::Value, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            Ok(Some(d.deserialize_any(UrlVisitor)?))
        }
    }

    deserializer.deserialize_option(OptionalUrlVisitor)
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
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn deserialize_option_hashes<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Hashes>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionalHashesVisitor;
    impl<'de> Visitor<'de> for OptionalHashesVisitor {
        type Value = Option<Hashes>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("null or string or list of strings")
        }

        fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, d: D) -> std::result::Result<Self::Value, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            let inner = d.deserialize_any(HashVisitor)?;
            Ok(Some(Hashes(inner)))
        }
    }

    struct HashVisitor;
    impl<'de> Visitor<'de> for HashVisitor {
        type Value = Vec<Hash>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("hash string or list of hash strings")
        }

        fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Hash::from_str(s).map(|hs| vec![hs]).map_err(E::custom)
        }

        fn visit_seq<S>(self, mut seq: S) -> std::result::Result<Self::Value, S::Error>
        where
            S: SeqAccess<'de>,
        {
            let mut v: Vec<Hash> = Vec::new();
            while let Some(item) = seq.next_element()? {
                match Hash::from_str(item).map_err(de::Error::custom) {
                    Ok(hs) => v.push(hs),
                    Err(e) => return Err(e),
                }
            }

            Ok(v)
        }
    }

    deserializer.deserialize_option(OptionalHashesVisitor)
}

/// A representation of the download files' hashes of a Scoop app manifest.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hashes(Vec<Hash>);

impl Deref for Hashes {
    type Target = Vec<Hash>;
    fn deref(&self) -> &Self::Target {
        &self.0
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

        Ok(Manifest {
            name,
            path,
            bucket,
            data,
            _private: (),
        })
    }

    /// Extract download urls from this manifest, in following order:
    ///
    /// 1. if "64bit" urls are available, return;
    /// 2. then if "32bit" urls are available, return;
    /// 3. fallback to return common urls.
    pub fn get_download_urls(&self) -> Urls {
        let manifest = &self.data;

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
        let manifest = &self.data;

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
