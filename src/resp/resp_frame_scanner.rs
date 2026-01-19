use crate::{
    ClientError, Error, Result,
    resp::{
        ARRAY_TAG, BLOB_ERROR_TAG, BOOL_TAG, BULK_STRING_TAG, DOUBLE_TAG, ERROR_TAG, INTEGER_TAG,
        MAP_TAG, NIL_TAG, PUSH_TAG, SET_TAG, SIMPLE_STRING_TAG, VERBATIM_STRING_TAG,
    },
};
use memchr::memchr;

pub struct RespFrameScanner<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> RespFrameScanner<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    #[inline(always)]
    pub fn scan(&mut self) -> Result<usize> {
        let start_pos = self.pos;
        self.scan_value()?;
        Ok(self.pos - start_pos)
    }

    #[inline]
    fn scan_crlf(&mut self) -> Result<()> {
        let rem = &self.buf[self.pos..];
        let i = memchr(b'\r', rem).ok_or(Error::EOF)?;
        if i + 1 >= rem.len() || rem[i + 1] != b'\n' {
            return Err(Error::EOF);
        }
        self.pos += i + 2;
        Ok(())
    }

    #[inline]
    fn scan_len(&mut self) -> Result<isize> {
        let start = self.pos;
        self.scan_crlf()?;
        atoi::atoi(&self.buf[start..(self.pos - 2)])
            .ok_or_else(|| Error::Client(ClientError::CannotParseNumber))
    }

    fn scan_value(&mut self) -> Result<()> {
        if self.pos >= self.buf.len() {
            return Err(Error::EOF);
        }

        let tag = self.buf[self.pos];
        self.pos += 1;

        match tag {
            SIMPLE_STRING_TAG | ERROR_TAG | INTEGER_TAG | DOUBLE_TAG | NIL_TAG | BOOL_TAG => {
                self.scan_crlf()
            }

            BULK_STRING_TAG | BLOB_ERROR_TAG | VERBATIM_STRING_TAG => {
                let len = self.scan_len()?;
                if len == -1 {
                    // Null bulk string
                    return Ok(());
                }
                if len < 0 {
                    return Err(Error::Client(ClientError::CannotParseBulkString));
                }
                let need = self.pos + len as usize + 2;
                if self.buf.len() < need {
                    return Err(Error::EOF);
                }
                if &self.buf[self.pos + len as usize..need] != b"\r\n" {
                    return Err(Error::Client(ClientError::CannotParseBulkString));
                }
                self.pos = need;
                Ok(())
            }
            ARRAY_TAG | SET_TAG | PUSH_TAG => {
                let len = self.scan_len()? as usize;
                for _ in 0..len {
                    self.scan_value()?;
                }
                Ok(())
            }
            MAP_TAG => {
                let len = self.scan_len()? as usize * 2;
                for _ in 0..len {
                    self.scan_value()?;
                }
                Ok(())
            }

            tag => Err(Error::Client(ClientError::UnknownRespTag(tag as char))),
        }
    }
}
