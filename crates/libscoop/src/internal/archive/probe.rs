//! Bounded probing of archive files.
//!
//! The prober reads at most one kilobyte from the front of a file (plus one
//! kilobyte at the ISO primary-volume-descriptor offset) and decides the format
//! from magic bytes. When the prefix is a PE image the payload appended to the
//! stub is located through the image header and probed there.
//!
//! Validation of a detected header is delegated to the registry: a format is
//! only reported when its detector recognises the bounded slice it was given.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::{Error, Fallible};

use super::{FORMATS, Format, ISO_PVD_OFFSET, Probe, ProbeKind, lookup_name};

/// Number of bytes read from the front of the file.
const PREFIX_LEN: usize = 1024;

/// Magic bytes that announce an archive payload inside a self-extracting stub.
const PAYLOAD_MAGICS: &[&[u8]] = &[
    b"PK\x03\x04",
    b"7z\xBC\xAF\x27\x1C",
    b"Rar!\x1A\x07",
    b"MSCF",
    b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1",
    b"\x1F\x8B",
    b"BZh",
    b"\xFD7zXZ\x00",
    b"\x28\xB5\x2F\xFD",
];

/// A bounded window read from a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Prefix {
    /// Bytes read from the file, at most [`PREFIX_LEN`] long.
    pub(crate) bytes: Vec<u8>,
    /// Absolute offset of the first byte of `bytes` in the file.
    pub(crate) start: u64,
}

/// Probe a downloaded file without extracting it.
///
/// Returns the declared format (from the name), the detected format (from the
/// bytes), and the offset at which the archive begins.
pub fn probe(path: &Path) -> Fallible<Probe> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let declared = declared_format(name)?;
    let Some(mut detected) = detect_path(path, declared.as_ref())? else {
        return Err(Error::Extract {
            archive: path.to_path_buf(),
            reason: "unknown archive format".to_string(),
        });
    };
    if let Some(format) = super::compressed_tar(detected.format, name) {
        detected.format = format;
    }
    Ok(Probe {
        declared: declared.map(|d| d.format),
        detected: detected.format,
        offset: detected.offset,
        kind: detected.kind,
    })
}

/// The format a name claims, together with the matching registry entry.
pub(crate) struct Declared {
    pub(crate) format: Format,
    spec: &'static super::FormatSpec,
}

/// Resolve the declared format for a file name, if any.
///
/// Returns an error when the name matches an archive name that cannot be
/// extracted, so probing never hides an unsupported download.
pub(crate) fn declared_format(name: &str) -> Fallible<Option<Declared>> {
    let Some(spec) = lookup_name(name) else {
        return Ok(None);
    };
    if spec.extractors.is_empty() {
        return Err(Error::Extract {
            archive: Path::new(name).to_path_buf(),
            reason: format!("{:?} archives are not supported yet", spec.format),
        });
    }
    Ok(Some(Declared {
        format: spec.format,
        spec,
    }))
}

/// A detection result with the offset it applies to.
pub(crate) struct Detected {
    pub(crate) format: Format,
    pub(crate) offset: u64,
    pub(crate) kind: ProbeKind,
}

/// Detect the format of the file at `path`.
///
/// A name that declares a compressed tar (`.tar.gz`, `.tgz`, and so on) always
/// resolves to [`Format::Tar`]: the compressor wraps a tar and the extraction
/// path decodes it before reading the archive.
///
/// The declared format is used to break ties for formats without a magic
/// signature of their own (Inno, NSIS and split continuations).
pub(crate) fn detect_path(path: &Path, declared: Option<&Declared>) -> Fallible<Option<Detected>> {
    let mut file = File::open(path)?;
    let prefix = read_window_at(&mut file, 0)?;

    if prefix.bytes.starts_with(b"MZ") {
        return match detect_pe(&mut file, &prefix, declared)? {
            Some(detected) => Ok(Some(detected)),
            None => Ok(Some(Detected {
                format: Format::Exe,
                offset: 0,
                kind: ProbeKind::Prefix,
            })),
        };
    }

    if let Some(format) = detect_prefix(&prefix.bytes, declared) {
        return Ok(Some(Detected {
            format,
            offset: 0,
            kind: ProbeKind::Prefix,
        }));
    }

    detect_iso_file(&mut file)
}

fn detect_iso_file(file: &mut File) -> Fallible<Option<Detected>> {
    let prefix = read_window_at(file, ISO_PVD_OFFSET)?;
    if prefix.bytes.is_empty() {
        return Ok(None);
    }
    match super::detect_iso(&prefix.bytes, prefix.start) {
        Some(Format::Iso) => Ok(Some(Detected {
            format: Format::Iso,
            offset: 0,
            kind: ProbeKind::Prefix,
        })),
        _ => Ok(None),
    }
}

/// Formats whose header is shared with a container that is probed separately
/// need the declared name to disambiguate them.
fn is_ambiguous(format: Format) -> bool {
    matches!(format, Format::Tar | Format::Lzma | Format::Lzh)
}

/// Whether a detector should be consulted for a declared name.
fn detector_allowed(format: Format, declared: Option<&Declared>) -> bool {
    !is_ambiguous(format) || declared.is_some_and(|d| d.spec.format == format)
}

fn detect_prefix(bytes: &[u8], declared: Option<&Declared>) -> Option<Format> {
    scan_window(bytes, 0, declared)
}

/// Run every applicable detector over one bounded window.
fn scan_window(bytes: &[u8], offset: u64, declared: Option<&Declared>) -> Option<Format> {
    FORMATS
        .iter()
        .filter(|spec| detector_allowed(spec.format, declared))
        .find_map(|spec| spec.detect.and_then(|detector| detector(bytes, offset)))
}

fn detect_pe(
    file: &mut File,
    prefix: &Prefix,
    declared: Option<&Declared>,
) -> Fallible<Option<Detected>> {
    let Some((lfanew, section_end)) = pe_layout(&prefix.bytes) else {
        return Ok(None);
    };

    if let Some(payload) = usize::try_from(lfanew)
        .ok()
        .and_then(|at| prefix.bytes.get(at..))
        && let Some(detected) = probe_payload_window(payload, lfanew, declared)
    {
        return Ok(Some(detected));
    }

    let from_lfanew = read_window_at(file, lfanew)?;
    if let Some(detected) = probe_payload_window(&from_lfanew.bytes, lfanew, declared) {
        return Ok(Some(detected));
    }

    // A payload may follow the last section rather than the PE header itself.
    let suffix_end = file.metadata()?.len();
    if let Some(suffix_start) = section_end
        && suffix_start > lfanew
        && suffix_start < suffix_end
    {
        let tail = read_window_at(file, suffix_start)?;
        if let Some(detected) = probe_payload_window(&tail.bytes, suffix_start, declared) {
            return Ok(Some(detected));
        }
    }

    if let Some(declared) = declared
        && matches!(
            declared.spec.format,
            Format::Inno | Format::Nsis | Format::Split
        )
    {
        return Ok(Some(Detected {
            format: declared.spec.format,
            offset: 0,
            kind: ProbeKind::PeStub,
        }));
    }

    Ok(None)
}

/// Find the earliest magic signature in `window` and validate it there.
fn probe_payload_window(window: &[u8], base: u64, declared: Option<&Declared>) -> Option<Detected> {
    let mut earliest: Option<usize> = None;
    for magic in PAYLOAD_MAGICS {
        if let Some(position) = find_magic(window, magic)
            && earliest.is_none_or(|seen| position < seen)
        {
            earliest = Some(position);
        }
    }
    let position = earliest?;
    let payload = window.get(position..)?;
    let format = scan_window(payload, base + position as u64, declared)?;
    Some(Detected {
        format,
        offset: base + position as u64,
        kind: ProbeKind::PeStub,
    })
}

fn find_magic(window: &[u8], magic: &[u8]) -> Option<usize> {
    if magic.len() > window.len() {
        return None;
    }
    window
        .windows(magic.len())
        .position(|candidate| candidate == magic)
}

/// Read the PE payload offset and the end of the image's section table.
fn pe_layout(prefix: &[u8]) -> Option<(u64, Option<u64>)> {
    if prefix.len() < 0x40 || &prefix[0..2] != b"MZ" {
        return None;
    }
    let lfanew = u32::from_le_bytes(prefix.get(0x3C..0x40)?.try_into().ok()?) as u64;
    let section_end = pe_section_end(prefix, lfanew);
    Some((lfanew, section_end))
}

/// Compute the end of the last section, if the section table is in `prefix`.
fn pe_section_end(prefix: &[u8], lfanew: u64) -> Option<u64> {
    let base = usize::try_from(lfanew).ok()?;
    let coff = prefix.get(base..base.checked_add(24)?)?;
    if &coff[0..4] != b"PE\0\0" {
        return None;
    }
    let sections = u16::from_le_bytes(coff.get(6..8)?.try_into().ok()?) as usize;
    let optional_size = u16::from_le_bytes(coff.get(20..22)?.try_into().ok()?) as usize;
    let table = base.checked_add(24)?.checked_add(optional_size)?;
    let mut end = lfanew;
    for index in 0..sections {
        let entry = table.checked_add(index.checked_mul(40)?)?;
        let header = prefix.get(entry..entry.checked_add(40)?)?;
        let virtual_size = u32::from_le_bytes(header.get(8..12)?.try_into().ok()?) as u64;
        let raw_size = u32::from_le_bytes(header.get(16..20)?.try_into().ok()?) as u64;
        let raw_offset = u32::from_le_bytes(header.get(20..24)?.try_into().ok()?) as u64;
        end = end.max(raw_offset.saturating_add(virtual_size.max(raw_size)));
    }
    Some(end)
}

/// Read up to [`PREFIX_LEN`] bytes starting at `start`.
pub(crate) fn read_window_at(file: &mut File, start: u64) -> Fallible<Prefix> {
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::with_capacity(PREFIX_LEN);
    file.by_ref()
        .take(PREFIX_LEN as u64)
        .read_to_end(&mut bytes)?;
    Ok(Prefix { bytes, start })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_temp(dir: &TempDir, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(bytes).unwrap();
        path
    }

    fn detect_bytes(name: &str, bytes: &[u8]) -> Fallible<Probe> {
        let dir = TempDir::new().unwrap();
        let path = write_temp(&dir, name, bytes);
        probe(&path)
    }

    #[test]
    fn detects_zip_signatures() {
        for magic in [b"PK\x03\x04", b"PK\x05\x06", b"PK\x07\x08"] {
            let probe = detect_bytes("a.bin", magic).unwrap();
            assert_eq!(probe.detected, Format::Zip);
            assert_eq!(probe.declared, None);
            assert_eq!(probe.offset, 0);
            assert_eq!(probe.kind, ProbeKind::Prefix);
        }
    }

    #[test]
    fn detects_sevenzip() {
        let probe = detect_bytes("a.bin", b"7z\xBC\xAF\x27\x1Crest").unwrap();
        assert_eq!(probe.detected, Format::SevenZip);
    }

    #[test]
    fn detects_rar_both_versions() {
        let v4 = detect_bytes("a.bin", b"Rar!\x1A\x07\x00more").unwrap();
        assert_eq!(v4.detected, Format::Rar);
        let v5 = detect_bytes("a.bin", b"Rar!\x1A\x07\x01\x00more").unwrap();
        assert_eq!(v5.detected, Format::Rar);
    }

    #[test]
    fn detects_bzip2() {
        let probe = detect_bytes("a.bin", b"BZh9some").unwrap();
        assert_eq!(probe.detected, Format::Bzip2);
    }

    #[test]
    fn detects_xz() {
        let probe = detect_bytes("a.bin", b"\xFD7zXZ\x00rest").unwrap();
        assert_eq!(probe.detected, Format::Xz);
    }

    #[test]
    fn detects_gzip() {
        let probe = detect_bytes("a.bin", b"\x1F\x8B\x08\x00rest").unwrap();
        assert_eq!(probe.detected, Format::Gzip);
    }

    #[test]
    fn detects_zstd() {
        let probe = detect_bytes("a.bin", b"\x28\xB5\x2F\xFDrest").unwrap();
        assert_eq!(probe.detected, Format::Zstd);
    }

    #[test]
    fn detects_cab() {
        let probe = detect_bytes("a.bin", b"MSCF\x00\x00\x00\x00").unwrap();
        assert_eq!(probe.detected, Format::Cab);
    }

    #[test]
    fn detects_msi() {
        let probe = detect_bytes("a.bin", b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1rest").unwrap();
        assert_eq!(probe.detected, Format::Msi);
    }

    #[test]
    fn detects_lzma_alone() {
        let probe = detect_bytes("a.lzma", b"\x5D\x00\x00\x80\x00rest").unwrap();
        assert_eq!(probe.detected, Format::Lzma);
    }

    #[test]
    fn detects_lzh() {
        let probe = detect_bytes("a.lzh", b"\x00\x00-lh5-rest").unwrap();
        assert_eq!(probe.detected, Format::Lzh);
    }

    #[test]
    fn detects_iso_at_pvd_offset() {
        let mut bytes = vec![0u8; ISO_PVD_OFFSET as usize + 64];
        let at = ISO_PVD_OFFSET as usize;
        bytes[at..at + 7].copy_from_slice(b"\x01CD001\x01");
        let probe = detect_bytes("a.iso", &bytes).unwrap();
        assert_eq!(probe.detected, Format::Iso);
    }

    #[test]
    fn detects_tar_ustar_header() {
        let header = ustar_header(b"hello.txt", 5, 600);
        let probe = detect_bytes("a.tar", &header).unwrap();
        assert_eq!(probe.detected, Format::Tar);
    }

    #[test]
    fn rejects_corrupt_tar_checksum() {
        let mut header = ustar_header(b"hello.txt", 5, 600);
        header[0] = b'X';
        let err = detect_bytes("a.tar", &header).unwrap_err();
        assert!(matches!(err, Error::Extract { .. }));
    }

    #[test]
    fn missing_payload_is_exe() {
        let mut bytes = vec![0u8; 0x200];
        bytes[0..2].copy_from_slice(b"MZ");
        let probe = detect_bytes("a.unknown", &bytes).unwrap();
        assert_eq!(probe.detected, Format::Exe);
        assert_eq!(probe.kind, ProbeKind::Prefix);
    }

    #[test]
    fn pe_stub_payload_is_detected() {
        let mut bytes = vec![0u8; 0x200];
        bytes[0..2].copy_from_slice(b"MZ");
        let lfanew = 0x80u32;
        bytes[0x3C..0x40].copy_from_slice(&lfanew.to_le_bytes());
        let at = lfanew as usize;
        bytes[at..at + 4].copy_from_slice(b"PE\0\0");
        bytes[at + 4..at + 6].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[at + 6..at + 8].copy_from_slice(&0u16.to_le_bytes());
        bytes[at + 20..at + 22].copy_from_slice(&0u16.to_le_bytes());
        bytes[at + 24..at + 30].copy_from_slice(b"7z\xBC\xAF\x27\x1C");
        let probe = detect_bytes("a.unknown", &bytes).unwrap();
        assert_eq!(probe.detected, Format::SevenZip);
        assert_eq!(probe.offset, lfanew as u64 + 24);
        assert_eq!(probe.kind, ProbeKind::PeStub);
    }

    #[test]
    fn declared_format_survives_mismatch() {
        let probe = detect_bytes("a.zip", b"7z\xBC\xAF\x27\x1Crest").unwrap();
        assert_eq!(probe.declared, Some(Format::Zip));
        assert_eq!(probe.detected, Format::SevenZip);
    }

    #[test]
    fn declared_extensionless_is_unknown() {
        let err = detect_bytes("a.bin", b"not an archive").unwrap_err();
        match err {
            Error::Extract { reason, .. } => assert_eq!(reason, "unknown archive format"),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn unreadable_file_is_io_error() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("missing.zip");
        assert!(matches!(probe(&missing), Err(Error::Io(_))));
    }

    fn ustar_header(name: &[u8], size: u64, total: usize) -> Vec<u8> {
        let mut header = vec![0u8; total];
        header[..name.len()].copy_from_slice(name);
        header[100..108].copy_from_slice(b"0000644\0");
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        let size_field = format!("{:011o}\0", size);
        header[124..136].copy_from_slice(size_field.as_bytes());
        header[136..148].copy_from_slice(b"00000000000\0");
        header[156..157].copy_from_slice(b"0");
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        header[148..156].copy_from_slice(b"        ");
        let checksum: u32 = header[..512].iter().map(|b| u32::from(*b)).sum::<u32>();
        let field = format!("{:06o}\0 ", checksum);
        header[148..156].copy_from_slice(field.as_bytes());
        header
    }
}
