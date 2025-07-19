use crate::{
    Result,
    resp::{BulkString, RespSerializer},
    tests::log_try_init,
};
use serde::Serialize;
use smallvec::{SmallVec, smallvec};
use std::collections::HashMap;

fn serialize<T>(value: T) -> Result<String>
where
    T: serde::Serialize,
{
    let mut serializer = RespSerializer::new();
    value.serialize(&mut serializer)?;
    let buf = serializer.get_output();
    Ok(String::from_utf8(buf.to_vec())?)
}

#[test]
fn bool() -> Result<()> {
    log_try_init();

    let result = serialize(true)?;
    assert_eq!("#t\r\n", result);

    let result = serialize(false)?;
    assert_eq!("#f\r\n", result);

    Ok(())
}

#[test]
fn integer() -> Result<()> {
    log_try_init();

    let result = serialize(12)?;
    assert_eq!(":12\r\n", result);

    let result = serialize(0)?;
    assert_eq!(":0\r\n", result);

    let result = serialize(-12)?;
    assert_eq!(":-12\r\n", result);

    Ok(())
}

#[test]
fn float() -> Result<()> {
    log_try_init();

    let result = serialize(12.12)?;
    assert_eq!(",12.12\r\n", result);

    let result = serialize(0.0)?;
    assert_eq!(",0.0\r\n", result);

    let result = serialize(-12.12)?;
    assert_eq!(",-12.12\r\n", result);

    Ok(())
}

#[test]
fn char() -> Result<()> {
    log_try_init();

    let result = serialize('c')?;
    assert_eq!("+c\r\n", result);

    Ok(())
}

#[test]
fn str() -> Result<()> {
    log_try_init();

    let result = serialize("OK")?;
    assert_eq!("+OK\r\n", result);

    let result = serialize("OK".to_owned())?;
    assert_eq!("+OK\r\n", result);

    Ok(())
}

#[test]
fn bytes() -> Result<()> {
    log_try_init();

    let bytes: BulkString = b"abc".into();
    let result = serialize(bytes)?;
    assert_eq!("$3\r\nabc\r\n", result);

    Ok(())
}

#[test]
fn option() -> Result<()> {
    log_try_init();

    let result = serialize(Some(12))?;
    assert_eq!(":12\r\n", result);

    let result = serialize(Option::<i64>::None)?;
    assert_eq!("_\r\n", result);

    let result = serialize(Some(12.12))?;
    assert_eq!(",12.12\r\n", result);

    let result = serialize(Option::<f64>::None)?;
    assert_eq!("_\r\n", result);

    let result = serialize(Some("OK"))?;
    assert_eq!("+OK\r\n", result);

    let result = serialize(Option::<&str>::None)?;
    assert_eq!("_\r\n", result);

    Ok(())
}

#[test]
fn unit() -> Result<()> {
    log_try_init();

    let result = serialize(())?;
    assert_eq!("_\r\n", result);

    Ok(())
}

#[test]
fn unit_struct() -> Result<()> {
    log_try_init();

    #[derive(Serialize)]
    struct Unit;

    let result = serialize(Unit)?;
    assert_eq!("_\r\n", result);

    Ok(())
}

#[test]
fn newtype_struct() -> Result<()> {
    log_try_init();

    #[derive(Serialize)]
    struct Millimeters(u8);

    let result = serialize(Millimeters(12))?;
    assert_eq!(":12\r\n", result);

    Ok(())
}

#[test]
fn seq() -> Result<()> {
    log_try_init();

    let result = serialize([12, 13])?;
    assert_eq!("*2\r\n:12\r\n:13\r\n", result);

    let result = serialize(vec![12, 13])?;
    assert_eq!("*2\r\n:12\r\n:13\r\n", result);

    let result = serialize(smallvec![12, 13] as SmallVec<[i32; 2]>)?;
    assert_eq!("*2\r\n:12\r\n:13\r\n", result);

    Ok(())
}

#[test]
fn tuple() -> Result<()> {
    log_try_init();

    let result = serialize((12, 13))?;
    assert_eq!("*2\r\n:12\r\n:13\r\n", result);

    Ok(())
}

#[test]
fn tuple_struct() -> Result<()> {
    log_try_init();

    #[derive(Serialize)]
    struct Rgb(u8, u8, u8);

    let result = serialize(Rgb(12, 13, 14))?;
    assert_eq!("*3\r\n:12\r\n:13\r\n:14\r\n", result);

    Ok(())
}

#[test]
fn map() -> Result<()> {
    log_try_init();

    let result = serialize(HashMap::from([(12, 13), (14, 15)]))?;
    assert!(
        result == "%2\r\n:12\r\n:13\r\n:14\r\n:15\r\n"
            || result == "%2\r\n:14\r\n:15\r\n:12\r\n:13\r\n"
    );

    Ok(())
}

#[test]
fn _struct() -> Result<()> {
    log_try_init();

    #[derive(Serialize)]
    struct Person {
        pub id: u64,
        pub name: String,
    }

    let result = serialize(Person {
        id: 12,
        name: "Mike".to_owned(),
    })?;
    assert_eq!("%2\r\n+id\r\n:12\r\n+name\r\n+Mike\r\n", result);

    Ok(())
}

#[test]
fn _enum() -> Result<()> {
    log_try_init();

    #[derive(Serialize)]
    enum E {
        A,                         // unit_variant
        B(u8),                     // newtype_variant
        C(u8, u8),                 // tuple_variant
        D { r: u8, g: u8, b: u8 }, // struct_variant
    }

    let result = serialize(E::A)?;
    assert_eq!("+A\r\n", result);

    let result = serialize(E::B(12))?;
    assert_eq!("%1\r\n+B\r\n:12\r\n", result);

    let result = serialize(E::C(12, 13))?;
    assert_eq!("%1\r\n+C\r\n*2\r\n:12\r\n:13\r\n", result);

    let result = serialize(E::D {
        r: 12,
        g: 13,
        b: 14,
    })?;
    assert_eq!(
        "%1\r\n+D\r\n%3\r\n+r\r\n:12\r\n+g\r\n:13\r\n+b\r\n:14\r\n",
        result
    );

    Ok(())
}
