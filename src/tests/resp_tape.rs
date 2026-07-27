//! Unit tests for the fixed-width tape node codec (`resp_tape`).

use crate::resp::{
    ARRAY_TAG, BULK_STRING_TAG, INTEGER_TAG, MAP_TAG, MAX_TAPE_PAYLOAD, PUSH_TAG, RespTape,
    RespTapeMut, SET_TAG, SIMPLE_STRING_TAG, TAPE_LEN_TAG, TAPE_NODE_SIZE, TapeNode,
    is_collection_tag,
};

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
            let node = TapeNode::new(tag, payload);
            assert_eq!(tag, node.tag(), "tag lost for payload {payload:#x}");
            assert_eq!(payload, node.payload(), "payload lost for tag {tag:#x}");
        }
    }
}

#[test]
fn tag_and_payload_do_not_bleed_into_each_other() {
    // A maximal payload must not flip any tag bit, and a full tag byte must not
    // corrupt the payload.
    let node = TapeNode::new(0xFF, MAX_TAPE_PAYLOAD);
    assert_eq!(0xFF, node.tag());
    assert_eq!(MAX_TAPE_PAYLOAD, node.payload());

    let node = TapeNode::new(0x00, MAX_TAPE_PAYLOAD);
    assert_eq!(0x00, node.tag());
    assert_eq!(MAX_TAPE_PAYLOAD, node.payload());
}

#[test]
fn max_payload_is_56_bits() {
    // The payload occupies exactly the low 7 bytes.
    assert_eq!(MAX_TAPE_PAYLOAD, (1u64 << 56) - 1);
}

#[test]
fn node_size_is_a_power_of_two() {
    assert_eq!(8, TAPE_NODE_SIZE);
}

#[test]
fn is_collection_tag_matches_only_collections() {
    for &tag in &[ARRAY_TAG, MAP_TAG, SET_TAG, PUSH_TAG] {
        assert!(
            is_collection_tag(tag),
            "{tag:#x} should be a collection tag"
        );
        assert!(TapeNode::new(tag, 0).is_collection());
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
            !is_collection_tag(tag),
            "{tag:#x} should not be a collection tag"
        );
        assert!(!TapeNode::new(tag, 0).is_collection());
    }
}

#[test]
fn push_returns_sequential_indices_and_reads_back() {
    let mut builder = RespTapeMut::default();
    assert_eq!(0, builder.node_count());
    assert!(builder.is_empty());

    let nodes = [
        (ARRAY_TAG, 4u64),
        (TAPE_LEN_TAG, 2),
        (BULK_STRING_TAG, 100),
        (INTEGER_TAG, 200),
    ];

    for (i, &(tag, payload)) in nodes.iter().enumerate() {
        assert_eq!(i, builder.push(tag, payload));
        assert_eq!(i + 1, builder.node_count());
    }

    let tape = builder.split_freeze();
    // Splitting hands the frame its nodes and leaves the builder empty for the next.
    assert_eq!(nodes.len(), tape.node_count());
    assert_eq!(nodes.len() * TAPE_NODE_SIZE, tape.byte_len());
    assert!(builder.is_empty());

    for (i, &(tag, payload)) in nodes.iter().enumerate() {
        let node = tape.node(i);
        assert_eq!(tag, node.tag());
        assert_eq!(payload, node.payload());
    }
}

#[test]
fn patch_overwrites_payload_and_keeps_tag() {
    let mut builder = RespTapeMut::default();
    // A collection head is emitted with a placeholder `next`, then back-patched
    // once its subtree is known.
    let head = builder.push(ARRAY_TAG, 0);
    builder.push(TAPE_LEN_TAG, 3);

    builder.patch(head, ARRAY_TAG, 42);

    let tape = builder.split_freeze();
    let node = tape.node(head);
    assert_eq!(ARRAY_TAG, node.tag());
    assert_eq!(42, node.payload());
    // The neighbouring len node is untouched.
    let len_node = tape.node(head + 1);
    assert_eq!(TAPE_LEN_TAG, len_node.tag());
    assert_eq!(3, len_node.payload());
}

#[test]
fn patch_can_change_the_tag_too() {
    let mut builder = RespTapeMut::default();
    let idx = builder.push(ARRAY_TAG, 1);
    builder.patch(idx, MAP_TAG, 9);
    let node = builder.split_freeze().node(idx);
    assert_eq!(MAP_TAG, node.tag());
    assert_eq!(9, node.payload());
}

#[test]
fn manual_flat_collection_layout_walks_correctly() {
    // Build the tape the parser would emit for a 2-element array `[10, 20]`:
    //   [0] head  (next = 4, past the whole collection)
    //   [1] len   (2)
    //   [2] scalar integer, offset 4
    //   [3] scalar integer, offset 9
    let mut builder = RespTapeMut::default();
    let head = builder.push(ARRAY_TAG, 0);
    builder.push(TAPE_LEN_TAG, 2);
    builder.push(INTEGER_TAG, 4);
    builder.push(INTEGER_TAG, 9);
    let next = builder.node_count() as u64;
    builder.patch(head, ARRAY_TAG, next);
    let tape = builder.split_freeze();

    // Head reads back as a collection skipping to index 4 (one past the tape).
    let head_node = tape.node(head);
    assert!(head_node.is_collection());
    assert_eq!(4, head_node.payload());
    // Its companion len node gives the exact child count, also via `collection_len`.
    assert_eq!(2, tape.node(head + 1).payload());
    assert_eq!(Some(2), tape.collection_len(head));
    // Children start at head + 2 and are scalars.
    assert!(!tape.node(head + 2).is_collection());
    assert!(!tape.node(head + 3).is_collection());
}

#[test]
fn collection_len_reports_unreadable_rather_than_panicking() {
    // A tape too short to hold the companion node was not produced by the parser;
    // the formatters rely on `None` instead of an out-of-bounds read.
    let mut builder = RespTapeMut::default();
    builder.push(ARRAY_TAG, 0);
    let tape = builder.split_freeze();
    assert_eq!(None, tape.collection_len(0));

    assert_eq!(None, RespTape::default().collection_len(0));
}
