use crate::{Error, Result, resp::Command};
use bytes::{BufMut, BytesMut};
use tokio_util::codec::Encoder;

pub(crate) struct CommandEncoder;

impl Encoder<&Command> for CommandEncoder {
    type Error = Error;

    #[inline]
    fn encode(&mut self, command: &Command, buf: &mut BytesMut) -> Result<()> {
        // This per-command `reserve` looks like churn on a large pipeline (1000
        // commands = 1000 reserves), so it is tempting to collapse it into one
        // pre-computed reservation covering the whole pipeline (as redis-rs does).
        // Don't: it was measured and it changes nothing. `FramedWrite` flushes as
        // soon as the write buffer reaches its `backpressure_boundary` (tokio-util's
        // 8 KiB INITIAL_CAPACITY), so the buffer self-caps at ~8 KiB and these
        // reserves are cheap no-ops (capacity already sufficient, no realloc).
        // Pre-reserving the whole pipeline would instead inflate the buffer past
        // 8 KiB, against the streaming model and the write-buffer shrink policy.
        let bytes = command.bytes();
        buf.reserve(bytes.len());
        buf.put(bytes.as_ref());
        Ok(())
    }
}
