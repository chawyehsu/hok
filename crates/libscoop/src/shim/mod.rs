#![allow(dead_code)]
mod pe;
mod resource;

use std::path::Path;
use tracing::warn;

use hok_shim_spec::{Config, Utf16String, encode, to_text};

use crate::{Event, Session, error::Fallible, internal, package::Package};

/// Resource name carrying the shim configuration, as `hok-shim` reads it.
const CONFIG_RESOURCE_NAME: &str = "HOK_SHIM_CONFIG";

/// Language id attached to the injected configuration resource.
const CONFIG_RESOURCE_LANGUAGE: u16 = 0x0409;

/// The `hok-shim` launcher template, embedded by `build.rs`.
#[cfg(windows)]
include!(concat!(env!("OUT_DIR"), "/embedded_shim.rs"));

#[derive(Debug)]
pub struct Shim<'a> {
    name: &'a str,
    real_name: &'a str,
    ty: ShimType,
    args: Option<Vec<&'a str>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShimType {
    /// Bash script
    ///
    /// A shim will be treated as a Bash script if it does not have a file
    /// extension.
    Bash,

    /// Batch script
    ///
    /// A shim will be treated as a Batch script if it has a `.bat`/`.cmd` file
    /// extension.
    Batch,

    /// Executable
    ///
    /// A shim will be treated as an executable if it has a `.exe`/`.com` file
    /// extension.
    Exe,

    /// Java JAR
    ///
    /// A shim will be treated as a Java JAR if it has a `.jar` file extension.
    Java,

    /// PowerShell script
    ///
    /// A shim will be treated as a PowerShell script if it has a `.ps1` file
    /// extension.
    PowerShell,

    /// Python script
    ///
    /// A shim will be treated as a Python script if it has a `.py` file
    /// extension.
    Python,
}

impl Shim<'_> {
    pub fn new(def: Vec<&str>) -> Shim<'_> {
        let length = def.len();
        assert_ne!(length, 0);

        let real_name = def[0];
        let name = if length == 1 {
            internal::path::leaf_base(real_name).unwrap_or(real_name)
        } else {
            def[1]
        };

        let args = if length < 2 {
            None
        } else {
            Some(def[2..].to_vec())
        };

        let ty = Path::new(real_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext.to_lowercase().as_str() {
                "bat" | "cmd" => ShimType::Batch,
                "exe" | "com" => ShimType::Exe,
                "jar" => ShimType::Java,
                "ps1" => ShimType::PowerShell,
                "py" => ShimType::Python,
                _ => ShimType::Bash,
            })
            .unwrap_or(ShimType::Bash);

        Shim {
            name,
            real_name,
            ty,
            args,
        }
    }
}

/// Create shims for a package.
///
/// The package must already be committed under `apps`, since shim targets are
/// resolved there rather than in the cache.
pub fn add(session: &Session, package: &Package) -> Fallible<()> {
    let config = session.config();
    let shims_dir = config.root_path().join("shims");

    let Some(bins) = package.manifest().bin() else {
        return Ok(());
    };

    // TODO: script shims (bash, batch, java, powershell, python) still need a
    // generated wrapper on `PATH`; only native targets are shimmed today.
    let shims = bins
        .into_iter()
        .map(Shim::new)
        .filter(|shim| shim.ty == ShimType::Exe)
        .collect::<Vec<_>>();

    if shims.is_empty() {
        return Ok(());
    }

    internal::fs::ensure_dir(&shims_dir)?;

    if let Some(tx) = session.emitter() {
        let _ = tx.send(Event::PackageShimAddStart);
    }

    let app_path = config.root_path().join("apps").join(package.name());
    let version = if config.no_junction() {
        package.installed_version().unwrap_or("current")
    } else {
        "current"
    };

    for shim in shims {
        let target = internal::path::normalize_path(app_path.join(version).join(shim.real_name));
        let shim_config = Config {
            target: Utf16String::from(target.to_string_lossy().as_ref()),
            args: shim
                .args
                .unwrap_or_default()
                .into_iter()
                .map(Utf16String::from)
                .collect(),
            ..Config::default()
        };

        let shim_path = shims_dir.join(shim.name);

        install(package, &shims_dir, &shim_path, &shim_config, &target)?;

        if let Some(tx) = session.emitter() {
            let name = shim_path.with_extension("exe");
            let _ = tx.send(Event::PackageShimAddProgress(
                name.file_name().unwrap().to_string_lossy().to_string(),
            ));
        }
    }

    if let Some(tx) = session.emitter() {
        let _ = tx.send(Event::PackageShimAddDone);
    }

    Ok(())
}

fn install(
    package: &Package,
    shims_dir: &Path,
    shim_path: &Path,
    config: &Config,
    target: &Path,
) -> Fallible<()> {
    let exe_path = shim_path.with_extension("exe");
    let sidecar_path = shim_path.with_extension("shim");

    back_up_conflict(package, shims_dir, &exe_path);
    back_up_conflict(package, shims_dir, &sidecar_path);

    let mut image = HOK_SHIM_BYTES.to_vec();
    pe::set_subsystem(&mut image, subsystem_of(target)).map_err(crate::Error::Custom)?;
    internal::fs::write_atomic(&exe_path, &image)?;

    let payload = encode(config).map_err(|err| crate::Error::Custom(format!("{err:?}")))?;
    resource::write(
        &exe_path,
        &wides(CONFIG_RESOURCE_NAME),
        &payload,
        Some(CONFIG_RESOURCE_LANGUAGE),
    )
    .map_err(crate::Error::Custom)?;

    internal::fs::write_atomic(&sidecar_path, to_text(config).as_bytes())?;

    Ok(())
}

/// The subsystem a shim for `target` should carry.
///
/// A shim takes the subsystem of the program it launches, so a GUI target gets
/// a GUI shim and never flashes a console. Anything unreadable counts as
/// console: a console image misread as GUI would detach the console the child
/// needs.
fn subsystem_of(target: &Path) -> u16 {
    let image = std::fs::read(target).unwrap_or_default();
    match pe::subsystem(&image) {
        Some(pe::IMAGE_SUBSYSTEM_WINDOWS_GUI) => pe::IMAGE_SUBSYSTEM_WINDOWS_GUI,
        _ => pe::IMAGE_SUBSYSTEM_WINDOWS_CUI,
    }
}

/// Move an existing shim owned by another package aside, the way Scoop does:
/// `<name>.<ext>.<oldpkg>`, so the other package can take it back on removal.
fn back_up_conflict(package: &Package, shims_dir: &Path, path: &Path) {
    if !path.exists() {
        return;
    }

    let Some(owner) = shim_owner(shims_dir, path) else {
        // Not a shim of ours; leave it alone rather than silently hiding it.
        return;
    };

    let pkg_name = package.name();
    if owner == pkg_name {
        return;
    }

    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
    let backup = path.with_file_name(format!("{file_name}.{owner}"));

    warn!(
        "shim '{}' is owned by package '{}', backing it up to '{}'",
        file_name,
        owner,
        backup.file_name().unwrap().to_string_lossy()
    );

    if let Err(err) = std::fs::rename(path, &backup) {
        warn!("failed to back up shim '{}': {}", file_name, err);
    }
}

/// Which package installed the shim at `path`, derived from its configuration
/// target (`<root>\apps\<pkg>\...`).
fn shim_owner(shims_dir: &Path, path: &Path) -> Option<String> {
    let sidecar = path.with_extension("shim");
    let text = std::fs::read_to_string(sidecar).ok()?;
    let config = hok_shim_spec::from_text(&text).ok()?;

    let target = config.target.as_utf16().to_display_string();
    let apps_dir = shims_dir.parent()?.join("apps");
    let relative = Path::new(&target).strip_prefix(&apps_dir).ok()?;
    relative
        .components()
        .next()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
}

fn wides(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Remove shims for a package.
pub fn remove(session: &Session, package: &Package) -> Fallible<()> {
    assert!(package.is_installed());

    let config = session.config();
    let shims_dir = config.root_path().join("shims");

    if let Some(bins) = package.manifest().bin() {
        let pkg_name = package.name();
        let shims_dir_entries = shims_dir
            .read_dir()?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShimRemoveStart);
        }

        for shim in bins.into_iter().map(Shim::new) {
            let mut shim_path = shims_dir.join(shim.name);
            let exts = match shim.ty {
                ShimType::Exe => vec!["exe", "shim"],
                ShimType::PowerShell => vec!["cmd", "ps1", ""],
                _ => vec!["cmd", ""],
            };

            for ext in exts.into_iter() {
                let alt_ext = format!("{}.{}", ext, pkg_name);
                shim_path.set_extension(alt_ext);

                if shim_path.exists() {
                    if let Some(tx) = session.emitter() {
                        let shim_name =
                            shim_path.file_name().unwrap().to_string_lossy().to_string();
                        let _ = tx.send(Event::PackageShimRemoveProgress(shim_name));
                    }

                    std::fs::remove_file(&shim_path)?;
                } else {
                    // this is for removing the `pkg_name` suffix added by the
                    // `alt_ext` above
                    shim_path.set_extension("");

                    shim_path.set_extension(ext);

                    if let Some(tx) = session.emitter() {
                        let shim_name =
                            shim_path.file_name().unwrap().to_string_lossy().to_string();
                        let _ = tx.send(Event::PackageShimRemoveProgress(shim_name));
                    }

                    let _ = std::fs::remove_file(&shim_path);

                    // restore alter shim
                    let fname = shim_path.file_name().unwrap().to_str().unwrap();
                    let mut alt_shims = shims_dir_entries
                        .iter()
                        .flat_map(|entry| {
                            let path = entry.path();
                            let name = path.file_name().unwrap().to_str().unwrap();

                            if name.starts_with(fname) && name != fname {
                                Some(entry)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();

                    if alt_shims.is_empty() {
                        continue;
                    }

                    // sort by modified time, so the latest one will be used
                    // when there are multiple alter shims for the same shim
                    if alt_shims.len() > 1 {
                        alt_shims.sort_by_key(|de| {
                            std::cmp::Reverse(de.metadata().unwrap().modified().unwrap())
                        });
                    }

                    let alt_shim = alt_shims.first().unwrap();
                    let alt_path = alt_shim.path();
                    let alt_path_new = alt_path.with_file_name(fname);
                    std::fs::rename(&alt_path, &alt_path_new)?;
                }
            }
        }

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShimRemoveDone);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
