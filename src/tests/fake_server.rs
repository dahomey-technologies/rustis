//! Minimal RESP3 responder, driven over any byte stream.
//!
//! It answers commands from a lookup table keyed on the command name, which is
//! enough to bring a real [`Client`](crate::client::Client) up: the handshake is
//! served like any other command, and a test only spells out the replies it
//! cares about. Unknown commands get an error reply, so a test that forgot one
//! fails on that error rather than hanging.
//!
//! Replies are written as raw bytes: the crate encodes requests only
//! ([`CommandEncoder`](crate::resp::CommandEncoder)), and has nothing that turns
//! a value into a server reply.

use std::collections::HashMap;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// A RESP3 `HELLO` reply, the one every connection reads before anything else.
pub(crate) const HELLO_REPLY: &[u8] = b"%7\r\n\
$6\r\nserver\r\n$5\r\nredis\r\n\
$7\r\nversion\r\n$5\r\n7.4.0\r\n\
$5\r\nproto\r\n:3\r\n\
$2\r\nid\r\n:1\r\n\
$4\r\nmode\r\n$10\r\nstandalone\r\n\
$4\r\nrole\r\n$6\r\nmaster\r\n\
$7\r\nmodules\r\n*0\r\n";

/// A scripted set of replies, one per command name.
#[derive(Clone, Default)]
pub(crate) struct FakeServer {
    replies: HashMap<String, Vec<u8>>,
}

impl FakeServer {
    /// A server that answers the handshake and nothing else.
    pub(crate) fn new() -> Self {
        let mut replies = HashMap::new();
        replies.insert("HELLO".to_owned(), HELLO_REPLY.to_vec());
        Self { replies }
    }

    /// Answers `command` — matched case-insensitively, as Redis does — with the
    /// raw RESP bytes `reply`.
    pub(crate) fn reply(mut self, command: &str, reply: &[u8]) -> Self {
        self.replies
            .insert(command.to_ascii_uppercase(), reply.to_vec());
        self
    }

    /// Reads requests off `stream` and writes the scripted reply to each, until
    /// the peer closes the stream.
    pub(crate) async fn serve<S>(&self, mut stream: S)
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 1024];

        loop {
            let n = match stream.read(&mut chunk).await {
                Ok(0) | Err(_) => return,
                Ok(n) => n,
            };
            buf.extend_from_slice(&chunk[..n]);

            // A read can carry several pipelined requests, or half of one; only
            // whole requests are answered, and the remainder waits for the next
            // read.
            let mut cursor = 0;
            while let Some((name, next)) = parse_request(&buf[cursor..]) {
                cursor += next;
                let reply =
                    self.replies.get(&name).cloned().unwrap_or_else(|| {
                        format!("-ERR unknown command '{name}'\r\n").into_bytes()
                    });
                if stream.write_all(&reply).await.is_err() {
                    return;
                }
            }
            buf.drain(..cursor);
        }
    }
}

/// Parses one request — an array of bulk strings, which is the only shape a
/// client sends — returning its command name uppercased and its length in
/// bytes. `None` while the request is still incomplete.
fn parse_request(buf: &[u8]) -> Option<(String, usize)> {
    let (count, mut pos) = parse_prefixed_len(buf, b'*')?;

    let mut name = None;
    for _ in 0..count {
        let (len, next) = parse_prefixed_len(&buf[pos..], b'$')?;
        pos += next;
        let end = pos.checked_add(len)?;
        // The argument and its trailing CRLF must both be there.
        if buf.len() < end + 2 {
            return None;
        }
        if name.is_none() {
            name = Some(String::from_utf8_lossy(&buf[pos..end]).to_ascii_uppercase());
        }
        pos = end + 2;
    }

    Some((name?, pos))
}

/// Parses a `<tag><len>\r\n` header, returning the length and the header size.
fn parse_prefixed_len(buf: &[u8], tag: u8) -> Option<(usize, usize)> {
    if buf.first() != Some(&tag) {
        return None;
    }
    let crlf = buf.windows(2).position(|w| w == b"\r\n")?;
    let len = std::str::from_utf8(&buf[1..crlf]).ok()?.parse().ok()?;
    Some((len, crlf + 2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_is_parsed_into_its_command_name() {
        let (name, len) = parse_request(b"*2\r\n$3\r\nget\r\n$1\r\nk\r\n").unwrap();
        assert_eq!("GET", name);
        assert_eq!(20, len);
    }

    #[test]
    fn an_incomplete_request_is_not_parsed() {
        assert!(parse_request(b"*2\r\n$3\r\nget\r\n$1\r\n").is_none());
        assert!(parse_request(b"*2\r\n$3\r\nge").is_none());
        assert!(parse_request(b"*2\r\n").is_none());
    }

    #[tokio::test]
    async fn the_handshake_and_a_scripted_command_are_answered_in_order() {
        let (client_side, server_side) = tokio::io::duplex(1024);
        let server = FakeServer::new().reply("PING", b"+PONG\r\n");
        tokio::spawn(async move { server.serve(server_side).await });

        let mut client_side = client_side;
        client_side
            .write_all(b"*2\r\n$5\r\nhello\r\n$1\r\n3\r\n*1\r\n$4\r\nping\r\n")
            .await
            .unwrap();

        let mut out = vec![0u8; HELLO_REPLY.len() + 7];
        client_side.read_exact(&mut out).await.unwrap();
        assert_eq!(&out[..HELLO_REPLY.len()], HELLO_REPLY);
        assert_eq!(&out[HELLO_REPLY.len()..], b"+PONG\r\n");
    }
}
