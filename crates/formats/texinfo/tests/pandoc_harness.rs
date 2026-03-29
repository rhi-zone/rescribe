//! Oracle harness: compare texinfo parse output against Pandoc.
//!
//! These tests are `#[ignore]` by default. They require:
//!   - `pandoc` binary on PATH with texinfo input support
//!
//! Run with:
//!   cargo test -q -p texinfo -- --ignored --nocapture
//!
//! The harness reports but does NOT fail on low text coverage — the goal is to
//! catalogue gaps, not gate CI.  Tests DO fail if the parser panics.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use texinfo::ast::{Block, Inline, TexinfoDoc};
use texinfo::parse;

// ── Path discovery ────────────────────────────────────────────────────────────

fn find_pandoc() -> Option<PathBuf> {
    if let Ok(out) = Command::new("sh")
        .args(["-c", "command -v pandoc"])
        .output()
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

fn pandoc_supports_texinfo() -> bool {
    if let Some(pandoc) = find_pandoc()
        && let Ok(out) = Command::new(&pandoc)
            .args(["--list-input-formats"])
            .output()
    {
        let formats = String::from_utf8_lossy(&out.stdout);
        return formats.lines().any(|l| l.trim() == "texinfo");
    }
    false
}

// ── Text extraction from AST ─────────────────────────────────────────────────

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
            Inline::Strong(children, _)
            | Inline::Emphasis(children, _)
            | Inline::Var(children, _)
            | Inline::Dfn(children, _)
            | Inline::DirectItalic(children, _)
            | Inline::DirectBold(children, _) => words_from_inlines(children, out),
            Inline::Link { children, .. } => words_from_inlines(children, out),
            Inline::Code(s, _)
            | Inline::Samp(s, _)
            | Inline::Kbd(s, _)
            | Inline::Key(s, _)
            | Inline::File(s, _)
            | Inline::Command(s, _)
            | Inline::Option(s, _)
            | Inline::Env(s, _)
            | Inline::Cite(s, _)
            | Inline::Roman(s, _)
            | Inline::SmallCaps(s, _)
            | Inline::DirectTypewriter(s, _)
            | Inline::NoBreak(s, _) => words_from_str(s, out),
            Inline::Acronym { abbrev, .. } | Inline::Abbr { abbrev, .. } => {
                words_from_str(abbrev, out);
            }
            Inline::FootnoteDef { content, .. } => words_from_inlines(content, out),
            Inline::Superscript(children, _) | Inline::Subscript(children, _) => {
                words_from_inlines(children, out);
            }
            Inline::CrossRef { node, .. } => words_from_str(node, out),
            _ => {}
        }
    }
}

fn words_from_blocks(blocks: &[Block], out: &mut Vec<String>) {
    for block in blocks {
        match block {
            Block::Heading { inlines, .. } | Block::Paragraph { inlines, .. } => {
                words_from_inlines(inlines, out);
            }
            Block::CodeBlock { content, .. } => words_from_str(content, out),
            Block::Blockquote { children, .. } => words_from_blocks(children, out),
            Block::List { items, .. } => {
                for item in items {
                    words_from_inlines(item, out);
                }
            }
            Block::DefinitionList { items, .. } => {
                for (term, desc) in items {
                    words_from_inlines(term, out);
                    words_from_blocks(desc, out);
                }
            }
            Block::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        words_from_inlines(cell, out);
                    }
                }
            }
            Block::Float { children, .. } => words_from_blocks(children, out),
            Block::RawBlock { content, .. } => words_from_str(content, out),
            _ => {}
        }
    }
}

fn extract_words(doc: &TexinfoDoc) -> Vec<String> {
    let mut out = Vec::new();
    words_from_blocks(&doc.blocks, &mut out);
    out
}

// ── Coverage computation ──────────────────────────────────────────────────────

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

fn pandoc_to_plain(pandoc: &Path, input: &str) -> Option<String> {
    let out = Command::new(pandoc)
        .args(["--from", "texinfo", "--to", "plain", "--quiet"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(input.as_bytes());
            }
            child.wait_with_output().ok()
        })?;
    if out.status.success() {
        String::from_utf8(out.stdout).ok()
    } else {
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Run the oracle harness against a Texinfo sample.
#[test]
#[ignore]
fn pandoc_texinfo_oracle() {
    if !pandoc_supports_texinfo() {
        eprintln!("SKIP: pandoc does not support texinfo input format");
        return;
    }

    let pandoc = find_pandoc().unwrap();

    let sample = r#"\input texinfo
@setfilename test.info
@settitle Test Document

@node Top
@top Test Document

@chapter Introduction

This is a test document with @strong{bold} and @emph{italic} text.

@section Lists

@itemize
@item First item
@item Second item
@end itemize

@enumerate
@item Step one
@item Step two
@end enumerate

@section Code

@example
int main() {
    return 0;
}
@end example

@bye
"#;

    // Parse with our parser
    let (doc, _diags) = parse(sample);
    let our_words = extract_words(&doc);

    // Parse with Pandoc
    if let Some(plain) = pandoc_to_plain(&pandoc, sample) {
        let ref_words: Vec<String> = plain
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
            "Texinfo oracle: {pct}% word coverage (ref={} ours={})",
            ref_words.len(),
            our_words.len()
        );
    } else {
        eprintln!("NOTE: pandoc failed to convert texinfo sample");
    }
}

/// Smoke test: parse a representative Texinfo sample without panicking.
#[test]
fn parse_sample_no_panic() {
    let sample = r#"\input texinfo
@setfilename test.info
@settitle Sample Document

@node Top
@top Sample

@chapter Introduction

A paragraph with @strong{bold} and @emph{italic} text.
Use @code{printf} and @file{config.h} with @option{-Wall}.

@section Lists

@itemize @bullet
@item First item
@item Second item with @code{code}
@end itemize

@enumerate
@item Ordered one
@item Ordered two
@end enumerate

@table @asis
@item Term
Definition text.
@end table

@section Code Blocks

@example
int main() {
    return 0;
}
@end example

@verbatim
raw text here
@end verbatim

@section Quotation

@quotation
Quoted passage.
@end quotation

@section Cross References

See @xref{Introduction} for more.
Also @ref{Lists} and @pxref{Code Blocks}.

@section Footnotes

Main text@footnote{A footnote.}.

@section Symbols

Ellipsis: @dots{} Copyright: @copyright{} TeX: @TeX{}

@section Links

Visit @uref{https://example.com, Example}.
Contact @email{user@@example.com, the author}.

@section Multitable

@multitable @columnfractions .5 .5
@headitem Name @tab Value
@item key @tab val
@end multitable

@section Menu

@menu
* Introduction:: The intro
* Lists:: List examples
@end menu

@bye
"#;
    let (doc, _diags) = parse(sample);
    assert!(!doc.blocks.is_empty(), "expected at least one block");
}
