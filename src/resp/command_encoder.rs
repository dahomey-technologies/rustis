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
        buf.reserve(calculate_buf_size(command));

        buf.put_u8(b'*');
        encode_integer(command.args.len() as i64 + 1, buf);
        encode_crlf(buf);
        encode_bulkstring(command.name.as_bytes(), buf);
        encode_command_args(&command.args, buf);
        Ok(())
    }
}

#[inline]
fn calculate_buf_size(command: &Command) -> usize {
    let mut buf_size = 0;

    // *<num_args>\r\n 
    let num_args = command.args.len() + 1;
    buf_size += if num_args <= 9 { 4 } else { 5 };

    // $<name_len>\r\n<name>\r\n
    let name = command.name.as_bytes();
    buf_size += if name.len() <= 9 { 6 + name.len() } else { 7 + name.len() };

    for arg in &command.args {
        // $<arg_len>\r\n<arg>\r\n
        buf_size += if arg.len() <= 9 { 6 + arg.len() } else { 7 + arg.len() };
    }

    buf_size
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
