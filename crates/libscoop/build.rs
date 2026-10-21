//! Locate the `hok-shim` launcher template, copy it into `OUT_DIR`, and expose
//! its bytes to the crate.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    if env::var_os("CARGO_CFG_WINDOWS").is_none() {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let workspace_dir = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf();

    // The profile of the current build, not a preference: a debug build that
    // silently embedded a stale release template would ship a launcher
    // unrelated to the sources under test.
    let profile = env::var("PROFILE").expect("PROFILE");
    let triple = env::var("TARGET").ok();

    let mut target_dirs = Vec::new();
    if let Some(dir) = env::var_os("CARGO_TARGET_DIR").filter(|dir| !dir.is_empty()) {
        target_dirs.push(PathBuf::from(dir));
    }
    if let Some(dir) = target_dir_from_out_dir(&out_dir) {
        target_dirs.push(dir);
    }
    target_dirs.push(workspace_dir.join("target"));
    target_dirs.dedup();

    println!("cargo:rerun-if-env-changed=CARGO_TARGET_DIR");
    println!("cargo:rerun-if-changed=build.rs");

    let shim_name = "hok-shim.exe";
    let mut candidates = shim_candidates(&target_dirs, &profile, triple.as_deref(), shim_name);

    if !candidates.iter().any(|path| path.is_file()) {
        // Custom target dir to prevent deadlocks calling cargo from within the build script.
        // See:
        // - https://github.com/rust-lang/cargo/issues/8938
        // - https://github.com/rust-lang/cargo/issues/9096
        let custom_target_dir = out_dir.join("custom_target");
        std::fs::create_dir_all(&custom_target_dir).expect("create custom target dir");
        target_dirs.insert(0, custom_target_dir);

        let cargo_path = env::var("CARGO").expect("CARGO env var not set");
        let mut cmd = std::process::Command::new(cargo_path);
        cmd.arg("build").arg("-p").arg("hok-shim");
        if let Some(triple) = &triple {
            cmd.arg("--target").arg(triple);
        }
        cmd.arg("--target-dir").arg(&target_dirs[0]);

        let status = cmd
            .status()
            .expect("failed to run cargo build for hok-shim");
        if !status.success() {
            panic!("cargo build for hok-shim failed");
        }

        // Recompute the shim candidates after building with the custom target dir.
        candidates = shim_candidates(&target_dirs, &profile, triple.as_deref(), shim_name);
    }

    let Some(shim_src) = candidates.iter().find(|path| path.is_file()) else {
        panic!(
            "hok-shim template not found for profile '{profile}'.\n\
             Run `cargo build -p hok-shim` first.\n\
             Looked in:\n{}",
            candidates
                .iter()
                .map(|path| format!("  {}", path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        );
    };

    let shim_dest = out_dir.join(shim_name);
    fs::copy(shim_src, &shim_dest).expect("copy hok-shim into OUT_DIR");

    for candidate in &candidates {
        println!("cargo:rerun-if-changed={}", candidate.display());
    }

    let embedded = format!("pub const HOK_SHIM_BYTES: &[u8] = include_bytes!({shim_dest:?});\n");
    fs::write(out_dir.join("embedded_shim.rs"), embedded).expect("write embedded_shim.rs");
}

/// Every directory that may hold a shim binary for this build, most
/// authoritative first.
///
/// `OUT_DIR` always reflects the target directory Cargo actually used, even
/// when `--target-dir` or `[build] target-dir` did not reach this script as
/// `CARGO_TARGET_DIR`. The workspace `target/` stays as the last resort.
fn target_dir_from_out_dir(out_dir: &Path) -> Option<PathBuf> {
    out_dir
        .ancestors()
        .find(|dir| dir.join("release").is_dir() || dir.join("debug").is_dir())
        .map(Path::to_path_buf)
}

/// Shim locations within the candidate target directories, in the current
/// profile only, covering both the plain and the `--target <triple>` layouts.
fn shim_candidates(
    target_dirs: &[PathBuf],
    profile: &str,
    triple: Option<&str>,
    shim_name: &str,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for dir in target_dirs {
        if let Some(triple) = triple {
            candidates.push(dir.join(triple).join(profile).join(shim_name));
        }
        candidates.push(dir.join(profile).join(shim_name));
    }
    candidates
}
