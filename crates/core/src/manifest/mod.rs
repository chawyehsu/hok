mod hashstring;
mod license;
mod url;

use reqwest::IntoUrl;
use serde_json::Map;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::{self, Result};
use crate::fs::leaf_base;
use crate::utils;
use crate::Scoop;
use hashstring::{deserialize_option_hash, Hash};

////////////////////////////////////////////////////////////////////////////////
//  Manifest Custom Types
////////////////////////////////////////////////////////////////////////////////
type LicenseIdentifier = String;
type PersistType = BinType;
type Url = StringOrStringArray;
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
    #[serde(default, deserialize_with = "deserialize_option_hash")]
    pub hash: Option<Hash>,
    pub installer: Option<Installer>,
    pub post_install: Option<StringOrStringArray>,
    pub pre_install: Option<StringOrStringArray>,
    pub shortcuts: Option<Vec<ShortcutsType>>,
    pub uninstaller: Option<Uninstaller>,
    pub url: Option<Url>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Architecture {
    #[serde(rename = "32bit")]
    i386: Option<ArchitectureInner>,
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
    pub url: Option<Url>,
    pub hash: Option<HashExtraction>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AutoupdateArchitecture {
    #[serde(rename = "32bit")]
    i386: Option<AutoupdateArchitectureInner>,
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
pub struct ManifestRaw {
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
    #[serde(default, deserialize_with = "deserialize_option_hash")]
    pub hash: Option<Hash>,
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
    pub url: Option<Url>,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub name: String,
    pub path: PathBuf,
    pub bucket: Option<String>,
    pub data: ManifestRaw,
}

////////////////////////////////////////////////////////////////////////////////
//  Manifest impls
////////////////////////////////////////////////////////////////////////////////

impl Manifest {
    pub fn from_path<P: AsRef<Path> + ?Sized>(path: &P) -> Result<Manifest> {
        // We read the entire manifest json file into memory first and then
        // deserialize it, as this is *a lot* faster than reading via the
        // `serde_json::from_reader`. See https://github.com/serde-rs/json/issues/160
        //
        // Reading manifest json file is a bottleneck of the whole scoop-rs
        // project. We use `serde_json` because it's well documented and easy
        // to integrate. But I believe there should be an alternative to
        // `serde_json` which can parse json file much *faster*, perhaps
        // `simd_json` can be. See https://github.com/serde-rs/json-benchmark
        let mut bytes = Vec::new();
        File::open(path)?.read_to_end(&mut bytes)?;
        let manifest = serde_json::from_slice(&bytes);

        let data: ManifestRaw = manifest?;

        let name = leaf_base(path);
        let bucket = utils::extract_bucket_from(path);
        let path = path.as_ref().to_path_buf();

        Ok(Manifest {
            name,
            path,
            bucket,
            data,
        })
    }

    pub fn from_url<U: IntoUrl>(url: U, scoop: &Scoop) -> Result<Manifest> {
        let resp = scoop.http.get(url.as_str()).send();
        match resp {
            Ok(res) => {
                let path = PathBuf::from(url.as_str());
                let name = leaf_base(path.as_path());
                let raw = res.json().unwrap();
                Ok(Manifest {
                    name,
                    bucket: None,
                    path,
                    data: raw,
                })
            }
            Err(e) => Err(error::Error::from(e)),
        }
    }
}
