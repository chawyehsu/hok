use once_cell::unsync::OnceCell;
use scoop_hash::ChecksumBuilder;
use std::io::Read;
use tracing::{debug, info};

use crate::{
    env, error::Fallible, internal, persist, psmodule, shim, shortcut, Error, Event, QueryOption,
    Session,
};

use super::{
    download::{self, DownloadSize},
    manifest::InstallInfo,
    query, resolve, Package,
};

/// Options that may be used to tweak behavior of package sync operation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SyncOption {
    /// Assume YES on all prompts.
    ///
    /// # Note
    ///
    /// This option will also suppress the prompt for package candidate selection.
    /// A built-in candidate selection algorithm will be used to select the
    /// proper candidate. This may not be the desired behavior in some cases.
    ///
    /// Enabling this option will also suppress the calculation of download size.
    AssumeYes,

    /// Download package only.
    ///
    /// # Note
    ///
    /// To sync packages by just downloading and caching them without installing
    /// or upgrading, this option can be used. Transcation will be stopped after
    /// the download is done.
    DownloadOnly,

    /// Force operations on held packages.
    ///
    /// # Note
    ///
    /// Held packages are ignored during the replace, upgrade or uninstall
    /// operations by default. The option can be used to escape the hold and
    /// enforce operations on the held packages.
    ///
    /// Packages will be held again after the replace or upgrade operation.
    EscapeHold,

    /// Ignore local cache and force package download.
    ///
    /// # Note
    ///
    /// This option is not intended to be used with the [`Offline`][1]
    /// option.
    ///
    /// [1]: SyncOption::Offline
    IgnoreCache,

    /// Ignore transaction failure.
    ///
    /// The sync operation processes packages in the transaction one by one
    /// according to the dependency order. By default, the transaction will be
    /// aborted if any failure occurs during the operation.
    ///
    /// # Note
    ///
    /// This option can be used to ignore the failure and continue the operation
    /// to commit the remaining packages in the transaction.
    ///
    /// When a failure occurs, the operation will be stopped immediately and
    /// a rollback will be performed on the exact package causing the failure
    /// while successfully committed packages will be kept be as they are. The
    /// rest of the unpocessed packages will be skipped, and the error will be
    /// returned.
    ///
    /// **NO rollback will be performed if this option is enabled**, which means
    /// there may be broken packages being committed to the system.
    IgnoreFailure,

    /// Do not install dependencies.
    ///
    /// # Note
    ///
    /// By default, dependencies of the pending installation package will be
    /// resolved and installed **recursively** if they are not installed yet.
    /// One can opt in this option to disable the default behavior. However,
    /// it is not recommended to do so since it clearly breaks the dependency
    /// relationship, and may stop the dependents from working properly.
    NoDependencies,

    /// Stop checking hash of downloaded packages.
    ///
    /// # Note
    ///
    /// Integrity check helps to ensure the downloaded packages are not corrupted
    /// or tampered. Hash check will be performed by default. In some cases, user
    /// may want to skip the check to force the installation or upgrade of the
    /// packages. By opting in this option, the hash check will be skipped.
    ///
    /// It is highly **NOT** recommended to use this option unless you really
    /// know what you are doing.
    NoHashCheck,

    /// Do not upgrade packages.
    ///
    /// This option is not intended to be used with the [`OnlyUpgrade`][1] option.
    ///
    /// [1]: SyncOption::OnlyUpgrade
    NoUpgrade,

    /// Do not replace packages.
    ///
    /// # Note
    ///
    /// When a package is installed and a same-named package is proposed to be
    /// installed, a replace operation will be performed if the proposed package
    /// is from a different bucket from the installed one.
    ///
    /// By opting in this option, the replace operation will be suppressed.
    NoReplace,

    /// Offline mode.
    ///
    /// # Note
    ///
    /// This option is useful when user wants to install or upgrade packages
    /// with existing local cached packages. By opting in this option and having
    /// valid caches prepared, network access can be avoided to perform the sync
    /// operation. However, the transaction may fail if there is any package file
    /// missing or invalid cache.
    ///
    /// This option is basically the opposite of the [`IgnoreCache`][1] option.
    ///
    /// [1]: SyncOption::IgnoreCache
    Offline,

    /// Upgrade packages only.
    ///
    /// Use this option to specify a sync operation of only upgrading packages.
    ///
    /// This option is not intended to be used with the [`NoUpgrade`][1] option.
    ///
    /// [1]: SyncOption::NoUpgrade
    OnlyUpgrade,

    /// Uninstall packages.
    ///
    /// Use this option to specify a sync operation of only uninstalling packages.
    Remove,

    /// Purge uninstall.
    ///
    /// # Note
    ///
    /// By enabling this option, persistent data of the pending removal packages
    /// will be removed simultaneously.
    ///
    /// This option only takes effect with the [`Remove`][1] option.
    ///
    /// [1]: SyncOption::Remove
    Purge,

    /// Cascade uninstall.
    ///
    /// # Note
    ///
    /// By opt in this option, dependencies of the pending removal package
    /// will also be removed **recursively** if they are not required by other
    /// installed packages.
    ///
    /// This option only takes effect with the [`Remove`][1] option.
    ///
    /// [1]: SyncOption::Remove
    Cascade,

    /// Disable dependent check.
    ///
    /// # Note
    ///
    /// By default, a reverse dependencies check will be performed on the pending
    /// removal package. If any installed package depends on the pending removal
    /// package, the removal operation will be aborted.
    ///
    /// The default behavior can be modified by opting in this option, however,
    /// it is not recommended to do so since it clearly breaks the dependency
    /// relationship, and may stop the dependents from working properly.
    ///
    /// This option only takes effect with the [`Remove`][1] option.
    ///
    /// [1]: SyncOption::Remove
    NoDependentCheck,
}

/// Transaction of sync operation.
///
/// # Note
///
/// A transaction is a set of packages that will be installed, upgraded, replaced
/// or removed. The transaction is calculated by the sync operation and can be
/// used to prompt the user to confirm the operation.
#[derive(Clone)]
pub struct Transaction {
    /// Packages that will be installed with the transaction.
    install: OnceCell<Vec<Package>>,

    /// Packages that will be upgraded with the transaction.
    upgrade: OnceCell<Vec<Package>>,

    /// Packages that will be replaced with the transaction.
    replace: OnceCell<Vec<Package>>,

    /// Packages that will be removed with the transaction.
    remove: OnceCell<Vec<Package>>,

    /// Total download size of the transaction.
    download_size: OnceCell<DownloadSize>,
}

impl Transaction {
    fn new() -> Transaction {
        Transaction {
            install: OnceCell::new(),
            upgrade: OnceCell::new(),
            replace: OnceCell::new(),
            remove: OnceCell::new(),
            download_size: OnceCell::new(),
        }
    }

    fn set_install(&self, packages: Vec<Package>) {
        let _ = self.install.set(packages);
    }

    fn set_upgrade(&self, packages: Vec<Package>) {
        let _ = self.upgrade.set(packages);
    }

    fn set_replace(&self, packages: Vec<Package>) {
        let _ = self.replace.set(packages);
    }

    fn set_remove(&self, packages: Vec<Package>) {
        let _ = self.remove.set(packages);
    }

    fn set_download_size(&self, download_size: DownloadSize) -> bool {
        self.download_size.set(download_size).is_ok()
    }

    fn add_view(&self) -> Vec<&Package> {
        self.install_view()
            .into_iter()
            .chain(self.upgrade_view())
            .chain(self.replace_view())
            .flatten()
            .collect::<Vec<_>>()
    }

    /// Get packages that will be installed with the transaction.
    ///
    /// # Returns
    ///
    /// A reference to the vector of packages that will be installed or `None`
    /// if no packages will be installed.
    pub fn install_view(&self) -> Option<&Vec<Package>> {
        self.install.get()
    }

    /// Get packages that will be upgraded with the transaction.
    ///
    /// # Returns
    ///
    /// A reference to the vector of packages that will be upgraded or `None`
    /// if no packages will be upgraded.
    pub fn upgrade_view(&self) -> Option<&Vec<Package>> {
        self.upgrade.get()
    }

    /// Get packages that will be replaced with the transaction.
    ///
    /// # Returns
    ///
    /// A reference to the vector of packages that will be replaced or `None`
    /// if no packages will be replaced.
    pub fn replace_view(&self) -> Option<&Vec<Package>> {
        self.replace.get()
    }

    /// Get packages that will be removed with the transaction.
    ///
    /// # Returns
    ///
    /// A reference to the vector of packages that will be removed or `None`
    /// if no packages will be removed.
    pub fn remove_view(&self) -> Option<&Vec<Package>> {
        self.remove.get()
    }

    /// Get the total download size of the transaction.
    ///
    /// # Returns
    ///
    /// A `DownloadSize` reference that contains the total download size of the
    /// transaction.
    pub fn download_size(&self) -> Option<&DownloadSize> {
        self.download_size.get()
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

/// Sync operation: install and/or upgrade packages.
pub fn install(session: &Session, queries: &[&str], options: &[SyncOption]) -> Fallible<()> {
    let mut packages = vec![];

    let only_upgrade = options.contains(&SyncOption::OnlyUpgrade);
    let escape_hold = options.contains(&SyncOption::EscapeHold);

    if only_upgrade {
        packages = query::query_installed(session, queries, &[QueryOption::Upgradable])?;

        // Replace the packages with their upgradable references.
        packages = packages
            .into_iter()
            .map(|p| p.upgradable().cloned().unwrap())
            .collect::<Vec<_>>();
    } else {
        let synced = query::query_synced(session, &["*"], &[])?;

        for &query in queries {
            let mut matched = synced
                .iter()
                .filter(|&p| {
                    let (query_bucket, query_name) = query.split_once('/').unwrap_or(("", query));
                    let bucket_matched = query_bucket.is_empty() || p.bucket() == query_bucket;
                    let name_matched = p.name() == query_name;
                    bucket_matched && name_matched
                })
                .cloned()
                .collect::<Vec<_>>();

            match matched.len() {
                0 => return Err(Error::PackageNotFound(query.to_owned())),
                1 => {
                    let p = matched.pop().unwrap();

                    if p.is_held() && !escape_hold {
                        // Skipping held package returns nothing to frontend...
                        continue;
                    }

                    if !packages.contains(&p) {
                        packages.push(p);
                    }
                }
                _ => {
                    let is_held = matched.iter().any(|p| p.is_held());

                    if is_held && !escape_hold {
                        continue;
                    }

                    resolve::select_candidate(session, &mut matched)?;
                    let p = matched.pop().unwrap();
                    if !packages.contains(&p) {
                        packages.push(p);
                    }
                }
            }
        }
    };

    if packages.is_empty() {
        return Ok(());
    }

    let transaction = Transaction::default();

    let no_dependencies = options.contains(&SyncOption::NoDependencies);
    if !no_dependencies {
        resolve::resolve_dependencies(session, &mut packages)?;
    }

    let (installed, installable): (Vec<_>, Vec<_>) =
        packages.into_iter().partition(|p| p.is_installed());

    let (upgradable, replaceable): (Vec<_>, Vec<_>) = installed
        .into_iter()
        .partition(|p| p.is_strictly_installed());

    if !only_upgrade && !installable.is_empty() {
        transaction.set_install(installable);
    }

    let upgradable = upgradable
        .into_iter()
        .filter(|p| p.upgradable_version().is_some())
        .collect::<Vec<_>>();

    let no_upgrade = options.contains(&SyncOption::NoUpgrade);
    if !no_upgrade && !upgradable.is_empty() {
        if !escape_hold {
            let (_held, upgradable): (Vec<_>, Vec<_>) =
                upgradable.into_iter().partition(|p| p.is_held());

            if !upgradable.is_empty() {
                transaction.set_upgrade(upgradable);
            }
        } else {
            transaction.set_upgrade(upgradable);
        }
    }

    let no_replace = options.contains(&SyncOption::NoReplace);
    if !no_replace && !replaceable.is_empty() {
        transaction.set_replace(replaceable);
    }

    let reuse_cache = !options.contains(&SyncOption::IgnoreCache);

    let packages = transaction.add_view();
    if packages.is_empty() {
        return Ok(());
    }

    let mut set = download::PackageSet::new(session, &packages, reuse_cache)?;

    let assume_yes = options.contains(&SyncOption::AssumeYes);
    let offline = options.contains(&SyncOption::Offline);
    let mut should_offline = true;

    if !offline {
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageDownloadSizingStart);
        }

        let download_size = set.calculate_download_size()?;
        should_offline = download_size.total == 0;
        transaction.set_download_size(download_size);
    }

    if !assume_yes {
        if let Some(tx) = session.emitter() {
            if tx
                .send(Event::PromptTransactionNeedConfirm(transaction.clone()))
                .is_ok()
            {
                let rx = session.receiver().unwrap();
                let mut confirmed = false;

                while let Ok(event) = rx.recv() {
                    if let Event::PromptTransactionNeedConfirmResult(ret) = event {
                        confirmed = ret;
                        break;
                    }
                }

                if !confirmed {
                    return Ok(());
                }
            }
        }
    }

    if !should_offline {
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageDownloadStart);
        }

        set.download()?;

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageDownloadDone);
        }
    }

    let no_hash_check = options.contains(&SyncOption::NoHashCheck);
    if !no_hash_check {
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageIntegrityCheckStart);
        }

        let config = session.config();
        let cache_root = config.cache_path();

        let mut buf = [0; 1024 * 64];

        for &pkg in packages.iter() {
            if pkg.version() == "nightly" {
                info!("skip hash check for nightly package '{}'", pkg.name());
                continue;
            }

            let files = pkg.download_filenames();
            let hashes = pkg.download_hashes();
            let files_cnt = files.len();

            for (idx, (filename, hash)) in files.into_iter().zip(hashes.into_iter()).enumerate() {
                let path = cache_root.join(filename);

                let mut hasher = ChecksumBuilder::new().algo(hash.algorithm())?.build();

                if let Some(tx) = session.emitter() {
                    let progress = format!("{} ({}/{})", pkg.name(), idx + 1, files_cnt);
                    let _ = tx.send(Event::PackageIntegrityCheckProgress(progress));
                }

                let mut file = std::fs::File::open(path)?;
                loop {
                    let len = file.read(&mut buf)?;
                    if len == 0 {
                        break;
                    }
                    hasher.consume(&buf[..len]);
                }

                let actual = hasher.finalize();
                let expected = hash.value();
                if actual != expected {
                    let name = pkg.name().to_owned();
                    let url = pkg.download_urls()[idx].to_owned();
                    let ctx =
                        super::HashMismatchContext::new(name, url, expected.to_owned(), actual);
                    return Err(Error::HashMismatch(ctx));
                }
            }
        }

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageIntegrityCheckDone);
        }
    }

    let download_only = options.contains(&SyncOption::DownloadOnly);
    if !download_only {
        let config = session.config();
        let apps_dir = config.root_path().join("apps");

        // Log packages requiring PowerShell scripts that will be skipped
        let script_pkgs = packages
            .iter()
            .filter(|p| p.has_install_script())
            .map(|p| p.name().to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        if !script_pkgs.is_empty() {
            info!(
                "skipping packages requiring PowerShell scripts: {}",
                script_pkgs
            );
        }

        let packages_to_commit: Vec<&Package> = packages
            .iter()
            .filter(|p| !p.has_install_script())
            .cloned()
            .collect();

        for pkg in packages_to_commit.iter() {
            if let Some(tx) = session.emitter() {
                let _ = tx.send(Event::PackageCommitStart(pkg.name().to_owned()));
            }

            let working_dir = apps_dir.join(pkg.name()).join(pkg.version());
            internal::fs::ensure_dir(&working_dir)?;

            let filenames = pkg.download_filenames();

            let cache_root = config.cache_path();

            for filename in filenames.iter() {
                let src = cache_root.join(filename);
                if !src.exists() {
                    continue;
                }
                let dst = working_dir.join(filename);
                let _ = std::fs::remove_file(&dst);
                std::fs::copy(&src, &dst)?;
            }

            // Extract archives in the working directory
            extract_package(session, pkg, &working_dir)?;

            // Handle extract_dir/extract_to from manifest
            handle_extract_location(session, pkg, &working_dir)?;

            // Create the 'current' symlink if not using no_junction
            if !config.no_junction() {
                let current_link = apps_dir.join(pkg.name()).join("current");
                internal::fs::remove_symlink(&current_link)?;
                internal::fs::symlink_dir(&working_dir, &current_link)?;
            }

            let install_subdir = if config.no_junction() {
                pkg.version().to_string()
            } else {
                "current".to_string()
            };
            let install_base = apps_dir.join(pkg.name()).join(&install_subdir);

            // Save manifest.json
            let manifest_path = install_base.join("manifest.json");
            let manifest_json = serde_json::to_string_pretty(pkg.manifest().inner())?;
            std::fs::write(&manifest_path, manifest_json)?;

            // Write install.json
            let install_info = InstallInfo::new(
                get_arch_string(pkg),
                Some(pkg.bucket().to_owned()),
                pkg.download_urls().first().map(|u| u.to_string()),
            );
            let install_json_path = install_base.join("install.json");
            internal::fs::write_json(&install_json_path, &install_info)?;

            // Set up persist directories
            persist::link(session, pkg)?;

            // Add shims
            shim::add(session, pkg)?;

            // Add shortcuts
            shortcut::add(session, pkg)?;

            // Import PowerShell modules
            psmodule::add(session, pkg)?;

            // Set environment variables and path
            env::install(session, pkg)?;

            if let Some(tx) = session.emitter() {
                let _ = tx.send(Event::PackageCommitDone(pkg.name().to_owned()));
            }
        }
    }

    Ok(())
}

/// Get the architecture string for the current platform.
fn get_arch_string(pkg: &Package) -> String {
    if let Some(arch) = pkg.manifest().architecture() {
        if cfg!(target_arch = "x86") {
            return if arch.ia32.is_some() {
                "32bit".to_string()
            } else if arch.amd64.is_some() {
                "64bit".to_string()
            } else if arch.aarch64.is_some() {
                "arm64".to_string()
            } else {
                get_runtime_arch()
            };
        }
        if cfg!(target_arch = "x86_64") {
            return if arch.amd64.is_some() {
                "64bit".to_string()
            } else if arch.ia32.is_some() {
                "32bit".to_string()
            } else if arch.aarch64.is_some() {
                "arm64".to_string()
            } else {
                get_runtime_arch()
            };
        }
        if cfg!(target_arch = "aarch64") {
            return if arch.aarch64.is_some() {
                "arm64".to_string()
            } else if arch.amd64.is_some() {
                "64bit".to_string()
            } else if arch.ia32.is_some() {
                "32bit".to_string()
            } else {
                get_runtime_arch()
            };
        }
    }
    get_runtime_arch()
}

/// Determine the runtime architecture string.
fn get_runtime_arch() -> String {
    if cfg!(target_arch = "x86_64") {
        "64bit".to_string()
    } else if cfg!(target_arch = "x86") {
        "32bit".to_string()
    } else if cfg!(target_arch = "aarch64") {
        "arm64".to_string()
    } else {
        "64bit".to_string()
    }
}

/// Extract downloaded archives in the working directory.
fn extract_package(
    session: &Session,
    pkg: &Package,
    working_dir: &std::path::Path,
) -> Fallible<()> {
    let filenames = pkg.download_filenames();
    let config = session.config();
    let cache_root = config.cache_path();

    for filename in filenames.iter() {
        let archive_path = working_dir.join(filename);
        if !archive_path.exists() {
            continue;
        }

        if let Some(format) = internal::archive::Format::detect(&archive_path) {
            if let Some(tx) = session.emitter() {
                let _ = tx.send(Event::PackageCommitStart(format!(
                    "extracting archive for {}",
                    pkg.name()
                )));
            }

            // 7z can handle most formats; for tar.gz/tar.bz2 etc., we need to extract
            // in two steps with 7z (first decompress, then untar)
            let is_tar = filename.to_lowercase().ends_with(".tar");
            let is_compressed_tar = filename.to_lowercase().ends_with(".tar.gz")
                || filename.to_lowercase().ends_with(".tar.bz2")
                || filename.to_lowercase().ends_with(".tgz")
                || filename.to_lowercase().ends_with(".tbz2");

            if is_tar || is_compressed_tar {
                // 7z handles .tar files natively, and .tar.gz etc. automatically
                internal::archive::extract(&archive_path, working_dir)?;
            } else {
                // Extract to a temporary subdirectory first
                let extract_dir = working_dir.join(format!("__extract_{}", filename));
                internal::archive::extract(&archive_path, &extract_dir)?;

                // Move contents from extract_dir to working_dir
                let entries = std::fs::read_dir(&extract_dir)?;
                for entry in entries {
                    let entry = entry?;
                    let name = entry.file_name();
                    let src = entry.path();
                    let dst = working_dir.join(&name);
                    if dst.exists() {
                        let _ = std::fs::remove_file(&dst);
                    }
                    std::fs::rename(&src, &dst)?;
                }
                let _ = internal::fs::remove_dir(&extract_dir);
            }

            // Clean up the archive file after extraction
            let _ = std::fs::remove_file(&archive_path);
        }
    }

    Ok(())
}

/// Handle extract_dir and extract_to manifest fields.
///
/// - `extract_dir`: move extracted content into a subdirectory
/// - `extract_to`: move extracted content to a different directory
fn handle_extract_location(
    session: &Session,
    pkg: &Package,
    working_dir: &std::path::Path,
) -> Fallible<()> {
    let config = session.config();
    let apps_dir = config.root_path().join("apps");

    if let Some(extract_dir) = pkg.manifest().extract_dir() {
        // Move all extracted content into the extract_dir subdirectory
        let target_dir = working_dir.join(extract_dir[0]);
        internal::fs::ensure_dir(&target_dir)?;

        let entries = std::fs::read_dir(working_dir)?;
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name();
            let path = entry.path();
            // Skip the target directory itself
            if path == target_dir {
                continue;
            }
            // Skip extracted archive filenames (cache-style)
            if name.to_string_lossy().contains("#") {
                continue;
            }
            let dest = target_dir.join(&name);
            if dest.exists() {
                let _ = std::fs::remove_file(&dest);
            }
            std::fs::rename(&path, &dest)?;
        }
    }

    // extract_to is applied after extraction, moving content to a subdirectory
    // within the app's directory (not the version dir)
    if let Some(extract_to) = pkg.manifest().extract_to() {
        let target_dir = apps_dir
            .join(pkg.name())
            .join(pkg.version())
            .join(extract_to[0]);
        internal::fs::ensure_dir(&target_dir)?;

        let entries = std::fs::read_dir(working_dir)?;
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name();
            let path = entry.path();
            if path == target_dir {
                continue;
            }
            let dest = target_dir.join(&name);
            if dest.exists() {
                let _ = std::fs::remove_file(&dest);
            }
            std::fs::rename(&path, &dest)?;
        }
    }

    Ok(())
}

/// Sync operation: remove packages.
pub fn remove(session: &Session, queries: &[&str], options: &[SyncOption]) -> Fallible<()> {
    let mut packages = vec![];

    let installed = query::query_installed(session, &["*"], &[])?;
    let escape_hold = options.contains(&SyncOption::EscapeHold);

    for &name in queries {
        let mut matched = installed
            .iter()
            .filter(|&p| p.name() == name)
            .cloned()
            .collect::<Vec<_>>();

        if matched.is_empty() {
            return Err(Error::PackageNotFound(name.to_string()));
        }

        // It's impossible to have more than one installed packages for the same
        // package name.
        assert_eq!(matched.len(), 1);

        let pkg = matched.pop().unwrap();

        if pkg.is_held() && !escape_hold {
            continue;
        }

        packages.push(pkg);
    }

    let no_dependent_check = options.contains(&SyncOption::NoDependentCheck);
    if !no_dependent_check {
        let mut dependents = vec![];

        for pkg in packages.iter() {
            let mut result = installed
                .iter()
                .filter_map(|p| {
                    if packages.contains(p) {
                        return None;
                    }

                    let dep_names = p
                        .dependencies()
                        .into_iter()
                        .map(super::extract_name)
                        .collect::<Vec<_>>();

                    if dep_names.contains(&pkg.name().to_owned()) {
                        // p depends on pkg
                        Some((p.name().to_owned(), pkg.name().to_owned()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if result.is_empty() {
                continue;
            }

            dependents.append(&mut result);
        }

        if !dependents.is_empty() {
            return Err(Error::PackageDependentFound(dependents));
        }
    }

    let is_cascade = options.contains(&SyncOption::Cascade);
    if is_cascade {
        resolve::resolve_cascade(session, &mut packages, escape_hold)?;
    }

    if let Some(tx) = session.emitter() {
        let _ = tx.send(Event::PackageResolveDone);
    }

    let transaction = Transaction::default();

    // TODO: PowerShell hosting with execution context is not supported yet.
    // Perhaps at present we could call Scoop to do the removal for packages
    // using PS scripts...
    let (packages_with_script, _packages): (Vec<_>, Vec<_>) =
        packages.iter().partition(|p| p.has_uninstall_script());

    // TODO: support removal of packages with PowerShell script
    if !packages_with_script.is_empty() {
        let msg = format!("Found package(s) using PowerShell script:\n  {}\nRemoval of package with PowerShell script is not yet supported.",
        packages_with_script.iter().map(|p| p.name()).collect::<Vec<_>>().join("  "));
        return Err(Error::Custom(msg));
    }

    transaction.set_remove(packages);

    let assume_yes = options.contains(&SyncOption::AssumeYes);
    if !assume_yes {
        if let Some(tx) = session.emitter() {
            if tx
                .send(Event::PromptTransactionNeedConfirm(transaction.clone()))
                .is_ok()
            {
                let rx = session.receiver().unwrap();
                let mut confirmed = false;

                while let Ok(event) = rx.recv() {
                    if let Event::PromptTransactionNeedConfirmResult(ret) = event {
                        confirmed = ret;
                        break;
                    }
                }

                if !confirmed {
                    return Ok(());
                }
            }
        }
    }

    if let Some(packages) = transaction.remove_view() {
        let purge = options.contains(&SyncOption::Purge);
        let config = session.config();
        let root_dir = config.root_path();

        for package in packages.iter() {
            if let Some(tx) = session.emitter() {
                let _ = tx.send(Event::PackageCommitStart(package.name().to_owned()));
            }

            let app_dir = root_dir.join("apps").join(package.name());

            // TODO: pre_uninstall
            // TODO: uninstaller

            shim::remove(session, package)?;
            shortcut::remove(session, package)?;
            psmodule::remove(session, package)?;
            env::remove(session, package)?;
            persist::unlink(session, package)?;

            let current_lnk = app_dir.join("current");
            internal::fs::remove_symlink(current_lnk)?;

            // TODO: post_uninstall

            // Remove the app directory
            internal::fs::remove_dir(app_dir)?;

            if purge {
                if let Some(tx) = session.emitter() {
                    let _ = tx.send(Event::PackagePersistPurgeStart);
                }

                let persist_dir = config.root_path().join("persist").join(package.name());
                internal::fs::remove_dir(persist_dir)?;

                if let Some(tx) = session.emitter() {
                    let _ = tx.send(Event::PackagePersistPurgeDone);
                }
            }

            if let Some(tx) = session.emitter() {
                let _ = tx.send(Event::PackageCommitDone(package.name().to_owned()));
            }
        }
    }

    Ok(())
}
