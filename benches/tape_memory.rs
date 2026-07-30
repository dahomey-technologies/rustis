//! Memory cost of the parse tape, and the ceiling on narrowing its node.
//!
//! The tape indexes a collection reply with one fixed-width node per element,
//! plus two per collection (head + length). At [`TAPE_NODE_SIZE`] = 8 bytes that
//! is a cost proportional to the *element count*, while the reply's own size is
//! proportional to the *byte count* — so the tape's share of the footprint is
//! set entirely by the average element size, and grows as elements shrink. The
//! open question is whether that share is acceptable at scale, and whether a
//! narrower node would be worth the encoding it would require.
//!
//! Four measurements, one per part of that question:
//!
//! 1. **`footprint`** — a printed report, not a timing: reply bytes vs. tape
//!    bytes vs. allocations, over reply shapes from the worst case (very many
//!    tiny elements) to the case where the tape vanishes (few large elements).
//! 2. **`retained`** — the tape lives in a recycled block that is only returned
//!    after `shrink_hysteresis` quiet frames, so the bytes a frame *builds* are
//!    not the bytes a connection *holds*. This drives a spike then a quiet
//!    streak and reports the block still pinned at each step.
//! 3. **`throughput_by_elem_len`** — decode + deserialize at a constant element
//!    count and varying element size. A per-element cost shows up as a flat
//!    floor that dominates as elements shrink; a per-byte cost does not.
//! 4. **`node_width`** — synthetic write-then-read of N `u64` nodes vs. N `u32`
//!    nodes in a recycled buffer, with no parser involved. This is the **upper
//!    bound** on what halving the node width could buy: a real 32-bit node would
//!    additionally need a `len`-derived encoding or an out-of-band fallback for
//!    frames past its range, so the achievable gain is strictly smaller than the
//!    gap measured here. If the gap is small against the decode cost measured in
//!    (3), the question is closed without writing that prototype.
//!
//!    Read only the 100k row. At 10k nodes both variants fit in L2 and the
//!    measurement is bistable — each flips between two stable modes ~2 µs apart
//!    across runs, more than the width difference itself — so that row cannot
//!    arbitrate anything. The 100k row is reproducible run to run.
//!
//! Run with:
//!   cargo bench --features bench --bench tape_memory
//!
//! [`TAPE_NODE_SIZE`]: rustis internal, 8 bytes

use bytes::{BufMut, BytesMut};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rustis::resp::{
    BenchDecoder, BenchTape, bench_decode_to, bench_parse_only, bench_tape_footprint,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Allocation counter, so the footprint report can state how many allocations a
/// reply's tape costs and not only how many bytes. Local to this bench binary.
struct Counting;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
/// Bytes allocated and not yet freed. This is the figure the retained-memory
/// question actually needs: `BytesMut::capacity()` after a `split()` reports only
/// the tail's share, while the whole block stays pinned by the refcount.
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        LIVE_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(new_size.saturating_sub(layout.size()), Ordering::Relaxed);
        LIVE_BYTES.fetch_add(new_size, Ordering::Relaxed);
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Bytes currently allocated and not freed.
fn live_bytes() -> usize {
    LIVE_BYTES.load(Ordering::Relaxed)
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Counts the allocations and allocated bytes a closure causes.
fn measure_allocs<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
    let allocs_before = ALLOCS.load(Ordering::Relaxed);
    let bytes_before = ALLOC_BYTES.load(Ordering::Relaxed);
    let out = f();
    let allocs = ALLOCS.load(Ordering::Relaxed) - allocs_before;
    let bytes = ALLOC_BYTES.load(Ordering::Relaxed) - bytes_before;
    (out, allocs, bytes)
}

/// A flat RESP array of `n` bulk strings, each `elem_len` bytes.
fn build_array(n: usize, elem_len: usize) -> Vec<u8> {
    let elem = "x".repeat(elem_len);
    let mut buf = Vec::new();
    buf.extend_from_slice(format!("*{n}\r\n").as_bytes());
    for _ in 0..n {
        buf.extend_from_slice(format!("${elem_len}\r\n").as_bytes());
        buf.extend_from_slice(elem.as_bytes());
        buf.extend_from_slice(b"\r\n");
    }
    buf
}

/// A RESP array of `rows` sub-arrays, each holding `cols` bulk strings of
/// `elem_len` bytes — an `FT.AGGREGATE`-shaped nested reply.
fn build_nested(rows: usize, cols: usize, elem_len: usize) -> Vec<u8> {
    let elem = "x".repeat(elem_len);
    let mut buf = Vec::new();
    buf.extend_from_slice(format!("*{rows}\r\n").as_bytes());
    for _ in 0..rows {
        buf.extend_from_slice(format!("*{cols}\r\n").as_bytes());
        for _ in 0..cols {
            buf.extend_from_slice(format!("${elem_len}\r\n").as_bytes());
            buf.extend_from_slice(elem.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
    }
    buf
}

/// One row of the footprint report: the shape, and the nodes its tape must hold.
struct Shape {
    label: &'static str,
    reply: Vec<u8>,
    /// Expected node count, derived from the shape: one node per element, two per
    /// collection. Cross-checks the measurement against the documented cost model
    /// — a mismatch is an instrumentation error, not a finding.
    expected_nodes: usize,
}

/// A flat RESP array of `n` one-digit integers — 4 wire bytes per element, the
/// smallest an element can be, so the tape's share is at its absolute maximum.
fn build_integer_array(n: usize) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(format!("*{n}\r\n").as_bytes());
    for _ in 0..n {
        buf.extend_from_slice(b":1\r\n");
    }
    buf
}

fn shapes() -> Vec<Shape> {
    vec![
        Shape {
            label: "array 100k x :1 (4B elements: absolute worst case)",
            reply: build_integer_array(100_000),
            expected_nodes: 100_000 + 2,
        },
        Shape {
            label: "array 100k x 8B (SMEMBERS of short ids: worst case)",
            reply: build_array(100_000, 8),
            expected_nodes: 100_000 + 2,
        },
        Shape {
            label: "array 100k x 50B (LRANGE, the shape ARCH-01 cites)",
            reply: build_array(100_000, 50),
            expected_nodes: 100_000 + 2,
        },
        Shape {
            label: "array 1k x 4KiB (large elements: tape must vanish)",
            reply: build_array(1_000, 4096),
            expected_nodes: 1_000 + 2,
        },
        Shape {
            label: "nested 10k x 10 x 20B (FT.AGGREGATE)",
            reply: build_nested(10_000, 10, 20),
            // 10k rows x (2 collection nodes + 10 element nodes), plus the root pair.
            expected_nodes: 10_000 * 12 + 2,
        },
    ]
}

/// Prints the footprint table. Not a timing: these are byte counts, and one
/// measurement of each is exact.
fn report_footprint() {
    println!("\n=== tape_memory/footprint ===");
    println!(
        "{:<48} {:>12} {:>12} {:>7} {:>8} {:>12}",
        "shape", "reply B", "tape B", "tape %", "allocs", "alloc B"
    );
    for shape in shapes() {
        let ((frame_len, tape_bytes), allocs, alloc_bytes) =
            measure_allocs(|| bench_tape_footprint(&shape.reply));
        let expected = shape.expected_nodes * 8;
        assert_eq!(
            tape_bytes, expected,
            "{}: tape is {tape_bytes} B, the cost model says {expected} B",
            shape.label
        );
        let share = 100.0 * tape_bytes as f64 / (frame_len + tape_bytes) as f64;
        println!(
            "{:<48} {frame_len:>12} {tape_bytes:>12} {share:>6.1}% {allocs:>8} {alloc_bytes:>12}",
            shape.label
        );
    }
    println!();
}

/// Prints what a connection keeps pinned after a large reply, frame by frame,
/// under the shipped shrink policy (64 KiB target, factor 8, hysteresis 16).
fn report_retained() {
    println!("=== tape_memory/retained ===");
    println!(
        "  `tail cap` is what the decoder's own BytesMut reports; `live` is what is\n  \
         actually allocated and unfreed. They differ because a split tape leaves the\n  \
         decoder holding a short tail of a block it still pins entirely — so the tail\n  \
         capacity understates the memory held, and `live` is the figure that counts."
    );
    let spike = build_array(100_000, 8);
    let quiet = build_array(4, 8);

    // Both fixtures are built before the baseline so their own bytes do not count
    // as decoder-held memory.
    let mut decoder = BenchDecoder::new();
    let baseline = live_bytes();
    let row = |label: String, decoder: &BenchDecoder| {
        println!(
            "  {label:<30} tail cap {:>9} B   live {:>9} B",
            decoder.retained_tape_capacity(),
            live_bytes().saturating_sub(baseline)
        );
    };
    row("fresh decoder".to_string(), &decoder);
    let spike_tape = decoder.feed(&spike).expect("valid spike reply");
    row(format!("spike, tape {spike_tape} B"), &decoder);

    // The block is only released after enough quiet frames; report every step so
    // the hysteresis is visible rather than asserted.
    for frame in 1..=20 {
        decoder.feed(&quiet).expect("valid quiet reply");
        row(format!("quiet frame {frame}"), &decoder);
    }
    println!();
}

/// Writes then reads back `nodes` nodes of the given width, in a buffer recycled
/// across iterations — the tape's own access pattern, with no parser around it.
/// `u64` is today's node; `u32` is the narrowest plausible alternative.
/// The two loops must differ only in node width: same iteration count, same
/// payload arithmetic, same masking and tag packing. Anything else and the
/// comparison measures the loop bodies rather than the width.
fn drive_nodes_u64(buf: &mut BytesMut, nodes: usize) -> u64 {
    buf.clear();
    for i in 0..nodes {
        let payload = (i as u64).wrapping_mul(13);
        buf.put_u64_le(((b'$' as u64) << 56) | (payload & 0x00FF_FFFF_FFFF_FFFF));
    }
    let mut sum = 0u64;
    for chunk in buf.chunks_exact(8) {
        let word = u64::from_le_bytes(chunk.try_into().unwrap());
        sum = sum.wrapping_add(word & 0x00FF_FFFF_FFFF_FFFF);
    }
    sum
}

fn drive_nodes_u32(buf: &mut BytesMut, nodes: usize) -> u64 {
    buf.clear();
    for i in 0..nodes {
        let payload = (i as u64).wrapping_mul(13);
        buf.put_u32_le(((b'$' as u32) << 24) | (payload as u32 & 0x00FF_FFFF));
    }
    let mut sum = 0u64;
    for chunk in buf.chunks_exact(4) {
        let word = u32::from_le_bytes(chunk.try_into().unwrap());
        sum = sum.wrapping_add((word & 0x00FF_FFFF) as u64);
    }
    sum
}

fn bench_tape_memory(c: &mut Criterion) {
    report_footprint();
    report_retained();

    // --- Per-element cost, at constant element count. ---
    // 50k elements throughout, so the tape is the same 400 KB in every case and
    // only the payload size changes: the curve isolates the fixed per-element
    // cost from the per-byte cost.
    let mut by_len = c.benchmark_group("tape_memory/throughput_by_elem_len");
    for &elem_len in &[8usize, 16, 64, 256, 1024] {
        let reply = build_array(50_000, elem_len);
        by_len.throughput(Throughput::Elements(50_000));
        // `parse_only` builds the tape and drops the frame: the per-element cost
        // the tape is responsible for, with no serde allocation on top.
        by_len.bench_with_input(
            BenchmarkId::new("parse_only", elem_len),
            &reply,
            |b, reply| {
                let mut tape = BenchTape::new();
                b.iter(|| bench_parse_only(black_box(reply), &mut tape))
            },
        );
        // `decode_to` is the full caller-visible path, for scale: one `String`
        // allocation per element dwarfs the tape node and must not be mistaken
        // for it.
        by_len.bench_with_input(
            BenchmarkId::new("decode_to", elem_len),
            &reply,
            |b, reply| {
                b.iter(|| {
                    let out: Vec<String> = bench_decode_to(black_box(reply)).unwrap();
                    black_box(out);
                })
            },
        );
    }
    by_len.finish();

    // --- Ceiling on narrowing the node. ---
    let mut width = c.benchmark_group("tape_memory/node_width");
    for &nodes in &[10_000usize, 100_000] {
        let mut buf = BytesMut::with_capacity(nodes * 8);
        width.throughput(Throughput::Elements(nodes as u64));
        width.bench_with_input(BenchmarkId::new("u64_8B", nodes), &nodes, |b, &nodes| {
            b.iter(|| black_box(drive_nodes_u64(&mut buf, nodes)))
        });
        let mut buf = BytesMut::with_capacity(nodes * 8);
        width.bench_with_input(BenchmarkId::new("u32_4B", nodes), &nodes, |b, &nodes| {
            b.iter(|| black_box(drive_nodes_u32(&mut buf, nodes)))
        });
    }
    width.finish();
}

criterion_group!(benches, bench_tape_memory);
criterion_main!(benches);
