//! Archive detection, registry, path guard and extraction orchestration.
//!
//! The registry is the only place that answers "what can Hok extract?": one
//! ordered entry per [`Format`] with its name matcher, its bounded-prefix
//! detector, and its ordered extractor chain. Extractors always write into a
//! staging directory supplied by [`extract`]; the core moves that directory
//! into place only after the whole chain succeeds.
//!
//! Sibling modules provide the per-format extractors and their detector
//! validation. Each one exports `extract` with the signature below; the
//! registry in this file wires those functions into the extractor chains.
//!
//! ```text
//! fn extract(src: &Path, dest: &Path, probe: &Probe, observer: &dyn ExtractObserver)
//!     -> Fallible<ExtractSummary>;
//! ```

pub mod cab;
pub mod helper;
pub mod inno;
pub mod iso;
pub mod msi;
pub mod nsis;
pub mod probe;
pub mod rar;
pub mod sevenzip;
pub mod tar;
pub mod zip;

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Error, Fallible};

/// Offset of the ISO9660 primary volume descriptor.
pub(crate) const ISO_PVD_OFFSET: u64 = 0x8000;

#[doc(inline)]
pub use probe::probe;

pub(crate) use probe::{Detected, Prefix, declared_format, detect_path, read_window_at};

/// Result of probing an archive file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Probe {
    /// Format implied by the file name, if the name is a known archive name.
    pub declared: Option<Format>,
    /// Format actually detected from the file's bytes.
    pub detected: Format,
    /// Byte offset where the archive begins (non-zero for self-extracting stubs).
    pub offset: u64,
    /// How the detection was established.
    pub kind: ProbeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProbeKind {
    /// Detected from the bounded prefix at offset 0.
    Prefix,
    /// Detected from the payload appended to a PE image.
    PeStub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Format {
    Zip,
    SevenZip,
    Tar,
    Gzip,
    Bzip2,
    Xz,
    Lzma,
    Zstd,
    Rar,
    Msi,
    Inno,
    Nsis,
    Iso,
    Cab,
    Lzh,
    /// A PE image with no recognised payload.
    Exe,
    /// A wrapper archive containing one member that is itself an archive
    /// (`001`, or any nsplit continuation). Never returned by the prober for
    /// offset 0; only reported when the registry recognises the name.
    Split,
}

#[non_exhaustive]
pub struct ExtractOptions<'a> {
    /// Directory inside the archive that becomes the destination root.
    pub extract_dir: Option<&'a str>,
    /// Directory inside the destination where the archive is written.
    pub extract_to: Option<&'a str>,
    /// Progress observer. Entries are reported before they are written.
    pub observer: &'a dyn ExtractObserver,
}

pub trait ExtractObserver: Send + Sync {
    /// Called once per written archive entry, in archive order.
    /// Directories are never reported.
    fn entry_written(&self, entry: &str);
}

/// An [`ExtractObserver`] that ignores every entry.
pub struct NoopObserver;

impl ExtractObserver for NoopObserver {
    fn entry_written(&self, _entry: &str) {}
}

#[non_exhaustive]
pub struct ExtractReport {
    pub probe: Probe,
    /// Human-readable extractor name, e.g. "zip", "7z", "rar (helper: 7z.exe)".
    pub extractor: &'static str,
    pub entries: u64,
    pub bytes: u64,
    /// Non-fatal problems worth surfacing (skipped entries, etc.).
    pub warnings: Vec<String>,
}

/// What one extractor reports back to [`extract`].
#[non_exhaustive]
pub struct ExtractSummary {
    /// Human-readable extractor name, e.g. "zip", "7z", "rar (helper: 7z.exe)".
    pub extractor: &'static str,
    pub entries: u64,
    pub bytes: u64,
    pub warnings: Vec<String>,
}

/// One extractor in a format's chain.
///
/// The extraction workstream points these at the real module functions; the
/// initial wiring keeps them at [`unsupported_extractor`] so the registry
/// compiles before the extractor modules land.
pub(crate) struct Ops {
    pub(crate) name: &'static str,
    pub(crate) extract: ExtractFn,
}

pub(crate) type ExtractFn =
    fn(&Path, &Path, &Probe, &dyn ExtractObserver) -> Fallible<ExtractSummary>;

fn unsupported_extractor(
    src: &Path,
    _dest: &Path,
    _probe: &Probe,
    _observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    Err(Error::Extract {
        archive: src.to_path_buf(),
        reason: "extractor not wired yet".to_string(),
    })
}

/// Detector for a bounded window: `(window bytes, absolute offset)`.
pub(crate) type DetectFn = fn(&[u8], u64) -> Option<Format>;

/// Fixed-size registry entry.
pub(crate) struct FormatSpec {
    pub(crate) format: Format,
    /// Declaration check. The entry is only reachable through [`lookup_name`],
    /// so this is effectively always present.
    pub(crate) matches_name: fn(&str) -> bool,
    /// Detector for a bounded window. `None` means the format can only be
    /// declared, never detected.
    pub(crate) detect: Option<DetectFn>,
    /// Ordered extractor chain. An empty chain means the format is recognised
    /// by name but cannot be extracted yet.
    pub(crate) extractors: &'static [Ops],
}

/// The extractor registry: one ordered entry per [`Format`].
///
/// To plug in a real extractor, replace an `Ops` entry's `extract` pointer with
/// the module function (e.g. `zip::extract`). To add a format, add one line.
/// The fallbacks are chained left to right, see [`FormatSpec::extractors`].
static FORMATS: &[FormatSpec] = &[
    FormatSpec {
        format: Format::Zip,
        matches_name: matches_zip_name,
        detect: Some(detect_zip),
        extractors: &[Ops {
            name: "zip",
            extract: zip::extract_with_fallback,
        }],
    },
    FormatSpec {
        format: Format::SevenZip,
        matches_name: matches_sevenzip_name,
        detect: Some(detect_sevenzip),
        extractors: &[Ops {
            name: "7z",
            extract: sevenzip::extract_with_fallback,
        }],
    },
    FormatSpec {
        format: Format::Tar,
        matches_name: matches_tar_name,
        detect: Some(detect_tar),
        extractors: &[Ops {
            name: "tar",
            extract: tar::extract_with_fallback,
        }],
    },
    FormatSpec {
        format: Format::Gzip,
        matches_name: matches_gzip_name,
        detect: Some(detect_gzip),
        extractors: &[Ops {
            name: "tar",
            extract: tar::extract_with_fallback,
        }],
    },
    FormatSpec {
        format: Format::Bzip2,
        matches_name: matches_bzip2_name,
        detect: Some(detect_bzip2),
        extractors: &[Ops {
            name: "tar",
            extract: tar::extract_with_fallback,
        }],
    },
    FormatSpec {
        format: Format::Xz,
        matches_name: matches_xz_name,
        detect: Some(detect_xz),
        extractors: &[Ops {
            name: "tar",
            extract: tar::extract_with_fallback,
        }],
    },
    FormatSpec {
        format: Format::Lzma,
        matches_name: matches_lzma_name,
        detect: Some(detect_lzma),
        extractors: &[Ops {
            name: "lzma",
            extract: sevenzip::extract_with_fallback,
        }],
    },
    FormatSpec {
        format: Format::Zstd,
        matches_name: matches_zstd_name,
        detect: Some(detect_zstd),
        extractors: &[Ops {
            name: "tar",
            extract: tar::extract_with_fallback,
        }],
    },
    FormatSpec {
        format: Format::Rar,
        matches_name: matches_rar_name,
        detect: Some(detect_rar),
        extractors: &[Ops {
            name: "rar",
            extract: rar::extract_with_fallback,
        }],
    },
    FormatSpec {
        format: Format::Msi,
        matches_name: matches_msi_name,
        detect: Some(detect_msi),
        extractors: &[Ops {
            name: "msi",
            extract: msi::extract_with_fallback,
        }],
    },
    FormatSpec {
        format: Format::Inno,
        matches_name: matches_inno_name,
        detect: Some(no_detector),
        extractors: &[Ops {
            name: "inno",
            extract: inno::extract_with_fallback,
        }],
    },
    FormatSpec {
        format: Format::Nsis,
        matches_name: matches_nsis_name,
        detect: Some(no_detector),
        extractors: &[Ops {
            name: "nsis",
            extract: nsis::extract,
        }],
    },
    FormatSpec {
        format: Format::Iso,
        matches_name: matches_iso_name,
        detect: Some(detect_iso),
        extractors: &[Ops {
            name: "iso",
            extract: iso::extract_with_fallback,
        }],
    },
    FormatSpec {
        format: Format::Cab,
        matches_name: matches_cab_name,
        detect: Some(detect_cab),
        extractors: &[Ops {
            name: "cab",
            extract: cab::extract_cab,
        }],
    },
    FormatSpec {
        format: Format::Lzh,
        matches_name: matches_lzh_name,
        detect: Some(detect_lzh),
        extractors: &[Ops {
            name: "lzh",
            extract: cab::extract_cab,
        }],
    },
    FormatSpec {
        format: Format::Exe,
        matches_name: matches_exe_name,
        detect: None,
        extractors: &[Ops {
            name: "exe",
            extract: unsupported_extractor,
        }],
    },
    FormatSpec {
        format: Format::Split,
        matches_name: matches_split_name,
        detect: None,
        extractors: &[Ops {
            name: "split",
            extract: unsupported_extractor,
        }],
    },
];
// Name matchers

fn matches_zip_name(name: &str) -> bool {
    ends_with_any(
        name,
        &[".zip", ".nupkg", ".jar", ".war", ".ear", ".whl", ".xpi"],
    )
}

fn matches_sevenzip_name(name: &str) -> bool {
    ends_with_any(name, &[".7z"])
}

fn matches_tar_name(name: &str) -> bool {
    ends_with_any(
        name,
        &[
            ".tar", ".tar.gz", ".tgz", ".tar.bz2", ".tbz2", ".tbz", ".tar.xz", ".txz", ".tar.zst",
            ".tzst",
        ],
    )
}

fn matches_gzip_name(name: &str) -> bool {
    ends_with_any(name, &[".gz"])
}

fn matches_bzip2_name(name: &str) -> bool {
    ends_with_any(name, &[".bz2"])
}

fn matches_xz_name(name: &str) -> bool {
    ends_with_any(name, &[".xz"])
}

fn matches_lzma_name(name: &str) -> bool {
    ends_with_any(name, &[".lzma"])
}

fn matches_zstd_name(name: &str) -> bool {
    ends_with_any(name, &[".zst"])
}

fn matches_rar_name(name: &str) -> bool {
    ends_with_any(name, &[".rar", ".r00"])
}

fn matches_msi_name(name: &str) -> bool {
    ends_with_any(name, &[".msi", ".msp"])
}

fn matches_nsis_name(_name: &str) -> bool {
    false
}

fn matches_inno_name(_name: &str) -> bool {
    false
}

fn matches_iso_name(name: &str) -> bool {
    ends_with_any(name, &[".iso", ".img"])
}

fn matches_cab_name(name: &str) -> bool {
    ends_with_any(name, &[".cab"])
}

fn matches_lzh_name(name: &str) -> bool {
    ends_with_any(name, &[".lzh", ".lha"])
}

fn matches_exe_name(_name: &str) -> bool {
    false
}

fn matches_split_name(name: &str) -> bool {
    name.ends_with(".001") || (name.contains(".part") && name.ends_with(".rar"))
}

fn ends_with_any(name: &str, suffixes: &[&str]) -> bool {
    suffixes.iter().any(|suffix| name.ends_with(suffix))
}

// Detectors. Each returns its own format when the window starts with a valid
// header for that format.

fn detect_zip(prefix: &[u8], _offset: u64) -> Option<Format> {
    let recognized = prefix.starts_with(b"PK\x03\x04")
        || prefix.starts_with(b"PK\x05\x06")
        || prefix.starts_with(b"PK\x07\x08");
    recognized.then_some(Format::Zip)
}

fn detect_sevenzip(prefix: &[u8], _offset: u64) -> Option<Format> {
    prefix
        .starts_with(b"7z\xBC\xAF\x27\x1C")
        .then_some(Format::SevenZip)
}

fn detect_rar(prefix: &[u8], _offset: u64) -> Option<Format> {
    prefix.starts_with(b"Rar!\x1A\x07").then_some(Format::Rar)
}

fn detect_gzip(prefix: &[u8], _offset: u64) -> Option<Format> {
    prefix.starts_with(b"\x1F\x8B").then_some(Format::Gzip)
}

fn detect_bzip2(prefix: &[u8], _offset: u64) -> Option<Format> {
    prefix.starts_with(b"BZh").then_some(Format::Bzip2)
}

fn detect_xz(prefix: &[u8], _offset: u64) -> Option<Format> {
    prefix.starts_with(b"\xFD7zXZ\x00").then_some(Format::Xz)
}

fn detect_zstd(prefix: &[u8], _offset: u64) -> Option<Format> {
    prefix
        .starts_with(b"\x28\xB5\x2F\xFD")
        .then_some(Format::Zstd)
}

fn detect_lzma(prefix: &[u8], _offset: u64) -> Option<Format> {
    prefix.starts_with(b"\x5D\x00\x00").then_some(Format::Lzma)
}

fn detect_msi(prefix: &[u8], _offset: u64) -> Option<Format> {
    prefix
        .starts_with(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1")
        .then_some(Format::Msi)
}

fn detect_cab(prefix: &[u8], _offset: u64) -> Option<Format> {
    prefix.starts_with(b"MSCF").then_some(Format::Cab)
}

fn detect_lzh(prefix: &[u8], _offset: u64) -> Option<Format> {
    (prefix.len() >= 5 && &prefix[2..5] == b"-lh").then_some(Format::Lzh)
}

fn detect_iso(prefix: &[u8], offset: u64) -> Option<Format> {
    if offset != ISO_PVD_OFFSET || prefix.len() < 6 {
        return None;
    }
    let is_descriptor = matches!(prefix[0], 0 | 1 | 2 | 255);
    (is_descriptor && &prefix[1..6] == b"CD001").then_some(Format::Iso)
}

fn detect_tar(prefix: &[u8], _offset: u64) -> Option<Format> {
    tar_header(prefix).then_some(Format::Tar)
}

/// Map a compressed container format that wraps a tar to [`Format::Tar`].
///
/// The prober cannot peek inside a compressed stream within its budget, so the
/// declared name decides: `.tar.gz`, `.tgz`, `.tar.xz`, and so on are tar
/// archives wrapped in a compressor.
pub(crate) fn compressed_tar(format: Format, name: &str) -> Option<Format> {
    let name = name.to_ascii_lowercase();
    let wraps_tar = name.ends_with(".tar.gz")
        || name.ends_with(".tgz")
        || name.ends_with(".tar.bz2")
        || name.ends_with(".tbz2")
        || name.ends_with(".tbz")
        || name.ends_with(".tar.xz")
        || name.ends_with(".txz")
        || name.ends_with(".tar.zst")
        || name.ends_with(".tzst");
    if !wraps_tar {
        return None;
    }
    matches!(
        format,
        Format::Gzip | Format::Bzip2 | Format::Xz | Format::Zstd
    )
    .then_some(Format::Tar)
}

fn tar_header(prefix: &[u8]) -> bool {
    if prefix.len() < 512 {
        return false;
    }
    let valid_checksum = checksum_matches(&prefix[..512]);
    if &prefix[257..262] == b"ustar" {
        valid_checksum && prefix[156] != 0
    } else {
        valid_checksum
    }
}

fn checksum_matches(header: &[u8]) -> bool {
    let Some(field) = header.get(148..156) else {
        return false;
    };
    let digits = field
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .copied()
        .collect::<Vec<u8>>();
    if digits.is_empty() {
        return false;
    }
    let Ok(text) = std::str::from_utf8(&digits) else {
        return false;
    };
    let Ok(stored) = u64::from_str_radix(text, 8) else {
        return false;
    };
    let sum = |signed: bool| -> i64 {
        header[..512]
            .iter()
            .enumerate()
            .map(|(index, byte)| {
                if (148..156).contains(&index) {
                    32
                } else if signed {
                    i64::from(*byte as i8)
                } else {
                    i64::from(*byte)
                }
            })
            .sum()
    };
    i64::try_from(stored) == Ok(sum(true)) || i64::try_from(stored) == Ok(sum(false))
}

fn no_detector(_prefix: &[u8], _offset: u64) -> Option<Format> {
    None
}

/// Whether a download name looks like an archive (used for download pairing).
pub fn is_archive_name(name: &str) -> bool {
    lookup_name(name).is_some()
}

pub(crate) fn lookup_name(name: &str) -> Option<&'static FormatSpec> {
    FORMATS
        .iter()
        .find(|spec| spec.format != Format::Exe && (spec.matches_name)(name))
}

pub(crate) fn spec_for(format: Format) -> Option<&'static FormatSpec> {
    FORMATS.iter().find(|spec| spec.format == format)
}

/// A rejection produced by the [`guard`], carried through the chain so another
/// extractor can never retry an archive that contains an escaping entry.
#[derive(Debug)]
pub(crate) struct UnsafeArchive {
    pub(crate) entry: String,
}

impl Error {
    /// Whether this failure is a path-guard rejection.
    pub(crate) fn is_unsafe_archive(&self) -> bool {
        matches!(
            self,
            Error::Extract { reason, .. } if reason.contains("escapes the destination")
        )
    }
}

impl std::fmt::Display for UnsafeArchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "entry '{}' escapes the destination", self.entry)
    }
}

/// The single check that keeps an archive entry inside its destination.
///
/// Rejects absolute paths, prefixes (drive letters and UNC), and any `..`
/// component. Returns the path joined onto `dest`.
pub(crate) fn guard(dest: &Path, entry: &Path) -> Fallible<PathBuf> {
    let text = entry.to_string_lossy();
    let separators_normalized: String = text.replace('\\', "/");
    let normalized: &str = separators_normalized.as_ref();
    let entry = Path::new(normalized);
    if entry.is_absolute() {
        return Err(Error::Extract {
            archive: entry.to_path_buf(),
            reason: format!(
                "{}",
                UnsafeArchive {
                    entry: entry.display().to_string()
                }
            ),
        });
    }
    let mut relative = PathBuf::new();
    for component in entry.components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(Error::Extract {
                    archive: entry.to_path_buf(),
                    reason: UnsafeArchive {
                        entry: entry.display().to_string(),
                    }
                    .to_string(),
                });
            }
        }
    }
    Ok(dest.join(relative))
}

/// Extract `src` into `dest`, honouring `options`.
///
/// The chosen extractor writes into a fresh temporary directory; only a fully
/// successful extraction is moved into `dest`. `extract_dir` selects a subtree
/// of the archive root and `extract_to` selects a subdirectory of `dest`.
pub fn extract(src: &Path, dest: &Path, options: &ExtractOptions<'_>) -> Fallible<ExtractReport> {
    let probe = probe::probe(src)?;
    if probe.detected == Format::Exe {
        return Err(Error::Extract {
            archive: src.to_path_buf(),
            reason: "the file is an executable, not an archive".to_string(),
        });
    }
    let spec = spec_for(probe.detected).ok_or_else(|| Error::Extract {
        archive: src.to_path_buf(),
        reason: format!("unsupported archive format {:?}", probe.detected),
    })?;
    if spec.extractors.is_empty() {
        return Err(Error::Extract {
            archive: src.to_path_buf(),
            reason: format!("{:?} archives are not supported yet", probe.detected),
        });
    }

    let target = match options.extract_to {
        Some(to) => guard(dest, Path::new(to))?,
        None => dest.to_path_buf(),
    };
    let target_existed = target.exists();
    if target_existed {
        std::fs::create_dir_all(&target)?;
    } else if let Some(parent) = target.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let work = temp_root(&target)?;
    let staging = work.path.join("archive-root");
    std::fs::create_dir_all(&staging)?;

    let mut last_error = None;
    let mut summary = None;
    for ops in spec.extractors {
        match (ops.extract)(src, &staging, &probe, options.observer) {
            Ok(result) => {
                summary = Some(result);
                break;
            }
            Err(error) => {
                // A missing helper is final: another extractor would report
                // the same absence. Everything else may try the next entry in
                // the chain; the staging directory is discarded first.
                if is_fatal(&error) {
                    return Err(error);
                }
                last_error = Some(error);
                if staging.exists() {
                    super::fs::empty_dir(&staging)?;
                }
            }
        }
    }
    let Some(summary) = summary else {
        let error = last_error.unwrap_or_else(|| Error::Extract {
            archive: src.to_path_buf(),
            reason: "no extractor was available".to_string(),
        });
        return Err(error);
    };

    let selected = match options.extract_dir {
        Some(dir) => select_subtree(&staging, dir)?,
        None => staging.clone(),
    };
    move_tree(&selected, &target, !target_existed)?;

    Ok(ExtractReport {
        probe,
        extractor: summary.extractor,
        entries: summary.entries,
        bytes: summary.bytes,
        warnings: summary.warnings,
    })
}

/// Whether an extractor failure must stop the registry chain.
///
/// Path-guard rejections and missing helper binaries are decisions about the
/// archive itself, not parse failures, so neither may be retried by another
/// extractor.
fn is_fatal(error: &Error) -> bool {
    matches!(error, Error::HelperNotFound { .. })
}

/// Locate `extract_dir` inside the staging root, surfacing the top-level names
/// when it is missing.
fn select_subtree(staging: &Path, dir: &str) -> Fallible<PathBuf> {
    let dir = dir.replace('\\', "/");
    let candidate = guard(staging, Path::new(&dir))?;
    if candidate.is_dir() {
        return Ok(candidate);
    }
    let mut available = std::fs::read_dir(staging)?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    available.sort();
    Err(Error::ExtractDirNotFound {
        dir: dir.to_string(),
        available: available.join(", "),
    })
}

/// Move everything under `from` into `into`, merging with existing entries.
fn move_tree(from: &Path, into: &Path, create_into: bool) -> Fallible<()> {
    if create_into {
        std::fs::create_dir_all(into)?;
    }
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        let destination = into.join(entry.file_name());
        move_entry(&source, &destination)?;
    }
    Ok(())
}

fn move_entry(source: &Path, destination: &Path) -> Fallible<()> {
    if destination.exists() {
        if source.is_dir() && destination.is_dir() {
            return move_tree(source, destination, false);
        }
        remove_path(destination)?;
    }
    match std::fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(_) if source.is_file() => {
            std::fs::copy(source, destination)?;
            std::fs::remove_file(source)?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

fn remove_path(path: &Path) -> Fallible<()> {
    if path.is_dir() {
        super::fs::remove_dir(path)?;
    } else if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// A temporary directory that removes itself when dropped.
///
/// Lives inside `dest` when `dest` is a directory, otherwise next to it, so the
/// final move stays on the same volume.
struct TempRoot {
    path: PathBuf,
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = super::fs::remove_dir(&self.path);
    }
}

fn temp_root(dest: &Path) -> Fallible<TempRoot> {
    let parent = if dest.is_dir() {
        dest.to_path_buf()
    } else {
        dest.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    std::fs::create_dir_all(&parent)?;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut name = OsString::from(".hok-extract-");
    name.push(std::process::id().to_string());
    name.push("-");
    name.push(COUNTER.fetch_add(1, Ordering::Relaxed).to_string());
    let mut path = parent.join(name);
    let mut attempt = 0u64;
    while path.exists() {
        attempt += 1;
        path = parent.join(format!(
            ".hok-extract-{}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed),
            attempt
        ));
    }
    std::fs::create_dir(&path)?;
    Ok(TempRoot { path })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::TempDir;

    #[test]
    fn archive_names_match_the_registry() {
        for name in [
            "a.zip",
            "a.nupkg",
            "a.7z",
            "a.tar",
            "a.tar.gz",
            "a.tar.bz2",
            "a.tar.xz",
            "a.tgz",
            "a.txz",
            "a.tbz2",
            "a.tar.zst",
            "a.gz",
            "a.bz2",
            "a.xz",
            "a.zst",
            "a.lzma",
            "a.rar",
            "a.part1.rar",
            "a.msi",
            "a.iso",
            "a.img",
            "a.cab",
            "a.lzh",
        ] {
            assert!(is_archive_name(name), "{name} should be an archive name");
        }
    }

    #[test]
    fn non_archive_names_do_not_match() {
        for name in ["a.txt", "A.ZIP", "a", "a.001.zip.txt"] {
            assert!(!is_archive_name(name), "{name} should not match");
        }
    }

    #[test]
    fn guard_rejects_escapes() {
        let dest = Path::new("dest");
        assert!(guard(dest, Path::new("../evil")).is_err());
        assert!(guard(dest, Path::new("a/../../evil")).is_err());
        assert!(guard(dest, Path::new("/etc/passwd")).is_err());
        assert!(guard(dest, Path::new("a\\..\\..\\evil")).is_err());
        assert!(guard(dest, Path::new("dir/file.txt")).is_ok());
        assert!(guard(dest, Path::new("./dir/file.txt")).is_ok());
    }

    #[test]
    fn guard_returns_the_joined_path() {
        let joined = guard(Path::new("dest"), Path::new("a/b.txt")).unwrap();
        assert_eq!(joined, Path::new("dest/a/b.txt"));
    }

    #[test]
    fn extract_dir_missing_lists_available_entries() {
        let staging = TempDir::new().unwrap();
        std::fs::create_dir(staging.path().join("alpha")).unwrap();
        std::fs::create_dir(staging.path().join("beta")).unwrap();
        let error = select_subtree(staging.path(), "gamma").unwrap_err();
        match error {
            Error::ExtractDirNotFound { dir, available } => {
                assert_eq!(dir, "gamma");
                assert_eq!(available, "alpha, beta");
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn extract_dir_selects_the_named_subtree() {
        let staging = TempDir::new().unwrap();
        let selected = staging.path().join("alpha");
        std::fs::create_dir(&selected).unwrap();
        let found = select_subtree(staging.path(), "alpha").unwrap();
        assert_eq!(found, selected);
    }

    #[test]
    fn unknown_format_is_an_error() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("b.txt");
        let mut file = File::create(&src).unwrap();
        file.write_all(b"plain text").unwrap();
        let dest = dir.path().join("out");
        let observer = CountingObserver::default();
        let options = ExtractOptions {
            extract_dir: None,
            extract_to: None,
            observer: &observer,
        };
        assert!(matches!(
            extract(&src, &dest, &options),
            Err(Error::Extract { .. })
        ));
    }

    #[test]
    fn extract_to_must_stay_inside_dest() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("a.zip");
        let mut file = File::create(&src).unwrap();
        file.write_all(b"PK\x03\x04").unwrap();
        let dest = dir.path().join("out");
        let observer = CountingObserver::default();
        let options = ExtractOptions {
            extract_dir: None,
            extract_to: Some("../escape"),
            observer: &observer,
        };
        assert!(extract(&src, &dest, &options).is_err());
    }

    #[test]
    fn move_tree_merges_directories() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("from");
        let into = dir.path().join("into");
        std::fs::create_dir_all(from.join("shared")).unwrap();
        std::fs::create_dir_all(into.join("shared")).unwrap();
        std::fs::write(from.join("shared/a.txt"), b"a").unwrap();
        std::fs::write(from.join("top.txt"), b"top").unwrap();
        std::fs::write(into.join("shared/b.txt"), b"b").unwrap();
        move_tree(&from, &into, false).unwrap();
        assert!(into.join("top.txt").is_file());
        assert!(into.join("shared/a.txt").is_file());
        assert!(into.join("shared/b.txt").is_file());
    }

    #[test]
    fn failed_extraction_discards_staging() {
        let dir = TempDir::new().unwrap();
        let src = dir.path().join("a.zip");
        std::fs::write(&src, b"PK\x03\x04").unwrap();
        let dest = dir.path().join("out");
        let observer = NoopObserver;
        let options = ExtractOptions {
            extract_dir: None,
            extract_to: None,
            observer: &observer,
        };
        assert!(extract(&src, &dest, &options).is_err());
        let leftovers = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(".hok-extract-"))
            .collect::<Vec<_>>();
        assert!(leftovers.is_empty(), "leftovers: {leftovers:?}");
        assert!(!dest.exists());
    }

    #[derive(Default)]
    struct CountingObserver {
        entries: Mutex<Vec<String>>,
    }

    impl ExtractObserver for CountingObserver {
        fn entry_written(&self, entry: &str) {
            self.entries.lock().unwrap().push(entry.to_string());
        }
    }
}
