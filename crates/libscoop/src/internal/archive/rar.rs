//! RAR extractor backed by the `rars` crate.
//!
//! The archive is parsed straight from its path with the offset taken from the
//! probe, so a self-extracting stub is skipped and never read into memory.
//! Multivolume sets are extracted through the volume API.

use std::fs;
use std::fs::FileTimes;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use rars::ArchiveReadOptions;

use super::ExtractObserver;
use super::ExtractSummary;
use super::Probe;
use super::helper;
use super::helper::SEVEN_ZIP_ARGS;
use crate::error::Error;
use crate::error::Fallible;

/// Extract a RAR archive into `dest`, seeking to [`Probe::offset`] first.
pub fn extract(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let parsed = parse_at(src, probe.offset)?;
    run_one(&parsed.archive, dest, observer, src)
}

/// Extract every volume of a RAR set, in order.
///
/// Only the first volume may carry a probe offset; continuation volumes always
/// begin with the RAR signature.
pub fn extract_volumes(
    volumes: &[PathBuf],
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let Some(first) = volumes.first() else {
        return Err(Error::Extract {
            archive: PathBuf::new(),
            reason: "rar volume set is empty".to_owned(),
        });
    };
    let archives = volumes
        .iter()
        .enumerate()
        .map(|(index, path)| {
            parse_at(path, if index == 0 { probe.offset } else { 0 }).map(|parsed| parsed.archive)
        })
        .collect::<Fallible<Vec<_>>>()?;

    let mut out = Outcome::default();
    let mut passthrough = None;
    let result = rars::extract_volumes_to_with_options(
        &archives,
        ArchiveReadOptions::new(),
        |meta| match out.visit(dest, observer, meta) {
            Ok(writer) => Ok(writer),
            Err(err) => {
                passthrough = Some(err);
                Err(rars::Error::from(io::Error::other("entry rejected")))
            }
        },
    );
    finish(result, passthrough, &mut out, first)
}

/// Extract a RAR archive, falling back to the `7z.exe` helper.
pub fn extract_with_fallback(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    match extract(src, dest, probe, observer) {
        Ok(summary) => Ok(summary),
        Err(err) if helper::find(helper::SEVEN_ZIP).is_none() => Err(err),
        Err(_) => helper::extract_archive(helper::SEVEN_ZIP, SEVEN_ZIP_ARGS, src, dest, observer),
    }
}

fn run_one(
    archive: &rars::Archive,
    dest: &Path,
    observer: &dyn ExtractObserver,
    src: &Path,
) -> Fallible<ExtractSummary> {
    let mut out = Outcome::default();
    let mut passthrough = None;
    let result = archive.extract_to_with_options(ArchiveReadOptions::new(), |meta| {
        match out.visit(dest, observer, meta) {
            Ok(writer) => Ok(writer),
            Err(err) => {
                passthrough = Some(err);
                Err(rars::Error::from(io::Error::other("entry rejected")))
            }
        }
    });
    finish(result, passthrough, &mut out, src)
}

fn finish(
    result: rars::Result<()>,
    passthrough: Option<Error>,
    out: &mut Outcome,
    src: &Path,
) -> Fallible<ExtractSummary> {
    match (result, passthrough) {
        (_, Some(err)) => Err(err),
        (Err(err), None) => Err(extract_error(src, err)),
        (Ok(()), None) => {
            out.apply_times();
            Ok(ExtractSummary {
                extractor: "rar",
                entries: out.entries,
                bytes: out.bytes(),
                warnings: std::mem::take(&mut out.warnings),
            })
        }
    }
}

/// Parse the archive beginning at `offset`, choosing the family from its magic.
///
/// Only the first volume of a set may carry an SFX stub, and the RAR readers
/// locate the signature from the start of the file. A stub is therefore skipped
/// into a temporary file that starts at the archive, so the readers never see
/// the executable bytes.
fn parse_at(src: &Path, offset: u64) -> Fallible<ParsedArchive> {
    if offset == 0 {
        let mut file = fs::File::open(src)?;
        let family = rars::ArchiveReader::detect(&{
            let mut magic = [0u8; 8];
            use std::io::Read;
            let mut read = 0;
            while read < magic.len() {
                match file.read(&mut magic[read..])? {
                    0 => break,
                    bytes => read += bytes,
                }
            }
            magic[..read].to_vec()
        })
        .map_err(|err| extract_error(src, err))?
        .family;
        let archive = parse_file(src, family)?;
        return Ok(ParsedArchive {
            archive,
            _temp: None,
        });
    }

    let temp = TempArchive::strip_stub(src, offset)?;
    parse_at(&temp.path, 0).map(|parsed| ParsedArchive {
        archive: parsed.archive,
        _temp: Some(temp),
    })
}

fn parse_file(path: &Path, family: rars::ArchiveFamily) -> Fallible<rars::Archive> {
    let options = ArchiveReadOptions::new();
    match family {
        rars::ArchiveFamily::Rar13 => rars::rar13::Archive::parse_path(path)
            .map(rars::Archive::Rar13)
            .map_err(|err| extract_error(path, err)),
        rars::ArchiveFamily::Rar15To40 => rars::rar15_40::Archive::parse_path(path)
            .map(rars::Archive::Rar15To40)
            .map_err(|err| extract_error(path, err)),
        _ => rars::rar50::Archive::parse_path(path)
            .map(rars::Archive::Rar50Plus)
            .map_err(|err| extract_error(path, err)),
    }
}

struct ParsedArchive {
    archive: rars::Archive,
    _temp: Option<TempArchive>,
}

/// A copy of an archive with its self-extracting stub removed.
struct TempArchive {
    path: PathBuf,
    _dir: std::path::PathBuf,
}

impl TempArchive {
    fn strip_stub(src: &Path, offset: u64) -> Fallible<TempArchive> {
        use std::io::Seek;
        use std::io::SeekFrom;

        let mut file = fs::File::open(src)?;
        file.seek(SeekFrom::Start(offset))?;
        let dir = std::env::temp_dir().join(format!(
            "hok-rar-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        ));
        fs::create_dir_all(&dir)?;
        let path = dir.join("archive.rar");
        let mut out = fs::File::create(&path)?;
        io::copy(&mut file, &mut out)?;
        Ok(TempArchive { path, _dir: dir })
    }
}

impl Drop for TempArchive {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_dir(&self._dir);
    }
}

fn extract_error(archive: &Path, err: impl std::fmt::Display) -> Error {
    Error::Extract {
        archive: archive.to_path_buf(),
        reason: err.to_string(),
    }
}

#[derive(Default)]
struct Outcome {
    entries: u64,
    bytes: u64,
    warnings: Vec<String>,
    times: Vec<(PathBuf, SystemTime)>,
    writers: Vec<std::sync::Arc<std::sync::atomic::AtomicU64>>,
}

impl Outcome {
    fn visit(
        &mut self,
        dest: &Path,
        observer: &dyn ExtractObserver,
        meta: &rars::ExtractedEntryMeta,
    ) -> Fallible<Box<dyn Write>> {
        let name = meta.name_lossy().replace('\\', "/");
        let target = super::guard(dest, Path::new(&name))?;

        if meta.is_directory {
            fs::create_dir_all(&target)?;
            return Ok(Box::new(io::sink()));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        observer.entry_written(&name);
        let file = fs::File::create(&target)?;
        self.entries = self.entries.saturating_add(1);
        if let Some(time) = dos_to_system_time(meta.file_time) {
            self.times.push((target, time));
        }
        let writer = CountingWriter {
            inner: io::BufWriter::new(file),
            len: Default::default(),
        };
        self.writers.push(writer.len.clone());
        Ok(Box::new(writer))
    }

    fn bytes(&self) -> u64 {
        self.writers
            .iter()
            .map(|len| len.load(std::sync::atomic::Ordering::Relaxed))
            .sum()
    }

    fn apply_times(&self) {
        for (path, time) in &self.times {
            if let Ok(file) = fs::File::open(path) {
                let _ = file.set_times(FileTimes::new().set_modified(*time));
            }
        }
    }
}

/// Counts the bytes written so extraction totals do not depend on metadata.
struct CountingWriter<W> {
    inner: W,
    len: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.len
            .fetch_add(written as u64, std::sync::atomic::Ordering::Relaxed);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Convert a DOS date/time packed as `date << 16 | time` into a
/// [`SystemTime`], interpreting the local timestamp as UTC.
pub fn dos_to_system_time(file_time: u32) -> Option<SystemTime> {
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
    super::zip::dos_to_system_time(year, month, day, hour, minute, second)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe_offset(offset: u64) -> Probe {
        Probe {
            declared: Some(super::super::Format::Rar),
            detected: super::super::Format::Rar,
            offset,
            kind: super::super::ProbeKind::Prefix,
        }
    }

    fn build_rar(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = rars::Builder::new(rars::ArchiveVersion::Rar50).store(true);
        for (name, data) in entries {
            builder
                .add_bytes(name.as_bytes().to_vec(), data.to_vec(), None, None)
                .unwrap();
        }
        builder.to_bytes().unwrap()
    }

    fn observer() -> std::sync::Arc<CollectObserver> {
        std::sync::Arc::new(CollectObserver::default())
    }

    #[derive(Default)]
    struct CollectObserver(std::sync::Mutex<Vec<String>>);

    impl ExtractObserver for CollectObserver {
        fn entry_written(&self, entry: &str) {
            self.0.lock().unwrap().push(entry.to_owned());
        }
    }

    #[test]
    fn dos_to_system_time_rejects_zero() {
        assert!(dos_to_system_time(0).is_none());
    }

    #[test]
    fn dos_to_system_time_decodes_real_date() {
        let date = ((2024u32 - 1980) << 9) | (2 << 5) | 29;
        let time = (13u32 << 11) | (14 << 5) | (15 / 2);
        let seconds = dos_to_system_time((date << 16) | time)
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert_eq!(seconds, 1_709_212_454);
    }

    #[test]
    fn counting_writer_sums_bytes() {
        let mut writer = CountingWriter {
            inner: io::sink(),
            len: Default::default(),
        };
        writer.write_all(b"abc").unwrap();
        writer.write_all(b"de").unwrap();
        assert_eq!(writer.len.load(std::sync::atomic::Ordering::Relaxed), 5);
    }

    #[test]
    fn extract_writes_entries() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.rar");
        std::fs::write(
            &src,
            build_rar(&[("a.txt", b"alpha"), ("dir/b.txt", b"beta")]),
        )
        .unwrap();
        let dest = dir.path().join("out");
        fs::create_dir_all(&dest).unwrap();
        let observer = observer();
        let summary = extract(&src, &dest, &probe_offset(0), observer.as_ref()).unwrap();
        assert_eq!(summary.entries, 2);
        assert_eq!(summary.bytes, 9);
        assert_eq!(std::fs::read(dest.join("a.txt")).unwrap(), b"alpha");
        assert_eq!(std::fs::read(dest.join("dir/b.txt")).unwrap(), b"beta");
        assert_eq!(observer.0.lock().unwrap().len(), 2);
    }

    #[test]
    fn extract_honours_stub_offset() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.rar");
        let mut bytes = b"MZ stub".to_vec();
        bytes.extend_from_slice(&build_rar(&[("x.txt", b"payload")]));
        std::fs::write(&src, &bytes).unwrap();
        let dest = dir.path().join("out");
        fs::create_dir_all(&dest).unwrap();
        let observer = observer();
        let summary = extract(&src, &dest, &probe_offset(7), observer.as_ref()).unwrap();
        assert_eq!(summary.entries, 1);
        assert_eq!(std::fs::read(dest.join("x.txt")).unwrap(), b"payload");
    }

    #[test]
    fn extract_volumes_folds_split_members() {
        let dir = tempfile::tempdir().unwrap();
        let payload = vec![7u8; 200_000];
        let mut builder = rars::Builder::new(rars::ArchiveVersion::Rar50)
            .store(true)
            .volume_size(Some(64 * 1024));
        builder
            .add_bytes(b"big.bin".to_vec(), payload.clone(), None, None)
            .unwrap();
        let parts = builder.build_volumes(None).unwrap();
        let mut paths = Vec::new();
        for (index, part) in parts.iter().enumerate() {
            let path = dir.path().join(format!("a.part{}.rar", index + 1));
            std::fs::write(&path, part).unwrap();
            paths.push(path);
        }
        let dest = dir.path().join("out");
        fs::create_dir_all(&dest).unwrap();
        let observer = observer();
        let summary = extract_volumes(&paths, &dest, &probe_offset(0), observer.as_ref()).unwrap();
        assert_eq!(summary.entries, 1);
        assert_eq!(std::fs::read(dest.join("big.bin")).unwrap(), payload);
    }

    #[test]
    fn extract_rejects_zip_slip_names() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out");
        fs::create_dir_all(&dest).unwrap();
        let mut outcome = Outcome::default();
        let observer = observer();
        let meta = rars::ExtractedEntryMeta::new(b"../evil.txt".to_vec(), 0, 0x20, false);
        assert!(outcome.visit(&dest, observer.as_ref(), &meta).is_err());
    }
}
