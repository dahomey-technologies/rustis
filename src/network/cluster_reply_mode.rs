//! The client's picture of `CLIENT REPLY` across the nodes of a cluster.
//!
//! The server answers every command it is sent, unless `CLIENT REPLY` tells it
//! not to. A reply the server never sends must never be awaited: the connection
//! reports its requests in order, so one unanswerable request at the head stalls
//! every caller behind it.
//!
//! `ON` / `OFF` is sent to every node, so one flag describes the whole cluster
//! connection. `SKIP` is not: it silences whatever the node it reaches receives
//! next, so it is only correct on the nodes the next command reaches — one for a
//! key-routed command, several for a multi-shard one. It is therefore held here
//! until that command is routed, and travels with it.
//!
//! Sending it on its own would leave whichever node received it swallowing the
//! reply of some unrelated later command, which is the same miscount seen from
//! the other side.

use crate::resp::{ClientReplyMode, Command, CommandKind};

pub(super) struct ClusterReplyMode {
    /// Whether the nodes are answering, mirroring `CLIENT REPLY ON` / `OFF`.
    on: bool,
    /// A `CLIENT REPLY SKIP` held back until the command it silences is routed.
    pending_skip: Option<Command>,
}

impl ClusterReplyMode {
    /// A cluster connection starts answering.
    pub(super) fn new() -> Self {
        Self {
            on: true,
            pending_skip: None,
        }
    }

    /// Applies `command` to the mode and reports whether it must still be
    /// routed.
    ///
    /// `false` for a `SKIP`, which is held rather than sent. The mode moves
    /// here, before routing, so that [`Self::awaits_a_reply`] describes the
    /// command being routed: `CLIENT REPLY ON` is itself answered and must be
    /// filed, while `OFF` is not answered and must not be.
    pub(super) fn admit(&mut self, command: &Command) -> bool {
        match command.kind() {
            CommandKind::ClientReply(ClientReplyMode::On) => self.on = true,
            CommandKind::ClientReply(ClientReplyMode::Off) => self.on = false,
            CommandKind::ClientReply(ClientReplyMode::Skip) => {
                self.pending_skip = Some(command.clone());
                return false;
            }
            _ => (),
        }

        true
    }

    /// Whether the command currently being routed draws a reply, and so owes an
    /// entry in the request queue.
    ///
    /// While the nodes are silent nothing may be filed: a sub-request waiting
    /// for a reply that will never come sits at the head of the queue forever.
    pub(super) fn awaits_a_reply(&self) -> bool {
        self.on && self.pending_skip.is_none()
    }

    /// The held `SKIP`, to be emitted on each node the command being routed
    /// reaches, right before its own slice of that command.
    pub(super) fn held_skip(&self) -> Option<&Command> {
        self.pending_skip.as_ref()
    }

    /// Lifts the held `SKIP` out of the way of a command sent on the caller's
    /// behalf rather than by them — the lazily-held `MULTI`, which the skip does
    /// not belong to.
    pub(super) fn lift_held_skip(&mut self) -> Option<Command> {
        self.pending_skip.take()
    }

    pub(super) fn restore_held_skip(&mut self, skip: Option<Command>) {
        self.pending_skip = skip;
    }

    /// Drops a held `SKIP` without touching `ON` / `OFF`.
    ///
    /// Called once the command it belonged to has been routed — including when
    /// that routing failed, where it reached no node at all — and on
    /// reconnection, where it belonged to a command that never reached the wire
    /// on the socket that died. Keeping it would silence the first command of
    /// the new connection while that reply is still expected, shifting every
    /// response after it.
    pub(super) fn forget_held_skip(&mut self) {
        self.pending_skip = None;
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::ClusterReplyMode;
    use crate::resp::{Command, cmd};

    fn ping() -> Command {
        cmd("PING").into()
    }

    /// `CLIENT REPLY <mode>`, whose kind the builder derives from the arguments.
    fn client_reply(mode: &'static str) -> Command {
        cmd("CLIENT").arg("REPLY").arg(mode).into()
    }

    fn skip() -> Command {
        client_reply("SKIP")
    }

    /// An ordinary command is routed and answered.
    #[test]
    fn an_ordinary_command_is_routed_and_draws_a_reply() {
        let mut mode = ClusterReplyMode::new();

        assert!(mode.admit(&ping()));
        assert!(mode.awaits_a_reply());
    }

    /// `SKIP` is held rather than routed: sent on its own it would silence
    /// whatever the node it reached received next, which is not the command it
    /// was meant for.
    #[test]
    fn a_skip_is_held_back_instead_of_being_routed() {
        let mut mode = ClusterReplyMode::new();

        assert!(!mode.admit(&skip()));
        assert!(mode.held_skip().is_some());
    }

    /// The command that follows a `SKIP` is routed with it and draws no reply,
    /// so nothing may be filed for it.
    #[test]
    fn the_command_a_skip_silences_is_routed_but_not_awaited() {
        let mut mode = ClusterReplyMode::new();
        mode.admit(&skip());

        assert!(mode.admit(&ping()));
        assert!(!mode.awaits_a_reply());
        assert!(mode.held_skip().is_some(), "it travels with that command");

        mode.forget_held_skip();
        assert!(mode.awaits_a_reply(), "it applies to nothing further");
    }

    /// A silent connection awaits nothing, and `CLIENT REPLY OFF` is itself
    /// unanswered — the mode has to move before the command is judged, or its
    /// own reply would be awaited forever.
    #[test]
    fn a_silenced_connection_awaits_nothing_including_the_command_that_silenced_it() {
        let mut mode = ClusterReplyMode::new();

        assert!(mode.admit(&client_reply("OFF")));
        assert!(!mode.awaits_a_reply());

        assert!(mode.admit(&ping()));
        assert!(!mode.awaits_a_reply());
    }

    /// `CLIENT REPLY ON` is answered, so it must be filed — again only because
    /// the mode moved first.
    #[test]
    fn the_command_that_ends_the_silence_is_itself_answered() {
        let mut mode = ClusterReplyMode::new();
        mode.admit(&client_reply("OFF"));

        assert!(mode.admit(&client_reply("ON")));
        assert!(mode.awaits_a_reply());
    }

    /// A held `SKIP` belongs to the caller's command, not to a `MULTI` sent on
    /// their behalf: lifting it out of the way must not lose it.
    #[test]
    fn a_skip_lifted_for_an_injected_command_comes_back() {
        let mut mode = ClusterReplyMode::new();
        mode.admit(&skip());

        let lifted = mode.lift_held_skip();
        assert!(lifted.is_some());
        assert!(
            mode.held_skip().is_none(),
            "the injected command is not silenced"
        );
        assert!(mode.awaits_a_reply());

        mode.restore_held_skip(lifted);
        assert!(mode.held_skip().is_some());
    }

    /// A `SKIP` dropped on reconnection must not silence the first command of
    /// the new connection, whose reply is still expected. `ON` / `OFF` is
    /// untouched: it describes the caller's intent, which the new socket is
    /// restored to.
    #[test]
    fn forgetting_a_skip_leaves_the_silence_alone() {
        let mut mode = ClusterReplyMode::new();
        mode.admit(&client_reply("OFF"));
        mode.admit(&skip());

        mode.forget_held_skip();

        assert!(mode.held_skip().is_none());
        assert!(!mode.awaits_a_reply(), "the connection is still silent");
    }
}
