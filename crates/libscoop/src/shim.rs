#![allow(dead_code)]
use std::path::Path;

use crate::{error::Fallible, internal, package::Package, Event, Session};

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

impl<'a> Shim<'a> {
    pub fn new(def: Vec<&'a str>) -> Shim<'a> {
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

/// Add shims for a package.
///
/// Creates shim files in the Scoop `shims` directory for each binary defined
/// in the package's `bin` field.
pub fn add(session: &Session, package: &Package) -> Fallible<()> {
    let config = session.config();
    let shims_dir = config.root_path().join("shims");
    internal::fs::ensure_dir(&shims_dir)?;

    if let Some(bins) = package.manifest().bin() {
        let pkg_name = package.name();
        let version = if config.no_junction() {
            package
                .installed_version()
                .unwrap_or_else(|| package.version())
                .to_owned()
        } else {
            "current".to_string()
        };

        for shim_def in bins {
            let shim = Shim::new(shim_def);

            let shim_path = shims_dir.join(shim.name);
            let exts: Vec<String> = match shim.ty {
                ShimType::Exe => vec!["exe".to_string(), format!("{}.{}", shim.name, pkg_name)],
                ShimType::PowerShell => vec![
                    format!("{}.{}", shim.name, pkg_name),
                    format!("{}.cmd", shim.name),
                    format!("{}.ps1", shim.name),
                ],
                _ => vec![
                    format!("{}.{}", shim.name, pkg_name),
                    format!("{}.cmd", shim.name),
                ],
            };

            let real_path = shims_dir.join(&exts[0]);

            // Determine the target binary path
            let apps_dir = config.root_path().join("apps");
            let bin_dir = apps_dir.join(pkg_name).join(&version);
            let target = bin_dir.join(shim.real_name);

            create_shim(&real_path, &target, &shim)?;

            // Create alternate extension shims
            for alt_ext in exts.iter().skip(1) {
                let alt_shim_path = shims_dir.join(alt_ext);
                let _ = std::fs::remove_file(&alt_shim_path);
                let _ = std::fs::copy(&real_path, &alt_shim_path);
            }
        }
    }

    Ok(())
}

/// Create a shim file that forwards execution to the target binary.
fn create_shim(shim_path: &Path, target: &Path, shim: &Shim) -> Fallible<()> {
    // On Windows, create a batch file as a simple shim
    // A proper implementation would embed the shim.exe binary
    #[cfg(windows)]
    {
        let _ = std::fs::remove_file(shim_path);
        let content = create_shim_content(target, shim)?;
        std::fs::write(shim_path, &content)?;

        // Also create .bat/.cmd variants for Batch and PowerShell types
        if shim.ty == ShimType::Batch || shim.ty == ShimType::PowerShell {
            let bat_path = shim_path.with_extension("bat");
            std::fs::write(&bat_path, &content)?;
        }
    }

    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(shim_path);
        let content = create_shim_content(target, shim)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(shim_path, &content)?;
        let mut perms = std::fs::metadata(shim_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(shim_path, perms)?;
    }

    Ok(())
}

/// Generate shim script content for the given target.
fn create_shim_content(target: &Path, shim: &Shim) -> Fallible<String> {
    let target_str = target.to_string_lossy();

    let args_str = if let Some(args) = &shim.args {
        args.join(" ")
    } else {
        String::new()
    };

    #[cfg(windows)]
    {
        if shim.ty == ShimType::Exe || shim.ty == ShimType::Bash {
            return Ok(format!("@echo off\n\"{}\" {}\n", target_str, args_str));
        }
        if shim.ty == ShimType::Java {
            return Ok(format!(
                "@echo off\njava -jar \"{}\" {}\n",
                target_str, args_str
            ));
        }
        if shim.ty == ShimType::Python {
            return Ok(format!(
                "@echo off\npython \"{}\" {}\n",
                target_str, args_str
            ));
        }
    }

    #[cfg(unix)]
    {
        if shim.ty == ShimType::Exe || shim.ty == ShimType::Bash {
            return Ok(format!("#!/bin/sh\n\"{}\" {}\n", target_str, args_str));
        }
        if shim.ty == ShimType::Java {
            return Ok(format!(
                "#!/bin/sh\njava -jar \"{}\" {}\n",
                target_str, args_str
            ));
        }
        if shim.ty == ShimType::Python {
            return Ok(format!(
                "#!/bin/sh\npython \"{}\" {}\n",
                target_str, args_str
            ));
        }
    }

    Ok(format!("#!/bin/sh\n\"{}\" {}\n", target_str, args_str))
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
