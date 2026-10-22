use flume::{Receiver, Sender, bounded};

use crate::{
    bucket::BucketUpdateProgressContext,
    constant::EVENT_BUS_CAPACITY,
    package::{download::PackageDownloadProgressContext, sync::Transaction},
};

pub use crate::internal::archive::ExtractObserver;

/// Build [`ExtractOptions`][1] for one extraction.
///
/// [`ExtractOptions`][1] is `#[non_exhaustive]`, so callers outside this crate
/// cannot use a struct literal; this is the supported constructor.
///
/// [1]: crate::internal::archive::ExtractOptions
pub fn extract_options<'a>(
    observer: &'a dyn ExtractObserver,
    extract_dir: Option<&'a str>,
    extract_to: Option<&'a str>,
) -> crate::internal::archive::ExtractOptions<'a> {
    crate::internal::archive::ExtractOptions {
        extract_dir,
        extract_to,
        observer,
    }
}

/// Event bus for event transmission.
#[derive(Debug)]
pub struct EventBus {
    // Outbound channel, used to send events out from the session
    inner_tx: Sender<Event>,
    outer_rx: Receiver<Event>,

    // Inbound channel, used to receive events from outside
    outer_tx: Sender<Event>,
    inner_rx: Receiver<Event>,
}

impl EventBus {
    /// Create a new event bus.
    pub fn new() -> EventBus {
        let (inner_tx, outer_rx) = bounded(EVENT_BUS_CAPACITY);
        let (outer_tx, inner_rx) = bounded(EVENT_BUS_CAPACITY);
        Self {
            inner_tx,
            outer_rx,
            outer_tx,
            inner_rx,
        }
    }

    /// Get the sender of the event bus.
    ///
    /// This sender is used to send events into the session.
    pub fn sender(&self) -> Sender<Event> {
        self.outer_tx.clone()
    }

    /// Get the receiver of the event bus.
    ///
    /// This receiver is used to receive events from the session.
    pub fn receiver(&self) -> Receiver<Event> {
        self.outer_rx.clone()
    }

    /// Get the outbound sender of the event bus.
    pub(crate) fn inner_sender(&self) -> Sender<Event> {
        self.inner_tx.clone()
    }

    /// Get the inbound receiver of the event bus.
    pub(crate) fn inner_receiver(&self) -> &Receiver<Event> {
        &self.inner_rx
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Context of an archive being extracted into a package version.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PackageExtractContext {
    /// Package name.
    pub package: String,

    /// Archive label, i.e. the file name of the download.
    pub archive: String,
}

impl PackageExtractContext {
    /// Create a new extraction context.
    pub fn new(package: &str, archive: &str) -> PackageExtractContext {
        PackageExtractContext {
            package: package.to_owned(),
            archive: archive.to_owned(),
        }
    }

    /// Name of the package.
    #[inline]
    pub fn package(&self) -> &str {
        self.package.as_str()
    }

    /// Label of the archive.
    #[inline]
    pub fn archive(&self) -> &str {
        self.archive.as_str()
    }
}

/// Event that may be emitted during the execution of operations.
#[derive(Clone)]
#[non_exhaustive]
pub enum Event {
    /// Bucket update has made some progress.
    BucketUpdateProgress(BucketUpdateProgressContext),

    /// Bucket update has finished.
    BucketUpdateDone,

    /// Package has started to be committed.
    PackageCommitStart(String),

    /// Package has been committed.
    PackageCommitDone(String),

    /// Extraction of one archive of a package has started.
    PackageExtractingStart(PackageExtractContext),

    /// One archive entry has been written.
    PackageExtractingEntry(String),

    /// Extraction of one archive of a package has finished.
    ///
    /// `entries` and `bytes` describe what the extractor actually wrote,
    /// `extractor` is the human-readable extractor name that succeeded.
    PackageExtractingDone {
        /// Package name.
        package: String,
        /// Archive label, i.e. the file name of the download.
        archive: String,
        /// Format of the extracted archive.
        format: String,
        /// Number of archive entries written.
        entries: u64,
        /// Number of bytes written.
        bytes: u64,
        /// Extractor that performed the extraction.
        extractor: String,
        /// Non-fatal problems worth surfacing (skipped entries, etc.).
        warnings: Vec<String>,
    },

    /// Calculating download size has started.
    PackageDownloadSizingStart,

    /// Calculating download size has finished.
    PackageDownloadSizingDone,

    /// Package download has started.
    PackageDownloadStart,

    /// Package download has made some progress.
    PackageDownloadProgress(PackageDownloadProgressContext),

    /// Package download has finished.
    PackageDownloadDone,

    /// Package environment path(s) removal has started.
    PackageEnvPathRemoveStart,

    /// Package environment path(s) removal has finished.
    PackageEnvPathRemoveDone,

    /// Package environment variable(s) removal has started.
    PackageEnvVarRemoveStart,

    /// Package environment variable(s) removal has finished.
    PackageEnvVarRemoveDone,

    /// Package integrity check has started.
    PackageIntegrityCheckStart,

    /// Package integrity check has made some progress.
    PackageIntegrityCheckProgress(String),

    /// Package integrity check has finished.
    PackageIntegrityCheckDone,

    /// Package persist removal has started.
    PackagePersistPurgeStart,

    /// Package persist removal has finished.
    PackagePersistPurgeDone,

    /// Package PowerShell module removal has started.
    PackagePsModuleRemoveStart(String),

    /// Package PowerShell module removal has finished.
    PackagePsModuleRemoveDone,

    /// Package resolving has started.
    PackageResolveStart,

    /// Package resolving has finished.
    PackageResolveDone,

    /// Package shim removal has started.
    PackageShimRemoveStart,

    /// Package shim removal has made some progress.
    PackageShimRemoveProgress(String),

    /// Package shim removal has finished.
    PackageShimRemoveDone,

    /// Package shortcut removal has started.
    PackageShortcutRemoveStart,

    /// Package shortcut removal has made some progress.
    PackageShortcutRemoveProgress(String),

    /// Package shortcut removal has finished.
    PackageShortcutRemoveDone,

    /// Package sync operation has finished.
    PackageSyncDone,

    /// Prompt the user to confirm the transaction.
    PromptTransactionNeedConfirm(Transaction),

    /// Result of [`PromptTransactionNeedConfirm`][1].
    ///
    /// [1]: Event::PromptTransactionNeedConfirm
    PromptTransactionNeedConfirmResult(bool),

    /// Prompt the user to select a package from multiple candidates.
    PromptPackageCandidate(Vec<String>),

    /// Result of [`PromptPackageCandidate`][1].
    ///
    /// [1]: Event::PromptPackageCandidate
    PromptPackageCandidateResult(usize),
}
