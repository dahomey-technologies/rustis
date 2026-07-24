#![no_main]

//! Feeds bytes through the streaming `BufferDecoder`, split at fuzzer-chosen
//! byte boundaries, exercising the partial-frame / EOF-resume path.
//! Invariant: no panic or abort regardless of how frames are chunked.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct Input {
    data: Vec<u8>,
    splits: Vec<u8>,
}

fuzz_target!(|input: Input| {
    rustis::fuzz_api::decode_chunked(&input.data, &input.splits);
});
