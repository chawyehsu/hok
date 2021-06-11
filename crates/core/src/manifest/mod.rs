mod hashstring;
mod license;
mod url;

use serde_json::Map;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::Result;
use crate::fs::leaf_base;
use crate::utils;
use hashstring::{deserialize_option_hash, Hash};
use url::{deserialize_option_url, Url};

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
    #[serde(default, deserialize_with = "deserialize_option_hash")]
    pub hash: Option<Hash>,
    pub installer: Option<Installer>,
    pub post_install: Option<StringOrStringArray>,
    pub pre_install: Option<StringOrStringArray>,
    pub shortcuts: Option<Vec<ShortcutsType>>,
    pub uninstaller: Option<Uninstaller>,
    #[serde(default, deserialize_with = "deserialize_option_url")]
    pub url: Option<Url>,
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
    #[serde(default, deserialize_with = "deserialize_option_url")]
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

    // pub fn from_url<U: IntoUrl>(url: U, scoop: &Scoop) -> Result<Manifest> {
    //     let resp = scoop.http.get(url.as_str()).send();
    //     match resp {
    //         Ok(res) => {
    //             let path = PathBuf::from(url.as_str());
    //             let name = leaf_base(path.as_path());
    //             let raw = res.json().unwrap();
    //             Ok(Manifest {
    //                 name,
    //                 bucket: None,
    //                 path,
    //                 data: raw,
    //             })
    //         }
    //         Err(e) => Err(error::Error::from(e)),
    //     }
    // }

    /// Extract download urls from this manifest, in following order:
    ///
    /// 1. if "64bit" urls are available, return;
    /// 2. then if "32bit" urls are available, return;
    /// 3. fallback to return common urls.
    pub fn get_download_urls(&self) -> Option<Url> {
        let manifest = &self.data;
        let fallback_url = manifest.url.clone();

        match manifest.architecture.clone() {
            Some(arch) => {
                // Find amd64 urls first
                if arch.amd64.is_some() && utils::os_is_arch64() {
                    match arch.amd64.unwrap().url {
                        Some(url) => return Some(url),
                        None => {},
                    }
                }

                // Find ia32 urls if amd64 is not available
                if arch.ia32.is_some() {
                    match arch.ia32.unwrap().url {
                        Some(url) => return Some(url),
                        None => {},
                    }
                }
            },
            None => {}
        }

        // Final, fallback to common urls
        fallback_url
    }

    pub fn get_hashes(&self) -> Option<Hash> {
        let manifest = &self.data;

        if manifest.architecture.is_some() {
            let arch = manifest.architecture.clone().unwrap();
            if arch.amd64.is_some() && utils::os_is_arch64() {
                arch.amd64.clone().unwrap().hash
            } else if arch.ia32.is_some() {
                arch.ia32.clone().unwrap().hash
            } else {
                None
            }
        } else if manifest.hash.is_some() {
            manifest.hash.clone()
        } else {
            None
        }
    }
}
