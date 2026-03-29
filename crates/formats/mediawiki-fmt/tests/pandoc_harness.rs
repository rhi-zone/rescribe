//! Oracle harness: compare mediawiki-fmt parse output against Pandoc.
//!
//! These tests are `#[ignore]` by default. They require:
//!   - `~/git/pandoc/test/mediawiki-reader.wiki` (GPL corpus, never committed)
//!   - `pandoc` binary on PATH
//!
//! Run with:
//!   cargo test -q -p mediawiki-fmt -- --ignored --nocapture
//!
//! The harness reports but does NOT fail on low text coverage -- the goal is to
//! catalogue gaps, not gate CI. Tests DO fail if the parser panics.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use mediawiki_fmt::ast::{Block, Inline, MediawikiDoc};
use mediawiki_fmt::parse;

// -- Path discovery -----------------------------------------------------------

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/home/me".into()))
}

fn corpus_dir() -> Option<PathBuf> {
    let p = home_dir().join("git/pandoc/test");
    p.is_dir().then_some(p)
}

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

fn pandoc_to_plain(pandoc: &Path, file: &Path) -> Option<String> {
    let out = Command::new(pandoc)
        .args(["--from", "mediawiki", "--to", "plain", "--quiet"])
        .arg(file)
        .output()
        .ok()?;
    if out.status.success() { String::from_utf8(out.stdout).ok() } else { None }
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
            Inline::Text(s) => words_from_str(s, out),
            Inline::Bold(children)
            | Inline::Italic(children)
            | Inline::Underline(children)
            | Inline::Strikeout(children)
            | Inline::Superscript(children)
            | Inline::Subscript(children) => words_from_inlines(children, out),
            Inline::Code(s) => words_from_str(s, out),
            Inline::Link { text, .. } => words_from_str(text, out),
            Inline::FootnoteRef { content: Some(c), .. } => words_from_str(c, out),
            Inline::MathInline { source } => words_from_str(source, out),
            Inline::Template { content } => words_from_str(content, out),
            Inline::Nowiki { content } => words_from_str(content, out),
            Inline::Image { .. }
            | Inline::LineBreak
            | Inline::FootnoteRef { content: None, .. } => {}
        }
    }
}

fn words_from_blocks(blocks: &[Block], out: &mut Vec<String>) {
    for block in blocks {
        match block {
            Block::Paragraph { inlines, .. } | Block::Heading { inlines, .. } => {
                words_from_inlines(inlines, out);
            }
            Block::CodeBlock { content, .. } | Block::PreBlock { content, .. } => {
                words_from_str(content, out);
            }
            Block::Blockquote { children, .. } => words_from_blocks(children, out),
            Block::List { items, .. } => {
                for item in items {
                    words_from_blocks(item, out);
                }
            }
            Block::DefinitionList { items, .. } => {
                for item in items {
                    words_from_inlines(&item.term, out);
                    words_from_inlines(&item.desc, out);
                }
            }
            Block::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        words_from_inlines(&cell.inlines, out);
                    }
                }
            }
            Block::HorizontalRule | Block::RawBlock { .. } => {}
        }
    }
}

fn extract_words(doc: &MediawikiDoc) -> Vec<String> {
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

fn missing_words(reference: &[String], ours: &[String]) -> Vec<(String, usize)> {
    let mut ref_counts: HashMap<&str, usize> = HashMap::new();
    for w in reference {
        *ref_counts.entry(w.as_str()).or_default() += 1;
    }
    let mut our_counts: HashMap<&str, usize> = HashMap::new();
    for w in ours {
        *our_counts.entry(w.as_str()).or_default() += 1;
    }
    let mut missing: Vec<(String, usize)> = ref_counts
        .iter()
        .filter_map(|(w, &rc)| {
            let oc = *our_counts.get(*w).unwrap_or(&0);
            if oc < rc { Some((w.to_string(), rc - oc)) } else { None }
        })
        .collect();
    missing.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    missing
}

// -- Harness runner -----------------------------------------------------------

struct HarnessFile {
    filename: &'static str,
}

const CORPUS_FILES: &[HarnessFile] = &[
    HarnessFile { filename: "mediawiki-reader.wiki" },
];

fn run_harness(files: &[HarnessFile]) {
    let Some(corpus) = corpus_dir() else {
        eprintln!("SKIP: ~/git/pandoc/test/ not found");
        return;
    };

    let pandoc = find_pandoc();
    if pandoc.is_none() {
        eprintln!("NOTE: pandoc not found on PATH -- coverage comparison disabled");
    }

    eprintln!();
    eprintln!("{:<38} {:<8} Coverage", "File", "Parse");
    eprintln!("{}", "-".repeat(70));

    for entry in files {
        let file = corpus.join(entry.filename);

        let input = match std::fs::read_to_string(&file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{:<38} SKIP     ({})", entry.filename, e);
                continue;
            }
        };

        let (doc, diags) = parse(&input);

        let parse_col = "OK";
        let our_words = extract_words(&doc);

        let cov_col = if let Some(ref pbin) = pandoc {
            match pandoc_to_plain(pbin, &file) {
                None => "---  (pandoc failed)".to_string(),
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
                    let missing = missing_words(&ref_words, &our_words);

                    let result = format!(
                        "{pct:3}%  (ref={} ours={})",
                        ref_words.len(),
                        our_words.len()
                    );

                    eprintln!("{:<38} {:<8} {}", entry.filename, parse_col, result);

                    if !diags.is_empty() {
                        eprintln!("  diagnostics: {} warning(s)", diags.len());
                    }

                    if !missing.is_empty() {
                        let missing_str: Vec<String> = missing
                            .iter()
                            .take(20)
                            .map(|(w, n)| if *n > 1 { format!("{w}(x{n})") } else { w.clone() })
                            .collect();
                        eprintln!("  missing: {}", missing_str.join(", "));
                    }

                    continue;
                }
            }
        } else {
            format!("---  (pandoc not found, ours={})", our_words.len())
        };

        eprintln!("{:<38} {:<8} {}", entry.filename, parse_col, cov_col);
        if !diags.is_empty() {
            eprintln!("  diagnostics: {} warning(s)", diags.len());
        }
    }

    eprintln!("{}", "-".repeat(70));
}

// -- Tests --------------------------------------------------------------------

/// Run the oracle harness against the Pandoc mediawiki corpus files.
#[test]
#[ignore]
fn pandoc_mediawiki_corpus() {
    run_harness(CORPUS_FILES);
}

/// Smoke test: parse the built-in representative MediaWiki sample without
/// panicking. Runs in normal CI (not #[ignore]).
#[test]
fn parse_sample_no_panic() {
    let sample = include_str!("../../../../fixtures/mediawiki/oracle/input.mediawiki");
    let (doc, _diags) = parse(sample);
    assert!(!doc.blocks.is_empty(), "expected at least one block from sample input");
}
