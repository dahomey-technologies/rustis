#![no_main]

//! Drives `RespFrameParser::parse` directly with arbitrary bytes.
//! Invariant: parsing untrusted bytes never panics or aborts.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    rustis::fuzz_api::parse_frame(data);
});
