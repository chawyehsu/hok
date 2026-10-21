//! Locating and invoking helper binaries.
//!
//! Some formats have no complete pure-Rust implementation. Their extractors
//! delegate to an externally installed executable that is found only on `PATH`
//! and never installed automatically (ADR 0003).

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

use which::which;

use super::ExtractObserver;
use super::ExtractSummary;
use crate::error::Error;
use crate::error::Fallible;

/// The `7z.exe` helper, provided by the `7zip` package.
pub const SEVEN_ZIP: &str = "7z.exe";

/// The `innounp.exe` helper, provided by the `innounp` package.
pub const INNOUNP: &str = "innounp.exe";

/// Arguments for extracting every entry of `src` into `dest` with `7z.exe`.
///
/// `-aoa` overwrites without prompting, `-bd` disables the progress bar, and
/// `-y` answers yes to every prompt so the helper never waits on a console.
pub const SEVEN_ZIP_ARGS: &[&str] = &["x", "-aoa", "-bd", "-y"];

/// The Scoop package that provides a helper binary, if one is known.
///
/// Returns `None` for binaries Hok only reaches through `run`.
pub fn package_for(binary: &str) -> Option<&'static str> {
    if binary.eq_ignore_ascii_case(SEVEN_ZIP) || binary.eq_ignore_ascii_case("7z") {
        Some("7zip")
    } else if binary.eq_ignore_ascii_case(INNOUNP) {
        Some("innounp")
    } else {
        None
    }
}

/// Search `PATH` for `program` and return its full path.
///
/// Names containing a path separator are never searched for: a helper binary
/// must be located through `PATH` so extraction behaves the same regardless of
/// the working directory.
pub fn find(program: &str) -> Option<PathBuf> {
    if program.is_empty() || program.contains('/') || program.contains('\\') {
        return None;
    }
    which(program).ok()
}

/// Run a helper binary, capturing its output.
///
/// The program is resolved through `PATH` by the operating system. A missing
/// helper maps to [`Error::HelperNotFound`] when the binary is a known one, and
/// a non-zero exit status maps to [`Error::Extract`] carrying the helper's
/// standard error text.
pub fn run<P, I, S>(program: P, args: I, cwd: Option<&Path>) -> Fallible<Output>
where
    P: AsRef<str>,
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let program = program.as_ref();
    let mut command = Command::new(program);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = match command.output() {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(missing_helper(program));
        }
        Err(err) => {
            return Err(Error::Extract {
                archive: PathBuf::from(program),
                reason: err.to_string(),
            });
        }
    };
    if !output.status.success() {
        return Err(Error::Extract {
            archive: PathBuf::from(program),
            reason: failure_reason(&output),
        });
    }
    Ok(output)
}

/// Build the error reported when `program` cannot be launched.
pub fn missing_helper(program: &str) -> Error {
    match package_for(program) {
        Some(package) => Error::HelperNotFound {
            binary: program.to_owned(),
            package: package.to_owned(),
        },
        None => Error::Extract {
            archive: PathBuf::from(program),
            reason: "helper binary not found on PATH".to_owned(),
        },
    }
}

/// Locate a helper binary or produce the "not found" error for it.
///
/// Callers use this when they need the resolved path for display or for
/// `Command`, and want a single consistent error when the helper is absent.
pub fn require(program: &str) -> Fallible<PathBuf> {
    find(program).ok_or_else(|| missing_helper(program))
}

/// Run a helper that extracts `src` into `dest`, and report what it wrote.
///
/// `flags` are the helper's own switches (for example `x -y`); `src` and
/// `-o<dest>` are appended so the helper unpacks into `dest` and nowhere else.
/// Every file found under `dest` afterwards is reported through `observer`.
pub fn extract_archive(
    program: &str,
    flags: &[&str],
    src: &Path,
    dest: &Path,
    observer: &dyn ExtractObserver,
) -> Fallible<ExtractSummary> {
    require(program)?;
    let mut args = flags
        .iter()
        .map(|flag| (*flag).to_owned())
        .collect::<Vec<_>>();
    args.push(format!("-o{}", dest.display()));
    args.push(src.display().to_string());
    run(program, args, None)?;
    let extractor = if program.eq_ignore_ascii_case(SEVEN_ZIP) {
        "7z.exe"
    } else if program.eq_ignore_ascii_case(INNOUNP) {
        "innounp.exe"
    } else {
        "helper"
    };
    report_tree(src, dest, observer, Vec::new(), extractor)
}

/// Report every file under `dest` through `observer`.
///
/// Used after a helper or a system tool wrote the destination itself. Entry
/// paths are POSIX-style and relative to `dest`.
pub fn report_tree(
    archive: &Path,
    dest: &Path,
    observer: &dyn ExtractObserver,
    warnings: Vec<String>,
    extractor: &'static str,
) -> Fallible<ExtractSummary> {
    let mut entries = 0u64;
    let mut bytes = 0u64;
    let mut stack = vec![dest.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            let Ok(relative) = path.strip_prefix(dest) else {
                continue;
            };
            observer.entry_written(&relative.to_string_lossy().replace('\\', "/"));
            entries = entries.saturating_add(1);
            bytes = bytes.saturating_add(entry.metadata().map(|meta| meta.len()).unwrap_or(0));
        }
    }
    if entries == 0 {
        return Err(Error::Extract {
            archive: archive.to_path_buf(),
            reason: format!("{extractor} did not write any entries"),
        });
    }
    Ok(ExtractSummary {
        extractor,
        entries,
        bytes,
        warnings,
    })
}

fn failure_reason(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if !stderr.is_empty() {
        return stderr.to_owned();
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout = stdout.trim();
    if !stdout.is_empty() {
        return stdout.to_owned();
    }
    match output.status.code() {
        Some(code) => format!("helper exited with status {code}"),
        None => "helper was terminated by a signal".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    /// Serializes tests that mutate the process `PATH`.
    static PATH_LOCK: Mutex<()> = Mutex::new(());

    fn with_path<R>(dirs: &[&Path], body: impl FnOnce() -> R) -> R {
        let _guard = PATH_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let previous = env::var_os("PATH");
        let path = env::join_paths(dirs.iter().copied()).expect("join paths");
        // SAFETY: `PATH_LOCK` serializes access to the process environment.
        unsafe { env::set_var("PATH", path) };
        let result = body();
        // SAFETY: `PATH_LOCK` serializes access to the process environment.
        match previous {
            Some(previous) => unsafe { env::set_var("PATH", previous) },
            None => unsafe { env::remove_var("PATH") },
        }
        result
    }

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("create temp dir")
    }

    #[test]
    fn package_mapping_is_case_insensitive() {
        assert_eq!(package_for("7z.exe"), Some("7zip"));
        assert_eq!(package_for("7Z.EXE"), Some("7zip"));
        assert_eq!(package_for("7z"), Some("7zip"));
        assert_eq!(package_for("innounp.exe"), Some("innounp"));
        assert_eq!(package_for("INNOUNP.EXE"), Some("innounp"));
        assert_eq!(package_for("msiexec.exe"), None);
    }

    #[test]
    fn missing_helper_reports_package() {
        match missing_helper("7z.exe") {
            Error::HelperNotFound { binary, package } => {
                assert_eq!(binary, "7z.exe");
                assert_eq!(package, "7zip");
            }
            other => panic!("unexpected error: {other:?}"),
        }
        match missing_helper("innounp.exe") {
            Error::HelperNotFound { binary, package } => {
                assert_eq!(binary, "innounp.exe");
                assert_eq!(package, "innounp");
            }
            other => panic!("unexpected error: {other:?}"),
        }
        assert!(matches!(missing_helper("other.exe"), Error::Extract { .. }));
    }

    #[test]
    fn find_rejects_names_with_separators() {
        assert!(find("").is_none());
        assert!(find("tools/7z.exe").is_none());
        assert!(find("tools\\7z.exe").is_none());
    }

    #[test]
    fn find_locates_a_binary_on_path() {
        let dir = temp_dir();
        let exe = dir.path().join("7z.exe");
        std::fs::write(&exe, b"").expect("write fake helper");
        with_path(&[dir.path()], || {
            assert_eq!(find("7z.exe"), Some(exe.clone()));
            assert_eq!(find("nope.exe"), None);
        });
    }

    #[test]
    fn find_ignores_empty_and_missing_directories() {
        let dir = temp_dir();
        let missing = dir.path().join("missing");
        with_path(&[std::path::Path::new(""), missing.as_path()], || {
            assert_eq!(find("7z.exe"), None);
        });
    }

    #[test]
    fn failure_reason_prefers_stderr() {
        let output = Output {
            status: failure_status(),
            stdout: b"ignored stdout".to_vec(),
            stderr: b"  boom \r\n".to_vec(),
        };
        assert_eq!(failure_reason(&output), "boom");
    }

    #[test]
    fn failure_reason_falls_back_to_stdout_then_status() {
        let output = Output {
            status: failure_status(),
            stdout: b"out only".to_vec(),
            stderr: Vec::new(),
        };
        assert_eq!(failure_reason(&output), "out only");

        let output = Output {
            status: failure_status(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        let reason = failure_reason(&output);
        assert!(reason.starts_with("helper exited with status"));
    }

    #[test]
    fn run_reports_missing_helper_for_known_binary() {
        let dir = temp_dir();
        let result = with_path(&[dir.path()], || run("7z.exe", ["x"], None));
        match result {
            Err(Error::HelperNotFound { binary, package }) => {
                assert_eq!(binary, "7z.exe");
                assert_eq!(package, "7zip");
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn require_reports_missing_helper() {
        assert!(matches!(
            require("definitely-not-on-path.exe"),
            Err(Error::Extract { .. })
        ));
    }

    fn failure_status() -> std::process::ExitStatus {
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(1)
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(1 << 8)
        }
        #[cfg(not(any(unix, windows)))]
        {
            unreachable!("unsupported target for failure_status")
        }
    }
}
