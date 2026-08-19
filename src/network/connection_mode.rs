use crate::resp::CommandKind;

/// Where a reply goes, once the mode has been consulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplyRoute {
    /// Nothing awaits it: it belongs to a socket that is already gone.
    Dropped,
    /// Ordinary traffic — a push goes to its sink, anything else to its caller.
    Routed,
    /// A MONITOR line, which belongs to the MONITOR sink.
    MonitorSink,
    /// A reply to a command, handed straight to its caller: a connection that is
    /// monitoring carries no push, so there is nothing to sort out first.
    ToCaller,
}

/// What the connection is currently carrying.
///
/// `MONITOR` turns the connection into a one-way stream: the server keeps
/// sending lines nobody asked for until `RESET` ends it. Neither edge is
/// instantaneous — `MONITOR` is itself answered before the stream starts, and
/// `RESET` is answered after the last line — so entering and leaving each need
/// a state of their own. Reading the reply against the wrong one either hands a
/// monitor line to a caller waiting for its command, or hands a command's reply
/// to the monitor sink.
///
/// The mode is therefore the single authority on that question: [`route_reply`]
/// is the only place the five states are read, and it moves the mode on at the
/// same time as it answers.
///
/// [`route_reply`]: Self::route_reply
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionMode {
    Disconnected,
    Connected,
    /// `MONITOR` is queued or written; its own reply is still to come.
    EnteringMonitor,
    Monitor,
    /// `RESET` is queued or written; monitor lines may still be in flight ahead
    /// of its reply.
    LeavingMonitor,
}

impl ConnectionMode {
    pub(crate) fn is_disconnected(self) -> bool {
        self == Self::Disconnected
    }

    /// Whether the connection was carrying a MONITOR stream, edges included.
    pub(crate) fn is_monitoring(self) -> bool {
        matches!(self, Self::Monitor | Self::EnteringMonitor)
    }

    /// Marks the socket as gone and hands back what the connection was carrying,
    /// which is what a reconnection has to restore.
    pub(crate) fn disconnect(&mut self) -> Self {
        std::mem::replace(self, Self::Disconnected)
    }

    /// A `MONITOR` message is queued: its reply comes before the stream.
    pub(crate) fn enter_monitor(&mut self) {
        *self = Self::EnteringMonitor;
    }

    /// A `RESET` queued while monitoring ends the stream — but only after the
    /// lines already in flight.
    pub(crate) fn observe_queued(&mut self, kind: CommandKind) {
        if *self == Self::Monitor && kind == CommandKind::Reset {
            *self = Self::LeavingMonitor;
        }
    }

    /// Decides where a reply goes, and moves the mode on if that reply is an edge.
    ///
    /// `is_monitor_line` tells a line of the MONITOR stream from anything else.
    pub(crate) fn route_reply(&mut self, is_monitor_line: bool) -> ReplyRoute {
        match *self {
            Self::Disconnected => ReplyRoute::Dropped,
            Self::Connected => ReplyRoute::Routed,
            // The reply to `MONITOR` itself. The stream starts after it.
            Self::EnteringMonitor => {
                *self = Self::Monitor;
                ReplyRoute::ToCaller
            }
            Self::Monitor => {
                if is_monitor_line {
                    ReplyRoute::MonitorSink
                } else {
                    ReplyRoute::ToCaller
                }
            }
            Self::LeavingMonitor => {
                if is_monitor_line {
                    ReplyRoute::MonitorSink
                } else {
                    // The reply to `RESET`, behind the last line of the stream.
                    *self = Self::Connected;
                    ReplyRoute::ToCaller
                }
            }
        }
    }

    /// Adopts what a remade connection carries.
    ///
    /// A stream is only resumed if its sink is still there to receive it; the
    /// caller that asked for it may have dropped in the meantime.
    pub(crate) fn restore(&mut self, was_monitoring: bool, has_monitor_sink: bool) {
        *self = if was_monitoring && has_monitor_sink {
            Self::Monitor
        } else {
            Self::Connected
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: bool = true;
    const REPLY: bool = false;

    #[test]
    fn a_connected_reply_is_routed_normally() {
        let mut mode = ConnectionMode::Connected;
        assert_eq!(ReplyRoute::Routed, mode.route_reply(REPLY));
        assert_eq!(ConnectionMode::Connected, mode);
    }

    #[test]
    fn a_dead_connection_drops_what_it_still_reads() {
        let mut mode = ConnectionMode::Connected;
        assert_eq!(ConnectionMode::Connected, mode.disconnect());
        assert_eq!(ReplyRoute::Dropped, mode.route_reply(LINE));
    }

    #[test]
    fn the_reply_to_monitor_goes_to_its_caller_and_starts_the_stream() {
        let mut mode = ConnectionMode::Connected;
        mode.enter_monitor();

        assert_eq!(ReplyRoute::ToCaller, mode.route_reply(REPLY));
        assert_eq!(ConnectionMode::Monitor, mode);
        assert_eq!(ReplyRoute::MonitorSink, mode.route_reply(LINE));
    }

    #[test]
    fn a_command_answered_while_monitoring_still_reaches_its_caller() {
        let mut mode = ConnectionMode::Monitor;
        assert_eq!(ReplyRoute::ToCaller, mode.route_reply(REPLY));
        assert_eq!(ConnectionMode::Monitor, mode);
    }

    #[test]
    fn reset_ends_the_stream_only_behind_the_lines_already_in_flight() {
        let mut mode = ConnectionMode::Monitor;
        mode.observe_queued(CommandKind::Reset);
        assert_eq!(ConnectionMode::LeavingMonitor, mode);

        // Lines written before `RESET` reached the server still belong to the sink.
        assert_eq!(ReplyRoute::MonitorSink, mode.route_reply(LINE));
        assert_eq!(ConnectionMode::LeavingMonitor, mode);

        // The reply to `RESET` itself closes it.
        assert_eq!(ReplyRoute::ToCaller, mode.route_reply(REPLY));
        assert_eq!(ConnectionMode::Connected, mode);
        assert_eq!(ReplyRoute::Routed, mode.route_reply(REPLY));
    }

    #[test]
    fn reset_outside_the_stream_changes_nothing() {
        let mut mode = ConnectionMode::Connected;
        mode.observe_queued(CommandKind::Reset);
        assert_eq!(ConnectionMode::Connected, mode);

        // `RESET` sent before `MONITOR` is answered does not pre-empt the stream.
        let mut mode = ConnectionMode::EnteringMonitor;
        mode.observe_queued(CommandKind::Reset);
        assert_eq!(ConnectionMode::EnteringMonitor, mode);
    }

    #[test]
    fn an_ordinary_command_never_moves_the_mode() {
        let mut mode = ConnectionMode::Monitor;
        mode.observe_queued(CommandKind::Other);
        assert_eq!(ConnectionMode::Monitor, mode);
    }

    #[test]
    fn a_remade_connection_resumes_a_stream_that_still_has_a_sink() {
        let mut mode = ConnectionMode::Connected;
        let was = mode.disconnect();
        mode.restore(was.is_monitoring(), true);
        assert_eq!(ConnectionMode::Connected, mode);

        let mut mode = ConnectionMode::EnteringMonitor;
        let was = mode.disconnect();
        assert!(was.is_monitoring());
        mode.restore(was.is_monitoring(), true);
        assert_eq!(ConnectionMode::Monitor, mode);
    }

    #[test]
    fn a_stream_whose_sink_is_gone_is_not_resumed() {
        let mut mode = ConnectionMode::Monitor;
        let was = mode.disconnect();
        mode.restore(was.is_monitoring(), false);
        assert_eq!(ConnectionMode::Connected, mode);
    }
}
