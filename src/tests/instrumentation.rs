use std::{fs, path::Path};

/// Directories whose logging is meant to carry connection context.
const INSTRUMENTED_DIRS: [&str; 2] = ["src/network", "src/client"];

/// Reads every `.rs` file under `dir`, relative to the crate root.
fn source_files(dir: &str) -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let mut files = Vec::new();

    for entry in fs::read_dir(&root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            let name = path.strip_prefix(env!("CARGO_MANIFEST_DIR")).unwrap();
            files.push((
                name.display().to_string(),
                fs::read_to_string(&path).unwrap(),
            ));
        }
    }

    assert!(!files.is_empty(), "no source file found under {dir}");
    files
}

/// The connection tag identifies which connection an event came from. It belongs
/// to the surrounding span, where a subscriber can read it as a field and a
/// downstream collector can index it.
///
/// Written into the message instead, it becomes an opaque `[…] ` prefix that
/// only a human re-reading text can use — and it has to be repeated at every
/// call site, which is how it drifts. This test is what keeps it out of the
/// messages once the spans carry it.
#[test]
fn the_connection_tag_never_goes_back_into_a_message() {
    let mut offenders = Vec::new();

    for dir in INSTRUMENTED_DIRS {
        for (name, source) in source_files(dir) {
            for (index, line) in source.lines().enumerate() {
                // The manual form: a `[{}]`/`[{tag}]` prefix opening a log
                // message, fed by the connection tag.
                if line.contains("[{}]") || line.contains("[{tag}]") {
                    offenders.push(format!("{name}:{}: {}", index + 1, line.trim()));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "the connection tag must be a span field, not a message prefix:\n{}",
        offenders.join("\n")
    );
}
