use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::path::Path;

use crate::constants::{REGEX_HASH, SPDX_LIST};
use crate::error::{Context, Fallible};

/// A [`Manifest`] basically defines a package that is available to be installed
/// via Scoop. It's a JSON file containing all the specification needed by Scoop
/// to interact with, such as version, artifact urls and hashes, and scripts.
///
/// Following the [schema] of manifest, custom deserialzers have been implemented
/// to deserialize a Scoop manifest JSON file into a `Manifest` instance.
///
/// [schema]: https://github.com/ScoopInstaller/Scoop/blob/master/schema.json
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    /// The path is used to determine the location of the manifest file.
    path: String,
    /// The actual manifest representation.
    inner: ManifestSpec,
    /// The hash of the manifest.
    hash: String,
}

/// [`ManifestSpec`] represents the actual data structure of a Scoop manifest.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ManifestSpec {
    pub version: String,
    pub description: Option<String>,
    pub homepage: String,
    pub license: License,
    pub depends: Option<Vectorized<String>>,
    pub innosetup: Option<bool>,
    pub cookie: Option<HashMap<String, String>>,
    pub architecture: Option<Architecture>,
    pub url: Option<Vectorized<String>>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_vertorized_hash")]
    pub hash: Option<Vectorized<String>>,
    pub extract_dir: Option<Vectorized<String>>,
    pub extract_to: Option<Vectorized<String>>,
    pub pre_install: Option<Vectorized<String>>,
    pub installer: Option<Installer>,
    pub post_install: Option<Vectorized<String>>,
    pub pre_uninstall: Option<Vectorized<String>>,
    pub uninstaller: Option<Uninstaller>,
    pub post_uninstall: Option<Vectorized<String>>,
    pub bin: Option<Vectorized<Vectorized<String>>>,
    pub env_add_path: Option<Vectorized<String>>,
    pub env_set: Option<HashMap<String, String>>,
    pub shortcuts: Option<Vec<Vec<String>>>,
    pub persist: Option<Vectorized<Vectorized<String>>>,
    pub psmodule: Option<Psmodule>,
    pub suggest: Option<HashMap<String, Vectorized<String>>>,
    pub checkver: Option<Checkver>,
    pub autoupdate: Option<Autoupdate>,
    pub notes: Option<Vectorized<String>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct License {
    identifier: String,
    url: Option<String>,
}

/// A [`Vectorized<T>`] represents a derivative [`Vec<T>`] data structure which
/// can be constructed from either an array of T **or a single T**. That means
/// when the input is a single T, it will also be deserialized to a vector of T
/// with the only T element.
///
/// There are some fields of a [`ManifestSpec`] using this type. In general,
/// when the type of value of a field is `stringOrArrayOfStrings` defined in
/// Scoop's manifest schema, it will be deserialized to a Vectorized\<String>.
/// To illustrate, `notes`, `pre_install` and `post_install` are these kind of
/// fields.
///
/// It is also used for the `stringOrArrayOfStringsOrAnArrayOfArrayOfStrings`,
/// a tow times wrapped vector of strings. `bin` and `persist` are these kind
/// of fields.
#[derive(Clone, Debug, Serialize)]
pub struct Vectorized<T>(Vec<T>);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Architecture {
    #[serde(rename = "32bit")]
    pub ia32: Option<ArchitectureSpec>,
    #[serde(rename = "64bit")]
    pub amd64: Option<ArchitectureSpec>,
    #[serde(rename = "arm64")]
    pub aarch64: Option<ArchitectureSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Installer {
    args: Option<Vectorized<String>>,
    file: Option<String>,
    keep: Option<bool>,
    script: Option<Vectorized<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Uninstaller {
    pub args: Option<Vectorized<String>>,
    pub file: Option<String>,
    pub script: Option<Vectorized<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Psmodule {
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Sourceforge {
    pub project: Option<String>,
    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Checkver {
    #[serde(alias = "re")]
    pub regex: Option<String>,
    pub url: Option<String>,
    #[serde(alias = "jp")]
    pub jsonpath: Option<String>,
    pub xpath: Option<String>,
    pub reverse: Option<bool>,
    pub replace: Option<String>,
    pub useragent: Option<String>,
    pub script: Option<Vectorized<String>>,
    pub sourceforge: Option<Sourceforge>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Autoupdate {
    pub architecture: Option<AutoupdateArchitecture>,
    pub extract_dir: Option<Vectorized<String>>,
    pub hash: Option<Vectorized<HashExtraction>>,
    pub notes: Option<Vectorized<String>>,
    pub url: Option<Vectorized<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchitectureSpec {
    pub bin: Option<Vectorized<Vectorized<String>>>,
    pub checkver: Option<Checkver>,
    pub env_add_path: Option<Vectorized<String>>,
    pub env_set: Option<HashMap<String, String>>,
    pub extract_dir: Option<Vectorized<String>>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_vertorized_hash")]
    pub hash: Option<Vectorized<String>>,
    pub installer: Option<Installer>,
    pub post_install: Option<Vectorized<String>>,
    pub post_uninstall: Option<Vectorized<String>>,
    pub pre_install: Option<Vectorized<String>>,
    pub pre_uninstall: Option<Vectorized<String>>,
    pub shortcuts: Option<Vec<Vec<String>>>,
    pub uninstaller: Option<Uninstaller>,
    pub url: Option<Vectorized<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AutoupdateArchitecture {
    #[serde(rename = "32bit")]
    pub ia32: Option<AutoupdateArchSpec>,
    #[serde(rename = "64bit")]
    pub amd64: Option<AutoupdateArchSpec>,
    #[serde(rename = "arm64")]
    pub aarch64: Option<AutoupdateArchSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HashExtraction {
    pub find: Option<String>,
    pub regex: Option<String>,
    #[serde(alias = "jp")]
    pub jsonpath: Option<String>,
    pub xpath: Option<String>,
    pub mode: Option<HashExtractionMode>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AutoupdateArchSpec {
    pub extract_dir: Option<Vectorized<String>>,
    pub hash: Option<Vectorized<HashExtraction>>,
    pub url: Option<Vectorized<String>>,
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
//  Custom Deserializers
////////////////////////////////////////////////////////////////////////////////

impl<'de, T> Deserialize<'de> for Vectorized<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VectorizedVisitor<T>(PhantomData<T>);
        impl<'de, T> Visitor<'de> for VectorizedVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = Vec<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("single item or array of items")
            }

            #[inline]
            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                T::deserialize(serde_json::Value::String(s.to_owned()))
                    .map(|val| vec![val])
                    .map_err(|e| de::Error::custom(e))
            }

            fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
            where
                S: SeqAccess<'de>,
            {
                let mut ret = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(item) = seq.next_element()? {
                    ret.push(item)
                }
                Ok(ret)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut remap = serde_json::Map::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((k, v)) = map.next_entry()? {
                    remap.insert(k, v);
                }
                T::deserialize(serde_json::Value::Object(remap))
                    .map(|val| vec![val])
                    .map_err(|e| de::Error::custom(e))
            }
        }

        Ok(Vectorized(
            deserializer.deserialize_any(VectorizedVisitor(PhantomData))?,
        ))
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
                f.write_str("a license string or a map with identifier field")
            }

            #[inline]
            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // TODO: validate SPDX identifier
                Ok(License::new(s.to_owned(), None))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut identifier: Result<String, A::Error> =
                    Err(de::Error::missing_field("identifier"));
                let mut url = None;

                while let Some((key, value)) = map.next_entry()? {
                    match key {
                        "identifier" => identifier = Ok(value),
                        "url" => url = Some(value),
                        _ => {
                            // skip invalid fields
                            let _ = map.next_value()?;
                            continue;
                        }
                    }
                }

                Ok(License::new(identifier?, url))
            }
        }

        Ok(deserializer.deserialize_any(LicenseVisitor)?)
    }
}

impl<'de> Deserialize<'de> for Sourceforge {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SourceforgeVisitor;
        impl<'de> Visitor<'de> for SourceforgeVisitor {
            type Value = Sourceforge;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a valid sourceforge check string or map with path field")
            }

            #[inline]
            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let (project, path) = match s.split_once("/") {
                    Some((a, b)) => (Some(a.to_owned()), b.to_owned()),
                    None => (None, s.to_owned()),
                };
                Ok(Sourceforge { project, path })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut project = None;
                let mut path: Result<String, A::Error> = Err(de::Error::missing_field("path"));

                while let Some((key, value)) = map.next_entry()? {
                    match key {
                        "project" => project = Some(value),
                        "path" => path = Ok(value),
                        _ => {
                            // skip invalid fields
                            let _ = map.next_value()?;
                            continue;
                        }
                    }
                }

                Ok(Sourceforge {
                    project,
                    path: path?,
                })
            }
        }

        Ok(deserializer.deserialize_any(SourceforgeVisitor)?)
    }
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
                f.write_str("a checkver string or a checkver map")
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
                    sourceforge: None,
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
                let mut sourceforge = None;

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
                        "script" => script = Some(map.next_value()?),
                        "sourceforge" => sourceforge = Some(map.next_value()?),
                        _ => {
                            // skip invalid fields
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
                    sourceforge,
                })
            }
        }

        Ok(deserializer.deserialize_any(CheckverVisitor)?)
    }
}

/// Custom deserializing function used to deserialize and validate the hash field
fn deserialize_vertorized_hash<'de, D>(
    deserializer: D,
) -> Result<Option<Vectorized<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let val = Option::<Vectorized<String>>::deserialize(deserializer)?;
    if val.is_none() {
        return Ok(None);
    }
    let hashes = val.unwrap();
    // validate hashes
    for hash in hashes.0.iter().map(|s| s.as_str()) {
        if !REGEX_HASH.is_match(&hash) {
            return Err(de::Error::invalid_value(
                de::Unexpected::Str(&hash),
                &"a valid hash string",
            ));
        }
    }
    Ok(Some(hashes))
}

////////////////////////////////////////////////////////////////////////////////
//  Implementations for types
////////////////////////////////////////////////////////////////////////////////

impl Manifest {
    /// Create a [`Manifest`] representation of a manfest JSON file with the
    /// given path.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    /// use scoop_core::types::Manifest;
    ///
    /// let path = PathBuf::from(r"C:\Scoop\buckets\main\bucket\unzip.json");
    /// let manifest = Manifest::parse(path);
    /// assert!(manifest.is_err());
    /// ```
    ///
    /// ## Errors
    ///
    /// If the process fails to read the file, this method will return a
    /// [`std::io::error::Error`].
    ///
    /// It returns a `serde_json::Error` when the JSON deserialization fails.
    pub fn parse<P: AsRef<Path>>(path: P) -> Fallible<Manifest> {
        let path = path.as_ref();

        // Read the entire manifest JSON file into memory firstly and then
        // deserialize it as this way is *a lot* faster than reading via
        // `serde_json::from_reader`.
        //
        // Discussion in https://github.com/serde-rs/json/issues/160
        let mut bytes = Vec::new();
        File::open(path)
            .with_context(|| format!("failed to open manifest file: {}", path.display()))?
            .read_to_end(&mut bytes)
            .with_context(|| format!("failed to read manifest file: {}", path.display()))?;

        // Parsing manifest files is the key bottleneck of the entire
        // project. We use `serde_json` because it's well documented and easy
        // to integrate. But I believe there should be an alternative to
        // `serde_json` which can parse JSON files much *faster*. Perhaps
        // `simd_json` can be the one. See https://github.com/serde-rs/json-benchmark
        let inner: ManifestSpec = serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to parse manifest file: {}", path.display()))?;
        let path = path.to_string_lossy().to_string();
        // let mut checksum = scoop_hash::Checksum::new("sha256");
        // checksum.consume(&bytes);
        // let hash = checksum.result();
        let hash = String::from("0");

        Ok(Manifest { path, inner, hash })
    }

    /// Return the JSON file path of this manifest.
    #[inline]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Return the `version` of this manifest.
    #[inline]
    pub fn version(&self) -> &str {
        self.inner.version.as_str()
    }

    /// Return the `description` of this manifest.
    #[inline]
    pub fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    /// Return the `homepage` of this manifest.
    #[inline]
    pub fn homepage(&self) -> &str {
        &self.inner.homepage
    }

    /// Return the `license` of this manifest.
    #[inline]
    pub fn license(&self) -> &License {
        &self.inner.license
    }

    /// Return the `depends` of this manifest.
    #[inline]
    pub fn raw_dependencies(&self) -> Option<Vec<&str>> {
        self.inner.depends.as_ref().map(|v| v.devectorize())
    }

    #[inline]
    pub fn manifest_hash(&self) -> &str {
        &self.hash
    }

    /// Return all executables of this manifest.
    #[inline]
    pub fn executables(&self) -> Option<Vec<&str>> {
        match self.bin() {
            None => None,
            Some(shim_defs) => {
                let mut bins = Vec::new();
                for def in shim_defs {
                    match def.len() {
                        0 => unreachable!(),
                        1 => bins.push(def[0]),
                        _ => bins.push(def[1]),
                    }
                }
                Some(bins)
            }
        }
    }

    pub fn supported_arch(&self) -> Vec<String> {
        let mut ret = vec![];
        let arch = self.architecture();
        if arch.is_some() {
            let arch = arch.unwrap();
            if arch.ia32.is_some() {
                ret.push("ia32".to_string());
            }
            if arch.amd64.is_some() {
                ret.push("amd64".to_string());
            }
        }
        ret
    }

    #[inline]
    pub fn architecture(&self) -> Option<&Architecture> {
        self.inner.architecture.as_ref()
    }

    #[inline]
    pub fn bin(&self) -> Option<Vec<Vec<&str>>> {
        let mut ret = self.inner.bin.as_ref();

        if let Some(arch) = self.architecture() {
            // ia32
            if cfg!(target_arch = "x86") {
                if let Some(spec) = &arch.ia32 {
                    if let Some(bin) = spec.bin.as_ref() {
                        ret = Some(bin);
                    }
                }
            }
            // amd64
            if cfg!(target_arch = "x86_64") {
                if let Some(spec) = &arch.amd64 {
                    if let Some(bin) = spec.bin.as_ref() {
                        ret = Some(bin);
                    }
                }
            }
        }

        ret.map(|v| v.devectorize())
    }

    #[inline]
    pub fn checkver(&self) -> Option<&Checkver> {
        self.inner.checkver.as_ref()
    }

    #[inline]
    pub fn cookie(&self) -> Option<&HashMap<String, String>> {
        self.inner.cookie.as_ref()
    }

    /// Returns the dependencies of this manifest.
    pub fn dependencies(&self) -> Vec<String> {
        let mut deps = HashSet::new();

        if let Some(raw_depends) = self.raw_dependencies() {
            raw_depends.into_iter().for_each(|dep| {
                deps.insert(dep.to_owned());
            });
        }

        if self.innosetup() {
            deps.insert("innounp".to_owned());
        }

        [self.pre_install(), self.post_install()]
            .iter()
            .for_each(|hook| {
                if let Some(script_block) = hook {
                    let s = script_block.join("\r\n");

                    if s.contains("Expand-7zipArchive") {
                        deps.insert("7zip".to_owned());
                    }
                    if s.contains("Expand-MsiArchive") {
                        deps.insert("lessmsi".to_owned());
                    }
                    if s.contains("Expand-InnoArchive") {
                        deps.insert("innounp".to_owned());
                    }
                    if s.contains("Expand-DarkArchive") {
                        deps.insert("dark".to_owned());
                    }
                }
            });

        deps.into_iter().collect()
    }

    #[inline]
    pub fn extract_dir(&self) -> Option<Vec<&str>> {
        let mut ret = self.inner.extract_dir.as_ref();

        if let Some(arch) = self.architecture() {
            // ia32
            if cfg!(target_arch = "x86") {
                if let Some(spec) = &arch.ia32 {
                    if let Some(extract_dir) = spec.extract_dir.as_ref() {
                        ret = Some(extract_dir);
                    }
                }
            }
            // amd64
            if cfg!(target_arch = "x86_64") {
                if let Some(spec) = &arch.amd64 {
                    if let Some(extract_dir) = spec.extract_dir.as_ref() {
                        ret = Some(extract_dir);
                    }
                }
            }
        }
        ret.map(|v| v.devectorize())
    }

    #[inline]
    pub fn extract_to(&self) -> Option<Vec<&str>> {
        self.inner.extract_to.as_ref().map(|v| v.devectorize())
    }

    #[inline]
    pub fn innosetup(&self) -> bool {
        self.inner.innosetup.unwrap_or(false)
    }

    #[inline]
    pub fn pre_install(&self) -> Option<Vec<&str>> {
        let mut ret = self.inner.pre_install.as_ref();

        if let Some(arch) = self.architecture() {
            // ia32
            if cfg!(target_arch = "x86") {
                if let Some(spec) = &arch.ia32 {
                    if let Some(pre_install) = spec.pre_install.as_ref() {
                        ret = Some(pre_install);
                    }
                }
            }
            // amd64
            if cfg!(target_arch = "x86_64") {
                if let Some(spec) = &arch.amd64 {
                    if let Some(pre_install) = spec.pre_install.as_ref() {
                        ret = Some(pre_install);
                    }
                }
            }
        }

        ret.map(|v| v.devectorize())
    }

    #[inline]
    pub fn post_install(&self) -> Option<Vec<&str>> {
        let mut ret = self.inner.post_install.as_ref();

        if let Some(arch) = self.architecture() {
            // ia32
            if cfg!(target_arch = "x86") {
                if let Some(spec) = &arch.ia32 {
                    if let Some(post_install) = spec.post_install.as_ref() {
                        ret = Some(post_install);
                    }
                }
            }
            // amd64
            if cfg!(target_arch = "x86_64") {
                if let Some(spec) = &arch.amd64 {
                    if let Some(post_install) = spec.post_install.as_ref() {
                        ret = Some(post_install);
                    }
                }
            }
        }

        ret.map(|v| v.devectorize())
    }

    #[inline]
    pub fn pre_uninstall(&self) -> Option<Vec<&str>> {
        let mut ret = self.inner.pre_uninstall.as_ref();

        if let Some(arch) = self.architecture() {
            // ia32
            if cfg!(target_arch = "x86") {
                if let Some(spec) = &arch.ia32 {
                    if let Some(pre_uninstall) = spec.pre_uninstall.as_ref() {
                        ret = Some(pre_uninstall);
                    }
                }
            }
            // amd64
            if cfg!(target_arch = "x86_64") {
                if let Some(spec) = &arch.amd64 {
                    if let Some(pre_uninstall) = spec.pre_uninstall.as_ref() {
                        ret = Some(pre_uninstall);
                    }
                }
            }
        }

        ret.map(|v| v.devectorize())
    }

    #[inline]
    pub fn post_uninstall(&self) -> Option<Vec<&str>> {
        let mut ret = self.inner.post_uninstall.as_ref();

        if let Some(arch) = self.architecture() {
            // ia32
            if cfg!(target_arch = "x86") {
                if let Some(spec) = &arch.ia32 {
                    if let Some(post_uninstall) = spec.post_uninstall.as_ref() {
                        ret = Some(post_uninstall);
                    }
                }
            }
            // amd64
            if cfg!(target_arch = "x86_64") {
                if let Some(spec) = &arch.amd64 {
                    if let Some(post_uninstall) = spec.post_uninstall.as_ref() {
                        ret = Some(post_uninstall);
                    }
                }
            }
        }

        ret.map(|v| v.devectorize())
    }

    #[inline]
    pub fn installer(&self) -> Option<&Installer> {
        let mut ret = self.inner.installer.as_ref();

        if let Some(arch) = self.architecture() {
            // ia32
            if cfg!(target_arch = "x86") {
                if let Some(spec) = &arch.ia32 {
                    if let Some(installer) = spec.installer.as_ref() {
                        ret = Some(installer);
                    }
                }
            }
            // amd64
            if cfg!(target_arch = "x86_64") {
                if let Some(spec) = &arch.amd64 {
                    if let Some(installer) = spec.installer.as_ref() {
                        ret = Some(installer);
                    }
                }
            }
        }
        ret
    }

    #[inline]
    pub fn uninstaller(&self) -> Option<&Uninstaller> {
        let mut ret = self.inner.uninstaller.as_ref();

        if let Some(arch) = self.architecture() {
            // ia32
            if cfg!(target_arch = "x86") {
                if let Some(spec) = &arch.ia32 {
                    if let Some(uninstaller) = spec.uninstaller.as_ref() {
                        ret = Some(uninstaller);
                    }
                }
            }
            // amd64
            if cfg!(target_arch = "x86_64") {
                if let Some(spec) = &arch.amd64 {
                    if let Some(uninstaller) = spec.uninstaller.as_ref() {
                        ret = Some(uninstaller);
                    }
                }
            }
        }
        ret
    }

    #[inline]
    pub fn shortcuts(&self) -> Option<&Vec<Vec<String>>> {
        let mut ret = self.inner.shortcuts.as_ref();

        if let Some(arch) = self.architecture() {
            // ia32
            if cfg!(target_arch = "x86") {
                if let Some(spec) = &arch.ia32 {
                    if let Some(shortcuts) = spec.shortcuts.as_ref() {
                        ret = Some(shortcuts);
                    }
                }
            }
            // amd64
            if cfg!(target_arch = "x86_64") {
                if let Some(spec) = &arch.amd64 {
                    if let Some(shortcuts) = spec.shortcuts.as_ref() {
                        ret = Some(shortcuts);
                    }
                }
            }
        }
        ret
    }

    #[inline]
    pub fn suggest(&self) -> Option<&HashMap<String, Vectorized<String>>> {
        self.inner.suggest.as_ref()
    }

    #[inline]
    pub fn is_nightly(&self) -> bool {
        self.version() == "nightly"
    }

    /// Extract download urls from this manifest, in following order:
    ///
    /// 1. return "64bit" urls for amd64 arch if available;
    /// 2. return "32bit" urls for ia32 arch if available;
    /// 3. fallback to return common urls.
    pub fn url(&self) -> Vec<&str> {
        let mut ret = self.inner.url.as_ref();

        if let Some(arch) = self.architecture() {
            // ia32
            if cfg!(target_arch = "x86") {
                if let Some(spec) = &arch.ia32 {
                    if let Some(url) = spec.url.as_ref() {
                        ret = Some(url);
                    }
                }
            }
            // amd64
            if cfg!(target_arch = "x86_64") {
                if let Some(spec) = &arch.amd64 {
                    if let Some(url) = spec.url.as_ref() {
                        ret = Some(url);
                    }
                }
            }
        }

        // The unwrap is safe, according to the manifest schema at least one of
        // noarch url or amd64/ia32 url is required
        ret.map(|v| v.devectorize()).unwrap_or_default()
    }

    /// NOTE: this method will drop all urls without corresponding hash. That
    /// means it will return an empty vector if no hash is found, typically a
    /// package with a `nightly` version.
    pub fn url_with_hash(&self) -> Vec<(&str, &str)> {
        std::iter::zip(self.url(), self.hash()).collect()
    }

    /// Extract file hashes from this manifest, in following order:
    ///
    /// 1. return "64bit" hashes for amd64 arch if available;
    /// 2. return "32bit" hashes for ia32 arch if available;
    /// 3. fallback to return common hashes.
    pub fn hash(&self) -> Vec<&str> {
        let mut ret = self.inner.hash.as_ref();

        if let Some(arch) = self.architecture() {
            // ia32
            if cfg!(target_arch = "x86") {
                if let Some(spec) = &arch.ia32 {
                    if let Some(hash) = spec.hash.as_ref() {
                        ret = Some(hash);
                    }
                }
            }
            // amd64
            if cfg!(target_arch = "x86_64") {
                if let Some(spec) = &arch.amd64 {
                    if let Some(hash) = spec.hash.as_ref() {
                        ret = Some(hash);
                    }
                }
            }
        }
        ret.map(|v| v.devectorize()).unwrap_or_default()
    }
}

impl License {
    fn new(identifier: String, mut url: Option<String>) -> License {
        // SPDX identifier detection
        let id = identifier.as_str();
        let is_spdx = SPDX_LIST.contains(id);
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
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}

impl Installer {
    #[inline]
    pub fn args(&self) -> Option<Vec<&str>> {
        self.args.as_ref().map(|v| v.devectorize())
    }

    #[inline]
    pub fn file(&self) -> Option<&str> {
        self.file.as_ref().map(|s| s.as_str())
    }

    #[inline]
    pub fn script(&self) -> Option<Vec<&str>> {
        self.script.as_ref().map(|v| v.devectorize())
    }

    #[inline]
    pub fn keep(&self) -> bool {
        self.keep.unwrap_or(false)
    }
}

impl Uninstaller {
    #[inline]
    pub fn args(&self) -> Option<Vec<&str>> {
        self.args.as_ref().map(|v| v.devectorize())
    }

    #[inline]
    pub fn file(&self) -> Option<&str> {
        self.file.as_ref().map(|s| s.as_str())
    }

    #[inline]
    pub fn script(&self) -> Option<Vec<&str>> {
        self.script.as_ref().map(|v| v.devectorize())
    }
}

impl Vectorized<String> {
    pub fn devectorize(&self) -> Vec<&str> {
        self.0.iter().map(|s| s.as_str()).collect()
    }
}

impl Vectorized<Vectorized<String>> {
    pub fn devectorize(&self) -> Vec<Vec<&str>> {
        self.0
            .iter()
            .map(|v| v.0.iter().map(|s| s.as_str()).collect())
            .collect()
    }
}

impl From<Vectorized<String>> for Vec<String> {
    fn from(veced: Vectorized<String>) -> Self {
        veced.0
    }
}

impl From<Vectorized<Vectorized<String>>> for Vec<Vec<String>> {
    fn from(veced: Vectorized<Vectorized<String>>) -> Self {
        veced.0.into_iter().map(|v| v.0).collect()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstallInfo {
    architecture: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    bucket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    hold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

impl InstallInfo {
    pub fn parse<P: AsRef<Path>>(path: P) -> Fallible<InstallInfo> {
        let path = path.as_ref();
        let mut bytes = Vec::new();
        File::open(path)
            .with_context(|| format!("failed to open install info file: {}", path.display()))?
            .read_to_end(&mut bytes)
            .with_context(|| format!("failed to read install info file: {}", path.display()))?;
        Ok(serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to parse install info file: {}", path.display()))?)
    }

    #[inline]
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    #[inline]
    pub fn arch(&self) -> &str {
        &self.architecture
    }

    #[inline]
    pub fn held(&self) -> bool {
        self.hold.unwrap_or(false)
    }
}
