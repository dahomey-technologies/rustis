use bytes::BytesMut;
use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use rustis::resp::{
    BufferDecoder, HashMapExt, RawValueDecoder, RespDecoder, RespDeserializer2, Value, ValueDecoder,
};
use serde::{de::DeserializeOwned, Deserialize};
use std::collections::HashMap;
use tokio_util::codec::Decoder;

#[inline]
fn decode_value(buf: &[u8]) -> Value {
    let mut bytes = BytesMut::from(buf);
    let mut value_decoder = ValueDecoder;
    value_decoder.decode(&mut bytes).unwrap().unwrap()
}

#[inline]
fn deserialize<T>(buf: &[u8]) -> T
where
    T: DeserializeOwned,
{
    let mut bytes = BytesMut::from(buf);
    let mut resp_decoder = RespDecoder::<T>::default();
    resp_decoder.decode(&mut bytes).unwrap().unwrap()

    // let buf = buf.to_vec();
    // let mut deserializer = rustis::resp::RespDeserializer::from_bytes(&buf);
    // T::deserialize(&mut deserializer).unwrap()
}

#[inline]
fn deserialize2<T>(buf: &[u8]) -> T
where
    T: DeserializeOwned,
{
    let mut bytes = BytesMut::from(buf);
    let mut buffer_decoder = BufferDecoder;
    let buf = buffer_decoder.decode(&mut bytes).unwrap().unwrap();

    let mut deserializer = rustis::resp::RespDeserializer::from_bytes(&buf);
    T::deserialize(&mut deserializer).unwrap()
}

#[inline]
fn deserialize3<T>(buf: &[u8]) -> T
where
    T: DeserializeOwned,
{
    let mut decoder = RawValueDecoder;
    let mut bytes: BytesMut = buf.into();
    let raw_values = decoder.decode(&mut bytes).unwrap().unwrap();

    let mut deserializer = RespDeserializer2::new(buf, raw_values);
    T::deserialize(&mut deserializer).unwrap()
}

#[inline]
fn deserialize4<T>(buf: &[u8]) -> T
where
    T: DeserializeOwned,
{
    let value = decode_value(buf);
    T::deserialize(value).unwrap()
}

fn deserialize_string_from_value(b: &mut Bencher) {
    b.iter(|| {
        let value = decode_value(b"$5\r\nhello\r\n");
        let _: String = value.into().unwrap();
    });
}

fn deserialize_string_from_value_serde(b: &mut Bencher) {
    b.iter(|| {
        let _: String = deserialize4(b"$5\r\nhello\r\n");
    });
}

fn deserialize_string_from_serde(b: &mut Bencher) {
    b.iter(|| {
        let _: String = deserialize(b"$5\r\nhello\r\n");
    });
}

fn deserialize_string_from_serde_with_copy(b: &mut Bencher) {
    b.iter(|| {
        let _: String = deserialize2(b"$5\r\nhello\r\n");
    });
}

fn deserialize_string_from_serde_with_copy2(b: &mut Bencher) {
    b.iter(|| {
        let _: String = deserialize3(b"$5\r\nhello\r\n");
    });
}

fn deserialize_int_from_value(b: &mut Bencher) {
    b.iter(|| {
        let value = decode_value(b":12\r\n");
        let _: i64 = value.into().unwrap();
    });
}

fn deserialize_int_from_value_serde(b: &mut Bencher) {
    b.iter(|| {
        let _: i64 = deserialize4(b":12\r\n");
    });
}

fn deserialize_int_from_serde(b: &mut Bencher) {
    b.iter(|| {
        let _: i64 = deserialize(b":12\r\n");
    });
}

fn deserialize_int_from_serde_with_copy(b: &mut Bencher) {
    b.iter(|| {
        let _: i64 = deserialize2(b":12\r\n");
    });
}

fn deserialize_int_from_serde_with_copy2(b: &mut Bencher) {
    b.iter(|| {
        let _: i64 = deserialize3(b":12\r\n");
    });
}

#[derive(Debug, Deserialize)]
pub struct Person {
    pub id: u64,
    pub name: String,
}

fn deserialize_struct_from_value(b: &mut Bencher) {
    b.iter(|| {
        let value = decode_value(b"*4\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n");
        let mut values: HashMap<String, Value> = value.into().unwrap();

        let _ = Person {
            id: values.remove_or_default("id").into().unwrap(),
            name: values.remove_or_default("name").into().unwrap(),
        };
    });
}

fn deserialize_struct_from_value_serde(b: &mut Bencher) {
    b.iter(|| {
        let _: Person = deserialize4(b"*4\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n");
    });
}

fn deserialize_struct_from_serde(b: &mut Bencher) {
    b.iter(|| {
        let _: Person = deserialize(b"*4\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n");
    });
}

fn deserialize_struct_from_serde_with_copy(b: &mut Bencher) {
    b.iter(|| {
        let _: Person = deserialize2(b"*4\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n");
    });
}

fn deserialize_struct_from_serde_with_copy2(b: &mut Bencher) {
    b.iter(|| {
        let _: Person = deserialize3(b"*4\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n");
    });
}

fn bench_deserialize_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize_string");
    group
        .bench_function(
            "deserialize_string_from_value",
            deserialize_string_from_value,
        )
        .bench_function(
            "deserialize_string_from_value_serde",
            deserialize_string_from_value_serde,
        )
        .bench_function(
            "deserialize_string_from_serde",
            deserialize_string_from_serde,
        )
        .bench_function(
            "deserialize_string_from_serde_with_copy",
            deserialize_string_from_serde_with_copy,
        )
        .bench_function(
            "deserialize_string_from_serde_with_copy2",
            deserialize_string_from_serde_with_copy2,
        );
    group.finish();
}

fn bench_deserialize_int(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize_int");
    group
        .bench_function("deserialize_int_from_value", deserialize_int_from_value)
        .bench_function(
            "deserialize_int_from_value_serde",
            deserialize_int_from_value_serde,
        )
        .bench_function("deserialize_int_from_serde", deserialize_int_from_serde)
        .bench_function(
            "deserialize_int_from_serde_with_copy",
            deserialize_int_from_serde_with_copy,
        )
        .bench_function(
            "deserialize_int_from_serde_with_copy2",
            deserialize_int_from_serde_with_copy2,
        );
    group.finish();
}

fn bench_deserialize_struct(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize_struct");
    group
        .bench_function(
            "deserialize_struct_from_value",
            deserialize_struct_from_value,
        )
        .bench_function(
            "deserialize_struct_from_value_serde",
            deserialize_struct_from_value_serde,
        )
        .bench_function(
            "deserialize_struct_from_serde",
            deserialize_struct_from_serde,
        )
        .bench_function(
            "deserialize_struct_from_serde_with_copy",
            deserialize_struct_from_serde_with_copy,
        )
        .bench_function(
            "deserialize_struct_from_serde_with_copy2",
            deserialize_struct_from_serde_with_copy2,
        );
    group.finish();
}

criterion_group!(
    bench,
    // bench_deserialize_string,
    // bench_deserialize_int,
    bench_deserialize_struct
);
criterion_main!(bench);
