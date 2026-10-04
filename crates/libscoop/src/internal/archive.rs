#![allow(dead_code)]
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Format {
    /// .bz2
    Bz2,
    /// .gz
    Gzip,
    /// .rar
    Rar,
    /// .7z, .xz, .tar
    XZip,
    /// .zip
    Zip,
    /// .zstd
    Zst,
}

impl Format {
    pub fn detect<P: AsRef<Path>>(path: P) -> Option<Format> {
        let ext = path
            .as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());
        match ext.as_deref() {
            Some("bz2") | Some("tbz2") | Some("tbz") => Some(Format::Bz2),
            Some("gz") | Some("tgz") => Some(Format::Gzip),
            Some("rar") => Some(Format::Rar),
            Some("7z") | Some("xz") | Some("lzma") | Some("iso") | Some("lzh") | Some("nupkg")
            | Some("tar") => Some(Format::XZip),
            Some("zip") => Some(Format::Zip),
            Some("zst") => Some(Format::Zst),
            _ => None,
        }
    }
}

/// Find the path to the 7z executable.
fn find_7z_exe() -> Option<PathBuf> {
    // Try 7z.exe on PATH
    if crate::internal::os::is_program_available("7z.exe") {
        return Some(PathBuf::from("7z.exe"));
    }
    if crate::internal::os::is_program_available("7z") {
        return Some(PathBuf::from("7z"));
    }

    // Try Scoop's own bundled 7z at <scoop_root>\apps\7zip\current\7z.exe
    let scoop_root = std::env::var_os("SCOOP")?;
    let candidate = PathBuf::from(&scoop_root)
        .join("apps")
        .join("7zip")
        .join("current")
        .join("7z.exe");
    if candidate.exists() {
        return Some(candidate);
    }

    // Try relative to Scoop's shims: <shim_dir>\..\apps\7zip\current\7z.exe
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let candidate = parent
                .join("..")
                .join("apps")
                .join("7zip")
                .join("current")
                .join("7z.exe");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Determine if the file is a self-extracting 7z archive (PE header).
fn is_7z_sfx(path: &Path) -> bool {
    if let Ok(mut file) = std::fs::File::open(path) {
        let mut header = [0u8; 4];
        if file.read(&mut header).is_ok() {
            return header == *b"MZ\0\0";
        }
    }
    false
}

/// Extract an archive to the given destination directory using 7z.
///
/// This function supports the following archive formats: `.zip`, `.7z`, `.gz`,
/// `.bz2`, `.rar`, `.tar`, `.xz`, `.lzh`, `.iso`, `.zst`, `.nupkg`.
pub fn extract<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> crate::error::Fallible<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    crate::internal::fs::ensure_dir(dst)?;

    let exe = find_7z_exe().ok_or_else(|| {
        crate::Error::Custom(
            "7z executable not found. Please install the 7zip package via Scoop or add 7z to PATH."
                .to_string(),
        )
    })?;

    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("x")
        .arg("-y")
        .arg(format!("-o{}", dst.to_string_lossy()));

    if is_7z_sfx(src) {
        cmd.arg("-sfx");
    }

    cmd.arg(src);

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(crate::Error::Custom(format!(
            "failed to extract archive '{}':\nstdout: {}\nstderr: {}",
            src.display(),
            stdout,
            stderr
        )));
    }

    Ok(())
}
