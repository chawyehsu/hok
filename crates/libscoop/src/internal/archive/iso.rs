//! Iso / img extractor backed by the `iso9660` crate.

use std::collections::VecDeque;
use std::fs;
use std::io;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;

use iso9660::DirectoryEntry;
use iso9660::ISO9660Reader;
use iso9660::ISODirectory;

use super::ExtractObserver;
use super::ExtractSummary;
use super::Probe;
use super::helper;
use super::helper::SEVEN_ZIP_ARGS;
use crate::error::Error;
use crate::error::Fallible;

type Source = OffsetReader<fs::File>;

/// Extract an ISO image into `dest`.
///
/// The ISO9660 volume descriptor sits at block 16 of the image itself, so a
/// self-extracting stub in front of the image is skipped via
/// [`Probe::offset`] by the block reader.
pub fn extract(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let file = fs::File::open(src)?;
    let reader = OffsetReader {
        inner: file,
        offset: probe.offset,
    };
    let image = iso9660::ISO9660::new(reader).map_err(|err| extract_error(src, err))?;
    let (entries, bytes, warnings) = walk_all(image.root, dest, observer)?;

    Ok(ExtractSummary {
        extractor: "iso",
        entries,
        bytes,
        warnings,
    })
}

/// Walk every directory of the image, in archive order.
fn walk_all(
    root: ISODirectory<Source>,
    dest: &Path,
    observer: &dyn ExtractObserver,
) -> Fallible<(u64, u64, Vec<String>)> {
    let mut entries = 0u64;
    let mut bytes = 0u64;
    let mut warnings = Vec::new();
    let mut pending: VecDeque<(ISODirectory<Source>, String)> = VecDeque::new();
    pending.push_back((root, String::new()));
    while let Some((directory, prefix)) = pending.pop_front() {
        for entry in directory.contents() {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    warnings.push(format!("skipped directory entry: {err}"));
                    continue;
                }
            };
            let name = clean_name(entry.identifier());
            if name.is_empty() {
                continue;
            }
            match entry {
                DirectoryEntry::Directory(child) => {
                    pending.push_back((child, join(&prefix, &name)));
                }
                DirectoryEntry::File(file) => {
                    let path = join(&prefix, &name);
                    let Ok(target) = super::guard(dest, Path::new(&path)) else {
                        warnings.push(format!("skipped entry '{path}' outside the destination"));
                        continue;
                    };
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    observer.entry_written(&path);
                    let mut reader = file.read();
                    let mut out = io::BufWriter::new(fs::File::create(&target)?);
                    match io::copy(&mut reader, &mut out) {
                        Ok(written) => {
                            entries = entries.saturating_add(1);
                            bytes = bytes.saturating_add(written);
                        }
                        Err(err) => warnings.push(format!("skipped entry '{path}': {err}")),
                    }
                }
            }
        }
    }
    Ok((entries, bytes, warnings))
}

/// Extract an ISO image, falling back to the `7z.exe` helper.
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

/// ISO 9660 identifiers carry `;1` version suffixes and NUL padding. The `.`
/// and `..` directory entries become empty names and are skipped; the crate
/// already strips the version and a trailing extension dot.
fn clean_name(identifier: &str) -> String {
    let name = identifier.trim_end_matches('\0');
    match name {
        "." | ".." | "" => String::new(),
        other => other.to_owned(),
    }
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}/{name}")
    }
}

fn extract_error(archive: &Path, err: impl std::fmt::Display) -> Error {
    Error::Extract {
        archive: archive.to_path_buf(),
        reason: err.to_string(),
    }
}

/// Adapts a seekable file to the block reader `iso9660` expects, with a
/// configurable byte offset in front of block 0.
struct OffsetReader<R> {
    inner: R,
    offset: u64,
}

impl<R: io::Read + io::Seek> ISO9660Reader for OffsetReader<R> {
    fn read_at(&mut self, buf: &mut [u8], lba: u64) -> io::Result<usize> {
        self.inner
            .seek(SeekFrom::Start(self.offset.saturating_add(lba * 2048)))?;
        let mut filled = 0;
        while filled < buf.len() {
            match self.inner.read(&mut buf[filled..])? {
                0 => break,
                read => filled += read,
            }
        }
        Ok(filled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_name_drops_dot_entries() {
        assert_eq!(clean_name("."), "");
        assert_eq!(clean_name(".."), "");
        assert_eq!(clean_name("PADDED\0"), "PADDED");
        assert_eq!(clean_name("README.TXT"), "README.TXT");
    }

    #[test]
    fn join_handles_empty_prefix() {
        assert_eq!(join("", "a"), "a");
        assert_eq!(join("dir", "a"), "dir/a");
        assert_eq!(join("dir/sub", "a"), "dir/sub/a");
    }

    #[test]
    fn offset_reader_maps_blocks_with_offset() {
        let mut bytes = vec![0u8; 4096 + 8];
        bytes[4096 + 3] = 7;
        let mut reader = OffsetReader {
            inner: io::Cursor::new(bytes),
            offset: 0,
        };
        let mut buf = [0u8; 8];
        let read = reader.read_at(&mut buf, 2).unwrap();
        assert_eq!(read, 8);
        assert_eq!(buf[3], 7);
    }

    #[test]
    fn offset_reader_honours_byte_offset() {
        let mut bytes = vec![0u8; 16];
        bytes[8] = 9;
        let mut reader = OffsetReader {
            inner: io::Cursor::new(bytes),
            offset: 8,
        };
        let mut buf = [0u8; 4];
        reader.read_at(&mut buf, 0).unwrap();
        assert_eq!(buf[0], 9);
    }
}
