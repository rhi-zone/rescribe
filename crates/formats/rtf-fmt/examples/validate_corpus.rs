//! Parse a directory of RTF files and report diagnostics statistics.
//!
//! Usage:
//!   cargo run -p rtf-fmt --example validate_corpus -- /path/to/rtf/files

use rtf_fmt::parse;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn collect_rtf_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(d: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(d) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().and_then(|x| x.to_str()) == Some("rtf") {
                    out.push(p);
                }
            }
        }
    }
    walk(dir, &mut out);
    out
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/govdocs_rtf".into());
    let files = collect_rtf_files(Path::new(&dir));

    let mut total = 0usize;
    let mut with_diag = 0usize;
    let mut empty_output = 0usize;
    let mut unknown_words: HashMap<String, usize> = HashMap::new();
    let mut parse_errors = 0usize;

    for path in &files {
        let content = match std::fs::read(path) {
            Ok(c) => c,
            Err(_) => {
                parse_errors += 1;
                continue;
            }
        };
        let input = match String::from_utf8(content) {
            Ok(s) => s,
            Err(_) => {
                parse_errors += 1;
                continue;
            }
        };
        let (doc, diags) = parse(&input);
        total += 1;
        if !diags.is_empty() {
            with_diag += 1;
            for d in &diags {
                if let Some(word) = d.message.strip_prefix("unknown control word: \\") {
                    *unknown_words.entry(word.to_string()).or_insert(0) += 1;
                }
            }
        }
        if doc.blocks.is_empty() {
            empty_output += 1;
        }
    }

    println!("{total} files parsed");
    if parse_errors > 0 {
        println!("{parse_errors} files failed to read");
    }
    println!(
        "{with_diag} files ({:.0}%) with diagnostics",
        with_diag * 100 / total.max(1)
    );
    println!(
        "{empty_output} files ({:.0}%) with empty parsed output",
        empty_output * 100 / total.max(1)
    );
    println!();

    let mut words: Vec<_> = unknown_words.into_iter().collect();
    words.sort_by(|a, b| b.1.cmp(&a.1));
    println!("Top unknown control words (by file count):");
    for (word, count) in words.iter().take(40) {
        println!("  {count:5}  \\{word}");
    }
}
