use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};

/// What a connection is doing, read from a live
/// [`Client`](crate::client::Client) with
/// [`stats`](crate::client::Client::stats).
///
/// A snapshot, not a view: the fields are read one after another, so they
/// describe the connection over the instant the call spans rather than at one
/// point in it. That is enough for a gauge or a health endpoint, and it is why
/// nothing here should be used to decide control flow.
///
/// The two counters are monotone over the life of the client and reset by
/// nothing, so a rate is the difference between two snapshots.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ClientStats {
    /// Commands waiting to be written, plus those written and waiting for their
    /// reply.
    ///
    /// A pipeline counts as its command count, not as one message.
    pub queued_commands: usize,

    /// Bytes those commands hold, the quantity
    /// [`BackpressureConfig::max_queued_bytes`](crate::client::BackpressureConfig::max_queued_bytes)
    /// bounds.
    pub queued_bytes: usize,

    /// The most [`queued_bytes`](Self::queued_bytes) ever reached.
    ///
    /// This is what sizes the budget: a mark that never approaches it means the
    /// budget is not the thing shaping the workload.
    pub queued_bytes_high_water: usize,

    /// Commands refused with
    /// [`ClientError::SendQueueFull`](crate::ClientError::SendQueueFull) since
    /// the client connected.
    ///
    /// The caller is told per command; only this says how often it happens.
    pub shed_commands: u64,

    /// Reconnections completed since the client connected.
    ///
    /// A connection that recovers on its own leaves no other trace: the client
    /// keeps working and the commands in flight are replayed or failed
    /// individually.
    pub reconnections: u64,

    /// Whether the connection is up right now.
    ///
    /// `false` covers both a link that is down and one that is backing off
    /// between attempts; neither is terminal. See
    /// [`is_terminated`](crate::client::Client::is_terminated) for the state
    /// that is.
    pub connected: bool,
}

/// The live counters [`ClientStats`] is a snapshot of.
///
/// One per connection, shared between the network task that writes them and the
/// client handles that read them. Every write is `Relaxed` and comes from the
/// single network task, so the counters cost an uncontended store on paths that
/// already own a buffer and a syscall.
#[derive(Debug, Default)]
pub(crate) struct StatsRecorder {
    queued_commands: AtomicUsize,
    queued_bytes: AtomicUsize,
    queued_bytes_high_water: AtomicUsize,
    shed_commands: AtomicU64,
    reconnections: AtomicU64,
    connected: AtomicBool,
    /// Empty until the handshake reports one, and re-read on every reconnection.
    /// A cluster keeps it empty: its nodes have versions of their own.
    server_version: Mutex<Option<Arc<str>>>,
}

impl StatsRecorder {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Publishes the queue totals the network task keeps as plain fields.
    ///
    /// Called once per loop iteration rather than at each of the ten sites that
    /// move the totals: the fields only change inside the loop body, so a
    /// reader cannot observe the difference and the accounting keeps one owner.
    pub(crate) fn set_queued(&self, commands: usize, bytes: usize) {
        self.queued_commands.store(commands, Ordering::Relaxed);
        self.queued_bytes.store(bytes, Ordering::Relaxed);
        self.queued_bytes_high_water
            .fetch_max(bytes, Ordering::Relaxed);
    }

    pub(crate) fn record_shed(&self) {
        self.shed_commands.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_reconnection(&self) {
        self.reconnections.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn set_connected(&self, connected: bool) {
        self.connected.store(connected, Ordering::Relaxed);
    }

    pub(crate) fn set_server_version(&self, version: Option<Arc<str>>) {
        if let Ok(mut guard) = self.server_version.lock() {
            *guard = version;
        }
    }

    pub(crate) fn server_version(&self) -> Option<Arc<str>> {
        self.server_version.lock().ok()?.clone()
    }

    pub(crate) fn connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub(crate) fn snapshot(&self) -> ClientStats {
        ClientStats {
            queued_commands: self.queued_commands.load(Ordering::Relaxed),
            queued_bytes: self.queued_bytes.load(Ordering::Relaxed),
            queued_bytes_high_water: self.queued_bytes_high_water.load(Ordering::Relaxed),
            shed_commands: self.shed_commands.load(Ordering::Relaxed),
            reconnections: self.reconnections.load(Ordering::Relaxed),
            connected: self.connected.load(Ordering::Relaxed),
        }
    }
}
