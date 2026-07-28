//! Guard against the exact failure that let `events.rs`/`batch.rs`/`writer.rs`
//! silently rot: merge commit `383d4e6adf` dropped the `mod events; mod
//! batch; mod writer;` lines from `lib.rs` while resolving an unrelated
//! merge conflict. The three files stayed on disk, fully formed, but were
//! invisible to `cargo build`/`cargo test`/`cargo clippy` — none of them
//! error on a `.rs` file that simply isn't part of the module tree. The
//! crate was signed off as 5-Production (reader-streaming, reader-batch,
//! writer-streaming all "done") while three of its five required APIs had
//! been unreachable for months.
//!
//! This test walks `src/` from `lib.rs`, follows every file-backed `mod
//! name;` declaration, and fails if any `.rs` file under `src/` is not
//! reached. It is a heuristic (it does not parse Rust, just greps for `mod`
//! declarations), but it is sufficient to catch "this file exists but
//! nothing declares it" — which is exactly what a compiler can never catch.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn mod_declarations(text: &str) -> Vec<String> {
    // Matches `mod foo;`, `pub mod foo;`, `pub(crate) mod foo;` etc. — any
    // *file-backed* module declaration (no `{ ... }` body). Deliberately not
    // a full parser: good enough to catch "nothing declares this file".
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("mod ").or_else(|| {
            line.strip_prefix("pub ")
                .map(str::trim_start)
                .and_then(|r| r.strip_prefix("mod "))
        }) else {
            // Handle `pub(crate) mod foo;` / `pub(super) mod foo;`.
            let Some(after_pub) = line.strip_prefix("pub(") else {
                continue;
            };
            let Some(close) = after_pub.find(')') else {
                continue;
            };
            let after = after_pub[close + 1..].trim_start();
            let Some(rest) = after.strip_prefix("mod ") else {
                continue;
            };
            if let Some(name) = extract_file_backed_name(rest) {
                out.push(name);
            }
            continue;
        };
        if let Some(name) = extract_file_backed_name(rest) {
            out.push(name);
        }
    }
    out
}

/// Given the text after `mod `, return the module name if this is a
/// file-backed declaration (`name;`), or `None` for an inline body
/// (`name { ... }`) or anything else that doesn't end in `;` on this line.
fn extract_file_backed_name(rest: &str) -> Option<String> {
    let rest = rest.trim_end();
    let name_end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    let name = &rest[..name_end];
    let after_name = rest[name_end..].trim_start();
    if after_name.starts_with(';') {
        Some(name.to_string())
    } else {
        None
    }
}

fn resolve(child_dir: &Path, name: &str) -> Option<PathBuf> {
    let flat = child_dir.join(format!("{name}.rs"));
    if flat.is_file() {
        return Some(flat);
    }
    let nested = child_dir.join(name).join("mod.rs");
    if nested.is_file() {
        return Some(nested);
    }
    None
}

fn collect_reachable(entry: &Path) -> BTreeSet<PathBuf> {
    let mut reachable = BTreeSet::new();
    let mut stack = vec![entry.to_path_buf()];
    while let Some(path) = stack.pop() {
        if reachable.contains(&path) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            reachable.insert(path);
            continue;
        };
        reachable.insert(path.clone());
        let dir = path.parent().unwrap().to_path_buf();
        let base = path.file_name().unwrap().to_string_lossy().to_string();
        let child_dir = if matches!(base.as_str(), "lib.rs" | "main.rs" | "mod.rs") {
            dir
        } else {
            dir.join(path.file_stem().unwrap())
        };
        for name in mod_declarations(&text) {
            if let Some(resolved) = resolve(&child_dir, &name) {
                stack.push(resolved);
            }
        }
    }
    reachable
}

fn all_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out
}

#[test]
fn every_src_file_is_reachable_from_lib_rs() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let entry = src_dir.join("lib.rs");
    assert!(entry.is_file(), "expected {entry:?} to exist");

    let reachable = collect_reachable(&entry);
    let all = all_rs_files(&src_dir);

    let orphans: Vec<&PathBuf> = all.iter().filter(|f| !reachable.contains(*f)).collect();

    assert!(
        orphans.is_empty(),
        "orphaned .rs file(s) under src/ not reachable from lib.rs via `mod` \
         declarations — this is exactly the failure mode that let events.rs/\
         batch.rs/writer.rs silently rot after commit 383d4e6adf dropped their \
         `mod` lines. Files that exist on disk but aren't wired into the module \
         tree are invisible to cargo build/test/clippy: {orphans:#?}"
    );
}
