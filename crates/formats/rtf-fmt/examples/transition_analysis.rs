//! Node-transition prevalence analysis for RTF corpora.
//!
//! For every (parent, child) containment pair and every
//! (parent: prev → next) sibling-sequence pair observed across a corpus of
//! RTF files, counts how many *documents* (not occurrences) contain that
//! transition.  Compares against a fixture directory and reports gaps.
//!
//! Usage:
//!   cargo run -p rtf-fmt --example transition_analysis -- \
//!     --corpus  fuzz/corpus/fuzz_rtf_reader \
//!     --fixtures fixtures/rtf \
//!     [--top 40]

use rtf_fmt::{Block, Inline, parse};
use std::collections::{HashMap, HashSet};

// ── Kind labels ───────────────────────────────────────────────────────────────

fn block_kind(b: &Block) -> &'static str {
    match b {
        Block::Paragraph { .. } => "paragraph",
        Block::Heading { level: 1, .. } => "heading:1",
        Block::Heading { level: 2, .. } => "heading:2",
        Block::Heading { level: 3, .. } => "heading:3",
        Block::Heading { .. } => "heading:N",
        Block::CodeBlock { .. } => "code_block",
        Block::Blockquote { .. } => "blockquote",
        Block::List { ordered: true, .. } => "list_ordered",
        Block::List { .. } => "list_unordered",
        Block::Table { .. } => "table",
        Block::HorizontalRule { .. } => "hr",
    }
}

fn inline_kind(i: &Inline) -> &'static str {
    match i {
        Inline::Text { .. } => "text",
        Inline::Bold { .. } => "bold",
        Inline::Italic { .. } => "italic",
        Inline::Underline { .. } => "underline",
        Inline::Strikethrough { .. } => "strike",
        Inline::Code { .. } => "code",
        Inline::Link { .. } => "link",
        Inline::Image { .. } => "image",
        Inline::LineBreak { .. } => "line_break",
        Inline::SoftBreak { .. } => "soft_break",
        Inline::Superscript { .. } => "super",
        Inline::Subscript { .. } => "sub",
        Inline::FontSize { .. } => "font_size",
        Inline::Color { .. } => "color",
        Inline::AllCaps { .. } => "all_caps",
        Inline::SmallCaps { .. } => "small_caps",
        Inline::Hidden { .. } => "hidden",
        Inline::CharSpan { .. } => "char_span",
        Inline::Font { .. } => "font",
        Inline::BgColor { .. } => "bg_color",
        Inline::Lang { .. } => "lang",
    }
}

// ── Transition collection ─────────────────────────────────────────────────────

type Trans = String;

/// Collect all transitions in a document into `out` (deduped per document).
fn collect_blocks(blocks: &[Block], parent: &str, out: &mut HashSet<Trans>) {
    let kinds: Vec<&str> = blocks.iter().map(block_kind).collect();

    // containment: parent → each child kind
    for &k in &kinds {
        out.insert(format!("{parent} > {k}"));
    }
    // sibling sequence: parent: prev → next
    for w in kinds.windows(2) {
        out.insert(format!("{parent}: {} → {}", w[0], w[1]));
    }

    // recurse
    for block in blocks {
        let bk = block_kind(block);
        match block {
            Block::Paragraph { inlines, .. } => collect_inlines(inlines, bk, out),
            Block::Heading { inlines, .. } => collect_inlines(inlines, bk, out),
            Block::Blockquote { children, .. } => collect_blocks(children, bk, out),
            Block::List { items, .. } => {
                for item in items {
                    collect_blocks(item, "list_item", out);
                }
            }
            Block::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        collect_inlines(cell, "table_cell", out);
                    }
                }
            }
            Block::CodeBlock { .. } | Block::HorizontalRule { .. } => {}
        }
    }
}

fn collect_inlines(inlines: &[Inline], parent: &str, out: &mut HashSet<Trans>) {
    let kinds: Vec<&str> = inlines.iter().map(inline_kind).collect();

    for &k in &kinds {
        out.insert(format!("{parent} > {k}"));
    }
    for w in kinds.windows(2) {
        out.insert(format!("{parent}: {} → {}", w[0], w[1]));
    }

    for inline in inlines {
        let ik = inline_kind(inline);
        let children = match inline {
            Inline::Bold { children, .. }
            | Inline::Italic { children, .. }
            | Inline::Underline { children, .. }
            | Inline::Strikethrough { children, .. }
            | Inline::Superscript { children, .. }
            | Inline::Subscript { children, .. }
            | Inline::FontSize { children, .. }
            | Inline::Color { children, .. }
            | Inline::AllCaps { children, .. }
            | Inline::SmallCaps { children, .. }
            | Inline::Hidden { children, .. }
            | Inline::CharSpan { children, .. }
            | Inline::Font { children, .. }
            | Inline::BgColor { children, .. }
            | Inline::Lang { children, .. }
            | Inline::Link { children, .. } => Some(children.as_slice()),
            _ => None,
        };
        if let Some(ch) = children {
            collect_inlines(ch, ik, out);
        }
    }
}

// ── File discovery ────────────────────────────────────────────────────────────

fn find_rtf_files(dir: &str) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                out.extend(find_rtf_files(&p.to_string_lossy()));
            } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("rtf")) {
                out.push(p);
            }
        }
    }
    out
}

// ── Analysis ──────────────────────────────────────────────────────────────────

/// Returns (transition → doc_count, total_docs_parsed).
fn analyze(dir: &str) -> (HashMap<Trans, usize>, usize) {
    let files = find_rtf_files(dir);
    let mut counts: HashMap<Trans, usize> = HashMap::new();
    let mut total = 0usize;

    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let (ast, _) = parse(&bytes);
        if ast.blocks.is_empty() {
            continue; // skip files our parser extracted nothing from
        }
        let mut doc_trans: HashSet<Trans> = HashSet::new();
        collect_blocks(&ast.blocks, "doc", &mut doc_trans);
        for t in doc_trans {
            *counts.entry(t).or_insert(0) += 1;
        }
        total += 1;
    }

    (counts, total)
}

// ── CLI ───────────────────────────────────────────────────────────────────────

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let corpus_dir =
        flag(&args, "--corpus").unwrap_or_else(|| "fuzz/corpus/fuzz_rtf_reader".into());
    let fixtures_dir = flag(&args, "--fixtures").unwrap_or_else(|| "fixtures/rtf".into());
    let top_n: usize = flag(&args, "--top")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);

    eprint!("Analyzing corpus ({corpus_dir})…  ");
    let (corpus_counts, corpus_total) = analyze(&corpus_dir);
    eprintln!(
        "{corpus_total} docs, {} distinct transitions",
        corpus_counts.len()
    );

    eprint!("Analyzing fixtures ({fixtures_dir})…  ");
    let (fixture_counts, fixture_total) = analyze(&fixtures_dir);
    eprintln!(
        "{fixture_total} docs, {} distinct transitions",
        fixture_counts.len()
    );

    let fixture_set: HashSet<&Trans> = fixture_counts.keys().collect();

    // Sort corpus transitions by doc frequency descending
    let mut ranked: Vec<(&Trans, usize)> = corpus_counts.iter().map(|(k, &v)| (k, v)).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

    let covered = ranked
        .iter()
        .filter(|(t, _)| fixture_set.contains(t))
        .count();
    let gaps: Vec<_> = ranked
        .iter()
        .filter(|(t, _)| !fixture_set.contains(t))
        .collect();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(
        "  Corpus : {corpus_total} docs, {} transitions",
        corpus_counts.len()
    );
    println!(
        "  Fixtures: {fixture_total} docs, {} transitions",
        fixture_counts.len()
    );
    println!(
        "  Covered : {covered} / {} corpus transitions",
        ranked.len()
    );
    println!("  Gaps    : {}", gaps.len());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("── Gaps (top {top_n} by corpus frequency) ──────────────────────");
    println!("{:>6}  {:>4}%  transition", "docs", "pct");
    println!("{}", "─".repeat(55));
    for (trans, count) in gaps.iter().take(top_n) {
        let pct = *count * 100 / corpus_total.max(1);
        println!("{:>6}  {:>4}%  {}", count, pct, trans);
    }
    println!();

    println!("── Covered transitions ─────────────────────────────────────────");
    println!("{:>6}  {:>4}%  transition", "docs", "pct");
    println!("{}", "─".repeat(55));
    for (trans, count) in ranked
        .iter()
        .filter(|(t, _)| fixture_set.contains(t))
        .take(top_n)
    {
        let pct = *count * 100 / corpus_total.max(1);
        println!("{:>6}  {:>4}%  {}", count, pct, trans);
    }
}
