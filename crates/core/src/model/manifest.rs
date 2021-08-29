use once_cell::sync::Lazy;
use regex::Regex;
use regex::RegexBuilder;
use serde::de;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;
use serde_json::Map;
use std::collections::HashSet;
use std::convert::Infallible;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::Path;
use std::result::Result;
use std::str::FromStr;

use crate::license;
use crate::util::block_on;
use crate::Config;
use crate::HttpClient;
use crate::ScoopResult;

////////////////////////////////////////////////////////////////////////////////
//  Manifest Custom Enums
////////////////////////////////////////////////////////////////////////////////
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
    args: Option<VecItem>,
    file: Option<String>,
    script: Option<VecItem>,
    keep: Option<bool>,
}

impl Installer {
    #[inline]
    pub fn get_args(&self) -> Option<&VecItem> {
        self.args.as_ref()
    }

    #[inline]
    pub fn get_file(&self) -> Option<&String> {
        self.file.as_ref()
    }

    #[inline]
    pub fn get_script(&self) -> Option<&VecItem> {
        self.script.as_ref()
    }

    #[inline]
    pub fn is_keep(&self) -> bool {
        self.keep.unwrap_or(false)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Uninstaller {
    pub args: Option<VecItem>,
    pub file: Option<String>,
    pub script: Option<VecItem>,
}

impl Uninstaller {
    #[inline]
    pub fn get_args(&self) -> Option<&VecItem> {
        self.args.as_ref()
    }

    #[inline]
    pub fn get_file(&self) -> Option<&String> {
        self.file.as_ref()
    }

    #[inline]
    pub fn get_script(&self) -> Option<&VecItem> {
        self.script.as_ref()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchitectureInner {
    bin: Option<Bins>,
    checkver: Option<Checkver>,
    env_add_path: Option<VecItem>,
    env_set: Option<Map<String, serde_json::Value>>,
    extract_dir: Option<VecItem>,
    hash: Option<Hashes>,
    installer: Option<Installer>,
    post_install: Option<VecItem>,
    pre_install: Option<VecItem>,
    shortcuts: Option<Vec<Vec<String>>>,
    uninstaller: Option<Uninstaller>,
    url: Option<Urls>,
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

/// [`ManifestInner`] represents the actual structure of a Scoop app manifest.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ManifestInner {
    version: String,
    description: Option<String>,
    homepage: Option<String>,
    license: Option<License>,
    depends: Option<VecItem>,
    innosetup: Option<bool>,
    cookie: Option<Map<String, serde_json::Value>>,
    architecture: Option<Architecture>,
    url: Option<Urls>,
    hash: Option<Hashes>,
    extract_dir: Option<VecItem>,
    extract_to: Option<VecItem>,
    pre_install: Option<VecItem>,
    installer: Option<Installer>,
    uninstaller: Option<Uninstaller>,
    post_install: Option<VecItem>,
    bin: Option<Bins>,
    env_add_path: Option<VecItem>,
    env_set: Option<Map<String, serde_json::Value>>,
    shortcuts: Option<Vec<Vec<String>>>,
    persist: Option<Persist>,
    psmodule: Option<Psmodule>,
    suggest: Option<Map<String, serde_json::Value>>,
    checkver: Option<Checkver>,
    autoupdate: Option<Autoupdate>,
    notes: Option<VecItem>,
}

/// A [`Manifest`] basicly represents an app/package which can be installed by
/// Scoop. It is a JSON file that contains all the information, such as version,
/// downloading source of installers and excutables, etc. about an app/package.
///
/// This is deserialized using the `serde_json` crate with custom settings. It
/// has a [JSON Schema] which defines how a JSON file can be a valid Manifest.
///
/// [JSON Schema]: https://github.com/lukesampson/scoop/blob/master/schema.json
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    /// A manifest's path can be a local file path or a remote URL.
    path: String,
    inner: ManifestInner,
}

#[derive(Clone, Debug, Serialize)]
pub struct Checkver {
    pub regex: Option<String>,
    pub url: Option<String>,
    pub jsonpath: Option<String>,
    pub xpath: Option<String>,
    pub reverse: Option<bool>,
    pub replace: Option<String>,
    pub useragent: Option<String>,
    pub script: Option<VecItem>,
}

impl<'de> Deserialize<'de> for Checkver {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CheckverVisitor;
        impl<'de> Visitor<'de> for CheckverVisitor {
            type Value = Checkver;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("license string or map")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
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

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
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
                        "github" => {
                            let prefix: String = map.next_value()?;
                            url = Some(format!("{}/releases/latest", prefix));
                            regex = Some("/releases/tag/(?:v|V)?([\\d.]+)".to_owned());
                        }
                        "re" | "regex" => regex = Some(map.next_value()?),
                        "url" => url = Some(map.next_value()?),
                        "jp" | "jsonpath" => jsonpath = Some(map.next_value()?),
                        "xpath" => xpath = Some(map.next_value()?),
                        "reverse" => reverse = Some(map.next_value()?),
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
                        _ => {
                            // skip next_value
                            let _ = map.next_value()?;
                            continue;
                        }
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
    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(vec![T::from_str(s).ok().unwrap()])
    }

    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
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
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(VecItem(vec![s.to_owned()]))
    }
}

impl<'de> Deserialize<'de> for VecItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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
        let is_spdx = license::SPDX.contains(id);
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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
            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(License::new(s.to_owned(), None))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
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
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Self::validate(s) {
            false => anyhow::bail!("{} is not a valid hash string", s),
            true => Ok(Self(String::from(s))),
        }
    }
}

impl AsRef<str> for Hash {
    fn as_ref(&self) -> &str {
        self.0.as_str()
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Hashes(
            deserializer.deserialize_any(OneOrVecVisitor(PhantomData))?,
        ))
    }
}

impl Architecture {
    #[inline]
    pub fn ia32(&self) -> Option<&ArchitectureInner> {
        self.ia32.as_ref()
    }

    #[inline]
    pub fn amd64(&self) -> Option<&ArchitectureInner> {
        self.amd64.as_ref()
    }
}

impl ArchitectureInner {
    /// Return a `bin` field reference of this [`ArchitectureInner`].
    #[inline]
    pub fn bin(&self) -> Option<&Bins> {
        self.bin.as_ref()
    }

    #[inline]
    pub fn get_checkver(&self) -> Option<&Checkver> {
        self.checkver.as_ref()
    }

    #[inline]
    pub fn get_hash(&self) -> Option<&Hashes> {
        self.hash.as_ref()
    }

    #[inline]
    pub fn get_extract_dir(&self) -> Option<&VecItem> {
        self.extract_dir.as_ref()
    }

    #[inline]
    pub fn get_post_install(&self) -> Option<&VecItem> {
        self.post_install.as_ref()
    }

    #[inline]
    pub fn get_pre_install(&self) -> Option<&VecItem> {
        self.pre_install.as_ref()
    }

    #[inline]
    pub fn get_installer(&self) -> Option<&Installer> {
        self.installer.as_ref()
    }

    #[inline]
    pub fn get_uninstaller(&self) -> Option<&Uninstaller> {
        self.uninstaller.as_ref()
    }

    #[inline]
    pub fn get_shortcuts(&self) -> Option<&Vec<Vec<String>>> {
        self.shortcuts.as_ref()
    }

    #[inline]
    pub fn get_url(&self) -> Option<&Urls> {
        self.url.as_ref()
    }
}

impl ManifestInner {
    #[inline]
    pub fn get_architecture(&self) -> Option<&Architecture> {
        self.architecture.as_ref()
    }

    #[inline]
    pub fn get_hash(&self) -> Option<&Hashes> {
        self.hash.as_ref()
    }

    #[inline]
    pub fn get_url(&self) -> Option<&Urls> {
        self.url.as_ref()
    }

    #[inline]
    pub fn get_extract_dir(&self) -> Option<&VecItem> {
        match self.get_architecture() {
            None => {}
            Some(arch) => {
                // amd64
                if cfg!(target_arch = "x86_64") {
                    if arch.amd64().is_some() {
                        let extract_dir = arch.amd64().unwrap().get_extract_dir();
                        if extract_dir.is_some() {
                            return extract_dir;
                        }
                    }
                }

                // ia32
                if cfg!(target_arch = "x86") {
                    if arch.ia32().is_some() {
                        let extract_dir = arch.ia32().unwrap().get_extract_dir();
                        if extract_dir.is_some() {
                            return extract_dir;
                        }
                    }
                }
            }
        }

        // fallback, arch-less `extract_dir`
        self.extract_dir.as_ref()
    }

    #[inline]
    pub fn get_extract_to(&self) -> Option<&VecItem> {
        self.extract_to.as_ref()
    }

    #[inline]
    pub fn get_post_install(&self) -> Option<&VecItem> {
        match self.get_architecture() {
            None => {}
            Some(arch) => {
                // amd64
                if cfg!(target_arch = "x86_64") {
                    if arch.amd64().is_some() {
                        let post_install = arch.amd64().unwrap().get_post_install();
                        // ensure post_install script exists while return,
                        // or fallback to the arch-less post_install one.
                        if post_install.is_some() {
                            return post_install;
                        }
                    }
                }

                // ia32
                if cfg!(target_arch = "x86") {
                    if arch.ia32().is_some() {
                        let post_install = arch.ia32().unwrap().get_post_install();
                        if post_install.is_some() {
                            return post_install;
                        }
                    }
                }
            }
        }

        // fallback, arch-less `post_install`
        self.post_install.as_ref()
    }

    #[inline]
    pub fn get_pre_install(&self) -> Option<&VecItem> {
        match self.get_architecture() {
            None => {}
            Some(arch) => {
                // amd64
                if cfg!(target_arch = "x86_64") {
                    if arch.amd64().is_some() {
                        let pre_install = arch.amd64().unwrap().get_pre_install();
                        // ensure pre_install script exists while return,
                        // or fallback to the arch-less pre_install one.
                        if pre_install.is_some() {
                            return pre_install;
                        }
                    }
                }

                // ia32
                if cfg!(target_arch = "x86") {
                    if arch.ia32().is_some() {
                        let pre_install = arch.ia32().unwrap().get_pre_install();
                        if pre_install.is_some() {
                            return pre_install;
                        }
                    }
                }
            }
        }

        // fallback, arch-less `pre_install`
        self.pre_install.as_ref()
    }

    #[inline]
    pub fn get_installer(&self) -> Option<&Installer> {
        match self.get_architecture() {
            None => {}
            Some(arch) => {
                // amd64
                if cfg!(target_arch = "x86_64") {
                    if arch.amd64().is_some() {
                        let installer = arch.amd64().unwrap().get_installer();
                        // ensure installer script exists while return,
                        // or fallback to the arch-less installer one.
                        if installer.is_some() {
                            return installer;
                        }
                    }
                }

                // ia32
                if cfg!(target_arch = "x86") {
                    if arch.ia32().is_some() {
                        let installer = arch.ia32().unwrap().get_installer();
                        if installer.is_some() {
                            return installer;
                        }
                    }
                }
            }
        }

        // fallback, arch-less `installer`
        self.installer.as_ref()
    }

    /// Return a `bin` reference of this [`ManifestInner`].
    ///
    /// It returns arch-specific `bin` field when it is [`Some`], or it will
    /// return the arch-less one.
    #[inline]
    pub fn bin(&self) -> Option<&Bins> {
        if self.get_architecture().is_some() {
            let arch = self.get_architecture().unwrap();
            // amd64
            if cfg!(target_arch = "x86_64") {
                if arch.amd64().is_some() {
                    let bin = arch.amd64().unwrap().bin();
                    // ensure arch-specific `bin` exists while return,
                    // or fallback to the arch-less one.
                    if bin.is_some() {
                        return bin;
                    }
                }
            }

            // ia32
            if cfg!(target_arch = "x86") {
                if arch.ia32().is_some() {
                    let bin = arch.ia32().unwrap().bin();
                    // ensure arch-specific `bin` exists while return,
                    // or fallback to the arch-less one.
                    if bin.is_some() {
                        return bin;
                    }
                }
            }
        }

        // fallback, arch-less `bin`
        self.bin.as_ref()
    }

    #[inline]
    pub fn get_uninstaller(&self) -> Option<&Uninstaller> {
        match self.get_architecture() {
            None => {}
            Some(arch) => {
                // amd64
                if cfg!(target_arch = "x86_64") {
                    if arch.amd64().is_some() {
                        let uninstaller = arch.amd64().unwrap().get_uninstaller();
                        // ensure installer script exists while return,
                        // or fallback to the arch-less installer one.
                        if uninstaller.is_some() {
                            return uninstaller;
                        }
                    }
                }

                // ia32
                if cfg!(target_arch = "x86") {
                    if arch.ia32().is_some() {
                        let uninstaller = arch.ia32().unwrap().get_uninstaller();
                        if uninstaller.is_some() {
                            return uninstaller;
                        }
                    }
                }
            }
        }

        // fallback, arch-less `uninstaller`
        self.uninstaller.as_ref()
    }

    #[inline]
    pub fn get_installer_script(&self) -> Option<&VecItem> {
        match self.get_installer() {
            None => None,
            Some(installer) => installer.get_script(),
        }
    }

    #[inline]
    pub fn get_uninstaller_script(&self) -> Option<&VecItem> {
        match self.get_uninstaller() {
            None => None,
            Some(uninstaller) => uninstaller.get_script(),
        }
    }

    #[inline]
    pub fn get_shortcuts(&self) -> Option<&Vec<Vec<String>>> {
        self.shortcuts.as_ref()
    }

    #[inline]
    pub fn get_suggest(&self) -> Option<&Map<String, serde_json::Value>> {
        self.suggest.as_ref()
    }
}

////////////////////////////////////////////////////////////////////////////////
//  Manifest impls
////////////////////////////////////////////////////////////////////////////////
impl Manifest {
    /// Create a [`Manifest`] representation of a manfest JSON file with the
    /// given path.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// let path = PathBuf::from(r"C:\Scoop\buckets\main\bucket\unzip.json");
    /// let manifest = Manifest::new(path);
    /// // Print version information of the manifest
    /// println!("{}", manifest.version());
    /// ```
    ///
    /// ## Errors
    ///
    /// If the process fails to read the file, this method will return a
    /// [`std::io::error::Error`].
    ///
    /// It returns a `serde_json::Error` when the JSON deserialization fails.
    pub fn new<P: AsRef<Path>>(path: P) -> ScoopResult<Manifest> {
        let path = path.as_ref().to_path_buf();

        // We read the entire manifest JSON file into memory first and then
        // deserialize it, as this is *a lot* faster than reading via the
        // `serde_json::from_reader`. See https://github.com/serde-rs/json/issues/160
        let mut bytes = Vec::new();
        File::open(path.as_path())?.read_to_end(&mut bytes)?;

        // Reading manifest JSON file is a bottleneck of the whole scoop-rs
        // project. We use `serde_json` because it's well documented and easy
        // to integrate. But I believe there should be an alternative to
        // `serde_json` which can parse JSON file much *faster*, perhaps
        // `simd_json` can be. See https://github.com/serde-rs/json-benchmark
        let inner: ManifestInner = serde_json::from_slice(&bytes)?;
        let path = path.to_string_lossy().to_string();
        // log::debug!("{:?}", inner);
        Ok(Manifest { path, inner })
    }

    /// Create a [`Manifest`] representation of a manfest JSON file with the
    /// given url.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// let url = "https://git.io/JcpUI";
    /// let manifest = Manifest::from_url(url);
    /// // Print version information of the manifest
    /// println!("{}", manifest.version());
    /// ```
    ///
    /// ## Errors
    ///
    /// If the process fails to download the file, this method will return a
    /// [`reqwest::Error`].
    ///
    /// It returns a `serde_json::Error` when the JSON deserialization fails.
    pub fn from_url<S: AsRef<str>>(config: &Config, url: S) -> ScoopResult<Manifest> {
        let url = url.as_ref().to_owned();
        let mut bytes = Vec::new();
        block_on(Self::download_manifest(config, url.clone(), &mut bytes))?;
        let inner: ManifestInner = serde_json::from_slice(&bytes)?;
        // log::debug!("{:?}", inner);
        Ok(Manifest { path: url, inner })
    }

    async fn download_manifest(
        config: &Config,
        url: String,
        data: &mut Vec<u8>,
    ) -> ScoopResult<()> {
        let http_client = HttpClient::new(config)?;
        let resp = http_client.get(url).send().await?;
        assert!(resp.status().is_success());
        data.extend(resp.bytes().await?.as_ref());
        Ok(())
    }

    /// Return the manifest JSON file path of this [`Manifest`].
    #[inline]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Return the `version` of this [`Manifest`].
    #[inline]
    pub fn version(&self) -> &str {
        self.inner.version.as_str()
    }

    /// Return the `description` of this [`Manifest`].
    #[inline]
    pub fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    /// Return the `homepage` of this [`Manifest`].
    #[inline]
    pub fn homepage(&self) -> Option<&str> {
        self.inner.homepage.as_deref()
    }

    /// Return the `license` of this [`Manifest`].
    #[inline]
    pub fn license(&self) -> Option<&License> {
        self.inner.license.as_ref()
    }

    #[inline]
    pub fn get_checkver(&self) -> Option<&Checkver> {
        self.inner.checkver.as_ref()
    }

    #[inline]
    pub fn architecture(&self) -> Option<&Architecture> {
        self.inner.get_architecture()
    }

    #[inline]
    pub fn get_cookie(&self) -> Option<&Map<String, serde_json::Value>> {
        self.inner.cookie.as_ref()
    }

    #[inline]
    pub fn get_extract_dir(&self) -> Option<&VecItem> {
        self.inner.get_extract_dir()
    }

    #[inline]
    pub fn get_extract_to(&self) -> Option<&VecItem> {
        self.inner.get_extract_to()
    }

    #[inline]
    pub fn get_post_install(&self) -> Option<String> {
        match self.inner.get_post_install() {
            None => None,
            Some(vi) => Some(vi.join("\r\n")),
        }
    }

    #[inline]
    pub fn get_installer_script(&self) -> Option<String> {
        match self.inner.get_installer_script() {
            None => None,
            Some(vi) => Some(vi.join("\r\n")),
        }
    }

    #[inline]
    pub fn get_uninstaller_script(&self) -> Option<String> {
        match self.inner.get_uninstaller_script() {
            None => None,
            Some(vi) => Some(vi.join("\r\n")),
        }
    }

    #[inline]
    pub fn get_pre_install(&self) -> Option<String> {
        match self.inner.get_pre_install() {
            None => None,
            Some(vi) => Some(vi.join("\r\n")),
        }
    }

    #[inline]
    pub fn get_shortcuts(&self) -> Option<&Vec<Vec<String>>> {
        self.inner.get_shortcuts()
    }

    #[inline]
    pub fn get_suggest(&self) -> Option<&Map<String, serde_json::Value>> {
        self.inner.get_suggest()
    }

    /// Return a `bin` reference of this [`Manifest`].
    ///
    /// It returns arch-specific `bin` field when it is [`Some`], or it will
    /// return the arch-less one.
    #[inline]
    pub fn bin(&self) -> Option<&Bins> {
        self.inner.bin()
    }

    /// Returns the dependencies of this manifest.
    pub fn get_deps(&self) -> Vec<String> {
        let mut deps = HashSet::new();
        // depends
        if self.inner.depends.is_some() {
            let depends = self.inner.depends.clone().unwrap();
            for dep in depends.iter() {
                drop(deps.insert(dep.to_owned()));
            }
        }
        // innosetup
        if self.is_innosetup() {
            drop(deps.insert("innounp".to_owned()));
        }
        // scripts
        [
            self.get_pre_install(),
            self.get_installer_script(),
            self.get_post_install(),
        ]
        .iter()
        .for_each(|script| {
            if let Some(s) = script {
                if s.contains("Expand-7zipArchive") || s.contains("extract_7zip") {
                    drop(deps.insert("7zip".to_owned()));
                }
                if s.contains("Expand-MsiArchive") || s.contains("extract_msi") {
                    drop(deps.insert("lessmsi".to_owned()));
                }
                if s.contains("Expand-InnoArchive") || s.contains("unpack_inno") {
                    drop(deps.insert("innounp".to_owned()));
                }
                if s.contains("Expand-DarkArchive") {
                    drop(deps.insert("dark".to_owned()));
                }
            }
        });
        // file extensions
        static RE1: Lazy<Regex> = Lazy::new(|| {
            let p = r"\.((gz)|(tar)|(tgz)|(lzma)|(bz)|(bz2)|(7z)|(rar)|(iso)|(xz)|(lzh)|(nupkg))$";
            RegexBuilder::new(p).build().unwrap()
        });
        static RE2: Lazy<Regex> = Lazy::new(|| {
            let p = r"\.(zst)$";
            RegexBuilder::new(p).build().unwrap()
        });
        self.get_url()
            .iter()
            .for_each(|url| {
                if RE1.is_match(url.as_str()) {
                    drop(deps.insert("7zip".to_owned()));
                }
                if RE2.is_match(url.as_str()) {
                    drop(deps.insert("zstd".to_owned()));
                }
            });

        deps.into_iter().collect()
    }

    /// Return the `innosetup` of this [`Manifest`].
    #[inline]
    pub fn is_innosetup(&self) -> bool {
        self.inner.innosetup.unwrap_or(false)
    }

    #[inline]
    pub fn is_nightly_version(&self) -> bool {
        self.version() == "nightly"
    }

    /// Extract download urls from this manifest, in following order:
    ///
    /// 1. return "64bit" urls for amd64 arch if available;
    /// 2. return "32bit" urls for ia32 arch if available;
    /// 3. fallback to return common urls.
    pub fn get_url(&self) -> &Urls {
        match self.architecture() {
            None => {}
            Some(arch) => {
                // arch amd64
                if cfg!(target_arch = "x86_64") {
                    if arch.amd64().is_some() {
                        let urls = arch.amd64().unwrap().get_url();
                        if urls.is_some() {
                            return urls.unwrap();
                        }
                    }
                }

                // arch ia32
                if cfg!(target_arch = "x86") {
                    if arch.ia32().is_some() {
                        let urls = arch.ia32().unwrap().get_url();
                        if urls.is_some() {
                            return urls.unwrap();
                        }
                    }
                }
            }
        }

        // Finally fallback to common urls.
        //
        // SAFETY: this unwrap is safe because a valid manifest must have at
        // least one download url.
        self.inner.get_url().unwrap()
    }

    /// Extract file hashes from this manifest, in following order:
    ///
    /// 1. return "64bit" hashes for amd64 arch if available;
    /// 2. return "32bit" hashes for ia32 arch if available;
    /// 3. fallback to return common hashes.
    pub fn get_hash(&self) -> Option<&Hashes> {
        // `nightly` version does not have hashes.
        if self.version() == "nightly" {
            return None;
        }

        match self.architecture() {
            None => {}
            Some(arch) => {
                // arch amd64
                if cfg!(target_arch = "x86_64") {
                    if arch.amd64().is_some() {
                        return arch.amd64().unwrap().get_hash();
                    }
                }

                // arch ia32
                if cfg!(target_arch = "x86") {
                    if arch.ia32().is_some() {
                        return arch.ia32().unwrap().get_hash();
                    }
                }
            }
        }

        // fallback
        self.inner.get_hash()
    }
}
