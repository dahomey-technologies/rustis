use crate::resp::{ClientReplyMode, CommandKind};

/// The client's picture of `CLIENT REPLY` on one connection.
///
/// The server answers every command it is sent, except when `CLIENT REPLY`
/// tells it not to. The read loop matches replies to the commands that await
/// them by position, so a reply the server never sends must never be awaited:
/// one miscount here shifts every later reply onto the wrong caller.
///
/// Two independent switches decide it:
///
/// * `ON` / `OFF` hold until the next `CLIENT REPLY`. `OFF` silences the whole
///   connection, its own reply included.
/// * `SKIP` silences exactly one command, the one that follows it, and leaves
///   the connection as it found it. `SKIP` is not answered either.
///
/// [`Self::admit`] is the only place both are read: it applies what a command
/// does to the mode and returns whether that same command is answered, so the
/// pending `SKIP` cannot be consumed twice or left behind.
pub(crate) struct ReplyMode {
    on: bool,
    /// A `SKIP` waiting for the command it silences.
    skip_next: bool,
}

impl ReplyMode {
    /// A connection starts answering.
    pub(crate) fn new() -> Self {
        Self {
            on: true,
            skip_next: false,
        }
    }

    /// Applies `kind` to the mode and reports whether the command it names is
    /// answered by the server.
    pub(crate) fn admit(&mut self, kind: CommandKind) -> bool {
        match kind {
            CommandKind::ClientReply(ClientReplyMode::On) | CommandKind::Reset => {
                self.on = true;
                self.skip_next = false;
            }
            CommandKind::ClientReply(ClientReplyMode::Off) => {
                self.on = false;
                self.skip_next = false;
            }
            CommandKind::ClientReply(ClientReplyMode::Skip) => {
                self.skip_next = true;
                // `SKIP` is not answered, and it must not consume itself.
                return false;
            }
            _ => (),
        }

        if !self.on {
            return false;
        }

        if self.skip_next {
            self.skip_next = false;
            return false;
        }

        true
    }

    /// Adopts the reply mode a remade connection was restored to.
    ///
    /// A `SKIP` waiting for the command it silences died with the old socket.
    pub(crate) fn restore(&mut self, on: bool) {
        self.on = on;
        self.skip_next = false;
    }

    /// Drops a pending `SKIP` without touching `ON` / `OFF`.
    pub(crate) fn forget_pending_skip(&mut self) {
        self.skip_next = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resp::StateSlot;

    const PLAIN: CommandKind = CommandKind::Other;
    const ON: CommandKind = CommandKind::ClientReply(ClientReplyMode::On);
    const OFF: CommandKind = CommandKind::ClientReply(ClientReplyMode::Off);
    const SKIP: CommandKind = CommandKind::ClientReply(ClientReplyMode::Skip);

    #[test]
    fn a_new_connection_answers() {
        let mut mode = ReplyMode::new();
        assert!(mode.admit(PLAIN));
        assert!(mode.admit(PLAIN));
    }

    #[test]
    fn off_silences_itself_and_everything_after_it() {
        let mut mode = ReplyMode::new();
        assert!(!mode.admit(OFF));
        assert!(!mode.admit(PLAIN));
        assert!(!mode.admit(PLAIN));
    }

    #[test]
    fn on_is_answered_and_reopens_the_connection() {
        let mut mode = ReplyMode::new();
        mode.admit(OFF);
        assert!(mode.admit(ON));
        assert!(mode.admit(PLAIN));
    }

    #[test]
    fn skip_silences_itself_and_the_next_command_only() {
        let mut mode = ReplyMode::new();
        assert!(!mode.admit(SKIP));
        assert!(!mode.admit(PLAIN));
        assert!(mode.admit(PLAIN));
    }

    #[test]
    fn two_skips_in_a_row_silence_one_command_each() {
        let mut mode = ReplyMode::new();
        assert!(!mode.admit(SKIP));
        assert!(!mode.admit(PLAIN));
        assert!(!mode.admit(SKIP));
        assert!(!mode.admit(PLAIN));
        assert!(mode.admit(PLAIN));
    }

    #[test]
    fn a_pending_skip_does_not_outlive_off_and_on() {
        let mut mode = ReplyMode::new();
        mode.admit(SKIP);
        mode.admit(OFF);
        assert!(mode.admit(ON));
        assert!(mode.admit(PLAIN));
    }

    #[test]
    fn reset_answers_and_restores_the_default_mode() {
        let mut mode = ReplyMode::new();
        mode.admit(OFF);
        mode.admit(SKIP);
        assert!(mode.admit(CommandKind::Reset));
        assert!(mode.admit(PLAIN));
    }

    #[test]
    fn connection_state_commands_are_answered_like_any_other() {
        let mut mode = ReplyMode::new();
        assert!(mode.admit(CommandKind::ConnectionState(StateSlot::Select)));
    }

    #[test]
    fn a_remade_connection_adopts_its_restored_mode() {
        let mut mode = ReplyMode::new();
        mode.admit(SKIP);
        mode.restore(false);
        assert!(!mode.admit(PLAIN));

        mode.restore(true);
        assert!(mode.admit(PLAIN));
    }

    #[test]
    fn forgetting_a_pending_skip_leaves_off_alone() {
        let mut mode = ReplyMode::new();
        mode.admit(OFF);
        mode.admit(SKIP);
        mode.forget_pending_skip();
        assert!(!mode.admit(PLAIN));
    }
}
