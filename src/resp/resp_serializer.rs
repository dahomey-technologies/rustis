use crate::{
    resp::{
        ARRAY_TAG, BULK_STRING_TAG, DOUBLE_TAG, ERROR_TAG, INTEGER_TAG, MAP_TAG, PUSH_FAKE_FIELD,
        PUSH_TAG, SET_TAG, SIMPLE_STRING_TAG,
    },
    Error,
};
use bytes::{BufMut, BytesMut};
use dtoa::Float;
use itoa::Integer;
use serde::{
    ser::{
        self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
    Serialize, Serializer,
};

pub(crate) const SET_FAKE_FIELD: &str = "~~~SET~~~";
pub(crate) const ERROR_FAKE_FIELD: &str = "---ERROR---";

pub struct RespSerializer {
    output: BytesMut,
    is_error: bool,
}

impl RespSerializer {
    pub fn new() -> Self {
        Self {
            output: BytesMut::new(),
            is_error: false,
        }
    }

    #[inline]
    pub fn get_output(self) -> BytesMut {
        self.output
    }

    fn serialize_raw_integer<I: Integer>(&mut self, i: I) {
        let mut temp = itoa::Buffer::new();
        let str = temp.format(i);
        self.output.put_slice(str.as_bytes());
        self.output.put_slice(b"\r\n");
    }

    fn serialize_integer<I: Integer>(&mut self, i: I) {
        self.output.put_u8(INTEGER_TAG);
        self.serialize_raw_integer(i);
    }

    fn serialize_float<F: Float>(&mut self, f: F) {
        let mut temp = dtoa::Buffer::new();
        let str = temp.format(f);
        self.output.put_u8(DOUBLE_TAG);
        self.output.put_slice(str.as_bytes());
        self.output.put_slice(b"\r\n");
    }
}

impl Default for RespSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Serializer for &'a mut RespSerializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.output.put_slice(if v { b"#t" } else { b"#f" });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.serialize_integer(v);
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_float(v);
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.serialize_float(v);
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        let str = v.encode_utf8(&mut buf);
        self.serialize_str(str)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        if self.is_error {
            self.is_error = false;
            self.output.put_u8(ERROR_TAG);
        } else {
            self.output.put_u8(SIMPLE_STRING_TAG);
        }
        self.output.put_slice(v.as_bytes());
        self.output.put_slice(b"\r\n");
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.output.put_u8(BULK_STRING_TAG);
        self.serialize_raw_integer(v.len());
        self.output.put_slice(v);
        self.output.put_slice(b"\r\n");
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.output.put_slice(b"_\r\n");
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        if name == ERROR_FAKE_FIELD {
            self.is_error = true;
        }
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        self.output.put_u8(MAP_TAG);
        self.output.put_u8(b'1');
        variant.serialize(&mut *self)?;
        value.serialize(&mut *self)?;
        Ok(())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let Some(len) = len else {
            return Err(ser::Error::custom("expecting len on sequence serialization"));
        };

        self.output.put_u8(ARRAY_TAG);
        self.serialize_raw_integer(len);
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        match name {
            PUSH_FAKE_FIELD => {
                self.output.put_u8(PUSH_TAG);
                self.serialize_raw_integer(len);
                Ok(self)
            }
            SET_FAKE_FIELD => {
                self.output.put_u8(SET_TAG);
                self.serialize_raw_integer(len);
                Ok(self)
            }
            _ => self.serialize_seq(Some(len)),
        }
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.output.put_u8(MAP_TAG);
        self.output.put_u8(b'1');
        variant.serialize(&mut *self)?;
        self.output.put_u8(ARRAY_TAG);
        self.serialize_raw_integer(len);
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let Some(len) = len else {
            return Err(ser::Error::custom("expecting len on map serialization"));
        };

        self.output.put_u8(MAP_TAG);
        self.serialize_raw_integer(len);
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.output.put_u8(MAP_TAG);
        self.output.put_u8(b'1');
        variant.serialize(&mut *self)?;
        self.output.put_u8(MAP_TAG);
        self.serialize_raw_integer(len);
        Ok(self)
    }
}

impl<'a> SerializeSeq for &'a mut RespSerializer {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> SerializeTuple for &'a mut RespSerializer {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> SerializeTupleStruct for &'a mut RespSerializer {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> SerializeTupleVariant for &'a mut RespSerializer {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> SerializeMap for &'a mut RespSerializer {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        key.serialize(&mut **self)
    }

    #[inline]
    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> SerializeStruct for &'a mut RespSerializer {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        key.serialize(&mut **self)?;
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> SerializeStructVariant for &'a mut RespSerializer {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        key.serialize(&mut **self)?;
        value.serialize(&mut **self)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
