use anyhow::Result;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;

use crate::fs;
use crate::utils;
use crate::Scoop;

////////////////////////////////////////////////////////////////////////////////
//  Manifest Custom Types
////////////////////////////////////////////////////////////////////////////////
type Hash = String;
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
pub enum HashType {
    Single(Hash),
    Multiple(Vec<Hash>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BinType {
    Single(String),
    Multiple(Vec<String>),
    Complex(Vec<StringOrStringArray>),
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
    pub env_set: Option<Value>,
    pub extract_dir: Option<StringOrStringArray>,
    pub hash: Option<HashType>,
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
    arch_32: Option<ArchitectureInner>,
    #[serde(rename = "64bit")]
    arch_64: Option<ArchitectureInner>,
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
    arch_32: Option<AutoupdateArchitectureInner>,
    #[serde(rename = "64bit")]
    arch_64: Option<AutoupdateArchitectureInner>,
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
    pub env_set: Option<Value>,
    pub extract_dir: Option<StringOrStringArray>,
    pub extract_to: Option<StringOrStringArray>,
    pub hash: Option<HashType>,
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

impl Manifest {
    /// Create an [`Manifest`] from the given [`PathBuf`].
    pub fn from_path<P: AsRef<Path> + ?Sized>(path: &P) -> Result<Manifest> {
        // We read the entire manifest json file into memory first and then
        // deserialize it, as this is *a lot* faster than reading via the
        // `serde_json::from_reader`. See https://github.com/serde-rs/json/issues/160
        let mut s = String::new();
        File::open(path)?.read_to_string(&mut s)?;
        let data = serde_json::from_str(&s)?;
        let name = fs::leaf_base(path);
        let bucket = utils::extract_bucket_from(path);
        let path = path.as_ref().to_path_buf();

        Ok(Manifest {
            name,
            path,
            bucket,
            data,
        })
    }

    pub fn from_url<T: AsRef<str>>(_url: T) -> Result<Manifest> {
        todo!()
    }
}

impl<'a> Scoop<'a> {
    /// Find and return local manifest represented as [`ScoopAppManifest`],
    /// using given `pattern`.
    ///
    /// bucket name prefix is support, for example:
    /// ```
    /// find_local_manifest("main/gcc")
    /// ```
    pub fn find_local_manifest<T: AsRef<str>>(&mut self, pattern: T) -> Result<Option<Manifest>> {
        // Detect given pattern whether having bucket name prefix
        let (bucket_name, app_name) = match pattern.as_ref().contains("/") {
            true => {
                let (a, b) = pattern.as_ref().split_once("/").unwrap();
                (Some(a), b)
            }
            false => (None, pattern.as_ref()),
        };

        match bucket_name {
            Some(bucket_name) => {
                let bucket = self.bucket_manager.get_bucket(bucket_name).unwrap();
                let manifest_path = bucket.manifest_dir().join(format!("{}.json", app_name));
                match manifest_path.exists() {
                    true => Ok(Some(Manifest::from_path(&manifest_path)?)),
                    false => Ok(None),
                }
            }
            None => {
                for bucket in self.bucket_manager.get_buckets() {
                    let manifest_path = bucket.1.manifest_dir().join(format!("{}.json", app_name));
                    match manifest_path.exists() {
                        true => return Ok(Some(Manifest::from_path(&manifest_path)?)),
                        false => {}
                    }
                }

                Ok(None)
            }
        }
    }

    // Deprecated, will be replaced by ScoopAppManifest::from_url()
    // #[deprecated]
    // pub fn manifest_from_url(&self, manifest_url: &str) -> Result<Value> {
    //   // Use proxy from Scoop's config
    //   let agent = match self.config["proxy"].clone() {
    //     Value::String(mut proxy) => {
    //       if !proxy.starts_with("http") {
    //         proxy.insert_str(0, "http://");
    //       }

    //       let proxy = ureq::Proxy::new(proxy)?;

    //       ureq::AgentBuilder::new()
    //         .proxy(proxy)
    //         .build()
    //     },
    //     _ => {
    //       ureq::AgentBuilder::new()
    //         .build()
    //     }
    //   };

    //   let body: serde_json::Value = agent.get(manifest_url)
    //     .call()?
    //     .into_json()?;

    //   Ok(body)
    // }
}
