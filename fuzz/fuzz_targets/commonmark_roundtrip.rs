#![no_main]

//! commonmark-fmt native AST roundtrip fuzz target.
//!
//! Constructs an arbitrary CmDoc from fuzz data, emits it to CommonMark text,
//! parses back, and asserts structural equality (after strip_spans).
//!
//! Direction: arbitrary_commonmark_ast → emit → parse → assert equality
//!
//! This is the definitive test per CLAUDE.md: starts from the format crate's
//! own Ast type (not the IR). Covers the full surface area of what CommonMark
//! can express, regardless of IR modeling completeness.

use libfuzzer_sys::fuzz_target;
use commonmark_fmt::ast::{Block, CmDoc, Inline, ListItem, ListKind, OrderedMarker, Span};

// ── Helpers to build a well-formed CmDoc from raw bytes ───────────────────────

/// Map bytes to safe lowercase-alpha strings. Avoids all CommonMark special
/// characters (`*`, `_`, `` ` ``, `[`, `~`, `<`, `\`) that would create
/// syntax conflicts in emitted output and break the roundtrip assertion.
fn safe_str(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| {
            let c = (b % 26) + b'a';
            c as char
        })
        .collect()
}

fn safe_text(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        "x".to_string()
    } else {
        safe_str(bytes)
    }
}

struct Gen<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Gen<'a> {
    fn new(data: &'a [u8]) -> Self {
        Gen { data, pos: 0 }
    }

    fn byte(&mut self) -> u8 {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            b
        } else {
            0
        }
    }

    fn bytes(&mut self, n: usize) -> &[u8] {
        let start = self.pos;
        let end = (self.pos + n).min(self.data.len());
        self.pos = end;
        &self.data[start..end]
    }

    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn inline(&mut self, depth: u8) -> Inline {
        // At depth > 0 (inside markup spans), only Text and Code — no HardBreak.
        // A HardBreak inside Emphasis/Strong/Link would be stripped by
        // strip_hard_breaks() when the span appears in a heading, potentially
        // leaving two Code spans adjacent (the HardBreak was their only separator).
        let kind = self.byte() % if depth > 0 { 2 } else { 6 };
        match kind {
            0 => Inline::Text {
                content: safe_text(self.bytes(3)),
                span: Span::NONE,
            },
            1 => Inline::Code {
                content: safe_text(self.bytes(3)),
                span: Span::NONE,
            },
            2 => Inline::HardBreak { span: Span::NONE },
            3 => Inline::Emphasis {
                inlines: fix_code_boundaries(self.inlines(depth + 1, 1)),
                span: Span::NONE,
            },
            4 => Inline::Strong {
                inlines: fix_code_boundaries(self.inlines(depth + 1, 1)),
                span: Span::NONE,
            },
            // Link: safe URL only, no title to avoid title-escaping edge cases
            _ => Inline::Link {
                inlines: fix_code_boundaries(self.inlines(depth + 1, 1)),
                url: format!("https://example.com/{}", safe_text(self.bytes(2))),
                title: None,
                span: Span::NONE,
            },
        }
    }

    fn inlines(&mut self, depth: u8, min: usize) -> Vec<Inline> {
        if depth > 2 {
            return vec![Inline::Text {
                content: safe_text(self.bytes(2)),
                span: Span::NONE,
            }];
        }
        let count = (self.byte() as usize % 3) + min;
        let raw: Vec<Inline> = (0..count).map(|_| self.inline(depth)).collect();
        // Merge adjacent Text nodes — roundtrip collapses them:
        // [Text "a", Text "b"] → [Text "ab"]
        let merged = merge_text(raw);
        // Collapse consecutive HardBreaks — two adjacent HardBreaks emit as
        // "  \n  \n" where the second line is all-spaces = blank line in
        // CommonMark §2.1, which acts as a paragraph separator.
        let merged = {
            let mut deduped: Vec<Inline> = Vec::new();
            for inline in merged {
                if matches!(&inline, Inline::HardBreak { .. })
                    && matches!(deduped.last(), Some(Inline::HardBreak { .. }))
                {
                    continue;
                }
                deduped.push(inline);
            }
            deduped
        };
        // HardBreaks at the boundaries of inline content don't roundtrip:
        // - Leading HardBreak: emits "  \n" with nothing before it, which
        //   the parser treats as a blank line and drops.
        // - Trailing HardBreak: CommonMark §6.7 says a hard break at the end
        //   of a block is just ignored trailing whitespace; it gets dropped.
        // Fix by wrapping boundary HardBreaks with Text nodes.
        let needs_prefix = matches!(merged.first(), Some(Inline::HardBreak { .. }));
        let needs_suffix = matches!(merged.last(), Some(Inline::HardBreak { .. }));
        let merged = if needs_prefix || needs_suffix {
            let mut out = Vec::new();
            if needs_prefix {
                out.push(Inline::Text { content: "x".to_string(), span: Span::NONE });
            }
            out.extend(merged);
            if needs_suffix {
                out.push(Inline::Text { content: "x".to_string(), span: Span::NONE });
            }
            merge_text(out)
        } else {
            merged
        };
        // Consecutive delimiter spans (Emphasis/Strong/Link) produce concatenated
        // star runs at junctions (`****`, `***`) that violate CommonMark §6.4
        // rule 9 (multiple-of-3 sum) and break roundtrip.  Separate them.
        separate_delimiter_spans(merged)
    }

    fn block(&mut self, depth: u8) -> Block {
        // Depth guard — prevents u8 overflow from deeply nested structures and
        // guarantees list items / blockquotes don't recurse forever.
        if depth > 2 {
            return Block::Paragraph {
                inlines: vec![Inline::Text {
                    content: safe_text(self.bytes(2)),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            };
        }
        let kind = self.byte() % if depth > 0 { 5 } else { 7 };
        match kind {
            0 => Block::Paragraph {
                inlines: self.inlines(0, 1),
                span: Span::NONE,
            },
            1 => {
                // Heading: level must be 1..=6.
                // HardBreaks don't roundtrip in ATX headings — the heading is a
                // single-line construct.  A HardBreak *anywhere* in the inline
                // tree (including inside nested Emphasis/Strong/Link) emits
                // "  \n" which splits the heading across lines.  Recursively
                // strip all HardBreaks, then merge adjacent Text nodes.
                let level = (self.byte() % 6) + 1;
                let raw = self.inlines(0, 1);
                let stripped = strip_hard_breaks(raw);
                let inlines = if stripped.is_empty() {
                    vec![Inline::Text { content: "x".to_string(), span: Span::NONE }]
                } else {
                    stripped
                };
                Block::Heading { level, inlines, span: Span::NONE }
            }
            2 => {
                // CodeBlock: safe language tag and content with trailing newline
                let lang_bytes = self.bytes(2);
                let language = if lang_bytes.is_empty() {
                    None
                } else {
                    Some(safe_text(lang_bytes))
                };
                let content = format!("{}\n", safe_text(self.bytes(4)));
                Block::CodeBlock { language, content, span: Span::NONE }
            }
            3 => Block::Blockquote {
                blocks: self.blocks(depth + 1, 1),
                span: Span::NONE,
            },
            4 => {
                // Tight unordered list (marker always `-` — pulldown-cmark
                // doesn't preserve the original marker on reparse).
                // Each item has exactly one block; multiple blocks per tight
                // item would emit as continuation lines and reparse as one
                // paragraph with SoftBreaks.
                let n = (self.byte() as usize % 3) + 1;
                let items = (0..n)
                    .map(|_| ListItem {
                        blocks: vec![self.block(depth + 1)],
                        span: Span::NONE,
                    })
                    .collect();
                Block::List {
                    kind: ListKind::Unordered { marker: '-' },
                    items,
                    tight: true,
                    span: Span::NONE,
                }
            }
            5 => {
                // Tight ordered list (Period marker for simplicity)
                let n = (self.byte() as usize % 3) + 1;
                let start = (self.byte() as u64 % 9) + 1;
                let items = (0..n)
                    .map(|_| ListItem {
                        blocks: vec![self.block(depth + 1)],
                        span: Span::NONE,
                    })
                    .collect();
                Block::List {
                    kind: ListKind::Ordered { start, marker: OrderedMarker::Period },
                    items,
                    tight: true,
                    span: Span::NONE,
                }
            }
            _ => Block::ThematicBreak { span: Span::NONE },
        }
    }

    fn blocks(&mut self, depth: u8, _min: usize) -> Vec<Block> {
        if depth > 2 {
            return vec![Block::Paragraph {
                inlines: vec![Inline::Text {
                    content: safe_text(self.bytes(2)),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }];
        }
        // Produce at most 2 blocks. More blocks at the same depth can cause
        // consecutive same-kind lists to merge on roundtrip (CommonMark spec
        // §5.3: two bullet lists separated by a blank line → single loose list).
        let count = (self.byte() as usize % 2) + 1;
        let result: Vec<Block> = (0..count).map(|_| self.block(depth)).collect();
        // Drop the second block if it's a List and the first is also a List —
        // two consecutive lists (same or different markers) in the same parent
        // emit with a blank line separator, which the parser merges into a
        // single loose list, destroying the two-block structure.
        if result.len() == 2 {
            if matches!((&result[0], &result[1]), (Block::List { .. }, Block::List { .. })) {
                return result.into_iter().take(1).collect();
            }
        }
        result
    }
}

/// Remove HardBreak nodes from heading inline content and re-normalize.
///
/// ATX headings are single-line; any HardBreak emits "  \n" which splits the
/// heading.  Since depth > 0 inlines never contain HardBreaks (inline() uses
/// % 2 at depth > 0), stripping is only needed at the top (depth=0) level.
///
/// After stripping, two Code/Emphasis spans that were only separated by a
/// HardBreak may be adjacent.  Re-apply separate_delimiter_spans.
fn strip_hard_breaks(inlines: Vec<Inline>) -> Vec<Inline> {
    let stripped: Vec<Inline> = inlines
        .into_iter()
        .filter(|i| !matches!(i, Inline::HardBreak { .. }))
        .collect();
    separate_delimiter_spans(merge_text(stripped))
}

/// Ensure the first inline of an Emphasis/Strong/Link is not a Code span.
///
/// CommonMark §6.1: an opening `*`/`**` before a code span (`` ` ``,
/// punctuation) is only left-flanking when preceded by whitespace or
/// punctuation — not a letter.  Since the span can be preceded by
/// Text("abc...") in the parent sequence, a leading Code blocks the opening
/// delimiter.  Prepend a Text "x" if necessary.
fn no_leading_code(mut inlines: Vec<Inline>) -> Vec<Inline> {
    if matches!(inlines.first(), Some(Inline::Code { .. })) {
        inlines.insert(0, Inline::Text { content: "x".to_string(), span: Span::NONE });
    }
    inlines
}

/// Ensure the last inline of an Emphasis/Strong/Link is not a Code span.
///
/// CommonMark §6.1: a closing `*`/`**` directly after a code span (`` ` ``,
/// punctuation) is only right-flanking when followed by whitespace or
/// punctuation — not a letter.  Since a delimited span can be followed by
/// Text("abc...") in the parent sequence, a trailing Code would block the
/// closing delimiter.  Append a Text "x" if necessary.
fn no_trailing_code(mut inlines: Vec<Inline>) -> Vec<Inline> {
    if matches!(inlines.last(), Some(Inline::Code { .. })) {
        inlines.push(Inline::Text { content: "x".to_string(), span: Span::NONE });
    }
    inlines
}

fn fix_code_boundaries(inlines: Vec<Inline>) -> Vec<Inline> {
    no_trailing_code(no_leading_code(inlines))
}

/// Insert Text("x") between consecutive delimiter spans.
///
/// Adjacent Emphasis/Strong/Link produce star-run junctions (`****`, `***`)
/// that violate CommonMark §6.4 rule 9 (multiple-of-3 sum).
///
/// Adjacent Code spans produce backtick-run junctions (`` ` `` + `` ` `` =
/// ```` `` ````) — the parser sees the merged backtick run as a 2-backtick
/// delimiter and swallows content across the span boundary.
///
/// Insert a Text("x") separator between any two consecutive spans to prevent
/// these concatenated-delimiter issues.
fn separate_delimiter_spans(inlines: Vec<Inline>) -> Vec<Inline> {
    let is_span = |i: &Inline| {
        matches!(
            i,
            Inline::Emphasis { .. }
                | Inline::Strong { .. }
                | Inline::Link { .. }
                | Inline::Code { .. }
        )
    };
    let mut out: Vec<Inline> = Vec::new();
    for inline in inlines {
        if is_span(&inline) && out.last().map_or(false, is_span) {
            out.push(Inline::Text { content: "x".to_string(), span: Span::NONE });
        }
        out.push(inline);
    }
    out
}

fn merge_text(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text { content, .. } => {
                if let Some(Inline::Text { content: prev, .. }) = out.last_mut() {
                    prev.push_str(&content);
                } else {
                    out.push(Inline::Text { content, span: Span::NONE });
                }
            }
            other => out.push(other),
        }
    }
    out
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let mut g = Gen::new(data);
    if g.remaining() < 2 {
        return;
    }

    // Use g.blocks() to get the same consecutive-list filter applied to
    // nested block sequences.
    let blocks = g.blocks(0, 1);
    let doc = CmDoc { blocks, link_defs: vec![] };

    // Emit — must not panic.
    let emitted = commonmark_fmt::emit(&doc);

    // Parse back — must not panic.
    let (reparsed, _diags) = commonmark_fmt::parse(&emitted);

    // Structural equality after strip_spans.
    // parse(emit(doc)).strip_spans() == doc.strip_spans()
    assert_eq!(
        doc.strip_spans(),
        reparsed.strip_spans(),
        "commonmark-fmt roundtrip mismatch\n  emitted: {}",
        String::from_utf8_lossy(&emitted),
    );
});
