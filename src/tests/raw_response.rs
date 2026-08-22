//! What a caller gets when it asks for the reply below serde.

use crate::{
    Error, ErrorKind, Result,
    client::{Client, CommandInterceptor, CustomInterceptor},
    resp::{RespFrameParser, RespResponse, RespTapeMut, cmd},
    tests::{
        fake_server::{FakeServer, duplex_config},
        log_try_init,
    },
};
use bytes::Bytes;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

/// One reply per RESP type, each scripted under the command that returns it.
const REPLIES: &[(&str, &[u8])] = &[
    ("PING", b"+PONG\r\n"),
    ("GET", b"$3\r\nabc\r\n"),
    // A payload holding a CRLF: a reply is bytes, not lines.
    ("GETRANGE", b"$4\r\na\r\nb\r\n"),
    ("INCR", b":42\r\n"),
    // The server's spelling of a float, which the value alone cannot rebuild.
    ("ZSCORE", b",1e+20\r\n"),
    ("SISMEMBER", b"#t\r\n"),
    ("LRANGE", b"*2\r\n$1\r\na\r\n:7\r\n"),
    ("HGETALL", b"%1\r\n$1\r\nf\r\n$1\r\nv\r\n"),
    ("SMEMBERS", b"~1\r\n$1\r\na\r\n"),
    // The two types the crate reads as strings: their tags survive here, where
    // reading the reply loses them.
    ("LOLWUT", b"=15\r\ntxt:Some string\r\n"),
    ("DEBUG", b"(3492890328409238509324850943850943825024385\r\n"),
];

/// A server that answers each command of `REPLIES` with its reply, plus whatever
/// `extra` adds.
fn scripted(extra: &[(&str, &[u8])]) -> FakeServer {
    let mut server = FakeServer::new();
    for (command, reply) in REPLIES.iter().chain(extra) {
        server = server.reply(command, reply);
    }
    server
}

/// Connects a client to `server` over a pipe.
async fn connect(server: FakeServer) -> Result<Client> {
    Client::connect(duplex_config(server, Arc::new(AtomicUsize::new(0)))).await
}

/// Parses a complete RESP reply into a response owning its bytes, as the decoder
/// hands one over.
fn parse(resp: &'static [u8]) -> RespResponse {
    let resp = Bytes::from_static(resp);
    let mut tape = RespTapeMut::default();
    let (frame, _) = RespFrameParser::new(&resp, &mut tape).parse().unwrap();
    RespResponse::new(resp.into(), frame)
}

/// A reply that came off the wire is handed back byte for byte.
///
/// Every RESP type the server can answer with, including the two a decoded value
/// cannot spell back: a float, whose text is the server's, and a bulk string
/// carrying a CRLF of its own.
#[tokio::test]
async fn a_wire_reply_is_handed_back_byte_for_byte() -> Result<()> {
    log_try_init();
    let client = connect(scripted(&[])).await?;

    for (command, reply) in REPLIES {
        let raw = client.send_raw(cmd(command), None).await?;
        assert_eq!(*reply, raw.as_bytes(), "{command}");
        assert!(!raw.is_error(), "{command}");
    }

    Ok(())
}

/// An error reply is a reply: it is handed back with the others, where the typed
/// path raises it.
///
/// A caller forwarding replies has to forward the failures too, and the frame is
/// the only form that carries the server's wording as it stands.
#[tokio::test]
async fn an_error_reply_is_handed_back_rather_than_raised() -> Result<()> {
    log_try_init();
    const REPLY: &[u8] = b"-WRONGTYPE Operation against a key holding the wrong kind of value\r\n";
    let client = connect(scripted(&[("LPUSH", REPLY)])).await?;

    let raw = client.send_raw(cmd("LPUSH"), None).await?;
    assert_eq!(REPLY, raw.as_bytes());
    assert!(raw.is_error());

    let typed: Result<i64> = client.send(cmd("LPUSH"), None).await;
    assert!(matches!(typed.unwrap_err().kind(), ErrorKind::Redis(_)));

    Ok(())
}

/// A null the parser did not keep bytes for is rendered as the RESP3 null.
///
/// `*-1` says a collection is absent and carries nothing else, so the decoder
/// drops its bytes. It is the one wire reply that comes back rewritten.
#[tokio::test]
async fn a_null_collection_is_rendered_as_the_resp3_null() -> Result<()> {
    log_try_init();
    let client = connect(scripted(&[("BLPOP", b"*-1\r\n")])).await?;

    let raw = client.send_raw(cmd("BLPOP"), None).await?;
    assert_eq!(b"_\r\n", raw.as_bytes());

    Ok(())
}

/// A reply the client built itself is rendered, since it has no wire bytes.
///
/// The cluster layer aggregates the sub-replies of a command split over shards,
/// and the client-side cache decodes what it retains. A double keeps the text it
/// arrived with, which is what makes its rendering faithful.
#[test]
fn a_synthesized_reply_is_rendered_as_resp3() -> Result<()> {
    assert_eq!(b"_\r\n", RespResponse::null().to_raw()?.as_bytes());
    assert_eq!(b":42\r\n", RespResponse::integer(42).to_raw()?.as_bytes());
    assert_eq!(
        b",1e+20\r\n",
        RespResponse::Double(1e20, Bytes::from_static(b"1e+20"))
            .to_raw()?
            .as_bytes()
    );
    assert_eq!(
        b"*2\r\n:1\r\n:2\r\n",
        RespResponse::integer_array(vec![1, 2]).to_raw()?.as_bytes()
    );
    // An element that is a whole frame keeps its own bytes, tag included.
    assert_eq!(
        b"*2\r\n=15\r\ntxt:Some string\r\n+OK\r\n",
        RespResponse::owned_array(vec![
            parse(b"=15\r\ntxt:Some string\r\n"),
            parse(b"+OK\r\n")
        ])
        .to_raw()?
        .as_bytes()
    );

    Ok(())
}

/// An element of a rendered reply is rendered too, at every depth.
///
/// An aggregated reply holds elements that are still indexed inside their own
/// frame, whose bytes cannot be sliced back out: an element is written from what
/// it reads as, and a nested collection from its own elements.
#[test]
fn a_rendered_reply_renders_its_elements() -> Result<()> {
    let elements: Vec<RespResponse> =
        parse(b"*3\r\n*1\r\n$1\r\na\r\n%1\r\n$1\r\nk\r\n:1\r\n~0\r\n")
            .into_collection_iter()?
            .collect::<Result<_>>()?;

    assert_eq!(
        b"*3\r\n*1\r\n$1\r\na\r\n%1\r\n$1\r\nk\r\n:1\r\n~0\r\n",
        RespResponse::owned_array(elements).to_raw()?.as_bytes()
    );

    Ok(())
}

/// An error handed back to the caller is still reported failed to the
/// interceptor.
///
/// The command failed on the server, whichever form its caller asked the reply
/// in, so an interceptor counting failures counts the same either way.
#[tokio::test]
async fn an_error_handed_back_is_still_reported_failed() -> Result<()> {
    log_try_init();

    #[derive(Default)]
    struct Recorder {
        failed: AtomicUsize,
    }

    impl CommandInterceptor for Arc<Recorder> {
        fn on_complete(&self, _command_name: &[u8], _elapsed: Duration, error: Option<&Error>) {
            if error.is_some() {
                self.failed.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    let recorder = Arc::new(Recorder::default());
    let mut config = duplex_config(
        scripted(&[("LPUSH", b"-WRONGTYPE against the wrong kind of value\r\n")]),
        Arc::new(AtomicUsize::new(0)),
    );
    config.interceptor = Some(CustomInterceptor::new(Arc::clone(&recorder)));
    let client = Client::connect(config).await?;

    assert!(client.send_raw(cmd("PING"), None).await?.as_bytes() == b"+PONG\r\n");
    assert!(client.send_raw(cmd("LPUSH"), None).await?.is_error());

    assert_eq!(1, recorder.failed.load(Ordering::SeqCst));

    Ok(())
}

/// The bytes are owned, so holding a reply holds no read buffer.
///
/// The network task recycles the block a frame is split from. A raw reply that
/// borrowed from it would pin the block for as long as its holder lived, which
/// is why it is copied out.
#[tokio::test]
async fn a_raw_reply_owns_its_bytes() -> Result<()> {
    log_try_init();
    let client = connect(scripted(&[])).await?;

    let raw = client.send_raw(cmd("GET"), None).await?;
    for _ in 0..8 {
        client.send_raw(cmd("LRANGE"), None).await?;
    }

    assert_eq!(b"$3\r\nabc\r\n", raw.as_bytes());
    assert_eq!(b"$3\r\nabc\r\n".to_vec(), raw.into_vec());

    Ok(())
}
