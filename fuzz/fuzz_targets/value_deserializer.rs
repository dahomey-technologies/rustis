#![no_main]

//! Parses arbitrary bytes into a `Value`, then deserializes that `Value` into a
//! range of concrete Rust types, exercising the coercions in
//! `value_deserializer.rs`.
//! Invariant: no panic or abort on any `Value` shape.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    rustis::fuzz_api::value_deserializer_roundtrip(data);
});
