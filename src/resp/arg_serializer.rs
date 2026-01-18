use crate::resp::ArgLayout;
use bytes::{BufMut, BytesMut};
use dtoa::Float;
use itoa::Integer;
use serde::{Serializer, ser};
use smallvec::SmallVec;

pub struct ArgSerializer<'a> {
    buffer: &'a mut BytesMut,
    args_layout: Option<&'a mut SmallVec<[ArgLayout; 10]>>,
}

impl<'a> ArgSerializer<'a> {
    #[inline]
    pub(crate) fn new(
        buffer: &'a mut BytesMut,
        args_layout: &'a mut SmallVec<[ArgLayout; 10]>,
    ) -> Self {
        Self {
            buffer,
            args_layout: Some(args_layout),
        }
    }

    #[inline]
    pub fn from_buffer(buffer: &'a mut BytesMut) -> Self {
        Self {
            buffer,
            args_layout: None,
        }
    }

    #[inline]
    fn serialize_integer<I: Integer>(&mut self, i: I) {
        let mut buf = itoa::Buffer::new();
        self.write_arg(buf.format(i).as_bytes());
    }

    #[inline]
    fn serialize_float<F: Float>(&mut self, f: F) {
        let mut buf = dtoa::Buffer::new();
        self.write_arg(buf.format(f).as_bytes());
    }

    /// Serializes a raw argument into the buffer using RESP format (BulkString).
    ///
    /// # Format
    /// `$Length\r\n Data\r\n`
    #[inline]
    pub fn write_arg(&mut self, data: &[u8]) {
        // 1. Write the RESP BulkString header ($Len\r\n)
        let data_len = data.len();
        let mut len_buf = itoa::Buffer::new();
        let len_str = len_buf.format(data_len);
        let len_bytes = len_str.as_bytes();
        let total_size = 1 + len_bytes.len() + 2 + data_len + 2;
        self.buffer.reserve(total_size);
        self.buffer.put_u8(b'$');
        self.buffer.put_slice(len_bytes);
        self.buffer.put_slice(b"\r\n");

        // 2. Capture the absolute position of the data for the index
        let start_pos = self.buffer.len();

        // 3. Write the actual data
        self.buffer.put_slice(data);
        self.buffer.put_slice(b"\r\n");

        // 4. Update the layout index
        if let Some(ref mut layout) = self.args_layout {
            layout.push(ArgLayout {
                start: start_pos as u64,
                len: data_len as u32,
                slot: 0,
                flags: 0,
            });
        }
    }
}

impl<'a> Serializer for &mut ArgSerializer<'a> {
    type Ok = ();
    type Error = crate::Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        const BOOL_VALS: [&[u8]; 2] = [b"0", b"1"];
        self.write_arg(BOOL_VALS[v as usize]);
        Ok(())
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_float(v);
        Ok(())
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.serialize_float(v);
        Ok(())
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        let str = v.encode_utf8(&mut buf);
        self.write_arg(str.as_bytes());
        Ok(())
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.write_arg(v.as_bytes());
        Ok(())
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.write_arg(v);
        Ok(())
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        // No-Op
        Ok(())
    }

    #[inline]
    fn serialize_some<T: ?Sized + ser::Serialize>(
        self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.serialize_str(variant)?;
        value.serialize(self)?;
        Ok(())
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }

    #[inline]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(self)
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(self)
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.serialize_str(variant)?;
        Ok(self)
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(self)
    }

    #[inline]
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(self)
    }
}

// Flattening
impl<'a> ser::SerializeSeq for &mut ArgSerializer<'a> {
    type Ok = ();
    type Error = crate::Error;

    #[inline]
    fn serialize_element<T: ?Sized + ser::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &mut ArgSerializer<'a> {
    type Ok = ();
    type Error = crate::Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &mut ArgSerializer<'a> {
    type Ok = ();
    type Error = crate::Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &mut ArgSerializer<'a> {
    type Ok = ();
    type Error = crate::Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &mut ArgSerializer<'a> {
    type Ok = ();
    type Error = crate::Error;

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        key.serialize(&mut **self)
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &mut ArgSerializer<'a> {
    type Ok = ();
    type Error = crate::Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        // by renaming a field to "", it's possible to serialize only its value
        if !key.is_empty() {
            self.serialize_str(key)?;
        }
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &mut ArgSerializer<'a> {
    type Ok = ();
    type Error = crate::Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        // by renaming a field to "", it's possible to serialize only its value
        if !key.is_empty() {
            self.serialize_str(key)?;
        }
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
