use crate::{
    resp::{CommandArg, Command, CommandArgs},
    Error, Result,
};
use bytes::{BufMut, BytesMut};
use tokio_util::codec::Encoder;

pub(crate) struct CommandEncoder;

impl Encoder<&Command> for CommandEncoder {
    type Error = Error;

    fn encode(&mut self, command: &Command, buf: &mut BytesMut) -> Result<()> {
        buf.put_u8(b'*');
        encode_integer(command.args.len() as i64 + 1, buf);
        encode_crlf(buf);
    
        encode_bulkstring(&CommandArg::from(command.name), buf);
        encode_command_args(&command.args, buf);

        Ok(())
    }
}

fn encode_bulkstring(bulk_string: &CommandArg, buf: &mut BytesMut) {
    match bulk_string {
        CommandArg::Nil => buf.put(&b"$-1\r\n"[..]),
        CommandArg::Integer(i) => {
            let mut temp = itoa::Buffer::new();
            let str = temp.format(*i);

            buf.put_u8(b'$');
            encode_integer(str.len() as i64, buf);
            encode_crlf(buf);
            buf.put(str.as_bytes());
            encode_crlf(buf);
        }
        CommandArg::F32(f) => {
            let mut temp = dtoa::Buffer::new();
            let str = temp.format(*f);

            buf.put_u8(b'$');
            encode_integer(str.len() as i64, buf);
            encode_crlf(buf);
            buf.put(str.as_bytes());
            encode_crlf(buf);
        }
        CommandArg::F64(f) => {
            let mut temp = dtoa::Buffer::new();
            let str = temp.format(*f);

            buf.put_u8(b'$');
            encode_integer(str.len() as i64, buf);
            encode_crlf(buf);
            buf.put(str.as_bytes());
            encode_crlf(buf);
        }
        _ => {
            buf.put_u8(b'$');
            encode_integer(bulk_string.len() as i64, buf);
            encode_crlf(buf);
            buf.put(bulk_string.as_bytes());
            encode_crlf(buf);
        }
    }
}

fn encode_command_args(command_args: &CommandArgs, buf: &mut BytesMut) {
    match command_args {
        CommandArgs::Empty => (),
        CommandArgs::Single(arg) => {
            encode_bulkstring(arg, buf);
        }
        CommandArgs::Array2(args) => {
            for arg in args.iter() {
                encode_bulkstring(arg, buf);
            }
        }
        CommandArgs::Array3(args) => {
            for arg in args.iter() {
                encode_bulkstring(arg, buf);
            }
        }
        CommandArgs::Array4(args) => {
            for arg in args.iter() {
                encode_bulkstring(arg, buf);
            }
        }
        CommandArgs::Array5(args) => {
            for arg in args.iter() {
                encode_bulkstring(arg, buf);
            }
        }
        CommandArgs::Vec(args) => {
            for arg in args.iter() {
                encode_bulkstring(arg, buf);
            }
        }
    }
}

fn encode_integer(i: i64, buf: &mut BytesMut) {
    let mut buffer = itoa::Buffer::new();
    let str = buffer.format(i);
    buf.put(str.as_bytes());
}

fn encode_crlf(buf: &mut BytesMut) {
    buf.put(&b"\r\n"[..]);
}
