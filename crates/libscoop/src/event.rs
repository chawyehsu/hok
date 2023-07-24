use flume::{bounded, Receiver, Sender};

use crate::{bucket::Bucket, constant::EVENT_BUS_CAPACITY};

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
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    BucketAddStarted(String),
    BucketAddFailed(String),
    BucketAddFinished(String),
    BucketListItem(Bucket),
    BucketUpdateStarted(String),
    BucketUpdateFailed(BucketUpdateFailedCtx),
    BucketUpdateSuccessed(String),
    BucketUpdateFinished,
    PackageResolveStart,
    PackageDownloadSizingStart,
    SelectPackage(Vec<String>),
    SelectPackageAnswer(usize),
    SessionTerminated,
}

#[derive(Debug, Clone)]
pub struct BucketUpdateFailedCtx {
    pub name: String,
    pub err_msg: String,
}
