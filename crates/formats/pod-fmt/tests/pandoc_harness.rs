//! Oracle harness: compare pod-fmt parse output against Pandoc (if available).
//!
//! These tests are `#[ignore]` by default. They require:
//!   - `pandoc` binary on PATH that supports `pod` input format
//!   - `~/git/pandoc/test/` corpus directory
//!
//! Run with:
//!   cargo test -q -p pod-fmt -- --ignored --nocapture

use std::path::{Path, PathBuf};
use std::process::Command;
use pod_fmt::ast::{Block, Inline, PodDoc};
use pod_fmt::parse;

// ── Path discovery ────────────────────────────────────────────────────────────

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/home/me".into()))
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

fn pandoc_supports_pod() -> bool {
    if let Some(pandoc) = find_pandoc()
        && let Ok(out) = Command::new(&pandoc)
            .args(["--list-input-formats"])
            .output()
    {
        let formats = String::from_utf8_lossy(&out.stdout);
        return formats.lines().any(|l| l.trim() == "pod");
    }
    false
}

// ── Text extraction ──────────────────────────────────────────────────────────

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
            Inline::Text(s, _) | Inline::Entity(s, _) => words_from_str(s, out),
            Inline::Bold(ch, _)
            | Inline::Italic(ch, _)
            | Inline::Underline(ch, _)
            | Inline::Filename(ch, _)
            | Inline::NonBreaking(ch, _) => words_from_inlines(ch, out),
            Inline::Link { label, url, .. } => {
                words_from_str(if label.is_empty() { url } else { label }, out);
            }
            Inline::Code(s, _) | Inline::IndexEntry(s, _) => words_from_str(s, out),
            Inline::Null(_) => {}
        }
    }
}

fn words_from_blocks(blocks: &[Block], out: &mut Vec<String>) {
    for block in blocks {
        match block {
            Block::Paragraph { inlines, .. } | Block::Heading { inlines, .. } => {
                words_from_inlines(inlines, out);
            }
            Block::CodeBlock { content, .. } => words_from_str(content, out),
            Block::List { items, .. } => {
                for item in items {
                    words_from_blocks(item, out);
                }
            }
            Block::DefinitionList { items, .. } => {
                for item in items {
                    words_from_inlines(&item.term, out);
                    words_from_blocks(&item.desc, out);
                }
            }
            Block::RawBlock { content, .. } | Block::ForBlock { content, .. } => {
                words_from_str(content, out);
            }
            Block::Encoding { .. } => {}
        }
    }
}

fn extract_words(doc: &PodDoc) -> Vec<String> {
    let mut out = Vec::new();
    words_from_blocks(&doc.blocks, &mut out);
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Oracle harness against Pandoc POD corpus (if available).
#[test]
#[ignore]
fn pandoc_pod_corpus() {
    if !pandoc_supports_pod() {
        eprintln!("SKIP: pandoc does not support 'pod' input format");
        return;
    }

    let corpus = home_dir().join("git/pandoc/test");
    if !corpus.is_dir() {
        eprintln!("SKIP: ~/git/pandoc/test/ not found");
        return;
    }

    // Look for pod-reader.pod or similar
    let candidates = ["pod-reader.pod"];
    for filename in &candidates {
        let file = corpus.join(filename);
        if !file.exists() {
            eprintln!("SKIP: {} not found", file.display());
            continue;
        }

        let input = std::fs::read_to_string(&file).unwrap();
        let (doc, diags) = parse(&input);
        let words = extract_words(&doc);
        eprintln!(
            "{}: parsed {} blocks, {} words, {} diagnostics",
            filename,
            doc.blocks.len(),
            words.len(),
            diags.len()
        );
    }
}

/// Smoke test: parse a representative POD sample without panicking.
/// Runs in normal CI (not #[ignore]).
#[test]
fn parse_sample_no_panic() {
    let sample = r#"=encoding UTF-8

=head1 NAME

Test - A test module

=head1 SYNOPSIS

    use Test;
    my $t = Test->new();

=head1 DESCRIPTION

This is a test module that does nothing useful.
It exists solely to test the POD parser.

Features include:

=over 4

=item * B<Bold> and I<italic> text

=item * C<Code> spans

=item * L<Links|https://example.com>

=back

=head2 Formatting

Use F</etc/config> for configuration.
The S<non breaking> text should not wrap.
See X<index entry> for the index.

=head2 Escapes

Special chars: E<lt> E<gt> E<amp> E<sol> E<verbar>

=begin html

<p>Some <b>HTML</b> content</p>

=end html

=for text Plain text content.

=over 4

=item Term One

Definition of term one.

=item Term Two

Definition of term two.

=back

=head1 SEE ALSO

L<perlpod>, L<Pod::Simple>

=cut
"#;
    let (doc, _diags) = parse(sample);
    assert!(
        !doc.blocks.is_empty(),
        "expected at least one block from sample input"
    );
}
