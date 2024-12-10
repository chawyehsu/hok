use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::constant::{REGEX_HASH, SPDX_LIST};
use crate::error::Fallible;
use crate::internal;

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
    path: PathBuf,

    /// The actual manifest specification.
    inner: ManifestSpec,

    /// The hash of the manifest.
    hash: String,
}

/// [`ManifestSpec`] represents the actual data structure of a Scoop manifest.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ManifestSpec {
    pub version: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub homepage: String,

    pub license: License,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub innosetup: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookie: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<Architecture>,

    /// Architecture-independent - `noarch` download url(s).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<Vectorized<HashString>>,

    /// The `extract_dir` field is used to define the directory to which the
    /// archive should be extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_dir: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_to: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_install: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub installer: Option<Installer>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_install: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_uninstall: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub uninstaller: Option<Uninstaller>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_uninstall: Option<Vectorized<String>>,

    /// The `bin` field is used to define binaries that need to be shimmed/added
    /// to the `shimes` directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<Vectorized<Vectorized<String>>>,

    /// The `env_add_path` field is used to define path(s) that need to be added
    /// to the `PATH` environment variable during installation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_add_path: Option<Vectorized<String>>,

    /// The `env_set` field is used to define environment variables that should
    /// be set during installation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_set: Option<HashMap<String, String>>,

    /// The `shortcuts` field is used to define shortcuts that need to be created
    /// in the `Scoop Apps` directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortcuts: Option<Vec<Vec<String>>>,

    /// The `persist` field is used to define files/directories that need to be
    /// persisted during uninstallation.
    #[serde(skip_serializing_if = "Option::is_none")]
    persist: Option<Vectorized<Vectorized<String>>>,

    /// The `psmodule` field is used to define PowerShell module that need to
    /// be imported during installation.
    #[serde(skip_serializing_if = "Option::is_none")]
    psmodule: Option<Psmodule>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggest: Option<HashMap<String, Vectorized<String>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkver: Option<Checkver>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub autoupdate: Option<Autoupdate>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vectorized<String>>,
}

/// License information of a Scoop package.
#[derive(Clone, Debug, Serialize)]
pub struct License {
    /// The identifier of the license, which is intended to be a SPDX license.
    identifier: String,

    /// The url to the license text.
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

/// A [`Vectorized<T>`] represents a derivative [`Vec<T>`] data structure which
/// can be constructed from either an array of T **or a single T**. That means
/// when the input is a single T, it will also be deserialized to a vector of T
/// with the only T element.
///
/// Custom (De)srializers are implemented for this type to support the above
/// behavior.
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
#[derive(Clone, Debug)]
pub struct Vectorized<T>(Vec<T>);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Architecture {
    /// Ia32 architecture specification.
    #[serde(rename = "32bit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ia32: Option<ArchitectureSpec>,

    /// Amd64 architecture specification.
    #[serde(rename = "64bit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amd64: Option<ArchitectureSpec>,

    /// Aarch64 architecture specification.
    #[serde(rename = "arm64")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aarch64: Option<ArchitectureSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Installer {
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Vectorized<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    script: Option<Vectorized<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Uninstaller {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vectorized<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<Vectorized<String>>,
}

/// PowerShell module information of a Scoop package.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Psmodule {
    name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Sourceforge {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,

    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Checkver {
    #[serde(alias = "re")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(alias = "jp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonpath: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub xpath: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub useragent: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sourceforge: Option<Sourceforge>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Autoupdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architecture: Option<AutoupdateArchitecture>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_dir: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<Vectorized<HashExtraction>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Vectorized<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchitectureSpec {
    /// Same as `ManifestSpec::bin`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<Vectorized<Vectorized<String>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkver: Option<Checkver>,

    /// Same as `ManifestSpec::env_add_path`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_add_path: Option<Vectorized<String>>,

    /// Same as `ManifestSpec::env_set`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_set: Option<HashMap<String, String>>,

    /// Same as `ManifestSpec::extract_dir`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_dir: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<Vectorized<HashString>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub installer: Option<Installer>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_install: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_uninstall: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_install: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_uninstall: Option<Vectorized<String>>,

    /// Same as `ManifestSpec::shortcuts`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortcuts: Option<Vec<Vec<String>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub uninstaller: Option<Uninstaller>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Vectorized<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AutoupdateArchitecture {
    #[serde(rename = "32bit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ia32: Option<AutoupdateArchSpec>,
    #[serde(rename = "64bit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amd64: Option<AutoupdateArchSpec>,
    #[serde(rename = "arm64")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aarch64: Option<AutoupdateArchSpec>,
}

#[derive(Clone, Debug, Serialize)]
pub enum HashString {
    Md5(String),
    Sha1(String),
    Sha256(String),
    Sha512(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HashExtraction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub find: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,

    #[serde(alias = "jp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonpath: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub xpath: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<HashExtractionMode>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AutoupdateArchSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_dir: Option<Vectorized<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<Vectorized<HashExtraction>>,

    #[serde(skip_serializing_if = "Option::is_none")]
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
//  Custom (De)serializers
////////////////////////////////////////////////////////////////////////////////

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Vectorized<T> {
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
                    .map_err(de::Error::custom)
            }
        }

        Ok(Vectorized(
            deserializer.deserialize_any(VectorizedVisitor(PhantomData))?,
        ))
    }
}

impl<T: Serialize> Serialize for Vectorized<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0.len() {
            0 => serializer.serialize_none(),
            1 => serializer.serialize_some(&self.0[0]),
            _ => serializer.collect_seq(self.0.iter()),
        }
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

                // It is needed to explicitly specify types `<String, String>`
                // of the key and value for the `next_entry` method here,
                // otherwise the deserializer will complain about the invalid
                // type of the key, which is basically similar to:
                // https://github.com/influxdata/pbjson/issues/55
                while let Some((key, value)) = map.next_entry::<String, String>()? {
                    match key.as_str() {
                        "identifier" => identifier = Ok(value),
                        "url" => url = Some(value),
                        _ => {
                            // skip invalid fields
                            map.next_value::<serde_json::Value>()?;
                            continue;
                        }
                    }
                }

                Ok(License::new(identifier?, url))
            }
        }

        deserializer.deserialize_any(LicenseVisitor)
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
                let (project, path) = match s.split_once('/') {
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

                while let Some((key, value)) = map.next_entry::<String, String>()? {
                    match key.as_str() {
                        "project" => project = Some(value),
                        "path" => path = Ok(value),
                        _ => {
                            // skip invalid fields
                            map.next_value::<serde_json::Value>()?;
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

        deserializer.deserialize_any(SourceforgeVisitor)
    }
}

impl<'de> Deserialize<'de> for HashString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HashStringVisitor;
        impl<'de> Visitor<'de> for HashStringVisitor {
            type Value = HashString;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a valid hash string")
            }

            #[inline]
            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                HashString::new(s).map_err(|e| E::custom(e))
            }
        }

        deserializer.deserialize_any(HashStringVisitor)
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

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "github" => {
                            let prefix = map.next_value::<String>()?;
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
                            map.next_value::<serde_json::Value>()?;
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

        deserializer.deserialize_any(CheckverVisitor)
    }
}

////////////////////////////////////////////////////////////////////////////////
//  Implementations for types
////////////////////////////////////////////////////////////////////////////////

/// Macro to generate architecture-specific fields.
macro_rules! arch_specific_field {
    ($self:ident, $field:ident) => {{
        let mut ret = $self.inner.$field.as_ref();

        if let Some(arch) = $self.inner.architecture.as_ref() {
            if cfg!(target_arch = "x86") {
                if let Some(ia32) = &arch.ia32 {
                    let $field = ia32.$field.as_ref();
                    if $field.is_some() {
                        ret = $field;
                    }
                }
            }

            if cfg!(target_arch = "x86_64") {
                if let Some(amd64) = &arch.amd64 {
                    let $field = amd64.$field.as_ref();
                    if $field.is_some() {
                        ret = $field;
                    }
                }
            }

            if cfg!(target_arch = "aarch64") {
                if let Some(aarch64) = &arch.aarch64 {
                    let $field = aarch64.$field.as_ref();
                    if $field.is_some() {
                        ret = $field;
                    }
                }
            }
        }
        ret
    }};
}

impl Manifest {
    /// Create a [`Manifest`] representation of a manfest JSON file with the
    /// given path.
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
        File::open(path)?.read_to_end(&mut bytes)?;

        // Parsing manifest files is the key bottleneck of the entire
        // project. We use `serde_json` because it's well documented and easy
        // to integrate. But I believe there should be an alternative to
        // `serde_json` which can parse JSON files much *faster*. Perhaps
        // `simd_json` can be the one. See https://github.com/serde-rs/json-benchmark
        let inner: ManifestSpec = serde_json::from_slice(&bytes).inspect_err(|e| {
            debug!("failed to parse manifest {}", path.display());
        })?;
        let path = internal::path::normalize_path(path);
        // let mut checksum = scoop_hash::Checksum::new("sha256");
        // checksum.consume(&bytes);
        // let hash = checksum.result();
        let hash = String::from("0");

        Ok(Manifest { path, inner, hash })
    }

    /// Return the file path of this manifest.
    #[inline]
    pub fn path(&self) -> &Path {
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

    // #[inline]
    // pub fn manifest_hash(&self) -> &str {
    //     &self.hash
    // }

    /// Return the `depends` of this manifest.
    ///
    /// This method returns the explicit dependencies defined in the manifest,
    /// while [`dependencies`] returns all dependencies including the implicit
    /// ones.
    ///
    /// # Note
    ///
    /// The format of a value in the `depends` field can be either `name` or
    /// `bucket/name`, for example: `7zip` or `main/7zip`.
    ///
    /// [`dependencies`]: #method.dependencies
    #[inline]
    pub fn depends(&self) -> Option<Vec<&str>> {
        self.inner.depends.as_ref().map(|v| v.devectorize())
    }

    #[inline]
    pub fn architecture(&self) -> Option<&Architecture> {
        self.inner.architecture.as_ref()
    }

    /// Get `bin` field of this manifest.
    pub fn bin(&self) -> Option<Vec<Vec<&str>>> {
        let ret = arch_specific_field!(self, bin);
        ret.map(|v| v.devectorize())
    }

    #[inline]
    pub fn checkver(&self) -> Option<&Checkver> {
        self.inner.checkver.as_ref()
    }

    /// Returns `cookie` defined in this manifest.
    #[inline]
    pub fn cookie(&self) -> Option<&HashMap<String, String>> {
        self.inner.cookie.as_ref()
    }

    /// Returns `env_add_path` defined in this manifest.
    pub fn env_add_path(&self) -> Option<Vec<&str>> {
        let ret = arch_specific_field!(self, env_add_path);
        ret.map(|v| v.devectorize())
    }

    /// Returns `env_set` defined in this manifest.
    pub fn env_set(&self) -> Option<&HashMap<String, String>> {
        arch_specific_field!(self, env_set)
    }

    /// Returns `extract_dir` defined in this manifest.
    pub fn extract_dir(&self) -> Option<Vec<&str>> {
        let ret = arch_specific_field!(self, extract_dir);
        ret.map(|v| v.devectorize())
    }

    /// Returns `extract_to` defined in this manifest.
    #[inline]
    pub fn extract_to(&self) -> Option<Vec<&str>> {
        self.inner.extract_to.as_ref().map(|v| v.devectorize())
    }

    #[inline]
    pub fn innosetup(&self) -> bool {
        self.inner.innosetup.unwrap_or(false)
    }

    #[inline]
    pub fn suggest(&self) -> Option<&HashMap<String, Vectorized<String>>> {
        self.inner.suggest.as_ref()
    }

    pub fn pre_install(&self) -> Option<Vec<&str>> {
        let ret = arch_specific_field!(self, pre_install);
        ret.map(|v| v.devectorize())
    }

    pub fn post_install(&self) -> Option<Vec<&str>> {
        let ret = arch_specific_field!(self, post_install);
        ret.map(|v| v.devectorize())
    }

    pub fn pre_uninstall(&self) -> Option<Vec<&str>> {
        let ret = arch_specific_field!(self, pre_uninstall);
        ret.map(|v| v.devectorize())
    }

    pub fn post_uninstall(&self) -> Option<Vec<&str>> {
        let ret = arch_specific_field!(self, post_uninstall);
        ret.map(|v| v.devectorize())
    }

    pub fn installer(&self) -> Option<&Installer> {
        arch_specific_field!(self, installer)
    }

    pub fn uninstaller(&self) -> Option<&Uninstaller> {
        arch_specific_field!(self, uninstaller)
    }

    /// Returns `persist` defined in this manifest.
    #[inline]
    pub fn persist(&self) -> Option<Vec<Vec<&str>>> {
        self.inner.persist.as_ref().map(|v| v.devectorize())
    }

    /// Returns `psmodule` defined in this manifest.
    #[inline]
    pub fn psmodule(&self) -> Option<&Psmodule> {
        self.inner.psmodule.as_ref()
    }

    pub fn shortcuts(&self) -> Option<Vec<Vec<&str>>> {
        let ret = arch_specific_field!(self, shortcuts);
        ret.map(|v| {
            v.iter()
                .map(|v| v.iter().map(|s| s.as_str()).collect())
                .collect()
        })
    }

    /// Extract download urls from this manifest:
    ///
    /// - For `amd64` return "64bit" urls if available else noarch urls;
    /// - For `ia32` return "32bit" urls if available else noarch urls;
    /// - For `aarch64` return "arm64" urls if available else noarch urls.
    pub fn url(&self) -> Vec<&str> {
        let ret = arch_specific_field!(self, url);
        // The unwrap is safe, according to the manifest schema, for a valid
        // manifest, at least one of the noarch url field or arch-specific url
        // field is required to be provided.
        ret.map(|v| v.devectorize()).unwrap_or_default()
    }

    /// Extract file hashes from this manifest, in following order:
    ///
    /// - For `amd64` return "64bit" hashes if available else noarch hashes;
    /// - For `ia32` return "32bit" hashes if available else noarch hashes;
    /// - For `aarch64` return "arm64" hashes if available else noarch hashes.
    pub fn hash(&self) -> Vec<&HashString> {
        let ret = arch_specific_field!(self, hash);
        ret.map(|v| v.devectorize()).unwrap_or_default()
    }

    /// Returns the dependencies of this manifest.
    ///
    /// This method returns all dependencies including the implicit ones, while
    /// [`depends`] returns the explicit dependencies defined in the `depends`
    /// field of the manifest.
    ///
    /// # Note
    ///
    /// The format of the value of a dependency can be either `name` or
    /// `bucket/name`, for example: `7zip` or `main/7zip`.
    ///
    /// [`depends`]: #method.depends
    pub(crate) fn dependencies(&self) -> Vec<String> {
        let mut deps = HashSet::new();

        if let Some(raw_depends) = self.depends() {
            deps.extend(raw_depends.into_iter().map(|s| s.to_owned()));
        }

        if self.innosetup() {
            deps.insert("main/innounp".to_owned());
        }

        let hook_scripts = [
            self.pre_install(),
            self.post_install(),
            self.installer().map(|i| i.script()).unwrap_or_default(),
            self.uninstaller().map(|u| u.script()).unwrap_or_default(),
            self.pre_uninstall(),
            self.post_uninstall(),
        ];

        hook_scripts.into_iter().for_each(|s| {
            if let Some(script_block) = s {
                let s = script_block.join("\r\n");

                if s.contains("Expand-7zipArchive") {
                    deps.remove("main/7zip");
                    deps.insert("7zip".to_owned());
                }
                if s.contains("Expand-MsiArchive") {
                    deps.remove("lessmsi");
                    deps.insert("main/lessmsi".to_owned());
                }
                if s.contains("Expand-InnoArchive") {
                    deps.remove("innounp");
                    deps.insert("main/innounp".to_owned());
                }
                if s.contains("Expand-DarkArchive") {
                    deps.remove("dark");
                    deps.insert("main/dark".to_owned());
                }
            }
        });

        deps.into_iter().collect()
    }

    /// Get shims defined in this manifest.
    ///
    /// # Note
    ///
    /// While [`bin()`][1] method returns the raw `bin` field of the manifest,
    /// this method returns the shim names defined in the `bin` field.
    ///
    /// [1]: #method.bin
    pub(crate) fn shims(&self) -> Option<Vec<&str>> {
        if let Some(shim_defs) = self.bin() {
            let mut shims = Vec::with_capacity(shim_defs.len());
            for def in shim_defs {
                match def.len() {
                    0 => {
                        debug!("invalid shim definition: {:?}", def);
                        continue;
                    }
                    1 => shims.push(def[0]),
                    _ => shims.push(def[1]),
                }
            }
            Some(shims)
        } else {
            None
        }
    }
}

impl License {
    /// Create a [`License`] representation.
    pub fn new(identifier: String, url: Option<String>) -> License {
        Self { identifier, url }
    }

    /// Return the identifier of this license.
    #[inline]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// Check if this license is a valid SPDX identifier.
    #[inline]
    pub fn is_spdx(&self) -> bool {
        SPDX_LIST.contains(self.identifier())
    }

    /// Return the url to the license text of this license.
    #[inline]
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}

impl fmt::Display for License {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let url = self.url();

        if let Some(url) = url {
            write!(f, "{} ({})", self.identifier, url)
        } else if self.is_spdx() {
            write!(
                f,
                "{} (https://spdx.org/licenses/{}.html)",
                self.identifier, self.identifier
            )
        } else {
            write!(f, "{}", self.identifier)
        }
    }
}

impl HashString {
    /// Create a [`HashString`] representation.
    pub fn new(raw: &str) -> Fallible<HashString> {
        if !REGEX_HASH.is_match(raw) {
            let msg = format!("invalid hash string: {}", raw);
            return Err(crate::Error::Custom(msg));
        }

        let (algo, hash) = raw.split_once(':').unwrap_or(("sha256", raw));
        let hash = hash.to_lowercase();
        match algo {
            "md5" => Ok(HashString::Md5(hash)),
            "sha1" => Ok(HashString::Sha1(hash)),
            "sha256" => Ok(HashString::Sha256(hash)),
            "sha512" => Ok(HashString::Sha512(hash)),
            _ => Err(crate::Error::Custom(format!(
                "unsupported hash algorithm: {}",
                algo
            ))),
        }
    }

    /// Return the hash algorithm.
    ///
    /// # Returns
    ///
    /// - `md5`
    /// - `sha1`
    /// - `sha256`
    /// - `sha512`
    pub fn algorithm(&self) -> &str {
        match self {
            HashString::Md5(_) => "md5",
            HashString::Sha1(_) => "sha1",
            HashString::Sha256(_) => "sha256",
            HashString::Sha512(_) => "sha512",
        }
    }

    /// Return the hash value.
    pub fn value(&self) -> &str {
        match self {
            HashString::Md5(s) => s,
            HashString::Sha1(s) => s,
            HashString::Sha256(s) => s,
            HashString::Sha512(s) => s,
        }
    }
}

impl fmt::Display for HashString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            HashString::Md5(s) => format!("md5:{}", s),
            HashString::Sha1(s) => format!("sha1:{}", s),
            HashString::Sha256(s) => format!("sha256:{}", s),
            HashString::Sha512(s) => format!("sha512:{}", s),
        };

        write!(f, "{}", s)
    }
}

impl Installer {
    #[inline]
    pub fn args(&self) -> Option<Vec<&str>> {
        self.args.as_ref().map(|v| v.devectorize())
    }

    #[inline]
    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
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

impl Psmodule {
    /// Return the `name` of the PowerShell module.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Uninstaller {
    #[inline]
    pub fn args(&self) -> Option<Vec<&str>> {
        self.args.as_ref().map(|v| v.devectorize())
    }

    #[inline]
    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }

    #[inline]
    pub fn script(&self) -> Option<Vec<&str>> {
        self.script.as_ref().map(|v| v.devectorize())
    }
}

impl Vectorized<HashString> {
    pub fn devectorize(&self) -> Vec<&HashString> {
        self.0.iter().collect()
    }
}

impl Vectorized<String> {
    pub fn devectorize(&self) -> Vec<&str> {
        self.0.iter().map(|s| s.as_str()).collect()
    }
}

impl Vectorized<Vectorized<String>> {
    pub fn devectorize(&self) -> Vec<Vec<&str>> {
        self.0.iter().map(|v| v.devectorize()).collect()
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
    #[serde(skip_serializing_if = "Option::is_none")]
    bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

impl InstallInfo {
    pub fn parse<P: AsRef<Path>>(path: P) -> Fallible<InstallInfo> {
        let path = path.as_ref();
        let mut bytes = Vec::new();
        File::open(path)?.read_to_end(&mut bytes)?;

        let info = serde_json::from_slice(&bytes).inspect_err(|e| {
            debug!("failed to parse install_info {}", path.display());
        })?;

        Ok(info)
    }

    #[inline]
    pub fn bucket(&self) -> Option<&str> {
        self.bucket.as_deref()
    }

    #[inline]
    pub fn arch(&self) -> &str {
        &self.architecture
    }

    #[inline]
    pub fn is_held(&self) -> bool {
        self.hold.unwrap_or(false)
    }

    #[inline]
    pub fn set_held(&mut self, flag: bool) {
        self.hold = Some(flag);
    }

    #[inline]
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}
