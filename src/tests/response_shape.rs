//! Confronts every declared response type with the reply shape the server
//! actually sent for it.
//!
//! `response_probe` writes one line per distinct
//! `(command, declared R, observed RESP kind)` while the suite runs. This
//! module reads that dump back and applies the rules below, which encode what
//! `RespDeserializer` does rather than what the command's doc-comment claims:
//! a `Null` read as `bool` is `false`, read as an integer it is `0`, and a
//! reply read as `()` is discarded whatever it carried. Each of those is a way
//! for a wrong `R` to return a plausible value forever.
//!
//! It runs as a second invocation, after a full suite run has produced the
//! dump, because the harness gives no ordering guarantee that would let one run
//! both collect and report:
//!
//! ```text
//! ./run_tests.sh
//! RUSTIS_RESPONSE_SHAPE_REPORT=1 ./run_tests.sh response_shape
//! ```
//!
//! # It reports, it does not assert
//!
//! What this reads is not a property of the code but the set of
//! `(command, declared R, observed kind)` triples the suite happened to reach,
//! and some shapes appear only in a state the run may or may not enter: a
//! blocking pop that times out, a `LPOP` on a list that is absent this time, a
//! `SET NX` that loses the race. The three rows of that kind in the baseline
//! were each added after one showed up. So an inedited row means "nobody has
//! looked at this shape yet", not "the code broke" -- and failing on it made the
//! job red at random, roughly one run in three, for a finding that needs a human
//! reading rather than a build verdict. Everything below therefore prints and
//! returns; `Report response shapes` in the workflow surfaces it.

use crate::tests::response_probe::dump_path;
use serial_test::serial;
use std::collections::BTreeSet;

/// A row the rules accept, with the reason it is accepted. Anything reported and
/// not listed here is printed as unexplained.
const BASELINE: &str = include_str!("response_shape_baseline.tsv");

/// What a declared `R` reduces to for the purpose of judging a reply.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Category {
    /// `()` — the reply is discarded, whatever it holds.
    Unit,
    Bool,
    Int,
    Float,
    Str,
    Collection,
    /// A struct or enum decoded from the reply.
    Custom,
    /// `Value` and friends: shape is the caller's business, never wrong here.
    Any,
}

/// A declared `R` reduced to what judging it needs: its category, and whether
/// it was wrapped in an `Option` — which is the type saying an absent reply is
/// expected, so a null stops being evidence of anything.
struct Declared {
    category: Category,
    optional: bool,
}

fn categorize(declared: &str) -> Declared {
    let declared = declared.trim();

    if let Some(inner) = declared
        .strip_prefix("Option<")
        .and_then(|d| d.strip_suffix('>'))
    {
        return Declared {
            category: categorize(inner).category,
            optional: true,
        };
    }

    Declared {
        category: category_of(declared),
        optional: false,
    }
}

fn category_of(declared: &str) -> Category {
    // A tuple is read off a collection whatever its members are.
    if declared.starts_with('(') && declared != "()" {
        return Category::Collection;
    }

    if let Some((head, _)) = declared.split_once('<') {
        return match head {
            "Vec" | "VecDeque" | "HashSet" | "BTreeSet" | "HashMap" | "BTreeMap" => {
                Category::Collection
            }
            _ => Category::Custom,
        };
    }

    match declared {
        "()" => Category::Unit,
        "bool" => Category::Bool,
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128"
        | "isize" => Category::Int,
        "f32" | "f64" => Category::Float,
        "String" | "BulkString" | "str" | "char" => Category::Str,
        "Value" => Category::Any,
        _ => Category::Custom,
    }
}

/// The reply kinds a declared type can decode without silently inventing a
/// value.
fn accepts(declared: &Declared, kind: &str) -> bool {
    // An empty collection is judged as the collection it is; the emptiness only
    // qualifies how much the observation proves.
    let base = kind.strip_prefix("Empty").unwrap_or(kind);

    // A null is what `Option` exists for, and nothing else here tolerates it:
    // every non-optional category turns it into a default — `false`, `0`, an
    // empty string — with no way for the caller to tell.
    if base == "Null" {
        return declared.optional || matches!(declared.category, Category::Unit | Category::Any);
    }

    match declared.category {
        // `Value` describes whatever came, null included.
        Category::Any => true,
        // Only `+OK` carries nothing. Anything else was data, and `()` threw it
        // away — the `cluster_getkeysinslot` defect exactly.
        Category::Unit => kind == "SimpleString(OK)",
        Category::Bool => matches!(
            kind,
            "Integer(0)" | "Integer(1)" | "Boolean" | "SimpleString(OK)"
        ),
        // Redis answers numbers as bulk strings as readily as as integers
        // (`GET` on a counter), and the deserializer parses the digits.
        Category::Int => base.starts_with("Integer") || base == "BulkString",
        Category::Float => base.starts_with("Integer") || matches!(base, "Double" | "BulkString"),
        Category::Str => matches!(base, "BulkString" | "SimpleString" | "SimpleString(OK)"),
        Category::Collection => matches!(base, "Array" | "IntegerArray" | "Set" | "Map" | "Push"),
        Category::Custom => matches!(
            base,
            "Array" | "Map" | "Set" | "BulkString" | "SimpleString" | "SimpleString(OK)"
        ),
    }
}

/// A row of the dump.
struct Row {
    command: String,
    declared: String,
    kind: String,
    /// `false` when the declared type refused the reply. Such a mismatch is
    /// loud — the caller got an error — and it is not what this report hunts.
    decoded: bool,
}

fn parse(content: &str) -> Vec<Row> {
    content
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut fields = line.split('\t');
            Some(Row {
                command: fields.next()?.to_owned(),
                declared: fields.next()?.to_owned(),
                kind: fields.next()?.trim().to_owned(),
                decoded: fields.next().is_none_or(|o| o.trim() != "refused"),
            })
        })
        .collect()
}

fn key(row: &Row) -> String {
    format!("{}\t{}\t{}", row.command, row.declared, row.kind)
}

/// `#[serial]` because the dump is a file every other test rewrites as it
/// records: read while the suite is running, it is a snapshot of a moving
/// target. Under `--test-threads=1` that cannot happen, but a bare
/// `RUSTIS_RESPONSE_SHAPE_REPORT=1 cargo test` -- the obvious thing to type --
/// runs this against a file being rewritten under it.
#[test]
#[serial]
fn response_shape_report() {
    if std::env::var("RUSTIS_RESPONSE_SHAPE_REPORT").is_err() {
        println!(
            "skipped: set RUSTIS_RESPONSE_SHAPE_REPORT=1 and run after a full suite run, \
             which is what fills {}",
            dump_path()
        );
        return;
    }

    let path = dump_path();
    let Ok(content) = std::fs::read_to_string(&path) else {
        println!("no probe dump at {path}: run the whole suite first, it is what writes it");
        return;
    };

    let rows = parse(&content);
    if rows.is_empty() {
        println!("{path} holds no observation: the suite recorded nothing");
        return;
    }

    let accepted: BTreeSet<String> = parse(BASELINE).iter().map(key).collect();

    let mut unexplained = Vec::new();
    for row in &rows {
        // A command that answered an error made no claim about its shape.
        if row.kind == "Error" {
            continue;
        }
        // The type refused the reply, so the caller was told rather than handed
        // a coerced value. That is a negative test, not a silent mismatch.
        if !row.decoded {
            continue;
        }
        if accepts(&categorize(&row.declared), &row.kind) {
            continue;
        }
        if accepted.contains(&key(row)) {
            continue;
        }
        unexplained.push(row);
    }

    println!(
        "{} observations, {} unexplained",
        rows.len(),
        unexplained.len()
    );

    if !unexplained.is_empty() {
        let mut report = String::from(
            "declared response types the server's reply contradicts.\n\
             Read each against COMMAND DOCS or the raw reply, then either fix the \
             type or add the row to response_shape_baseline.tsv with its reason.\n\n",
        );
        for row in &unexplained {
            report.push_str(&format!(
                "  {:<32} {:<40} answered {}\n",
                row.command, row.declared, row.kind
            ));
        }
        // Printed rather than panicked: see the module's "It reports, it does not
        // assert". The count on the line above is what the workflow reads to
        // decide whether there is anything to surface.
        println!("{report}");
    }
}
