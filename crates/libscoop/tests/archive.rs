//! End-to-end tests for the archive module against real fixture files.
//!
//! The fixtures under `tests/fixtures/archive` are committed archives, so these
//! tests never depend on a helper binary or on network access.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use libscoop::{
    ExtractObserver, ExtractReport, Format, extract, extract_options, is_archive_name,
    is_extractable, probe,
};

/// A scratch directory that removes itself when dropped.
fn scratch() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn sub(base: &Path, name: &str) -> PathBuf {
    let path = base.join(name);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("archive")
}

fn fixture(name: &str) -> PathBuf {
    fixtures().join(name)
}

#[derive(Clone, Default)]
struct Recorder {
    entries: Arc<Mutex<Vec<String>>>,
}

impl Recorder {
    fn entries(&self) -> Vec<String> {
        self.entries.lock().unwrap().clone()
    }
}

impl ExtractObserver for Recorder {
    fn entry_written(&self, entry: &str) {
        self.entries.lock().unwrap().push(entry.to_owned());
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn extract_into(
    fixture_name: &str,
    extract_dir: Option<&str>,
    extract_to: Option<&str>,
) -> (tempfile::TempDir, ExtractReport, Vec<String>) {
    let work = scratch();
    let dest = sub(work.path(), "dest");
    let recorder = Recorder::default();
    let options = extract_options(&recorder, extract_dir, extract_to);

    let report = extract(&fixture(fixture_name), &dest, &options)
        .unwrap_or_else(|e| panic!("failed to extract {fixture_name}: {e}"));

    (work, report, recorder.entries())
}

fn assert_same_contents(dest: &Path) {
    assert_eq!(read(&dest.join("hello.txt")), "hello from hok\n");
    assert_eq!(
        read(&dest.join("nested").join("inner.txt")),
        "nested payload\n"
    );
    assert!(dest.join("readme.md").is_file());
}

#[test]
fn zip_probe_and_extract() {
    let probe = probe(&fixture("a.zip")).unwrap();
    assert_eq!(probe.detected, Format::Zip);
    assert_eq!(probe.declared, Some(Format::Zip));

    let (work, report, entries) = extract_into("a.zip", None, None);
    assert_eq!(report.probe.detected, Format::Zip);
    assert_eq!(report.entries, 4);
    assert!(report.bytes > 0);
    assert!(
        report.warnings.is_empty(),
        "warnings: {:?}",
        report.warnings
    );

    assert_same_contents(&work.path().join("dest"));
    assert_eq!(entries.len() as u64, report.entries);
    assert!(entries.iter().any(|e| e.ends_with("inner.txt")));
}

#[test]
fn tar_gz_probe_and_extract() {
    let probe = probe(&fixture("a.tar.gz")).unwrap();
    // A compressed tar is a tar for extraction purposes; the compressor is
    // chosen from the file name.
    assert_eq!(probe.detected, Format::Tar);
    assert_eq!(probe.declared, Some(Format::Tar));

    let (work, report, entries) = extract_into("a.tar.gz", None, None);
    assert_eq!(report.entries, 4);
    assert_eq!(entries.len() as u64, report.entries);

    assert_same_contents(&work.path().join("dest"));
}

#[test]
fn tar_probe_and_extract() {
    let probe = probe(&fixture("a.tar")).unwrap();
    assert_eq!(probe.detected, Format::Tar);

    let (work, report, _) = extract_into("a.tar", None, None);
    assert_eq!(report.entries, 4);
    assert_same_contents(&work.path().join("dest"));
}

#[test]
fn compressed_tar_extract() {
    for name in ["a.tar.xz", "a.tar.bz2", "a.tar.zst"] {
        let (work, report, _) = extract_into(name, None, None);
        assert_eq!(report.entries, 4, "{name}");
        assert_same_contents(&work.path().join("dest"));
    }
}

#[test]
fn sevenzip_probe_and_extract() {
    let probe = probe(&fixture("a.7z")).unwrap();
    assert_eq!(probe.detected, Format::SevenZip);

    let (work, report, entries) = extract_into("a.7z", None, None);
    assert_eq!(report.entries, 4);
    assert_eq!(entries.len() as u64, report.entries);
    assert_same_contents(&work.path().join("dest"));
}

#[test]
fn extract_dir_selects_a_subtree() {
    let (work, report, _) = extract_into("a.zip", Some("nested"), None);

    let dest = work.path().join("dest");
    assert_eq!(read(&dest.join("inner.txt")), "nested payload\n");
    assert!(!dest.join("nested").exists());
    assert!(!dest.join("hello.txt").exists());
    assert!(report.entries >= 1);
}

#[test]
fn extract_to_places_contents_in_subdirectory() {
    let (work, _, _) = extract_into("a.zip", None, Some("payload"));
    assert_same_contents(&work.path().join("dest").join("payload"));
}

#[test]
fn path_traversal_entry_is_rejected() {
    let work = scratch();
    let traversal = work.path().join("traversal.zip");
    write_traversal_zip(&traversal);
    assert!(traversal.is_file(), "fixture must be written");

    let dest = sub(work.path(), "dest");
    let observer = Recorder::default();
    let options = extract_options(&observer, None, None);

    // The archive is rejected as a whole before anything is published.
    assert!(extract(&traversal, &dest, &options).is_err());

    assert!(
        !work.path().join("escape.txt").exists(),
        "path traversal wrote outside the destination"
    );
    assert!(
        !dest.join("escape.txt").exists(),
        "path traversal wrote an entry with `..` into the destination"
    );
}

#[test]
fn unknown_format_is_not_a_copy() {
    let work = scratch();
    let dest = sub(work.path(), "dest");
    let observer = Recorder::default();
    let options = extract_options(&observer, None, None);

    let result = extract(&fixture("b.txt"), &dest, &options);

    assert!(result.is_err());
    assert!(!dest.join("b.txt").exists());
}

#[test]
fn non_archive_detection() {
    assert!(!is_archive_name("b.txt"));
    assert!(!is_archive_name("7z.exe"));
    assert!(is_archive_name("a.zip"));
    assert!(is_archive_name("a.tar.gz"));
    assert!(is_archive_name("a.7z"));

    assert!(probe(&fixture("b.txt")).is_err());
}

/// A PE image with no payload after its single section.
fn bare_executable() -> Vec<u8> {
    let lfanew = 0x80usize;
    let section_end = 0x200usize;
    let raw_offset = 0x100usize;
    let mut bytes = vec![0u8; section_end];
    bytes[0..2].copy_from_slice(b"MZ");
    bytes[0x3C..0x40].copy_from_slice(&(lfanew as u32).to_le_bytes());
    bytes[lfanew..lfanew + 4].copy_from_slice(b"PE\0\0");
    bytes[lfanew + 4..lfanew + 6].copy_from_slice(&0x8664u16.to_le_bytes());
    bytes[lfanew + 6..lfanew + 8].copy_from_slice(&1u16.to_le_bytes());
    let section = lfanew + 24;
    let raw_size = (section_end - raw_offset) as u32;
    bytes[section + 16..section + 20].copy_from_slice(&raw_size.to_le_bytes());
    bytes[section + 20..section + 24].copy_from_slice(&(raw_offset as u32).to_le_bytes());
    bytes
}

#[test]
fn bare_executable_is_copied_not_extracted() {
    let work = scratch();
    let src = work.path().join("tool.exe");
    std::fs::write(&src, bare_executable()).unwrap();

    // A bare executable reports as `Format::Exe`: it is not an archive, so the
    // install path copies it instead of extracting it.
    assert_eq!(probe(&src).unwrap().detected, Format::Exe);
    assert!(!is_extractable(&src).unwrap());

    // Extraction still refuses it, which is what the copy path avoids.
    let dest = sub(work.path(), "dest");
    let observer = Recorder::default();
    let options = extract_options(&observer, None, None);
    match extract(&src, &dest, &options) {
        Ok(_) => panic!("a bare executable must not be extracted"),
        Err(err) => assert!(err.to_string().contains("not an archive"), "{err}"),
    }
}

#[test]
fn extraction_reports_entry_names() {
    // Entry callbacks name the written files in archive order; this is what the
    // CLI live line renders.
    let (_, report, entries) = extract_into("a.zip", None, None);
    assert_eq!(entries.len() as u64, report.entries);
    assert!(entries.iter().any(|e| e == "hello.txt"));
    assert!(entries.iter().any(|e| e.ends_with("inner.txt")));
    assert!(
        !entries
            .iter()
            .any(|e| e.ends_with('/') || e.ends_with('\\'))
    );
}

/// Build a zip whose single entry tries to escape the destination.
///
/// The `zip` writer sanitizes `..` out of names, so the archive is assembled by
/// hand: a local file header and a central directory entry carrying the raw
/// name `../escape.txt`.
fn write_traversal_zip(path: &Path) {
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for byte in data {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        !crc
    }

    let name = b"../escape.txt";
    let data = b"escaped";
    let crc = crc32(data);
    let size = data.len() as u32;
    let mut bytes = Vec::new();

    // Local file header (stored).
    bytes.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
    bytes.extend_from_slice(&20u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&crc.to_le_bytes());
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(name);
    bytes.extend_from_slice(data);

    let central_offset = bytes.len() as u32;
    // Central directory header.
    bytes.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
    bytes.extend_from_slice(&20u16.to_le_bytes());
    bytes.extend_from_slice(&20u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&crc.to_le_bytes());
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.extend_from_slice(&size.to_le_bytes());
    bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(name);
    let central_size = bytes.len() as u32 - central_offset;

    // End of central directory.
    bytes.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&central_size.to_le_bytes());
    bytes.extend_from_slice(&central_offset.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());

    std::fs::write(path, bytes).unwrap();
}
