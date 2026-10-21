//! Restoring modification times after extraction.
//!
//! Decoders that stream an archive entry to disk cannot know the entry's
//! timestamp before the file exists, so they queue `(path, time)` pairs and
//! apply them once the entry is closed. Doing it afterwards also keeps the
//! clone free of the older timestamps it would otherwise inherit.

use std::fs;
use std::fs::FileTimes;
use std::path::Path;
use std::time::SystemTime;

/// Set the modification time of `path`, ignoring failures.
///
/// A timestamp is metadata, not content: a decoder that restored every entry
/// correctly should not fail because the filesystem refused a timestamp.
///
/// The file is reopened for writing because a read-only handle is refused on
/// Windows; `set_times` needs write access, not read access.
pub fn set_mtime(path: &Path, time: SystemTime) {
    if let Ok(file) = fs::OpenOptions::new().write(true).open(path) {
        let _ = file.set_times(FileTimes::new().set_modified(time));
    }
}

/// Apply every `(path, time)` pair, ignoring individual failures.
pub fn apply_times(times: &[(std::path::PathBuf, SystemTime)]) {
    for (path, time) in times {
        set_mtime(path, *time);
    }
}

/// Convert an MS-DOS date and time into a [`SystemTime`].
///
/// A year below 1980 is not representable in the MS-DOS range and yields
/// `None`, as does any out-of-range field. DOS timestamps are local time; they
/// are interpreted as UTC because the archive carries no zone information.
pub fn dos_to_system_time(
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
    let date = chrono::NaiveDate::from_ymd_opt(i32::from(year), u32::from(month), u32::from(day))?;
    let time = date.and_hms_opt(u32::from(hour), u32::from(minute), u32::from(second))?;
    Some(time.and_utc().into())
}

/// Convert a DOS date/time packed as `date << 16 | time` into a
/// [`SystemTime`], interpreting the local timestamp as UTC.
pub fn packed_dos_to_system_time(file_time: u32) -> Option<SystemTime> {
    if file_time == 0 {
        return None;
    }
    let time = file_time & 0xffff;
    let date = (file_time >> 16) & 0xffff;
    let year = ((date >> 9) & 0x7f) as u16 + 1980;
    let month = ((date >> 5) & 0x0f) as u8;
    let day = (date & 0x1f) as u8;
    let hour = ((time >> 11) & 0x1f) as u8;
    let minute = ((time >> 5) & 0x3f) as u8;
    let second = ((time & 0x1f) * 2) as u8;
    dos_to_system_time(year, month, day, hour, minute, second)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    #[test]
    fn dos_time_converts_known_dates() {
        let epoch = dos_to_system_time(1980, 1, 1, 0, 0, 0).expect("1980 epoch");
        assert_eq!(
            epoch
                .duration_since(UNIX_EPOCH)
                .expect("after epoch")
                .as_secs(),
            315_532_800
        );

        let leap = dos_to_system_time(2024, 2, 29, 13, 14, 15).expect("leap day");
        assert_eq!(
            leap.duration_since(UNIX_EPOCH)
                .expect("after epoch")
                .as_secs(),
            1_709_212_455
        );
    }

    #[test]
    fn dos_time_rejects_out_of_range_fields() {
        assert!(dos_to_system_time(1979, 1, 1, 0, 0, 0).is_none());
        assert!(dos_to_system_time(2108, 1, 1, 0, 0, 0).is_none());
        assert!(dos_to_system_time(2000, 13, 1, 0, 0, 0).is_none());
        assert!(dos_to_system_time(2000, 1, 32, 0, 0, 0).is_none());
        assert!(dos_to_system_time(2000, 1, 1, 24, 0, 0).is_none());
        assert!(dos_to_system_time(2000, 1, 1, 0, 60, 0).is_none());
        assert!(dos_to_system_time(2000, 1, 1, 0, 0, 60).is_none());
    }

    #[test]
    fn dos_time_rejects_days_the_month_does_not_have() {
        assert!(dos_to_system_time(2001, 2, 29, 0, 0, 0).is_none());
        assert!(dos_to_system_time(2000, 4, 31, 0, 0, 0).is_none());
    }

    #[test]
    fn packed_dos_time_decodes_a_real_stamp() {
        let date: u32 = ((2021 - 1980) << 9) | (6 << 5) | 15;
        let time: u32 = (10 << 11) | (30 << 5);
        let stamp = packed_dos_to_system_time((date << 16) | time).expect("decode");
        assert_eq!(packed_dos_to_system_time(0), None);
        assert_eq!(
            stamp,
            dos_to_system_time(2021, 6, 15, 10, 30, 0).expect("same")
        );
    }

    #[test]
    fn set_mtime_updates_the_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("entry");
        std::fs::write(&path, b"x").expect("write");
        let time = dos_to_system_time(2020, 5, 6, 7, 8, 10).expect("timestamp");
        set_mtime(&path, time);
        let modified = std::fs::metadata(&path)
            .expect("metadata")
            .modified()
            .expect("modified");
        // Filesystems round timestamps, so compare at second granularity.
        let delta = modified
            .duration_since(time)
            .or_else(|err| Ok::<_, std::convert::Infallible>(err.duration()))
            .expect("delta");
        assert!(delta < std::time::Duration::from_secs(1), "delta {delta:?}");
    }

    #[test]
    fn set_mtime_ignores_a_missing_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        set_mtime(&dir.path().join("absent"), UNIX_EPOCH);
    }
}
