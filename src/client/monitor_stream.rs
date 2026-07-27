use crate::{
    Result,
    client::{Client, ClientPreparedCommand},
    commands::ConnectionCommands,
    network::PushReceiver,
};
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Deserializer, de};
use std::{
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};
use tracing::warn;

/// Stream to get [`MONITOR`](https://redis.io/commands/monitor/) command events
/// when the stream is dropped or closed, a reset command is sent to the Redis server
pub struct MonitorStream {
    closed: bool,
    receiver: PushReceiver,
    client: Client,
}

impl MonitorStream {
    pub(crate) fn new(receiver: PushReceiver, client: Client) -> Self {
        Self {
            closed: false,
            receiver,
            client,
        }
    }

    pub async fn close(&mut self) -> Result<()> {
        self.client.reset().await?;
        self.closed = true;
        Ok(())
    }
}

impl Stream for MonitorStream {
    type Item = MonitoredCommandInfo;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if self.closed {
            return Poll::Ready(None);
        }

        let this = self.get_mut();

        // An undecodable event must not end the stream: the consumer would stop
        // polling and never see another monitored command, on a feed that is
        // fully server-driven. Skip it and keep reading instead.
        loop {
            let Poll::Ready(event) = this.receiver.poll_next_unpin(cx) else {
                return Poll::Pending;
            };

            let Some(event) = event else {
                return Poll::Ready(None);
            };

            match event {
                Ok(resp_buf) => match resp_buf.to() {
                    Ok(info) => return Poll::Ready(Some(info)),
                    Err(e) => warn!("Cannot decode a monitor event: {e}"),
                },
                Err(e) => warn!("Error while receiving a monitor event: {e}"),
            }
        }
    }
}

impl Drop for MonitorStream {
    fn drop(&mut self) {
        if self.closed {
            return;
        }

        let _result = self.client.reset().forget();
    }
}

/// Result for the [`monitor`](crate::commands::BlockingCommands::monitor) command.
#[derive(Debug)]
#[non_exhaustive]
pub struct MonitoredCommandInfo {
    pub unix_timestamp_millis: f64,
    pub database: usize,
    pub server_addr: SocketAddr,
    pub command: String,
    pub command_args: Vec<String>,
}

impl<'de> Deserialize<'de> for MonitoredCommandInfo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let line = <&str>::deserialize(deserializer)?;
        parse_monitor_line(line).ok_or_else(|| {
            de::Error::custom(format!("Cannot parse result from MONITOR event: {line}"))
        })
    }
}

/// Parses one MONITOR event line, whose Redis format is
/// `<timestamp> [<db> <addr>] "arg0" "arg1" ...`.
///
/// The arguments are C-quoted (Redis `sdscatrepr`): each is wrapped in double
/// quotes and may itself contain spaces and escape sequences. Splitting on spaces
/// and stripping quotes by index — as the original code did — corrupted any
/// space-containing argument and, worse, underflowed `a.len() - 1` on an empty
/// token (e.g. from a double space inside an argument), panicking the consumer's
/// task on input any client of the monitored server can produce with `SET k "a  b"`.
fn parse_monitor_line(line: &str) -> Option<MonitoredCommandInfo> {
    // timestamp: everything up to the first space.
    let (timestamp, rest) = line.split_once(' ')?;
    let unix_timestamp_millis = timestamp.parse::<f64>().ok()?;

    // `[<db> <addr>]`: bracketed, addr contains no spaces.
    let rest = rest.strip_prefix('[')?;
    let (bracket, rest) = rest.split_once(']')?;
    let (database, server_addr) = bracket.split_once(' ')?;
    let database = database.parse::<usize>().ok()?;
    let server_addr = server_addr.parse::<SocketAddr>().ok()?;

    // The remainder is the quoted command followed by its quoted arguments.
    let mut quoted = parse_quoted_args(rest.trim_start())?;
    if quoted.is_empty() {
        return None;
    }
    let command = quoted.remove(0);

    Some(MonitoredCommandInfo {
        unix_timestamp_millis,
        database,
        server_addr,
        command,
        command_args: quoted,
    })
}

/// Splits a run of space-separated, double-quoted, C-escaped tokens into their
/// decoded values. Returns `None` on malformed quoting rather than panicking.
fn parse_quoted_args(mut s: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    loop {
        s = s.trim_start_matches(' ');
        if s.is_empty() {
            return Some(args);
        }
        let after_open = s.strip_prefix('"')?;
        let (value, rest) = decode_quoted(after_open)?;
        args.push(value);
        s = rest;
    }
}

/// Decodes one C-quoted token starting just after its opening quote, returning the
/// decoded string and the remaining input after the closing quote. Non-UTF-8 byte
/// escapes are rendered lossily (this is a debugging feed, not a data path).
fn decode_quoted(s: &str) -> Option<(String, &str)> {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                // Closing quote; return the value and the tail after it.
                let value = String::from_utf8_lossy(&out).into_owned();
                return Some((value, &s[i + 1..]));
            }
            b'\\' => {
                let esc = *bytes.get(i + 1)?;
                match esc {
                    b'"' => out.push(b'"'),
                    b'\\' => out.push(b'\\'),
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'b' => out.push(0x08),
                    b'a' => out.push(0x07),
                    b'x' => {
                        let hi = (*bytes.get(i + 2)? as char).to_digit(16)?;
                        let lo = (*bytes.get(i + 3)? as char).to_digit(16)?;
                        out.push((hi * 16 + lo) as u8);
                        i += 2; // consumed two extra hex digits
                    }
                    other => out.push(other),
                }
                i += 2;
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    // Ran off the end without a closing quote.
    None
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::parse_monitor_line;

    #[test]
    fn plain_command() {
        let info = parse_monitor_line("1339518083.107412 [0 127.0.0.1:60866] \"keys\" \"*\"")
            .expect("should parse");
        assert_eq!(0, info.database);
        assert_eq!("keys", info.command);
        assert_eq!(vec!["*".to_string()], info.command_args);
    }

    #[test]
    fn argument_with_spaces_is_not_split() {
        let info = parse_monitor_line("1.0 [0 127.0.0.1:6379] \"SET\" \"k\" \"a b c\"")
            .expect("should parse");
        assert_eq!("SET", info.command);
        assert_eq!(
            vec!["k".to_string(), "a b c".to_string()],
            info.command_args
        );
    }

    #[test]
    fn double_space_argument_does_not_panic() {
        // `SET k "a  b"` — the two consecutive spaces used to yield an empty token
        // and underflow `a.len() - 1`.
        let info = parse_monitor_line("1.0 [0 127.0.0.1:6379] \"SET\" \"k\" \"a  b\"")
            .expect("should parse");
        assert_eq!(vec!["k".to_string(), "a  b".to_string()], info.command_args);
    }

    #[test]
    fn escapes_are_decoded() {
        let info =
            parse_monitor_line("1.0 [0 127.0.0.1:6379] \"SET\" \"k\" \"a\\tb\\n\\x41\\\"c\"")
                .expect("should parse");
        assert_eq!(
            vec!["k".to_string(), "a\tb\nA\"c".to_string()],
            info.command_args
        );
    }

    #[test]
    fn malformed_returns_none_not_panic() {
        assert!(parse_monitor_line("").is_none());
        assert!(parse_monitor_line("notanumber [0 127.0.0.1:6379] \"PING\"").is_none());
        assert!(parse_monitor_line("1.0 [0 127.0.0.1:6379] \"unterminated").is_none());
    }
}
