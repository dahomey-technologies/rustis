//! Unit tests for the fixed-width tape node codec (`resp_tape`).

use crate::resp::{
    ARRAY_TAG, BULK_STRING_TAG, INTEGER_TAG, MAP_TAG, MAX_TAPE_PAYLOAD, PUSH_TAG, SET_TAG,
    SIMPLE_STRING_TAG, TAPE_LEN_TAG, TAPE_NODE_SIZE, encode, is_container_tag, node_count,
    node_payload, node_tag, patch_node, push_node, read_node,
};
use bytes::BytesMut;

#[test]
fn encode_decode_roundtrip() {
    // Tag and payload must survive a round-trip independently, across the full
    // range of payloads (including the 56-bit maximum).
    for &tag in &[
        SIMPLE_STRING_TAG,
        INTEGER_TAG,
        ARRAY_TAG,
        TAPE_LEN_TAG,
        0xFF,
    ] {
        for &payload in &[0u64, 1, 42, 0xABCD, MAX_TAPE_PAYLOAD / 2, MAX_TAPE_PAYLOAD] {
            let node = encode(tag, payload);
            assert_eq!(tag, node_tag(node), "tag lost for payload {payload:#x}");
            assert_eq!(payload, node_payload(node), "payload lost for tag {tag:#x}");
        }
    }
}

#[test]
fn tag_and_payload_do_not_bleed_into_each_other() {
    // A maximal payload must not flip any tag bit, and a full tag byte must not
    // corrupt the payload.
    let node = encode(0xFF, MAX_TAPE_PAYLOAD);
    assert_eq!(0xFF, node_tag(node));
    assert_eq!(MAX_TAPE_PAYLOAD, node_payload(node));

    let node = encode(0x00, MAX_TAPE_PAYLOAD);
    assert_eq!(0x00, node_tag(node));
    assert_eq!(MAX_TAPE_PAYLOAD, node_payload(node));
}

#[test]
fn max_payload_is_56_bits() {
    // The payload occupies exactly the low 7 bytes.
    assert_eq!(MAX_TAPE_PAYLOAD, (1u64 << 56) - 1);
}

#[test]
fn is_container_tag_matches_only_containers() {
    for &tag in &[ARRAY_TAG, MAP_TAG, SET_TAG, PUSH_TAG] {
        assert!(is_container_tag(tag), "{tag:#x} should be a container tag");
    }
    for &tag in &[
        SIMPLE_STRING_TAG,
        INTEGER_TAG,
        BULK_STRING_TAG,
        TAPE_LEN_TAG,
        b'_',
        b'#',
        b',',
        b'(',
        b'=',
        b'!',
    ] {
        assert!(
            !is_container_tag(tag),
            "{tag:#x} should not be a container tag"
        );
    }
}

#[test]
fn push_node_returns_sequential_indices_and_reads_back() {
    let mut tape = BytesMut::new();
    assert_eq!(0, node_count(&tape));

    let nodes = [
        (ARRAY_TAG, 4u64),
        (TAPE_LEN_TAG, 2),
        (BULK_STRING_TAG, 100),
        (INTEGER_TAG, 200),
    ];

    for (i, &(tag, payload)) in nodes.iter().enumerate() {
        assert_eq!(i, push_node(&mut tape, tag, payload));
        assert_eq!(i + 1, node_count(&tape));
    }

    // The buffer is exactly one node per entry, fixed width.
    assert_eq!(nodes.len() * TAPE_NODE_SIZE, tape.len());

    for (i, &(tag, payload)) in nodes.iter().enumerate() {
        let node = read_node(&tape, i);
        assert_eq!(tag, node_tag(node));
        assert_eq!(payload, node_payload(node));
    }
}

#[test]
fn patch_node_overwrites_payload_and_keeps_tag() {
    let mut tape = BytesMut::new();
    // A container head is emitted with a placeholder `next`, then back-patched
    // once its subtree is known.
    let head = push_node(&mut tape, ARRAY_TAG, 0);
    push_node(&mut tape, TAPE_LEN_TAG, 3);

    patch_node(&mut tape, head, ARRAY_TAG, 42);

    let node = read_node(&tape, head);
    assert_eq!(ARRAY_TAG, node_tag(node));
    assert_eq!(42, node_payload(node));
    // The neighbouring len node is untouched.
    let len_node = read_node(&tape, head + 1);
    assert_eq!(TAPE_LEN_TAG, node_tag(len_node));
    assert_eq!(3, node_payload(len_node));
}

#[test]
fn patch_node_can_change_the_tag_too() {
    let mut tape = BytesMut::new();
    let idx = push_node(&mut tape, ARRAY_TAG, 1);
    patch_node(&mut tape, idx, MAP_TAG, 9);
    let node = read_node(&tape, idx);
    assert_eq!(MAP_TAG, node_tag(node));
    assert_eq!(9, node_payload(node));
}

#[test]
fn manual_flat_container_layout_walks_correctly() {
    // Build the tape the parser would emit for a 2-element array `[10, 20]`:
    //   [0] head  (next = 4, past the whole container)
    //   [1] len   (2)
    //   [2] scalar integer, offset 4
    //   [3] scalar integer, offset 9
    let mut tape = BytesMut::new();
    let head = push_node(&mut tape, ARRAY_TAG, 0);
    push_node(&mut tape, TAPE_LEN_TAG, 2);
    push_node(&mut tape, INTEGER_TAG, 4);
    push_node(&mut tape, INTEGER_TAG, 9);
    let next = node_count(&tape) as u64;
    patch_node(&mut tape, head, ARRAY_TAG, next);

    // Head reads back as a container skipping to index 4 (one past the tape).
    let head_node = read_node(&tape, head);
    assert!(is_container_tag(node_tag(head_node)));
    assert_eq!(4, node_payload(head_node));
    // Its companion len node gives the exact child count.
    assert_eq!(2, node_payload(read_node(&tape, head + 1)));
    // Children start at head + 2 and are scalars.
    assert!(!is_container_tag(node_tag(read_node(&tape, head + 2))));
    assert!(!is_container_tag(node_tag(read_node(&tape, head + 3))));
}
