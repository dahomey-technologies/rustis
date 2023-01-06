use crate::{resp::Value, Error, Result};
use serde::{
    de::{DeserializeSeed, EnumAccess, IntoDeserializer, VariantAccess, Visitor},
    forward_to_deserialize_any, Deserializer,
};
use std::{
    collections::{hash_map, HashMap},
    str, vec,
};

impl<'de> Deserializer<'de> for Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::SimpleString(s) => visitor.visit_string(s),
            Value::Integer(i) => visitor.visit_i64(i),
            Value::Double(d) => visitor.visit_f64(d),
            Value::BulkString(bs) => visitor.visit_string(String::from_utf8(bs)?),
            Value::Boolean(b) => visitor.visit_bool(b),
            Value::Array(_) => todo!(),
            Value::Map(_) => todo!(),
            Value::Set(_) => todo!(),
            Value::Push(_) => todo!(),
            Value::Error(_) => todo!(),
            Value::Nil => todo!(),
        }
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => i != 0,
            Value::Double(d) => d != 0.,
            Value::SimpleString(s) if s == "OK" => true,
            Value::Nil => false,
            Value::BulkString(s) if s == b"0" || s == b"false" => false,
            Value::BulkString(s) if s == b"1" || s == b"true" => true,
            Value::Boolean(b) => b,
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to bool",
                    self
                )))
            }
        };

        visitor.visit_bool(result)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => i as i8,
            Value::Double(d) => d as i8,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(&s)?.parse::<i8>()?,
            Value::SimpleString(s) => s.parse::<i8>()?,
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to i8",
                    self
                )))
            }
        };

        visitor.visit_i8(result)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => i as i16,
            Value::Double(d) => d as i16,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(&s)?.parse::<i16>()?,
            Value::SimpleString(s) => s.parse::<i16>()?,
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to i16",
                    self
                )))
            }
        };

        visitor.visit_i16(result)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => i as i32,
            Value::Double(d) => d as i32,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(&s)?.parse::<i32>()?,
            Value::SimpleString(s) => s.parse::<i32>()?,
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to i32",
                    self
                )))
            }
        };

        visitor.visit_i32(result)
    }

    #[inline]
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => i,
            Value::Double(d) => d as i64,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(&s)?.parse::<i64>()?,
            Value::SimpleString(s) => s.parse::<i64>()?,
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to i64",
                    self
                )))
            }
        };

        visitor.visit_i64(result)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => i as u8,
            Value::Double(d) => d as u8,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(&s)?.parse::<u8>()?,
            Value::SimpleString(s) => s.parse::<u8>()?,
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to u8",
                    self
                )))
            }
        };

        visitor.visit_u8(result)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => i as u16,
            Value::Double(d) => d as u16,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(&s)?.parse::<u16>()?,
            Value::SimpleString(s) => s.parse::<u16>()?,
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to u16",
                    self
                )))
            }
        };

        visitor.visit_u16(result)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => i as u32,
            Value::Double(d) => d as u32,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(&s)?.parse::<u32>()?,
            Value::SimpleString(s) => s.parse::<u32>()?,
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to u32",
                    self
                )))
            }
        };

        visitor.visit_u32(result)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => i as u64,
            Value::Double(d) => d as u64,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(&s)?.parse::<u64>()?,
            Value::SimpleString(s) => s.parse::<u64>()?,
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to u64",
                    self
                )))
            }
        };

        visitor.visit_u64(result)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => i as f32,
            Value::Double(d) => d as f32,
            Value::BulkString(bs) => str::from_utf8(&bs)?.parse::<f32>()?,
            Value::Nil => 0.,
            Value::SimpleString(s) => s.parse::<f32>()?,
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse result {:?} to f32",
                    self
                )))
            }
        };

        visitor.visit_f32(result)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => i as f64,
            Value::Double(d) => d,
            Value::BulkString(bs) => str::from_utf8(&bs)?.parse::<f64>()?,
            Value::Nil => 0.,
            Value::SimpleString(s) => s.parse::<f64>()?,
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse result {:?} to f64",
                    self
                )))
            }
        };

        visitor.visit_f64(result)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result: char = match self {
            Value::BulkString(bs) => {
                let str = str::from_utf8(&bs)?;
                if str.len() == 1 {
                    str.chars().next().unwrap()
                } else {
                    return Err(Error::Client("Cannot parse to char".to_owned()));
                }
            }
            Value::SimpleString(str) => {
                if str.len() == 1 {
                    str.chars().next().unwrap()
                } else {
                    return Err(Error::Client("Cannot parse to char".to_owned()));
                }
            }
            Value::Nil => '\0',
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => return Err(Error::Client("Cannot parse to char".to_owned())),
        };

        visitor.visit_char(result)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    #[inline]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Double(d) => d.to_string(),
            Value::BulkString(s) => String::from_utf8(s)?,
            Value::Nil => String::from(""),
            Value::SimpleString(s) => s,
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to String",
                    self
                )))
            }
        };

        visitor.visit_string(result)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::BulkString(s) => s,
            Value::Nil => vec![],
            Value::SimpleString(s) => s.as_bytes().to_vec(),
            Value::Error(e) => return Err(Error::Redis(e)),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to byte buffer",
                    self
                )))
            }
        };

        visitor.visit_byte_buf(result)
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Nil => visitor.visit_none(),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => visitor.visit_some(self),
        }
    }

    #[inline]
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Nil => visitor.visit_unit(),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client("Expected nil".to_owned())),
        }
    }

    // Unit struct means a named value containing no data.
    #[inline]
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    #[inline]
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Array(values) => visitor.visit_seq(SeqAccess::new(values)),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client("Cannot parse sequence".to_owned())),
        }
    }

    #[inline]
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    #[inline]
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Array(values) => visitor.visit_map(SeqAccess::new(values)),
            Value::Map(values) => visitor.visit_map(MapAccess::new(values)),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client("Cannot parse map".to_owned())),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Array(values) => visitor.visit_map(RefSeqAccess::new(values)),
            Value::Map(values) => visitor.visit_map(RefMapAccess::new(values)),
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client("Cannot parse map".to_owned())),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::BulkString(bs) => {
                // Visit a unit variant.
                let str = str::from_utf8(&bs)?;
                visitor.visit_enum(str.into_deserializer())
            }
            Value::SimpleString(str) => {
                // Visit a unit variant.
                visitor.visit_enum(str.into_deserializer())
            }
            Value::Array(a) => {
                // Visit a newtype variant, tuple variant, or struct variant
                // as an array of 2 elements
                if a.len() == 2 {
                    visitor.visit_enum(Enum::from_array(a))
                } else {
                    Err(Error::Client(
                        "Array len must be 2 to parse an enum".to_owned(),
                    ))
                }
            }
            Value::Map(m) => {
                // Visit a newtype variant, tuple variant, or struct variant
                // as a map of 1 element
                if m.len() == 1 {
                    visitor.visit_enum(Enum::from_map(m))
                } else {
                    Err(Error::Client(
                        "Map len must be 1 to parse an enum".to_owned(),
                    ))
                }
            }
            Value::Error(e) => Err(Error::Redis(e)),
            _ => Err(Error::Client("Cannot parse enum".to_owned())),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

impl<'de, 'a> Deserializer<'de> for &'a Value {
    type Error = Error;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char string
        byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any
    }

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::BulkString(s) => s.as_slice(),
            Value::Nil => &[],
            Value::SimpleString(s) => s.as_bytes(),
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to bytes",
                    self
                )))
            }
        };

        visitor.visit_bytes(result)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::BulkString(s) => str::from_utf8(s)?,
            Value::Nil => "",
            Value::SimpleString(s) => s.as_str(),
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {:?} to str",
                    self
                )))
            }
        };

        visitor.visit_str(result)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }
}

pub struct SeqAccess {
    len: usize,
    iter: vec::IntoIter<Value>,
}

impl SeqAccess {
    pub fn new(values: Vec<Value>) -> Self {
        Self {
            len: values.len(),
            iter: values.into_iter(),
        }
    }
}

impl<'de> serde::de::SeqAccess<'de> for SeqAccess {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

/// in RESP, arrays can be seen as maps with a succession of keys and their values
impl<'de> serde::de::MapAccess<'de> for SeqAccess {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(key) => seed.deserialize(key).map(Some),
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len / 2)
    }
}

pub struct MapAccess {
    len: usize,
    iter: hash_map::IntoIter<Value, Value>,
    value: Option<Value>,
}

impl MapAccess {
    pub fn new(values: HashMap<Value, Value>) -> Self {
        Self {
            len: values.len(),
            iter: values.into_iter(),
            value: None,
        }
    }
}

impl<'de> serde::de::MapAccess<'de> for MapAccess {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

pub struct RefSeqAccess {
    len: usize,
    iter: vec::IntoIter<Value>,
}

impl RefSeqAccess {
    pub fn new(values: Vec<Value>) -> Self {
        Self {
            len: values.len(),
            iter: values.into_iter(),
        }
    }
}

/// in RESP, arrays can be seen as maps with a succession of keys and their values
impl<'de> serde::de::MapAccess<'de> for RefSeqAccess {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(key) => seed.deserialize(&key).map(Some),
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len / 2)
    }
}

pub struct RefMapAccess {
    len: usize,
    iter: hash_map::IntoIter<Value, Value>,
    value: Option<Value>,
}

impl RefMapAccess {
    pub fn new(values: HashMap<Value, Value>) -> Self {
        Self {
            len: values.len(),
            iter: values.into_iter(),
            value: None,
        }
    }
}

impl<'de> serde::de::MapAccess<'de> for RefMapAccess {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(&key).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(serde::de::Error::custom("value is missing")),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct Enum {
    variant_identifier: Value,
    variant_value: Value,
}

impl Enum {
    fn from_array(values: Vec<Value>) -> Self {
        let mut iter = values.into_iter();
        Self {
            variant_identifier: iter
                .next()
                .expect("array should have been tested as a 2-elements vector"),
            variant_value: iter
                .next()
                .expect("array should have been tested as a 2-elements vector"),
        }
    }

    fn from_map(values: HashMap<Value, Value>) -> Self {
        let mut iter = values.into_iter();
        let (variant_identifier, variant_value) = iter
            .next()
            .expect("map should have been tested as a 1-element map");
        Self {
            variant_identifier,
            variant_value,
        }
    }
}

impl<'de> EnumAccess<'de> for Enum {
    type Error = Error;
    type Variant = Value;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let val = seed.deserialize(self.variant_identifier)?;
        Ok((val, self.variant_value))
    }
}

impl<'de> VariantAccess<'de> for Value {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<()> {
        Err(Error::Client("Expected string or bulk string".to_owned()))
    }

    // Newtype variants are represented as map so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    // Tuple variants are represented as map of array so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Struct variants are represented as map of map so
    // deserialize the inner map here.
    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }
}
