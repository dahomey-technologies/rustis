use crate::{
    resp::{Command, CommandArgs},
    Error, Result,
};
use bytes::{BufMut, BytesMut};
use tokio_util::codec::Encoder;

pub(crate) struct CommandEncoder;

impl Encoder<&Command> for CommandEncoder {
    type Error = Error;

    #[inline]
    fn encode(&mut self, command: &Command, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(b'*');
        encode_integer(command.args.len() as i64 + 1, buf);
        encode_crlf(buf);
        encode_bulkstring(command.name.as_bytes(), buf);
        encode_command_args(&command.args, buf);
        Ok(())
    }
}

#[inline]
fn encode_bulkstring(arg: &[u8], buf: &mut BytesMut) {
    buf.put_u8(b'$');
    encode_integer(arg.len() as i64, buf);
    encode_crlf(buf);
    buf.put(arg);
    encode_crlf(buf);
}

#[inline]
fn encode_command_args(args: &CommandArgs, buf: &mut BytesMut) {
    for arg in args {
        encode_bulkstring(arg, buf);
    }
}

#[inline]
fn encode_integer(i: i64, buf: &mut BytesMut) {
    let mut buffer = itoa::Buffer::new();
    let str = buffer.format(i);
    buf.put(str.as_bytes());
}

#[inline]
fn encode_crlf(buf: &mut BytesMut) {
    buf.put(&b"\r\n"[..]);
}
