use crate::{Error, Result, resp::Value};
use serde::{
    Deserialize, Deserializer,
    de::{DeserializeSeed, EnumAccess, IntoDeserializer, VariantAccess, Visitor},
    forward_to_deserialize_any,
};
use std::{
    collections::{HashMap, hash_map},
    slice, str, vec,
};

impl<'de> Deserializer<'de> for &'de Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::SimpleString(s) => visitor.visit_borrowed_str(s),
            Value::Integer(i) => visitor.visit_i64(*i),
            Value::Double(d) => visitor.visit_f64(*d),
            Value::BulkString(bs) => visitor.visit_borrowed_bytes(bs),
            Value::Boolean(b) => visitor.visit_bool(*b),
            Value::Array(values) => visitor.visit_seq(SeqAccess::new(values)),
            Value::Map(values) => visitor.visit_map(MapAccess::new(values)),
            Value::Set(values) => visitor.visit_seq(SeqAccess::new(values)),
            Value::Push(values) => visitor.visit_seq(SeqAccess::new(values)),
            Value::Error(e) => Err(Error::Redis(e.clone())),
            Value::Nil => visitor.visit_none(),
        }
    }

    #[inline]
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => *i != 0,
            Value::Double(d) => *d != 0.,
            Value::SimpleString(s) if s == "OK" => true,
            Value::Nil => false,
            Value::BulkString(s) if s == b"0" || s == b"false" => false,
            Value::BulkString(s) if s == b"1" || s == b"true" => true,
            Value::Boolean(b) => *b,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {self:?} to bool"
                )));
            }
        };

        visitor.visit_bool(result)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => *i as i8,
            Value::Double(d) => *d as i8,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<i8>()?,
            Value::SimpleString(s) => s.parse::<i8>()?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {self:?} to i8"
                )));
            }
        };

        visitor.visit_i8(result)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => *i as i16,
            Value::Double(d) => *d as i16,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<i16>()?,
            Value::SimpleString(s) => s.parse::<i16>()?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {self:?} to i16"
                )));
            }
        };

        visitor.visit_i16(result)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => *i as i32,
            Value::Double(d) => *d as i32,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<i32>()?,
            Value::SimpleString(s) => s.parse::<i32>()?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {self:?} to i32"
                )));
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
            Value::Integer(i) => *i,
            Value::Double(d) => *d as i64,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<i64>()?,
            Value::SimpleString(s) => s.parse::<i64>()?,
            Value::Array(a) if a.len() == 1 => i64::deserialize(&a[0])?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {self:?} to i64"
                )));
            }
        };

        visitor.visit_i64(result)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => *i as u8,
            Value::Double(d) => *d as u8,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<u8>()?,
            Value::SimpleString(s) => s.parse::<u8>()?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {self:?} to u8"
                )));
            }
        };

        visitor.visit_u8(result)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => *i as u16,
            Value::Double(d) => *d as u16,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<u16>()?,
            Value::SimpleString(s) => s.parse::<u16>()?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {self:?} to u16"
                )));
            }
        };

        visitor.visit_u16(result)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => *i as u32,
            Value::Double(d) => *d as u32,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<u32>()?,
            Value::SimpleString(s) => s.parse::<u32>()?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {self:?} to u32"
                )));
            }
        };

        visitor.visit_u32(result)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => *i as u64,
            Value::Double(d) => *d as u64,
            Value::Nil => 0,
            Value::BulkString(s) => str::from_utf8(s)?.parse::<u64>()?,
            Value::SimpleString(s) => s.parse::<u64>()?,
            Value::Array(a) if a.len() == 1 => u64::deserialize(&a[0])?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {self:?} to u64"
                )));
            }
        };

        visitor.visit_u64(result)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => *i as f32,
            Value::Double(d) => *d as f32,
            Value::BulkString(bs) => str::from_utf8(bs)?.parse::<f32>()?,
            Value::Nil => 0.,
            Value::SimpleString(s) => s.parse::<f32>()?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse result {self:?} to f32"
                )));
            }
        };

        visitor.visit_f32(result)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Integer(i) => *i as f64,
            Value::Double(d) => *d,
            Value::BulkString(bs) => str::from_utf8(bs)?.parse::<f64>()?,
            Value::Nil => 0.,
            Value::SimpleString(s) => s.parse::<f64>()?,
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse result {self:?} to f64"
                )));
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
                let str = str::from_utf8(bs)?;
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
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => return Err(Error::Client("Cannot parse to char".to_owned())),
        };

        visitor.visit_char(result)
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
                    "Cannot parse value {self:?} to str"
                )));
            }
        };

        visitor.visit_borrowed_str(result)
    }

    #[inline]
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::Double(d) => d.to_string(),
            Value::BulkString(s) => str::from_utf8(s)?.to_owned(),
            Value::Nil => String::from(""),
            Value::SimpleString(s) => s.clone(),
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {self:?} to String"
                )));
            }
        };

        visitor.visit_string(result)
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
                    "Cannot parse value {self:?} to byte buffer"
                )));
            }
        };

        visitor.visit_borrowed_bytes(result)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let result = match self {
            Value::BulkString(s) => s.clone(),
            Value::Nil => vec![],
            Value::SimpleString(s) => s.as_bytes().to_vec(),
            Value::Error(e) => return Err(Error::Redis(e.clone())),
            _ => {
                return Err(Error::Client(format!(
                    "Cannot parse value {self:?} to byte buffer"
                )));
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
            Value::Array(values) if values.is_empty() => visitor.visit_none(),
            Value::Error(e) => Err(Error::Redis(e.clone())),
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
            Value::Integer(_) => visitor.visit_unit(),
            Value::SimpleString(_) => visitor.visit_unit(),
            Value::BulkString(bs) if bs.is_empty() => visitor.visit_unit(),
            Value::Array(a) if a.is_empty() => visitor.visit_unit(),
            Value::Set(s) if s.is_empty() => visitor.visit_unit(),
            Value::Map(m) if m.is_empty() => visitor.visit_unit(),
            Value::Error(e) => Err(Error::Redis(e.clone())),
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
            Value::Nil => visitor.visit_seq(NilSeqAccess),
            Value::Array(values) | Value::Set(values) | Value::Push(values) => {
                visitor.visit_seq(SeqAccess::new(values))
            }
            Value::Map(values) => visitor.visit_seq(MapAccess::new(values)),
            Value::Error(e) => Err(Error::Redis(e.clone())),
            _ => Err(Error::Client(format!(
                "Cannot parse sequence from value `{self}`"
            ))),
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
            Value::Error(e) => Err(Error::Redis(e.clone())),
            _ => Err(Error::Client("Cannot parse map".to_owned())),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        fn check_resp2_array(values: &[Value], fields: &'static [&'static str]) -> bool {
            if values.len() > fields.len() {
                true
            } else if let Some(Value::SimpleString(s)) = values.first() {
                fields.iter().any(|f| s == f)
            } else {
                false
            }
        }

        match self {
            Value::Array(values) => {
                if check_resp2_array(values, fields) {
                    visitor.visit_map(SeqAccess::new(values))
                } else {
                    visitor.visit_seq(SeqAccess::new(values))
                }
            }
            Value::Map(values) => visitor.visit_map(MapAccess::new(values)),
            Value::Error(e) => Err(Error::Redis(e.clone())),
            _ => Err(Error::Client("Cannot parse struct".to_owned())),
        }
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::BulkString(bs) => {
                // Visit a unit variant.
                let str = str::from_utf8(bs)?;
                visitor.visit_enum(str.into_deserializer())
            }
            Value::SimpleString(str) => {
                // Visit a unit variant.
                visitor.visit_enum(str.as_str().into_deserializer())
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
                    Err(Error::Client(format!(
                        "Map len must be 1 to parse enum {name} from {m:?}"
                    )))
                }
            }
            Value::Error(e) => Err(Error::Redis(e.clone())),
            _ => Err(Error::Client(format!(
                "Cannot parse enum `{name}` from `{self}`"
            ))),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct NilSeqAccess;

impl<'de> serde::de::SeqAccess<'de> for NilSeqAccess {
    type Error = Error;

    fn next_element_seed<T>(
        &mut self,
        _seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        Ok(None)
    }
}

struct SeqAccess<'de> {
    iter: slice::Iter<'de, Value>,
    len: usize,
    value: Option<&'de Value>,
}

impl<'de> SeqAccess<'de> {
    pub fn new(values: &'de [Value]) -> Self {
        Self {
            len: values.len(),
            iter: values.iter(),
            value: None,
        }
    }
}

impl<'de> serde::de::SeqAccess<'de> for SeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => {
                self.len -= 1;
                seed.deserialize(value).map(Some)
            }
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

/// in RESP, arrays can be seen as maps with a succession of keys and their values
impl<'de> serde::de::MapAccess<'de> for SeqAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(key) => match key {
                Value::Array(values) if values.len() == 2 => {
                    let key = &values[0];
                    self.value = Some(&values[1]);
                    seed.deserialize(key).map(Some)
                }
                _ => seed.deserialize(key).map(Some),
            },
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => match self.iter.next() {
                Some(value) => seed.deserialize(value),
                None => Err(serde::de::Error::custom(
                    "SeqAccess::next_value_seed: value is missing",
                )),
            },
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len / 2)
    }
}

struct MapAccess<'de> {
    len: usize,
    iter: hash_map::Iter<'de, Value, Value>,
    value: Option<&'de Value>,
}

impl<'de> MapAccess<'de> {
    pub fn new(values: &'de HashMap<Value, Value>) -> Self {
        Self {
            len: values.len(),
            iter: values.iter(),
            value: None,
        }
    }
}

impl<'de> serde::de::MapAccess<'de> for MapAccess<'de> {
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
            None => Err(serde::de::Error::custom("value is missing in map")),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

impl<'de> serde::de::SeqAccess<'de> for MapAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => seed.deserialize(ValuePair(key, value)).map(Some),
            None => Ok(None),
        }
    }
}

struct ValuePair<'de>(&'de Value, &'de Value);

impl<'de> Deserializer<'de> for ValuePair<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(2, visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq
        tuple_struct map struct enum identifier ignored_any
    }

    fn deserialize_tuple<V>(
        self,
        _len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        pub struct ValuePairSeqAccess<'de> {
            first: Option<&'de Value>,
            second: Option<&'de Value>,
        }

        impl<'de> serde::de::SeqAccess<'de> for ValuePairSeqAccess<'de> {
            type Error = Error;

            fn next_element_seed<T>(
                &mut self,
                seed: T,
            ) -> std::result::Result<Option<T::Value>, Self::Error>
            where
                T: DeserializeSeed<'de>,
            {
                if let Some(first) = self.first.take() {
                    seed.deserialize(first).map(Some)
                } else if let Some(second) = self.second.take() {
                    seed.deserialize(second).map(Some)
                } else {
                    Ok(None)
                }
            }
        }

        visitor.visit_seq(ValuePairSeqAccess {
            first: Some(self.0),
            second: Some(self.1),
        })
    }
}

struct Enum<'de> {
    variant_identifier: &'de Value,
    variant_value: &'de Value,
}

impl<'de> Enum<'de> {
    fn from_array(values: &'de [Value]) -> Self {
        let mut iter = values.iter();
        Self {
            variant_identifier: iter
                .next()
                .expect("array should have been tested as a 2-elements vector"),
            variant_value: iter
                .next()
                .expect("array should have been tested as a 2-elements vector"),
        }
    }

    fn from_map(values: &'de HashMap<Value, Value>) -> Self {
        let mut iter = values.iter();
        let (variant_identifier, variant_value) = iter
            .next()
            .expect("map should have been tested as a 1-element map");
        Self {
            variant_identifier,
            variant_value,
        }
    }
}

impl<'de> EnumAccess<'de> for Enum<'de> {
    type Error = Error;
    type Variant = &'de Value;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let val = seed.deserialize(self.variant_identifier)?;
        Ok((val, self.variant_value))
    }
}

impl<'de> VariantAccess<'de> for &'de Value {
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
