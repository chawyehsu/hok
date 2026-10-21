//! Archive files with no pure-Rust extractor in this build: Cab and Lzh.
//!
//! Both formats are extracted only through the `7z.exe` helper (ADR 0003).

use std::fs;
use std::io;
use std::path::Path;

use super::ExtractObserver;
use super::ExtractSummary;
use super::Probe;
use super::helper;
use super::helper::Helper;
use super::helper::SEVEN_ZIP_ARGS;
use crate::error::Error;
use crate::error::Fallible;

/// Extract a Cabinet (`.cab`) archive with the `cab` crate.
///
/// The crate decodes uncompressed, MSZIP and LZX folders; anything else fails
/// with a parse error, which the registry chain retries with the helper.
pub fn extract_cab(
    src: &Path,
    dest: &Path,
    _probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    let file = fs::File::open(src)?;
    let mut cabinet = cab::Cabinet::new(file).map_err(|err| error(src, err))?;
    let names = cabinet
        .folder_entries()
        .flat_map(|folder| {
            folder
                .file_entries()
                .map(|entry| entry.name().to_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut entries = 0u64;
    let mut bytes = 0u64;
    let mut warnings = Vec::new();
    for name in names {
        let rel = name.replace('\\', "/");
        let target = super::guard(dest, Path::new(&rel))?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        observer.entry_written(&rel);
        let mut reader = match cabinet.read_file(&name) {
            Ok(reader) => reader,
            Err(err) => {
                warnings.push(format!("skipped entry '{rel}': {err}"));
                continue;
            }
        };
        let mut out = io::BufWriter::new(fs::File::create(&target)?);
        match io::copy(&mut reader, &mut out) {
            Ok(written) => {
                entries = entries.saturating_add(1);
                bytes = bytes.saturating_add(written);
            }
            Err(err) => warnings.push(format!("skipped entry '{rel}': {err}")),
        }
    }
    Ok(ExtractSummary {
        extractor: "cab",
        entries,
        bytes,
        warnings,
    })
}

/// Extract a Cabinet archive, falling back to the `7z.exe` helper.
pub fn extract_cab_with_fallback(
    src: &Path,
    dest: &Path,
    probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    match extract_cab(src, dest, probe, observer) {
        Ok(summary) => Ok(summary),
        Err(err) if !Helper::SevenZip.available() => Err(err),
        Err(_) => helper::extract_archive(
            Helper::SevenZip,
            helper::SEVEN_ZIP_ARGS,
            src,
            dest,
            observer,
        ),
    }
}

/// Extract an LZH (`.lzh` / `.lha`) archive with the `7z.exe` helper.
pub fn extract_lzh(
    src: &Path,
    dest: &Path,
    _probe: &Probe,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    helper::require(Helper::SevenZip)?;
    helper::extract_archive(
        Helper::SevenZip,
        helper::SEVEN_ZIP_ARGS,
        src,
        dest,
        observer,
    )
}

fn error(archive: &Path, err: impl std::fmt::Display) -> Error {
    Error::Extract {
        archive: archive.to_path_buf(),
        reason: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_helper_reports_the_package() {
        match helper::missing_helper(Helper::SevenZip) {
            Error::HelperNotFound { binary, package } => {
                assert_eq!(binary, "7z.exe");
                assert_eq!(package, "7zip");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
