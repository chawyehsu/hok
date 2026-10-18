//! Tar and compressed-tar extractor backed by the `tar` crate.
//!
//! Bare single-file streams (`.gz`, `.bz2`, `.xz`, `.lzma`, `.zst`) are handled
//! here too: one output file named after the archive stem. A bare stream whose
//! decoded bytes begin with a valid tar header is treated as a tar.

use std::fs;
use std::fs::FileTimes;
use std::io;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use super::ExtractObserver;
use super::ExtractSummary;
use super::Format;
use super::Probe;
use super::helper;
use super::helper::SEVEN_ZIP_ARGS;
use super::zip::skip_to;
use crate::error::Error;
use crate::error::Fallible;

const HEADER_LEN: usize = 512;
const BLOCK: u64 = 512;

/// Extract a tar, a compressed tar, or a bare single-file stream.
pub fn extract(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    // A `.tar.*` archive is a tar wrapped in a compressor. The prober reports
    // it as a tar, so the compressor is chosen from the file name.
    let container = match probe.detected {
        Format::Tar => compressed_container(src),
        other => Some(other),
    };
    if container.is_none() {
        let mut file = fs::File::open(src)?;
        skip_to(&mut file, probe.offset)?;
        return extract_tar(BufReader::new(file), src, dest, observer);
    }

    let mut file = fs::File::open(src)?;
    skip_to(&mut file, probe.offset)?;
    let container = container.expect("checked above");
    let reader = decoder(container, file).map_err(|err| extract_error(src, err))?;

    // `PeekReader` lets the one decoded stream be consumed as either a tar or a
    // single file, without decoding twice.
    let mut peek = PeekReader::new(reader);
    let header = peek
        .peek(HEADER_LEN)
        .map_err(|err| extract_error(src, err))?;
    if looks_like_tar(&header) {
        return extract_tar(peek, src, dest, observer);
    }
    extract_stream(peek, &header, src, dest, observer)
}

/// Extract a tar family archive, falling back to the `7z.exe` helper.
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

/// The compressor wrapping a `.tar.*` archive, if the name says so.
fn compressed_container(src: &Path) -> Option<Format> {
    let name = src.file_name()?.to_string_lossy().to_ascii_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        Some(Format::Gzip)
    } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") || name.ends_with(".tbz") {
        Some(Format::Bzip2)
    } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        Some(Format::Xz)
    } else if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        Some(Format::Zstd)
    } else {
        None
    }
}

/// Build the decoder for a bare compressed stream.
fn decoder<R: Read + 'static>(format: Format, reader: R) -> io::Result<Box<dyn Read>> {
    let reader: Box<dyn Read> = match format {
        Format::Gzip => Box::new(flate2::read::MultiGzDecoder::new(reader)),
        Format::Bzip2 => Box::new(bzip2::read::BzDecoder::new(reader)),
        Format::Xz => Box::new(lzma_rust2::XzReader::new(reader, true)),
        Format::Lzma => Box::new(
            lzma_rust2::LzmaReader::new_mem_limit(reader, u32::MAX, None)
                .map_err(|err| io::Error::other(err.to_string()))?,
        ),
        Format::Zstd => Box::new(zstd::stream::read::Decoder::new(reader)?),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a compressed stream format",
            ));
        }
    };
    Ok(reader)
}

fn extract_tar<R: Read>(
    reader: R,
    src: &Path,
    dest: &Path,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let mut archive = tar::Archive::new(reader);
    let mut entries = 0u64;
    let mut bytes = 0u64;
    let mut warnings = Vec::new();
    let mut times = Vec::new();

    for entry in archive.entries().map_err(|err| extract_error(src, err))? {
        let mut entry = entry.map_err(|err| extract_error(src, err))?;
        let path = entry
            .path()
            .map_err(|err| extract_error(src, err))?
            .to_path_buf();
        let name = path.to_string_lossy().replace('\\', "/");
        let target = super::guard(dest, &path)?;

        let is_dir = entry.header().entry_type().is_dir();
        let size = entry.header().size().unwrap_or(0);
        let mtime = entry.header().mtime().ok();

        if !is_dir {
            observer.entry_written(&name);
        }

        // `unpack_in` refuses anything that resolves outside `dest` on its own;
        // the guard above is the single path check for every entry.
        match entry.unpack_in(dest) {
            Ok(true) => {}
            Ok(false) => {
                warnings.push(format!("skipped entry '{name}' outside the destination"));
                continue;
            }
            Err(err) => {
                warnings.push(format!("skipped entry '{name}': {err}"));
                continue;
            }
        }

        if is_dir {
            continue;
        }
        entries = entries.saturating_add(1);
        bytes = bytes.saturating_add(size);
        if let Some(mtime) = mtime {
            times.push((target, mtime));
        }
    }

    apply_times(&times);
    Ok(ExtractSummary {
        extractor: "tar",
        entries,
        bytes,
        warnings,
    })
}

fn extract_stream<R: Read>(
    mut reader: R,
    header: &[u8],
    src: &Path,
    dest: &Path,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let name = stream_name(src)?;
    let Ok(target) = super::guard(dest, Path::new(&name)) else {
        return Err(extract_error(
            src,
            "the output name escapes the destination",
        ));
    };
    observer.entry_written(&name);

    let mut out = io::BufWriter::new(fs::File::create(&target)?);
    let bytes = io::copy(&mut reader, &mut out).map_err(|err| extract_error(src, err))?;
    out.flush()?;
    Ok(ExtractSummary {
        extractor: "stream",
        entries: 1,
        bytes,
        warnings: Vec::new(),
    })
}

/// The output file name for a bare stream: the archive name without its final
/// extension, falling back to `data` when nothing is left.
pub fn stream_name(src: &Path) -> Fallible<String> {
    let stem = src
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned());
    let name = match stem {
        Some(stem) if !stem.is_empty() && stem != "." && stem != ".." => stem,
        _ => "data".to_owned(),
    };
    if name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err(Error::Extract {
            archive: src.to_path_buf(),
            reason: "cannot derive an output name from the archive name".to_owned(),
        });
    }
    Ok(name)
}

fn apply_times(times: &[(PathBuf, u64)]) {
    for (path, mtime) in times {
        if let Ok(file) = fs::File::open(path) {
            let time = UNIX_EPOCH + std::time::Duration::from_secs(*mtime);
            let _ = file.set_times(FileTimes::new().set_modified(time));
        }
    }
}

fn extract_error(archive: &Path, err: impl std::fmt::Display) -> Error {
    Error::Extract {
        archive: archive.to_path_buf(),
        reason: err.to_string(),
    }
}

/// Check whether `header` is a plausible tar header.
///
/// A ustar magic at offset 257 is authoritative. Otherwise an old tar header is
/// accepted only when its stored checksum matches, which rejects most binary
/// data that happens to begin with structured bytes.
pub fn looks_like_tar(header: &[u8]) -> bool {
    if header.len() < HEADER_LEN {
        return false;
    }
    let checksum_ok = parse_octal(&header[148..156]).is_some_and(|stored| {
        let computed = checksum(header);
        stored == computed || stored == computed + 256
    });
    // A `ustar` magic alone is not enough: the checksum must also agree, or
    // arbitrary binary data that happens to contain the bytes is accepted.
    checksum_ok && (&header[257..262] == b"ustar" || header[0] != 0)
}

fn checksum(header: &[u8]) -> u64 {
    header
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            if (148..156).contains(&index) {
                u64::from(b' ')
            } else {
                u64::from(*byte)
            }
        })
        .sum()
}

fn parse_octal(field: &[u8]) -> Option<u64> {
    let mut value = 0u64;
    let mut any = false;
    for byte in field
        .iter()
        .copied()
        .skip_while(|byte| *byte == b' ' || *byte == 0)
    {
        if !(b'0'..=b'7').contains(&byte) {
            break;
        }
        value = value.checked_mul(8)?.checked_add(u64::from(byte - b'0'))?;
        any = true;
    }
    any.then_some(value)
}

/// A reader that can replay a bounded prefix before continuing with the
/// underlying stream.
struct PeekReader<R> {
    inner: R,
    prefix: Vec<u8>,
    offset: usize,
}

impl<R: Read> PeekReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            prefix: Vec::new(),
            offset: 0,
        }
    }

    fn peek(&mut self, len: usize) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0u8; len];
        let mut filled = 0;
        while filled < len {
            match self.inner.read(&mut buffer[filled..])? {
                0 => break,
                read => filled += read,
            }
        }
        buffer.truncate(filled);
        self.prefix = buffer.clone();
        Ok(buffer)
    }
}

impl<R: Read> Read for PeekReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.offset < self.prefix.len() {
            let available = &self.prefix[self.offset..];
            let take = available.len().min(buf.len());
            buf[..take].copy_from_slice(&available[..take]);
            self.offset += take;
            return Ok(take);
        }
        self.inner.read(buf)
    }
}

/// Round `offset` up to the next 512-byte boundary, the way tar pads data.
pub fn pad_to_block(offset: u64) -> u64 {
    offset.div_ceil(BLOCK) * BLOCK
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::internal::archive::NoopObserver;

    fn probe(format: Format) -> Probe {
        Probe {
            declared: Some(format),
            detected: format,
            offset: 0,
            kind: crate::internal::archive::ProbeKind::Prefix,
        }
    }

    fn tar_header(name: &str, size: u64) -> [u8; HEADER_LEN] {
        let mut header = [0u8; HEADER_LEN];
        let bytes = name.as_bytes();
        header[..bytes.len()].copy_from_slice(bytes);
        header[100..108].copy_from_slice(b"0000644\0");
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        let size_field = format!("{size:011o}\0");
        header[124..136].copy_from_slice(size_field.as_bytes());
        header[136..148].copy_from_slice(b"00000000000\0");
        header[156] = b'0';
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        header[148..156].copy_from_slice(b"        ");
        let sum: u32 = header.iter().map(|byte| u32::from(*byte)).sum();
        let field = format!("{sum:06o}\0 ");
        header[148..156].copy_from_slice(field.as_bytes());
        header
    }

    fn build_tar(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, data) in entries {
            out.extend_from_slice(&tar_header(name, data.len() as u64));
            out.extend_from_slice(data);
            let pad = pad_to_block(data.len() as u64) - data.len() as u64;
            out.extend(std::iter::repeat_n(0u8, pad as usize));
        }
        out.extend(std::iter::repeat_n(0u8, 1024));
        out
    }

    #[test]
    fn looks_like_tar_accepts_ustar() {
        assert!(looks_like_tar(&tar_header("a.txt", 3)));
    }

    #[test]
    fn looks_like_tar_rejects_short_and_random() {
        assert!(!looks_like_tar(b"short"));
        assert!(!looks_like_tar(&vec![0x42u8; HEADER_LEN]));
    }

    #[test]
    fn pad_to_block_rounds_up() {
        assert_eq!(pad_to_block(0), 0);
        assert_eq!(pad_to_block(1), 512);
        assert_eq!(pad_to_block(512), 512);
        assert_eq!(pad_to_block(513), 1024);
    }

    #[test]
    fn stream_name_strips_extension() {
        assert_eq!(stream_name(Path::new("/x/tool.gz")).unwrap(), "tool");
        assert_eq!(
            stream_name(Path::new("/x/tool-1.0.tar.xz")).unwrap(),
            "tool-1.0.tar"
        );
        assert_eq!(stream_name(Path::new("/x/.gz")).unwrap(), ".gz");
    }

    #[test]
    fn peek_reader_replays_prefix_then_continues() {
        let mut reader = PeekReader::new(&b"0123456789"[..]);
        assert_eq!(reader.peek(4).unwrap(), b"0123");
        let mut rest = Vec::new();
        reader.read_to_end(&mut rest).unwrap();
        assert_eq!(rest, b"0123456789");
    }

    #[test]
    fn extract_plain_tar_writes_entries() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.tar");
        std::fs::write(
            &src,
            build_tar(&[("tree/a.txt", b"alpha"), ("b.txt", b"beta")]),
        )
        .unwrap();
        let dest = dir.path().join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let summary = extract(&src, &dest, &probe(Format::Tar), &NoopObserver).unwrap();
        assert_eq!(summary.entries, 2);
        assert_eq!(summary.bytes, 9);
        assert!(summary.warnings.is_empty());
        assert_eq!(std::fs::read(dest.join("tree/a.txt")).unwrap(), b"alpha");
        assert_eq!(std::fs::read(dest.join("b.txt")).unwrap(), b"beta");
    }

    #[test]
    fn extract_gzip_stream_as_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("hello.gz");
        let payload = b"single file payload";
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(payload).unwrap();
        std::fs::write(&src, encoder.finish().unwrap()).unwrap();
        let dest = dir.path().join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let summary = extract(&src, &dest, &probe(Format::Gzip), &NoopObserver).unwrap();
        assert_eq!(summary.entries, 1);
        assert_eq!(summary.bytes, payload.len() as u64);
        assert_eq!(std::fs::read(dest.join("hello")).unwrap(), payload);
    }

    #[test]
    fn extract_gzip_tar_detects_tar() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.tar.gz");
        let tar = build_tar(&[("inner.txt", b"inner")]);
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&tar).unwrap();
        std::fs::write(&src, encoder.finish().unwrap()).unwrap();
        let dest = dir.path().join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let summary = extract(&src, &dest, &probe(Format::Gzip), &NoopObserver).unwrap();
        assert_eq!(summary.entries, 1);
        assert_eq!(std::fs::read(dest.join("inner.txt")).unwrap(), b"inner");
    }

    #[test]
    fn extract_xz_tar_detects_tar() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.tar.xz");
        let tar = build_tar(&[("inner.txt", b"xz")]);
        let encoder =
            lzma_rust2::XzWriter::new(Vec::new(), lzma_rust2::XzOptions::default()).unwrap();
        let mut encoder = encoder;
        encoder.write_all(&tar).unwrap();
        std::fs::write(&src, encoder.finish().unwrap()).unwrap();
        let dest = dir.path().join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let summary = extract(&src, &dest, &probe(Format::Xz), &NoopObserver).unwrap();
        assert_eq!(summary.entries, 1);
        assert_eq!(std::fs::read(dest.join("inner.txt")).unwrap(), b"xz");
    }

    #[test]
    fn extract_zstd_stream_as_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("blob.zst");
        std::fs::write(&src, zstd::encode_all(&b"zstd payload"[..], 3).unwrap()).unwrap();
        let dest = dir.path().join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let summary = extract(&src, &dest, &probe(Format::Zstd), &NoopObserver).unwrap();
        assert_eq!(summary.entries, 1);
        assert_eq!(std::fs::read(dest.join("blob")).unwrap(), b"zstd payload");
    }

    #[test]
    fn extract_rejects_parent_dir_entries() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("bad.tar");
        std::fs::write(&src, build_tar(&[("../escape.txt", b"x")])).unwrap();
        let dest = dir.path().join("out");
        std::fs::create_dir_all(&dest).unwrap();
        assert!(extract(&src, &dest, &probe(Format::Tar), &NoopObserver).is_err());
    }

    #[test]
    fn unix_mtime_is_restored() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("mtime.tar");
        std::fs::write(&src, build_tar(&[("t.txt", b"t")])).unwrap();
        let dest = dir.path().join("out");
        std::fs::create_dir_all(&dest).unwrap();
        extract(&src, &dest, &probe(Format::Tar), &NoopObserver).unwrap();
        let modified = std::fs::metadata(dest.join("t.txt"))
            .unwrap()
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(modified <= 1);
    }

    #[test]
    fn extract_honours_stub_offset() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("sfx.tar");
        let mut bytes = vec![0u8; 64];
        bytes.extend_from_slice(&build_tar(&[("x.txt", b"x")]));
        std::fs::write(&src, &bytes).unwrap();
        let dest = dir.path().join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let probe = Probe {
            declared: Some(Format::Tar),
            detected: Format::Tar,
            offset: 64,
            kind: crate::internal::archive::ProbeKind::PeStub,
        };
        let summary = extract(&src, &dest, &probe, &NoopObserver).unwrap();
        assert_eq!(summary.entries, 1);
        assert_eq!(std::fs::read(dest.join("x.txt")).unwrap(), b"x");
    }
}
