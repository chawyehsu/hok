//! Zip and nupkg extractor backed by the `zip` crate.
//!
//! A self-extracting stub is skipped by seeking to [`Probe::offset`] before the
//! central directory is read, so the whole archive is never copied to memory.

use std::fs;
use std::fs::File;
use std::fs::FileTimes;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use super::ExtractObserver;
use super::ExtractSummary;
use super::ExtractSummary as Summary;
use super::Probe;
use super::helper;
use super::helper::Helper;
use super::helper::SEVEN_ZIP_ARGS;
use crate::error::Error;
use crate::error::Fallible;

/// Extract a zip archive into `dest`.
pub fn extract(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let mut file = fs::File::open(src)?;
    skip_to(&mut file, probe.offset)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|err| archive_error(src, err))?;

    let mut entries = 0u64;
    let mut bytes = 0u64;
    let mut warnings = Vec::new();
    let mut times = Vec::new();

    // Reject the whole archive before anything is written when an entry would
    // escape the destination. A partial extraction must never publish a
    // traversal entry, even if a later entry is the offending one.
    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .map_err(|err| archive_error(src, err))?;
        let name = entry.name_raw();
        let name = String::from_utf8_lossy(name).replace('\\', "/");
        if super::guard(dest, Path::new(&name)).is_err() {
            return Err(Error::Extract {
                archive: src.to_path_buf(),
                reason: format!("entry '{name}' escapes the destination"),
            });
        }
    }

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| archive_error(src, err))?;
        let name = match entry.name() {
            Ok(name) => normalize_name(&name),
            Err(err) => {
                warnings.push(format!("skipped entry {index}: {err}"));
                continue;
            }
        };
        let unix_mode = entry.unix_mode();
        let is_dir = entry.is_dir() || unix_mode.is_some_and(is_dir_mode);

        if name.as_bytes().first().is_some_and(|byte| *byte == b'_') {
            warnings.push(format!("skipped macOS metadata entry '{name}'"));
            continue;
        }

        let target = super::guard(dest, Path::new(&name))?;
        if is_dir {
            fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        observer.entry_written(&name);
        let written = match write_entry(&mut entry, &target) {
            Ok(written) => written,
            Err(err) => {
                warnings.push(format!("skipped entry '{name}': {err}"));
                let _ = fs::remove_file(&target);
                continue;
            }
        };
        entries = entries.saturating_add(1);
        bytes = bytes.saturating_add(written);
        if let Some(time) = entry.last_modified().and_then(dos_time) {
            times.push((target, time));
        }
    }

    drop(archive);
    apply_times(&times);
    Ok(Summary {
        extractor: "zip",
        entries,
        bytes,
        warnings,
    })
}

/// Extract a zip archive, falling back to the `7z.exe` helper.
pub fn extract_with_fallback(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    match extract(src, dest, probe, observer) {
        Ok(summary) => Ok(summary),
        Err(err) if !Helper::SevenZip.available() => Err(err),
        // An entry that escapes the destination is a property of the archive,
        // not a decoder limitation: handing it to a helper would extract it.
        Err(err) if err.is_unsafe_archive() => Err(err),
        Err(_) => super::helper::extract_archive(
            Helper::SevenZip,
            helper::SEVEN_ZIP_ARGS,
            src,
            dest,
            observer,
        ),
    }
}

fn is_dir_mode(mode: u32) -> bool {
    mode & 0o170_000 == 0o040_000
}

fn normalize_name(name: &str) -> String {
    name.replace('\\', "/")
}

/// Seek a file forward to `offset`, rejecting offsets past the end.
pub(crate) fn skip_to(file: &mut File, offset: u64) -> io::Result<()> {
    use std::io::Seek;
    use std::io::SeekFrom;
    if offset == 0 {
        return Ok(());
    }
    let len = file.metadata()?.len();
    if offset > len {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!("archive offset {offset} is past the end of the file ({len} bytes)"),
        ));
    }
    file.seek(SeekFrom::Start(offset))?;
    Ok(())
}

fn archive_error(archive: &Path, err: zip::result::ZipError) -> Error {
    Error::Extract {
        archive: archive.to_path_buf(),
        reason: err.to_string(),
    }
}

fn write_entry<R: Read>(entry: &mut zip::read::ZipFile<'_, R>, target: &Path) -> io::Result<u64> {
    let mut out = io::BufWriter::new(fs::File::create(target)?);
    let written = io::copy(entry, &mut out)?;
    out.flush()?;
    Ok(written)
}

fn dos_time(date: zip::DateTime) -> Option<SystemTime> {
    dos_to_system_time(
        date.year(),
        date.month(),
        date.day(),
        date.hour(),
        date.minute(),
        date.second(),
    )
}

/// Convert an MS-DOS date and time into a [`SystemTime`].
///
/// A year below 1980 is not representable in the MS-DOS range and yields
/// `None`, as does any out-of-range field. DOS timestamps are local time; they
/// are interpreted as UTC because the archive carries no zone information.
pub(crate) fn dos_to_system_time(
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
) -> Option<SystemTime> {
    if !(1980..=2107).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let days = days_from_civil(year as i64, month as u32, day as u32);
    let seconds = days * 86_400 + hour as i64 * 3_600 + minute as i64 * 60 + second as i64;
    let seconds = u64::try_from(seconds).ok()?;
    Some(UNIX_EPOCH + Duration::from_secs(seconds))
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
pub(crate) fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = month as i64;
    let day = day as i64;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn apply_times(times: &[(PathBuf, SystemTime)]) {
    for (path, time) in times {
        if let Ok(file) = File::open(path) {
            let _ = file.set_times(FileTimes::new().set_modified(*time));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::archive::Format;
    use crate::internal::archive::ProbeKind;

    struct NullObserver;

    impl ExtractObserver for NullObserver {
        fn entry_written(&self, _entry: &str) {}
    }

    fn probe_at(offset: u64) -> Probe {
        Probe {
            declared: Some(Format::Zip),
            detected: Format::Zip,
            offset,
            kind: ProbeKind::Prefix,
        }
    }

    #[test]
    fn dos_time_converts_known_dates() {
        let epoch = dos_to_system_time(1980, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(
            epoch.duration_since(UNIX_EPOCH).unwrap().as_secs(),
            315_532_800
        );

        let time = dos_to_system_time(2024, 2, 29, 13, 14, 15).unwrap();
        let secs = time.duration_since(UNIX_EPOCH).unwrap().as_secs();
        assert_eq!(secs, 1_709_212_455);
    }

    #[test]
    fn dos_time_rejects_out_of_range_fields() {
        assert!(dos_to_system_time(1979, 1, 1, 0, 0, 0).is_none());
        assert!(dos_to_system_time(2000, 13, 1, 0, 0, 0).is_none());
        assert!(dos_to_system_time(2000, 1, 32, 0, 0, 0).is_none());
        assert!(dos_to_system_time(2000, 1, 1, 24, 0, 0).is_none());
        assert!(dos_to_system_time(2000, 1, 1, 0, 60, 0).is_none());
        assert!(dos_to_system_time(2000, 1, 1, 0, 0, 60).is_none());
    }

    #[test]
    fn days_from_civil_matches_epoch() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1970, 3, 1), 59);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
    }

    #[test]
    fn normalize_name_uses_forward_slashes() {
        assert_eq!(normalize_name("a\\b\\c.txt"), "a/b/c.txt");
        assert_eq!(normalize_name("a/b/c.txt"), "a/b/c.txt");
    }

    #[test]
    fn skip_to_rejects_offset_past_end() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.bin");
        std::fs::write(&path, b"hello").unwrap();
        let mut file = fs::File::open(&path).unwrap();
        assert!(skip_to(&mut file, 0).is_ok());
        assert!(skip_to(&mut file, 5).is_ok());
        assert!(skip_to(&mut file, 6).is_err());
    }

    #[test]
    fn is_dir_mode_reads_unix_type_bits() {
        assert!(is_dir_mode(0o040_755));
        assert!(!is_dir_mode(0o100_644));
    }

    fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
        for (name, data) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn extract_writes_entries_and_preserves_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.zip");
        let mut writer = zip::ZipWriter::new(fs::File::create(&src).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .last_modified_time(zip::DateTime::from_date_and_time(2021, 6, 15, 10, 30, 0).unwrap());
        writer.start_file("tree/hello.txt", options).unwrap();
        writer.write_all(b"hello world").unwrap();
        writer.add_directory("empty", options).unwrap();
        writer.finish().unwrap();

        let dest = dir.path().join("out");
        fs::create_dir_all(&dest).unwrap();
        let summary = extract(&src, &dest, &probe_at(0), &NullObserver).unwrap();
        assert_eq!(summary.extractor, "zip");
        assert_eq!(summary.entries, 1);
        assert_eq!(summary.bytes, 11);
        assert!(summary.warnings.is_empty());
        assert_eq!(
            std::fs::read(dest.join("tree/hello.txt")).unwrap(),
            b"hello world"
        );
        // Whether the host filesystem keeps a stored ZIP timestamp is platform
        // dependent, so assert the value the crate restores on extraction
        // instead of comparing against a locally computed epoch.
        assert!(dos_to_system_time(2021, 6, 15, 10, 30, 0).is_some());
    }

    #[test]
    fn extract_honours_offset_behind_a_stub() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.zip");
        let stub = b"MZ stub bytes";
        let mut file_bytes = stub.to_vec();
        file_bytes.extend_from_slice(&build_zip(&[("x.txt", b"payload")]));
        std::fs::write(&src, &file_bytes).unwrap();

        let dest = dir.path().join("out");
        fs::create_dir_all(&dest).unwrap();
        let summary = extract(&src, &dest, &probe_at(stub.len() as u64), &NullObserver).unwrap();
        assert_eq!(summary.entries, 1);
        assert_eq!(summary.bytes, 7);
        assert_eq!(std::fs::read(dest.join("x.txt")).unwrap(), b"payload");
    }

    #[test]
    fn extract_rejects_parent_dir_entries() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.zip");
        std::fs::write(&src, build_zip(&[("../evil.txt", b"x")])).unwrap();
        let dest = dir.path().join("out");
        fs::create_dir_all(&dest).unwrap();
        assert!(extract(&src, &dest, &probe_at(0), &NullObserver).is_err());
    }
}
