use crate::{Error, Result, resp::Command};
use bytes::{BufMut, BytesMut};
use tokio_util::codec::Encoder;

pub(crate) struct CommandEncoder;

impl Encoder<&Command> for CommandEncoder {
    type Error = Error;

    #[inline]
    fn encode(&mut self, command: &Command, buf: &mut BytesMut) -> Result<()> {
        let bytes = command.bytes();
        buf.reserve(bytes.len());
        buf.put(bytes.as_ref());
        Ok(())
    }
}
