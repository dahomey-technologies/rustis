#![no_main]

//! Parses arbitrary bytes into a `Value` through `RespBuf::to`, exercising the
//! frame parser and `RespDeserializer` together.
//! Invariant: deserializing untrusted bytes never panics or aborts.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = rustis::fuzz_api::deserialize_to_value(data);
});
