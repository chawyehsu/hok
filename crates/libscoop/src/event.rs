use flume::{bounded, Receiver, Sender};

use crate::{
    bucket::BucketUpdateProgressContext,
    constant::EVENT_BUS_CAPACITY,
    package::{download::PackageDownloadProgressContext, sync::Transaction},
};

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

/// Event that may be emitted during the execution of operations.
#[derive(Clone)]
#[non_exhaustive]
pub enum Event {
    /// Bucket update has made some progress.
    BucketUpdateProgress(BucketUpdateProgressContext),

    /// Bucket update is finished.
    BucketUpdateDone,

    /// Package resolving is started.
    PackageResolveStart,

    /// Package resolving is finished.
    PackageResolveDone,

    /// Calculating download size is started.
    PackageDownloadSizingStart,

    /// Calculating download size is finished.
    PackageDownloadSizingDone,

    /// Package download is started.
    PackageDownloadStart,

    /// Package download has made some progress.
    PackageDownloadProgress(PackageDownloadProgressContext),

    /// Package download is finished.
    PackageDownloadDone,

    /// Package integrity check is started.
    PackageIntegrityCheckStart,

    /// Package sync operation is finished.
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
