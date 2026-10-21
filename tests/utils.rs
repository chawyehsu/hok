//! Shared harness for the `hok` end-to-end tests.
//!
//! Every test gets an isolated Scoop installation inside its own temporary
//! directory, so tests never read or write the developer's real `~/scoop`.
//! The environment is applied to the spawned `hok` process only; the test
//! process itself is never mutated, which keeps tests parallel-safe.
//!
//! # Interactive testing
//!
//! `hok` prompts (`cui::prompt_yes_no`, and the "Please select one, enter the
//! number to continue" prompt in `install`, `cat` and `home`) read from plain
//! stdin with `std::io::stdin().read_line`, so they do not need a terminal.
//! When interactive tests are written, prefer feeding piped stdin to
//! [`std::process::Command`] before reaching for a PTY harness. If a PTY turns
//! out to be required, the options are:
//!
//! 1. `expectrl` gated as `[target.'cfg(not(windows))'.dev-dependencies]`.
//! 2. A `portable-pty` based session, which is cross-platform but means
//!    hand-rolling the expect/send loop `expectrl` provides.

use std::{path::Path, path::PathBuf, process::Command};

use assert_fs::TempDir;

/// A test workspace for e2e tests.
#[allow(unused)]
pub struct TestWorkspace {
    /// Temporary directory root for the test workspace.
    tempdir: TempDir,

    /// Scoop root directory, exposed to `hok` as `SCOOP`.
    scoop_root: PathBuf,

    /// Scoop cache directory, exposed to `hok` as `SCOOP_CACHE`.
    scoop_cache: PathBuf,

    /// Working directory for spawned processes.
    workdir: PathBuf,
}

impl Default for TestWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl TestWorkspace {
    /// Create a new test workspace.
    pub fn new() -> Self {
        let tempdir = assert_fs::TempDir::new().expect("should create tempdir");
        let scoop_root = tempdir.path().join("scoop");
        let scoop_cache = tempdir.path().join("cache");
        let workdir = tempdir.path().join("workdir");

        std::fs::create_dir_all(&workdir).expect("should create workdir");

        Self {
            tempdir,
            scoop_root,
            scoop_cache,
            workdir,
        }
    }

    /// Get the `hok` CLI command, wired to this test workspace.
    pub fn app(&self) -> Command {
        let mut cmd = Command::new(insta_cmd::get_cargo_bin("hok"));

        cmd.env("SCOOP", self.scoop_root.as_os_str());
        cmd.env("SCOOP_CACHE", self.scoop_cache.as_os_str());

        // Force emoji support on Windows during tests by setting WT_SESSION
        // see:
        //   https://github.com/console-rs/console/issues/207
        //   https://github.com/microsoft/terminal/issues/1040
        #[cfg(windows)]
        {
            cmd.env("WT_SESSION", "1");
        }

        cmd.current_dir(self.workdir.as_path());
        cmd
    }

    /// Get the test workspace temporary directory.
    #[allow(unused)]
    pub fn tempdir(&self) -> &TempDir {
        &self.tempdir
    }

    /// Get the Scoop root directory.
    #[allow(unused)]
    pub fn scoop_root(&self) -> &Path {
        self.scoop_root.as_path()
    }

    /// Get the Scoop cache directory.
    #[allow(unused)]
    pub fn scoop_cache(&self) -> &Path {
        self.scoop_cache.as_path()
    }

    /// Get the working directory spawned processes run in.
    #[allow(unused)]
    pub fn workdir(&self) -> &Path {
        self.workdir.as_path()
    }

    /// Get a path inside the Scoop root directory.
    #[allow(unused)]
    pub fn scoop_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.scoop_root.join(path)
    }
}

/// Build the snapshot filters shared by all e2e tests.
///
/// The `Settings` is returned rather than bound here: the bind guard only keeps
/// the filters alive for its own scope, so binding inside the macro would drop
/// them before the assertion runs. Callers apply it with `settings.bind(|| ...)`.
///
/// The workspace is required because the temporary directory path is random
/// per run; a generic temp-dir pattern would also redact unrelated text.
#[allow(unused_macros)]
macro_rules! common_snapshot_settings {
    ($ws:expr) => {{
        let mut settings = insta::Settings::clone_current();

        // crossterm emits styling sequences even when stdout is a pipe, so
        // snapshots would otherwise record raw escapes.
        settings.add_filter(r"\x1b\[[0-9;]*[A-Za-z]", "");

        // The Scoop-specific paths are scrubbed before the workspace fallback
        // so a snapshot can still tell them apart from an arbitrary temp path.
        let root = $ws.scoop_root().display().to_string();
        settings.add_filter(&regex::escape(&root), "[SCOOP_ROOT]");

        let cache = $ws.scoop_cache().display().to_string();
        settings.add_filter(&regex::escape(&cache), "[SCOOP_CACHE]");

        let tempdir = $ws.tempdir().path().display().to_string();
        settings.add_filter(&regex::escape(&tempdir), "[TEMP]");

        // Windows paths and executable suffixes are normalized away so that
        // snapshots are identical across platforms.
        settings.add_filter(r"\\", "/");
        settings.add_filter(r"\.exe\b", "");

        settings
    }};
}

#[allow(unused_imports)]
pub(crate) use common_snapshot_settings;
