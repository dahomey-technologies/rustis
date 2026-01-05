use crate::{Error, Result, resp::NetworkCommand};
use bytes::{BufMut, BytesMut};
use tokio_util::codec::Encoder;

pub(crate) struct CommandEncoder;

impl Encoder<&NetworkCommand> for CommandEncoder {
    type Error = Error;

    #[inline]
    fn encode(&mut self, command: &NetworkCommand, buf: &mut BytesMut) -> Result<()> {
        let bytes = command.get_bytes();
        buf.reserve(bytes.len());
        buf.put(bytes.as_ref());
        Ok(())
    }
}
