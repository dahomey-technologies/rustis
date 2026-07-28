use crate::resp::{Command, StateSlot};

/// The connection-attached state a caller has set at runtime, so a new socket can
/// be brought back to the state the old one was in.
///
/// Each slot holds the last command that set it: restoring the state after a
/// reconnection is a replay of what the caller actually issued, so no option type
/// has to be modelled a second time here.
///
/// # Ownership
///
/// The network task owns this outright — it is a plain field of `NetworkHandler`,
/// lent as `&mut` to whichever connection is being (re)built. Nothing here is
/// shared with a client thread, and there is deliberately no `Arc` and no lock:
/// a `Client` clone never reaches this type, so exclusivity is proven by the
/// borrow checker instead of being enforced at runtime and documented in prose.
/// It is `Clone` so a `ClusterConnection` can hold a snapshot for the nodes a
/// topology change brings in mid-flight, which cannot reach the handler's copy from
/// where they are created. The handler is the single writer; the snapshot is a
/// read-only copy refreshed at one named point (`Connection::sync_connection_state`).
#[derive(Debug, Default, Clone)]
pub(crate) struct ConnectionState {
    slots: [Option<Command>; StateSlot::ALL.len()],
}

impl ConnectionState {
    /// Records the command that just set `slot`, replacing whatever set it before.
    pub(crate) fn record(&mut self, slot: StateSlot, command: &Command) {
        if let Some(entry) = self.slots.get_mut(slot.index()) {
            *entry = Some(command.clone());
        }
    }

    /// Forgets everything, as `RESET` does server-side.
    pub(crate) fn clear(&mut self) {
        self.slots = Default::default();
    }

    /// Drops a single slot, used when its replay was rejected by the server:
    /// keeping it would replay a command that fails on every reconnection.
    pub(crate) fn forget(&mut self, slot: StateSlot) {
        if let Some(entry) = self.slots.get_mut(slot.index()) {
            *entry = None;
        }
    }

    /// Whether a slot holds a command, so a caller can skip work the replay is
    /// about to redo.
    pub(crate) fn holds(&self, slot: StateSlot) -> bool {
        self.slots.get(slot.index()).is_some_and(Option::is_some)
    }

    /// The commands to replay, in `StateSlot` declaration order.
    pub(crate) fn commands(&self) -> Vec<(StateSlot, Command)> {
        StateSlot::ALL
            .iter()
            .filter_map(|slot| {
                self.slots
                    .get(slot.index())
                    .and_then(|entry| entry.as_ref())
                    .map(|command| (*slot, command.clone()))
            })
            .collect()
    }

    /// Whether a connection restored from this state answers commands.
    ///
    /// The network handler mirrors the reply mode to know how many responses to
    /// expect; deriving that mirror from the same slot the replay uses is what
    /// keeps the two in agreement across a reconnection.
    pub(crate) fn is_reply_on(&self) -> bool {
        self.slots
            .get(StateSlot::ReplyMode.index())
            .and_then(|entry| entry.as_ref())
            .is_none_or(|command| {
                command
                    .get_arg(1)
                    .is_none_or(|mode| mode.eq_ignore_ascii_case(b"ON"))
            })
    }
}
