//! Records the shape of every reply the suite observes, next to the response
//! type the caller declared for it.
//!
//! `Response` is a blanket impl over every `Deserialize` type, so nothing at
//! compile time relates a command's declared `R` to what the server answers,
//! and `RespDeserializer` coerces rather than refuses: a `Null` read as `bool`
//! is `false`, read as an integer it is `0`. A wrong `R` therefore returns a
//! plausible value instead of failing, and no amount of reading the command
//! declaration shows it. Only a live reply does.
//!
//! `Client::send` is the single point that holds both the command name and the
//! declared `R`, so the probe sits there and the existing per-command tests
//! supply the observations. `response_shape.rs` reads the dump back and applies
//! the compatibility rules.

use crate::resp::{Command, RespResponse, RespView};
use std::{
    collections::BTreeSet,
    io::Write,
    sync::{LazyLock, Mutex},
};

/// One observation: what was sent, what type the caller asked for, what came
/// back, and whether the type could decode it. The set deduplicates, so the
/// file holds distinct combinations rather than one line per call.
type Observation = (String, String, String, &'static str);

static OBSERVATIONS: LazyLock<Mutex<BTreeSet<Observation>>> =
    LazyLock::new(|| Mutex::new(BTreeSet::new()));

/// Commands whose first argument selects the subcommand. `CLUSTER INFO` and
/// `CLUSTER SHARDS` answer nothing alike, so the container name alone would
/// collapse observations that must stay apart.
const CONTAINER_COMMANDS: &[&[u8]] = &[
    b"ACL",
    b"CLIENT",
    b"CLUSTER",
    b"COMMAND",
    b"CONFIG",
    b"DEBUG",
    b"FUNCTION",
    b"LATENCY",
    b"MEMORY",
    b"MODULE",
    b"OBJECT",
    b"PUBSUB",
    b"SCRIPT",
    b"SENTINEL",
    b"SLOWLOG",
    b"XGROUP",
    b"XINFO",
];

/// The command's identity for the report: its name, plus the subcommand when
/// the name alone does not determine the reply.
pub(crate) fn label(command: &Command) -> String {
    let name = String::from_utf8_lossy(command.name()).to_uppercase();

    if CONTAINER_COMMANDS.contains(&name.as_bytes())
        && let Some(arg) = command.get_arg(0)
    {
        return format!("{name} {}", String::from_utf8_lossy(&arg).to_uppercase());
    }

    name
}

/// Classifies a reply by RESP kind, keeping the distinctions the deserializer
/// acts on: `+OK` against any other simple string, because `deserialize_bool`
/// reads the first as `true` and the second as `false`; `0`/`1` against any
/// other integer, for the same reason.
fn kind(response: &RespResponse) -> String {
    let Ok(view) = response.view() else {
        return "Unreadable".to_owned();
    };

    match view {
        RespView::SimpleString(ss) if ss == b"OK" => "SimpleString(OK)".to_owned(),
        RespView::SimpleString(_) => "SimpleString".to_owned(),
        RespView::Integer(i, _) if i == 0 || i == 1 => format!("Integer({i})"),
        RespView::Integer(..) => "Integer".to_owned(),
        RespView::Double(..) => "Double".to_owned(),
        RespView::BulkString(_) => "BulkString".to_owned(),
        RespView::Boolean(_) => "Boolean".to_owned(),
        RespView::IntegerArray(a) => empty_or("IntegerArray", a.is_empty()),
        RespView::OwnedArray(a) => empty_or("Array", a.is_empty()),
        RespView::Array(c) => empty_or("Array", c.len() == 0),
        RespView::Map(c) => empty_or("Map", c.len() == 0),
        RespView::Set(c) => empty_or("Set", c.len() == 0),
        RespView::Push(_) => "Push".to_owned(),
        RespView::Error(_) => "Error".to_owned(),
        RespView::Null => "Null".to_owned(),
    }
}

/// An empty collection is its own kind: `deserialize_option` maps it to `None`
/// exactly as it maps `Null`, so a type that only ever sees the empty case has
/// been proven against nothing.
fn empty_or(name: &str, is_empty: bool) -> String {
    if is_empty {
        format!("Empty{name}")
    } else {
        name.to_owned()
    }
}

/// Records one observation. Only a combination never seen before touches the
/// file, so a suite of a million calls writes a few hundred times.
pub(crate) fn record(
    label: String,
    declared: &'static str,
    response: &RespResponse,
    decoded: bool,
) {
    let observation = (
        label,
        normalize_declared(declared),
        kind(response),
        if decoded { "decoded" } else { "refused" },
    );

    let Ok(mut observations) = OBSERVATIONS.lock() else {
        return;
    };

    if observations.insert(observation) {
        flush(&observations);
    }
}

/// Strips the crate and module path `type_name` prints, so `R` reads as it does
/// in the command declaration.
fn normalize_declared(declared: &str) -> String {
    let mut out = String::with_capacity(declared.len());
    let mut segment_start = 0;

    for (i, c) in declared.char_indices() {
        if !is_path_char(c) {
            out.push_str(last_segment(&declared[segment_start..i]));
            out.push(c);
            segment_start = i + c.len_utf8();
        }
    }
    out.push_str(last_segment(&declared[segment_start..]));

    out
}

fn is_path_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == ':'
}

fn last_segment(path: &str) -> &str {
    match path.rsplit_once("::") {
        Some((_, tail)) => tail,
        None => path,
    }
}

/// Written on every new observation rather than at process exit: the test
/// harness offers no end-of-run hook, and a dump produced only by a clean exit
/// is missing precisely the run that found a panic.
fn flush(observations: &BTreeSet<Observation>) {
    let mut content = String::new();
    for (label, declared, kind, outcome) in observations {
        content.push_str(label);
        content.push('\t');
        content.push_str(declared);
        content.push('\t');
        content.push_str(kind);
        content.push('\t');
        content.push_str(outcome);
        content.push('\n');
    }

    let path = dump_path();
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::File::create(&path) {
        let _ = file.write_all(content.as_bytes());
    }
}

pub(crate) fn dump_path() -> String {
    std::env::var("RUSTIS_RESPONSE_PROBE")
        .unwrap_or_else(|_| "target/response_shape.tsv".to_owned())
}
