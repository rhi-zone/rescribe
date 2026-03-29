//! Oracle harness: compare vimwiki-fmt parse output against Pandoc.
//!
//! These tests are `#[ignore]` by default. They require:
//!   - `pandoc` binary on PATH (which supports `--from=vimwiki`)
//!
//! Run with:
//!   cargo test -q -p vimwiki-fmt -- --ignored --nocapture
//!
//! The harness reports but does NOT fail on low text coverage — the goal is to
//! catalogue gaps, not gate CI.  Tests DO fail if the parser panics.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use vimwiki_fmt::ast::{Block, Inline, VimwikiDoc};
use vimwiki_fmt::parse;

// -- Path discovery -----------------------------------------------------------

fn find_pandoc() -> Option<PathBuf> {
    if let Ok(out) = Command::new("sh").args(["-c", "command -v pandoc"]).output()
        && out.status.success()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        let p = PathBuf::from(s.trim());
        if p.is_file() {
            return Some(p);
        }
    }
    for prefix in &[
        "/home/me/.nix-profile/bin/pandoc",
        "/run/current-system/sw/bin/pandoc",
        "/nix/var/nix/profiles/default/bin/pandoc",
    ] {
        let p = Path::new(prefix);
        if p.is_file() {
            return Some(p.to_path_buf());
        }
    }
    None
}

// -- Pandoc oracle ------------------------------------------------------------

fn pandoc_to_plain(pandoc: &Path, input: &str) -> Option<String> {
    let out = Command::new(pandoc)
        .args(["--from", "vimwiki", "--to", "plain", "--quiet"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .ok()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(input.as_bytes()).ok()?;
            child.wait_with_output().ok()
        })?;
    if out.status.success() {
        String::from_utf8(out.stdout).ok()
    } else {
        None
    }
}

// -- Text extraction ----------------------------------------------------------

fn words_from_str(s: &str, out: &mut Vec<String>) {
    for w in s.split_whitespace() {
        let word: String = w.chars().filter(|c| c.is_alphabetic()).collect();
        if word.len() >= 2 {
            out.push(word.to_lowercase());
        }
    }
}

fn words_from_inlines(inlines: &[Inline], out: &mut Vec<String>) {
    for inline in inlines {
        match inline {
            Inline::Text(s, _) => words_from_str(s, out),
            Inline::Bold(children, _)
            | Inline::Italic(children, _)
            | Inline::Strikethrough(children, _)
            | Inline::Superscript(children, _)
            | Inline::Subscript(children, _) => words_from_inlines(children, out),
            Inline::Link { label, .. } => words_from_str(label, out),
            Inline::Code(s, _) => words_from_str(s, out),
            Inline::Image { .. } => {}
        }
    }
}

fn words_from_blocks(blocks: &[Block], out: &mut Vec<String>) {
    for block in blocks {
        match block {
            Block::Paragraph { inlines, .. }
            | Block::Heading { inlines, .. }
            | Block::Blockquote { inlines, .. } => {
                words_from_inlines(inlines, out);
            }
            Block::CodeBlock { content, .. } => words_from_str(content, out),
            Block::List { items, .. } => {
                for item in items {
                    words_from_inlines(&item.inlines, out);
                }
            }
            Block::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        words_from_inlines(cell, out);
                    }
                }
            }
            Block::DefinitionList { items, .. } => {
                for item in items {
                    words_from_inlines(&item.term, out);
                    words_from_inlines(&item.desc, out);
                }
            }
            Block::HorizontalRule { .. } => {}
        }
    }
}

fn extract_words(doc: &VimwikiDoc) -> Vec<String> {
    let mut out = Vec::new();
    words_from_blocks(&doc.blocks, &mut out);
    out
}

// -- Coverage computation -----------------------------------------------------

fn word_coverage(reference: &[String], ours: &[String]) -> f64 {
    if reference.is_empty() {
        return 1.0;
    }
    let mut ref_counts: HashMap<&str, usize> = HashMap::new();
    for w in reference {
        *ref_counts.entry(w.as_str()).or_default() += 1;
    }
    let mut our_counts: HashMap<&str, usize> = HashMap::new();
    for w in ours {
        *our_counts.entry(w.as_str()).or_default() += 1;
    }
    let matched: usize = ref_counts
        .iter()
        .map(|(w, &rc)| rc.min(*our_counts.get(*w).unwrap_or(&0)))
        .sum();
    matched as f64 / reference.len() as f64
}

// -- Tests --------------------------------------------------------------------

/// Run the oracle harness against the built-in sample.
#[test]
#[ignore]
fn pandoc_vimwiki_oracle() {
    let sample = include_str!("../../../../fixtures/vimwiki/oracle/input.wiki");

    let pandoc = match find_pandoc() {
        Some(p) => p,
        None => {
            eprintln!("SKIP: pandoc not found on PATH");
            return;
        }
    };

    // Parse — must not panic.
    let (doc, _diags) = parse(sample);
    let our_words = extract_words(&doc);

    match pandoc_to_plain(&pandoc, sample) {
        None => {
            eprintln!("SKIP: pandoc failed to convert vimwiki input");
        }
        Some(plain_text) => {
            let ref_words: Vec<String> = plain_text
                .split_whitespace()
                .map(|w| {
                    let word: String = w.chars().filter(|c| c.is_alphabetic()).collect();
                    word.to_lowercase()
                })
                .filter(|w| w.len() >= 2)
                .collect();

            let cov = word_coverage(&ref_words, &our_words);
            let pct = (cov * 100.0).round() as u32;
            eprintln!(
                "Oracle coverage: {pct}%  (ref={} ours={})",
                ref_words.len(),
                our_words.len()
            );
        }
    }
}

/// Smoke test: parse the built-in representative VimWiki sample without
/// panicking. Runs in normal CI (not #[ignore]).
#[test]
fn parse_sample_no_panic() {
    let sample = include_str!("../../../../fixtures/vimwiki/oracle/input.wiki");
    let (doc, _diags) = parse(sample);
    // Must produce at least one block.
    assert!(
        !doc.blocks.is_empty(),
        "expected at least one block from sample input"
    );
}
