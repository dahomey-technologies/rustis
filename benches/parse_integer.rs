use criterion::{Criterion, criterion_group, criterion_main};
use memchr::memchr;
use std::hint::black_box;

#[derive(Debug)]
pub enum Error {
    EOF,
    CannotParseInteger,
}

// --- VERSION 1 : MEMCHR + ATOI ---
fn parse_integer_atoi(buf: &[u8], pos: &mut usize) -> Result<i64, Error> {
    let rem = &buf[*pos..];
    let i = memchr(b'\r', rem).ok_or(Error::EOF)?;
    if i + 1 >= rem.len() || rem[i + 1] != b'\n' {
        return Err(Error::EOF);
    }

    let val = atoi::atoi::<i64>(&rem[..i]).ok_or(Error::CannotParseInteger)?;

    *pos += i + 2;
    Ok(val)
}

// --- VERSION 2 : MANUAL (*10) ---
fn parse_integer_manual(buf: &[u8], pos: &mut usize) -> Result<i64, Error> {
    let mut n = 0i64;
    let b_slice = &buf[*pos..];

    for (i, &b) in b_slice.iter().enumerate() {
        match b {
            b'0'..=b'9' => {
                n = n.wrapping_mul(10).wrapping_add((b - b'0') as i64);
            }
            b'\r' => {
                if i + 1 < b_slice.len() && b_slice[i + 1] == b'\n' {
                    *pos += i + 2;
                    return Ok(n);
                }
                return Err(Error::CannotParseInteger);
            }
            _ => return Err(Error::CannotParseInteger),
        }
    }
    Err(Error::EOF)
}

fn criterion_benchmark(c: &mut Criterion) {
    // On teste sur un petit entier (cas typique des longueurs RESP)
    let data_small = b"12\r\n";
    // On teste sur un grand entier
    let data_large = b"1234567890\r\n";

    let mut group = c.benchmark_group("Integer Parsing");

    group.bench_function("atoi_small", |b| {
        b.iter(|| {
            let mut pos = 0;
            let _ = parse_integer_atoi(black_box(data_small), &mut pos);
        })
    });

    group.bench_function("manual_small", |b| {
        b.iter(|| {
            let mut pos = 0;
            let _ = parse_integer_manual(black_box(data_small), &mut pos);
        })
    });

    group.bench_function("atoi_large", |b| {
        b.iter(|| {
            let mut pos = 0;
            let _ = parse_integer_atoi(black_box(data_large), &mut pos);
        })
    });

    group.bench_function("manual_large", |b| {
        b.iter(|| {
            let mut pos = 0;
            let _ = parse_integer_manual(black_box(data_large), &mut pos);
        })
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
