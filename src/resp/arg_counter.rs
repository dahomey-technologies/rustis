use serde::{Serializer, ser};

#[derive(Default)]
pub struct ArgCounter {
    pub count: usize,
}

impl Serializer for &mut ArgCounter {
    type Ok = ();
    type Error = crate::Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.count += 1;
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        // No-op
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_none()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

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
        value.serialize(&mut *self)?;
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }

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

impl ser::SerializeSeq for &mut ArgCounter {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_element<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl ser::SerializeTuple for &mut ArgCounter {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_element<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &mut ArgCounter {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl ser::SerializeTupleVariant for &mut ArgCounter {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl ser::SerializeMap for &mut ArgCounter {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_key<T: ?Sized + serde::Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        key.serialize(&mut **self)
    }

    fn serialize_value<T: ?Sized + serde::Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl ser::SerializeStruct for &mut ArgCounter {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.serialize_str(key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut ArgCounter {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.serialize_str(key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
