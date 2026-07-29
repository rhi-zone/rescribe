//! Cross-API fixture harness: exercises `events()`, `StreamingParser<H>`,
//! and the streaming writer directly against `{format}-fmt` crates — not
//! just the rescribe adapter's `parse()`/`emit()`, which is all
//! `tests/run.rs` ever drove. See `fixtures/spec.md` ("Cross-API harness")
//! and `crates/rescribe-fixtures/src/streaming_harness.rs` for the
//! equivalence definitions and the capability/known-failure mechanisms this
//! file instantiates.
//!
//! Scope (see the task report for the honest accounting): rst-fmt gets full
//! real checks for all three non-parse/emit APIs. html-fmt gets a real
//! streaming-writer byte-identity check plus a chunk-buffering-integrity
//! check, with its two reader APIs declared `NotApplicable` against the
//! crate's own documentation that html5ever/HTML5 tree construction makes
//! independent streaming readers impossible. ooxml-wml/pml/sml get
//! real `events()` checks (wml/pml fail against a newly-found bug, tracked
//! as a `KnownFailure`; sml passes) and, for sml only, a real
//! streaming-writer fidelity check. Every other format gets an explicit
//! `NotYetWired` capability declaration in `streaming_harness::CAPABILITIES`
//! / `NOT_YET_AUDITED` rather than silently not appearing anywhere.

use rescribe_fixtures::streaming_harness::{
    CAPABILITIES, NOT_YET_AUDITED, ObservableSink, adversarial_chunkings, assert_or_known_failure,
};
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures")
}

/// Every format tested in `tests/run.rs` must appear either in
/// `streaming_harness::CAPABILITIES` (a real per-API declaration) or in
/// `NOT_YET_AUDITED` (an honest "nobody has looked yet" placeholder) — never
/// silently absent from both.
#[test]
fn every_run_rs_format_has_a_capability_entry() {
    // Kept in sync with the #[test] fns in tests/run.rs by hand; see the
    // comment at the end of this list if it drifts.
    const RUN_RS_FORMATS: &[&str] = &[
        "markdown",
        "commonmark",
        "gfm",
        "html",
        "asciidoc",
        "mediawiki",
        "latex",
        "rst",
        "org",
        "creole",
        "djot",
        "textile",
        "muse",
        "t2t",
        "tikiwiki",
        "twiki",
        "vimwiki",
        "dokuwiki",
        "jira",
        "haddock",
        "pod",
        "man",
        "xwiki",
        "zimwiki",
        "bbcode",
        "texinfo",
        "markua",
        "fountain",
        "ansi",
        "csl-json",
        "native",
        "pandoc-json",
        "docbook",
        "fb2",
        "ipynb",
        "csv",
        "tsv",
        "opml",
        "ris",
        "bibtex",
        "biblatex",
        "typst",
        "jats",
        "endnotexml",
        "tei",
        "docx",
        "odt",
        "epub",
        "pptx",
        "xlsx",
        "pdf",
        "rtf",
        "multimarkdown",
        // writer-only formats (also worth a declaration even though this
        // harness doesn't yet wire writer-side events()/StreamingParser
        // equivalents — there is no reader-side events() to check for them,
        // but a streaming-writer declaration still belongs here):
        "beamer",
        "revealjs",
        "slidy",
        "s5",
        "dzslides",
        "slideous",
        "context",
        "ms",
        "icml",
        "chunkedhtml",
    ];
    for fmt in RUN_RS_FORMATS {
        let declared =
            CAPABILITIES.iter().any(|c| &c.format == fmt) || NOT_YET_AUDITED.contains(fmt);
        assert!(
            declared,
            "format {fmt:?} is tested in tests/run.rs but has no entry in \
             streaming_harness::CAPABILITIES or NOT_YET_AUDITED — every format must have an \
             explicit, reviewable capability declaration, never silent absence"
        );
    }
}

fn find_input(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.file_stem().is_some_and(|s| s == "input"))
}

// ---------------------------------------------------------------------------
// rst-fmt: events() vs parse(), fully wired
// ---------------------------------------------------------------------------

/// Reconstruct the exact [`rst_fmt::events::Event`] sequence `events()` must
/// produce for `doc`, directly from the AST `parse()` returned.
///
/// Equivalence definition: `events(input).collect::<Vec<_>>()` must equal
/// `ast_to_events(&parse(input).unwrap())` under `Event`'s own derived
/// `PartialEq` (which compares `Cow::Borrowed`/`Cow::Owned` by value, not by
/// variant) — i.e. **exact** event-sequence equality, not merely matching
/// shape. This is possible (and preferable to a lossy shape-only
/// comparison) because rst-fmt's own `Event` type already carries every
/// attribute the AST does, so there is no information to project away.
fn ast_to_events<'a>(doc: &rst_fmt::RstDoc<'a>) -> Vec<rst_fmt::events::Event<'a>> {
    let mut out = Vec::new();
    for b in &doc.blocks {
        block_events(b, &mut out);
    }
    out
}

fn block_events<'a>(b: &rst_fmt::Block<'a>, out: &mut Vec<rst_fmt::events::Event<'a>>) {
    use rst_fmt::Block;
    use rst_fmt::events::Event;
    match b {
        Block::Paragraph { inlines } => {
            out.push(Event::StartParagraph);
            inline_events(inlines, out);
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines } => {
            out.push(Event::StartHeading { level: *level });
            inline_events(inlines, out);
            out.push(Event::EndHeading);
        }
        Block::CodeBlock { language, content } => {
            out.push(Event::StartCodeBlock {
                language: language.clone(),
            });
            out.push(Event::CodeBlockContent(content.clone()));
            out.push(Event::EndCodeBlock);
        }
        Block::Blockquote { children } => {
            out.push(Event::StartBlockquote);
            for c in children {
                block_events(c, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for c in item {
                    block_events(c, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::DefinitionList { items } => {
            out.push(Event::StartDefinitionList);
            for item in items {
                out.push(Event::StartDefinitionTerm);
                inline_events(&item.term, out);
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                inline_events(&item.desc, out);
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::Figure { url, alt, caption } => {
            out.push(Event::StartFigure {
                url: url.clone(),
                alt: alt.clone(),
            });
            if let Some(cap) = caption {
                inline_events(cap, out);
            }
            out.push(Event::EndFigure);
        }
        Block::Image { url, alt, title } => out.push(Event::ImageBlock {
            url: url.clone(),
            alt: alt.clone(),
            title: title.clone(),
        }),
        Block::RawBlock { format, content } => out.push(Event::RawBlock {
            format: format.clone(),
            content: content.clone(),
        }),
        Block::Div {
            class,
            directive,
            children,
        } => {
            out.push(Event::StartDiv {
                class: class.clone(),
                directive: directive.clone(),
            });
            for c in children {
                block_events(c, out);
            }
            out.push(Event::EndDiv);
        }
        Block::HorizontalRule => out.push(Event::HorizontalRule),
        Block::Table { rows } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow {
                    is_header: row.is_header,
                });
                for cell in &row.cells {
                    out.push(Event::StartTableCell);
                    inline_events(cell, out);
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::FootnoteDef { label, inlines } => {
            out.push(Event::StartFootnoteDef {
                label: label.clone(),
            });
            inline_events(inlines, out);
            out.push(Event::EndFootnoteDef);
        }
        Block::MathDisplay { source } => out.push(Event::MathDisplay {
            source: source.clone(),
        }),
        Block::Admonition {
            admonition_type,
            children,
        } => {
            out.push(Event::StartAdmonition {
                admonition_type: admonition_type.clone(),
            });
            for c in children {
                block_events(c, out);
            }
            out.push(Event::EndAdmonition);
        }
    }
}

fn inline_events<'a>(inlines: &[rst_fmt::Inline<'a>], out: &mut Vec<rst_fmt::events::Event<'a>>) {
    use rst_fmt::Inline;
    use rst_fmt::events::Event;
    for i in inlines {
        match i {
            Inline::Text(t) => out.push(Event::Text(t.clone())),
            Inline::Emphasis(c) => {
                out.push(Event::StartEmphasis);
                inline_events(c, out);
                out.push(Event::EndEmphasis);
            }
            Inline::Strong(c) => {
                out.push(Event::StartStrong);
                inline_events(c, out);
                out.push(Event::EndStrong);
            }
            Inline::Strikeout(c) => {
                out.push(Event::StartStrikeout);
                inline_events(c, out);
                out.push(Event::EndStrikeout);
            }
            Inline::Underline(c) => {
                out.push(Event::StartUnderline);
                inline_events(c, out);
                out.push(Event::EndUnderline);
            }
            Inline::Subscript(c) => {
                out.push(Event::StartSubscript);
                inline_events(c, out);
                out.push(Event::EndSubscript);
            }
            Inline::Superscript(c) => {
                out.push(Event::StartSuperscript);
                inline_events(c, out);
                out.push(Event::EndSuperscript);
            }
            Inline::Code(c) => out.push(Event::Code(c.clone())),
            Inline::Link { url, children } => {
                out.push(Event::StartLink { url: url.clone() });
                inline_events(children, out);
                out.push(Event::EndLink);
            }
            Inline::Image { url, alt } => out.push(Event::InlineImage {
                url: url.clone(),
                alt: alt.clone(),
            }),
            Inline::LineBreak => out.push(Event::LineBreak),
            Inline::SoftBreak => out.push(Event::SoftBreak),
            Inline::FootnoteRef { label } => out.push(Event::FootnoteRef {
                label: label.clone(),
            }),
            Inline::FootnoteDef { label, children } => {
                out.push(Event::StartFootnoteDefInline {
                    label: label.clone(),
                });
                inline_events(children, out);
                out.push(Event::EndFootnoteDefInline);
            }
            Inline::SmallCaps(c) => {
                out.push(Event::StartSmallCaps);
                inline_events(c, out);
                out.push(Event::EndSmallCaps);
            }
            Inline::Quoted {
                quote_type,
                children,
            } => {
                out.push(Event::StartQuoted {
                    quote_type: quote_type.clone(),
                });
                inline_events(children, out);
                out.push(Event::EndQuoted);
            }
            Inline::MathInline { source } => out.push(Event::MathInline {
                source: source.clone(),
            }),
            Inline::RstSpan { role, children } => {
                out.push(Event::StartRstSpan { role: role.clone() });
                inline_events(children, out);
                out.push(Event::EndRstSpan);
            }
        }
    }
}

#[test]
fn rst_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("rst");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/rst dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let Ok(doc) = rst_fmt::parse(&input) else {
            continue; // adversarial fixtures that fail to parse: not this check's concern
        };
        let expected = ast_to_events(&doc);
        let actual: Vec<_> = rst_fmt::events(&input).collect();
        assert_eq!(
            expected,
            actual,
            "events() diverged from the AST projection for fixture {}",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of rst fixtures, got {checked}"
    );
}

/// Equivalence check: `StreamingParser` fed `input` under an adversarial
/// chunking must deliver the same event sequence `events()` delivers over
/// the whole input at once, with one documented, sanctioned exception:
/// forward-declared RST link targets, which `StreamingParser` does not
/// resolve (see `crates/formats/rst-fmt/src/batch.rs` module docs) while
/// `events()`/`parse()` pre-scan for them — fixtures exercising that are
/// excluded rather than flagged as a bug.
///
/// Wiring this check (previously nothing drove `StreamingParser` against
/// more than rst-fmt's own hand-picked 6 chunk-splitting cases) surfaced a
/// real, previously-unknown bug: multi-item RST definition lists get
/// closed and reopened as separate `StartDefinitionList`/`EndDefinitionList`
/// pairs per item in `StreamingParser`, instead of one list spanning all
/// items the way `events()` produces. Tracked as `KnownFailure { format:
/// "rst", api: "streaming_parser", .. }`.
#[test]
fn rst_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("rst");
    const SKIP: &[&str] = &["anonymous-link", "citation"];
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/rst dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if SKIP.contains(&name.as_str()) || name.starts_with("adv-") {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let Ok(input_str) = std::str::from_utf8(&input) else {
            continue;
        };
        let bulk: Vec<rst_fmt::OwnedEvent> =
            rst_fmt::events(input_str).map(|e| e.into_owned()).collect();
        // Count this fixture as checked as soon as we've computed its `events()`
        // baseline, not only when it passes: the `checked > N` assert below is a
        // sanity floor on how many fixtures were exercised, not a pass counter.
        // Incrementing it only on the non-break path made the floor dependent on
        // `read_dir` iteration order — whichever fixture the known-failure bug
        // (see the KnownFailure below) happens to land on first, `checked` would
        // freeze there, so the assert's pass/fail flipped with filesystem order
        // alone rather than test content.
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                rst_fmt::batch::StreamingParser::new(|e: rst_fmt::OwnedEvent| streamed.push(e));
            for chunk in chunks {
                parser.feed(&chunk);
            }
            parser.finish();
            if bulk != streamed {
                // Record only the first divergence (further chunkings/fixtures may
                // hit the same known bug repeatedly and would just add noise), but
                // keep iterating the remaining fixtures so `checked` reflects the
                // true number exercised regardless of which fixture the divergence
                // landed on — see the comment above `checked += 1`.
                if result.is_ok() {
                    result = Err(format!(
                        "StreamingParser diverged from events() for fixture {name} under \
                         chunking {chunking_name}:\n  events():         {bulk:?}\n  \
                         StreamingParser: {streamed:?}"
                    ));
                }
                break;
            }
        }
    }
    assert!(
        checked > 5,
        "expected to check several rst fixtures, got {checked}"
    );
    assert_or_known_failure("rst", "streaming_parser", result);
}

#[test]
fn rst_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("rst");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/rst dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let Ok(doc) = rst_fmt::parse(&input) else {
            continue;
        };
        let built = rst_fmt::build(&doc);

        let mut w = rst_fmt::Writer::new(Vec::<u8>::new());
        for e in rst_fmt::events(&input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        assert_eq!(
            built,
            streamed,
            "streaming Writer diverged from build() for fixture {}",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of rst fixtures, got {checked}"
    );
}

// ---------------------------------------------------------------------------
// org-fmt: events() vs parse(), fully wired
// ---------------------------------------------------------------------------
//
// org-fmt's `events()` is a genuinely independent implementation, not a
// `parse()`-then-walk wrapper — the dependency runs the other way. `EventIter`
// (defined in `parse.rs`, re-exported by `events.rs`) is a lazy pull parser
// over the source lines, and `parse()` is built on top of it by repeatedly
// calling `parse_next_block()`. So this equivalence check compares two real
// code paths: the AST `parse()` assembles, and the events `expand_block`/
// `expand_inline` unfold.
mod org_events_check {
    use super::{find_input, fixtures_root};
    use org_fmt::{Block, Inline, ListItemContent, OrgDoc, OwnedEvent};
    use std::borrow::Cow;

    /// Reconstruct the exact `OwnedEvent` sequence `events()` must produce for
    /// `doc`.
    ///
    /// `OrgDoc::metadata` deliberately produces no events: the `Event` enum has
    /// no metadata variant, so document metadata is out-of-band for the
    /// streaming API.
    fn org_ast_to_events(doc: &OrgDoc) -> Vec<OwnedEvent> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            org_block_events(b, &mut out);
        }
        out
    }

    fn org_block_events(b: &Block, out: &mut Vec<OwnedEvent>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(OwnedEvent::StartParagraph);
                org_inline_events(inlines, out);
                out.push(OwnedEvent::EndParagraph);
            }
            Block::Heading {
                level,
                todo,
                priority,
                tags,
                properties,
                scheduled,
                deadline,
                inlines,
                ..
            } => {
                // Heading attributes all ride on StartHeading; the heading's own
                // title text is the inline run between Start/EndHeading. Nested
                // headings are siblings in `OrgDoc::blocks`, not children, so
                // EndHeading immediately follows the title inlines.
                out.push(OwnedEvent::StartHeading {
                    level: *level,
                    todo: todo.clone(),
                    priority: priority.clone(),
                    tags: tags.clone(),
                    properties: properties.clone(),
                    scheduled: scheduled.clone(),
                    deadline: deadline.clone(),
                });
                org_inline_events(inlines, out);
                out.push(OwnedEvent::EndHeading);
            }
            Block::CodeBlock {
                language,
                header_args,
                name,
                content,
                ..
            } => out.push(OwnedEvent::CodeBlock {
                language: language.clone(),
                header_args: header_args.clone(),
                name: name.clone(),
                content: Cow::Owned(content.clone()),
            }),
            Block::Blockquote { children, .. } => {
                out.push(OwnedEvent::StartBlockquote);
                for c in children {
                    org_block_events(c, out);
                }
                out.push(OwnedEvent::EndBlockquote);
            }
            Block::List {
                ordered,
                start,
                items,
                ..
            } => {
                out.push(OwnedEvent::StartList {
                    ordered: *ordered,
                    start: *start,
                });
                for item in items {
                    out.push(OwnedEvent::StartListItem {
                        checkbox: item.checkbox,
                    });
                    for child in &item.children {
                        match child {
                            // `ListItemContent::Inline` is a bare inline run — the
                            // tree builder (`events::handle_event`, the
                            // `BlockFrame::ListItem { inline_buf, .. }` arm)
                            // accumulates inlines seen directly inside a list item
                            // with no enclosing paragraph, so the projection emits
                            // them unwrapped.
                            ListItemContent::Inline(inlines) => org_inline_events(inlines, out),
                            ListItemContent::Block(block) => org_block_events(block, out),
                        }
                    }
                    out.push(OwnedEvent::EndListItem);
                }
                out.push(OwnedEvent::EndList);
            }
            Block::Table { rows, .. } => {
                out.push(OwnedEvent::StartTable);
                for row in rows {
                    out.push(OwnedEvent::StartTableRow {
                        is_header: row.is_header,
                    });
                    for cell in &row.cells {
                        out.push(OwnedEvent::StartTableCell);
                        org_inline_events(cell, out);
                        out.push(OwnedEvent::EndTableCell);
                    }
                    out.push(OwnedEvent::EndTableRow);
                }
                out.push(OwnedEvent::EndTable);
            }
            Block::HorizontalRule { .. } => out.push(OwnedEvent::HorizontalRule),
            Block::DefinitionList { items, .. } => {
                out.push(OwnedEvent::StartDefinitionList);
                for item in items {
                    // Term then desc, per item: settled from `handle_event`'s
                    // EndDefinitionTerm arm (pushes a partial `DefinitionItem`)
                    // and EndDefinitionDesc arm (fills in `items.last_mut()`).
                    out.push(OwnedEvent::StartDefinitionTerm);
                    org_inline_events(&item.term, out);
                    out.push(OwnedEvent::EndDefinitionTerm);
                    out.push(OwnedEvent::StartDefinitionDesc);
                    org_inline_events(&item.desc, out);
                    out.push(OwnedEvent::EndDefinitionDesc);
                }
                out.push(OwnedEvent::EndDefinitionList);
            }
            Block::Div { inlines, .. } => {
                out.push(OwnedEvent::StartDiv);
                org_inline_events(inlines, out);
                out.push(OwnedEvent::EndDiv);
            }
            Block::RawBlock {
                format, content, ..
            } => out.push(OwnedEvent::RawBlock {
                format: format.clone(),
                content: content.clone(),
            }),
            Block::Figure { name, children, .. } => {
                out.push(OwnedEvent::StartFigure { name: name.clone() });
                for c in children {
                    org_block_events(c, out);
                }
                out.push(OwnedEvent::EndFigure);
            }
            Block::Caption { inlines, .. } => {
                out.push(OwnedEvent::StartCaption);
                org_inline_events(inlines, out);
                out.push(OwnedEvent::EndCaption);
            }
            Block::FootnoteDef { label, content, .. } => {
                // Block-level footnote definitions map to the Block* pair; the
                // inline `Inline::FootnoteDefinition` maps to
                // Start/EndFootnoteDefinition. Both pairs exist and are distinct.
                out.push(OwnedEvent::StartBlockFootnoteDef {
                    label: label.clone(),
                });
                org_inline_events(content, out);
                out.push(OwnedEvent::EndBlockFootnoteDef);
            }
            Block::Unknown { kind, .. } => {
                out.push(OwnedEvent::UnknownBlock { kind: kind.clone() })
            }
        }
    }

    fn org_inline_events(inlines: &[Inline], out: &mut Vec<OwnedEvent>) {
        for i in inlines {
            match i {
                Inline::Text { text, .. } => out.push(OwnedEvent::Text(Cow::Owned(text.clone()))),
                Inline::Bold(c, _) => {
                    out.push(OwnedEvent::StartBold);
                    org_inline_events(c, out);
                    out.push(OwnedEvent::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(OwnedEvent::StartItalic);
                    org_inline_events(c, out);
                    out.push(OwnedEvent::EndItalic);
                }
                Inline::Underline(c, _) => {
                    out.push(OwnedEvent::StartUnderline);
                    org_inline_events(c, out);
                    out.push(OwnedEvent::EndUnderline);
                }
                Inline::Strikethrough(c, _) => {
                    out.push(OwnedEvent::StartStrikethrough);
                    org_inline_events(c, out);
                    out.push(OwnedEvent::EndStrikethrough);
                }
                Inline::Superscript(c, _) => {
                    out.push(OwnedEvent::StartSuperscript);
                    org_inline_events(c, out);
                    out.push(OwnedEvent::EndSuperscript);
                }
                Inline::Subscript(c, _) => {
                    out.push(OwnedEvent::StartSubscript);
                    org_inline_events(c, out);
                    out.push(OwnedEvent::EndSubscript);
                }
                Inline::Code(s, _) => out.push(OwnedEvent::InlineCode(Cow::Owned(s.clone()))),
                Inline::Link { url, children, .. } => {
                    out.push(OwnedEvent::StartLink { url: url.clone() });
                    org_inline_events(children, out);
                    out.push(OwnedEvent::EndLink);
                }
                Inline::Image { url, .. } => out.push(OwnedEvent::InlineImage { url: url.clone() }),
                Inline::LineBreak { .. } => out.push(OwnedEvent::LineBreak),
                Inline::SoftBreak { .. } => out.push(OwnedEvent::SoftBreak),
                Inline::FootnoteRef { label, .. } => out.push(OwnedEvent::FootnoteRef {
                    label: label.clone(),
                }),
                Inline::FootnoteDefinition {
                    label, children, ..
                } => {
                    out.push(OwnedEvent::StartFootnoteDefinition {
                        label: label.clone(),
                    });
                    org_inline_events(children, out);
                    out.push(OwnedEvent::EndFootnoteDefinition);
                }
                Inline::MathInline { source, .. } => out.push(OwnedEvent::MathInline {
                    source: source.clone(),
                }),
                Inline::Timestamp { active, value, .. } => out.push(OwnedEvent::Timestamp {
                    active: *active,
                    value: value.clone(),
                }),
                Inline::ExportSnippet { backend, value, .. } => {
                    out.push(OwnedEvent::ExportSnippet {
                        backend: backend.clone(),
                        value: value.clone(),
                    })
                }
            }
        }
    }

    /// `events()` must equal the hand-written AST projection exactly, over
    /// every fixture in `fixtures/org/`.
    ///
    /// Coverage note (measured while writing this, not assumed): across the
    /// 89 `fixtures/org/` directories this exercises 58 of org-fmt's 59
    /// `Event` variants, so nearly every projection arm is load-bearing
    /// rather than a dead arm agreeing trivially. The one variant never
    /// produced is `Event::UnknownBlock`: `parse.rs` never constructs
    /// `Block::Unknown` (its only mention, at parse.rs:961, is a *match* arm
    /// in `expand_block`), so an unknown `#+BEGIN_FOO` block becomes a
    /// `Block::Div` and the block-kind string is silently dropped. That is a
    /// reader losslessness gap, tracked in TODO.md — not an events()/parse()
    /// divergence, so it does not belong in KNOWN_FAILURES for this check.
    #[test]
    fn org_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("org");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&root).expect("fixtures/org dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = org_fmt::parse(&input);
            let expected = org_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> = org_fmt::events(&input)
                .map(org_fmt::Event::into_owned)
                .collect();
            // Counted once both sequences exist, not once they match — this is
            // a coverage floor, not a pass counter.
            checked += 1;
            if expected != actual {
                let first_div = expected
                    .iter()
                    .zip(actual.iter())
                    .position(|(e, a)| e != a)
                    .unwrap_or(expected.len().min(actual.len()));
                let lo = first_div.saturating_sub(3);
                failures.push(format!(
                    "{}: first divergence at event #{first_div} \
                     (expected len {}, actual len {})\n  expected: {:?}\n  actual:   {:?}",
                    path.file_name().unwrap().to_string_lossy(),
                    expected.len(),
                    actual.len(),
                    &expected[lo..expected.len().min(first_div + 4)],
                    &actual[lo..actual.len().min(first_div + 4)],
                ));
            }
        }
        assert!(
            checked > 50,
            "expected to check a substantial number of org fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {}/{checked} org fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

/// `StreamingParser` fed an org fixture under an adversarial chunking must
/// deliver the same event sequence `events()` delivers over the whole input.
///
/// org-fmt's `batch.rs` module docs sanction exactly two behavioural
/// exceptions — loose lists emitted as separate single-item lists, and
/// drawers containing blank lines being split. Neither covers any of the
/// divergences this check actually finds: none of the three failing fixtures
/// contains a loose list or a drawer. They are three distinct, previously
/// unknown bugs, all downstream of `emit_block()` (batch.rs:190) re-parsing
/// each accumulated block in isolation, but each cutting the block in the
/// wrong place for a different reason. See the `KnownFailure` entry.
#[test]
fn org_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("org");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/org dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let Ok(input_str) = std::str::from_utf8(&input) else {
            continue;
        };
        let bulk: Vec<org_fmt::OwnedEvent> = org_fmt::events(input_str)
            .map(org_fmt::Event::into_owned)
            .collect();
        // Coverage floor, not a pass counter — see the rst equivalent.
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                org_fmt::StreamingParser::new(|e: org_fmt::OwnedEvent| streamed.push(e));
            for chunk in chunks {
                parser.feed(&chunk);
            }
            parser.finish();
            if bulk != streamed {
                if result.is_ok() {
                    result = Err(format!(
                        "StreamingParser diverged from events() for fixture {name} under \
                         chunking {chunking_name}:\n  events():         {bulk:?}\n  \
                         StreamingParser: {streamed:?}"
                    ));
                }
                break;
            }
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of org fixtures, got {checked}"
    );
    assert_or_known_failure("org", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(input)` must reproduce what
/// builder `build()` produces for the AST `parse(input)` returned.
///
/// org-fmt's `Writer` is not incrementally streaming — `writer.rs`'s own
/// module docs say "This implementation buffers all events, reconstructs the
/// AST, then emits", and `finish()` calls `emit::build` on the reconstructed
/// doc. That makes this check meaningful anyway: it exercises `events_to_doc`
/// / `DocBuilder`, a substantial second reconstruction of the AST from the
/// event stream, against the AST `parse()` built directly.
///
/// It fails on three fixtures against a genuine expressiveness gap in the
/// `Event` enum rather than a `DocBuilder` logic error — see the
/// `KnownFailure` entry.
#[test]
fn org_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("org");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/org dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = org_fmt::parse(&input);
        let built = org_fmt::build(&doc);

        let mut w = org_fmt::Writer::new(Vec::<u8>::new());
        for e in org_fmt::events(&input) {
            w.write_event(e.into_owned());
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        checked += 1;
        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {}:\n  build():  \
                 {built:?}\n  Writer(): {streamed:?}",
                path.display()
            ));
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of org fixtures, got {checked}"
    );
    assert_or_known_failure("org", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// asciidoc: events() vs parse(), fully wired — with a narrower claim than rst
// ---------------------------------------------------------------------------
//
// Honest scoping of what `asciidoc: events = Wired` means. Unlike rst-fmt,
// `asciidoc::parse` (parse.rs:15) is NOT an implementation independent of
// `events()`: it constructs an `EventIter` and drives the same
// `try_parse_block()` in a loop, discarding the event machinery. `events()` is
// that same loop plus `expand_block`/`expand_inline`. So this check validates
// exactly one thing — that the AST->event expansion layer (`expand_block`,
// `expand_inline`, and the `Frame` unwinding in `Iterator::next`) is faithful
// and correctly ordered. It cannot detect a *parsing* divergence, because
// there is only one parser. rst's version tests two independent code paths;
// this one tests one path plus its expansion layer.
//
// The check does have teeth within that scope: verified by mutation, dropping
// `EndParagraph` from the projection fails 62 of 85 fixtures.
mod asciidoc_events_check {
    use super::{find_input, fixtures_root};
    use asciidoc::{AsciiDoc, Block, DefinitionItem, ImageData, Inline, OwnedEvent, QuoteType};
    use std::borrow::Cow;
    use std::path::PathBuf;

    // AST → events projection
    // ---------------------------------------------------------------------------

    /// Reconstruct the exact [`asciidoc::Event`] sequence `events()` must produce
    /// for `doc`.
    ///
    /// Every `Block` and `Inline` variant of the crate's AST is covered; a new
    /// variant added to either enum makes this fail to compile, which is the point.
    fn ad_ast_to_events(doc: &AsciiDoc) -> Vec<OwnedEvent> {
        let mut out = vec![OwnedEvent::StartDocument];
        for b in &doc.blocks {
            ad_block_events(b, &mut out);
        }
        out.push(OwnedEvent::EndDocument);
        out
    }

    /// `Event::Figure` / `Event::InlineImage` carry `{ url, alt, title }` while the
    /// AST's `ImageData` carries `{ url, alt, width, height }` — there is no
    /// faithful mapping, so which AST field lands in `title` is a genuine contract
    /// ambiguity the types cannot settle. Resolved by consulting
    /// `EventIter::expand_block` / `expand_inline` in
    /// `crates/formats/asciidoc/src/parse.rs`, which both compute
    /// `image.height.or(image.width)`.
    ///
    /// (Noting for the record, since it is not a projection question: the reverse
    /// direction disagrees with itself — `events::handle_event` puts `title` back
    /// into `height` and never restores `width`, and `writer.rs` discards `title`
    /// outright. So `width` is unrecoverable through the event stream either way.)
    fn ad_image_title(image: &ImageData) -> Option<String> {
        image.height.clone().or_else(|| image.width.clone())
    }

    fn ad_block_events(b: &Block, out: &mut Vec<OwnedEvent>) {
        match b {
            Block::Paragraph {
                inlines,
                id,
                role,
                checked,
                ..
            } => {
                out.push(OwnedEvent::StartParagraph {
                    id: id.clone(),
                    role: role.clone(),
                    checked: *checked,
                });
                ad_inline_events(inlines, out);
                out.push(OwnedEvent::EndParagraph);
            }
            Block::Heading {
                level,
                inlines,
                id,
                role,
                ..
            } => {
                out.push(OwnedEvent::StartHeading {
                    level: *level,
                    id: id.clone(),
                    role: role.clone(),
                });
                ad_inline_events(inlines, out);
                out.push(OwnedEvent::EndHeading);
            }
            Block::CodeBlock {
                content, language, ..
            } => {
                out.push(OwnedEvent::StartCodeBlock {
                    language: language.clone(),
                });
                // Content is delivered as exactly one `CodeBlockContent` event —
                // `Block::CodeBlock` holds one `String`, and the event carries the
                // whole of it. (A chunked reader could legitimately split this;
                // `events()` operates on a materialised block, so it does not.)
                out.push(OwnedEvent::CodeBlockContent(Cow::Owned(content.clone())));
                out.push(OwnedEvent::EndCodeBlock);
            }
            Block::Blockquote {
                children,
                attribution,
                ..
            } => {
                out.push(OwnedEvent::StartBlockquote {
                    attribution: attribution.clone(),
                });
                for c in children {
                    ad_block_events(c, out);
                }
                out.push(OwnedEvent::EndBlockquote);
            }
            Block::List {
                ordered,
                items,
                style,
                ..
            } => {
                out.push(OwnedEvent::StartList {
                    ordered: *ordered,
                    style: style.clone(),
                });
                for item in items {
                    out.push(OwnedEvent::StartListItem);
                    for c in item {
                        ad_block_events(c, out);
                    }
                    out.push(OwnedEvent::EndListItem);
                }
                out.push(OwnedEvent::EndList);
            }
            Block::DefinitionList { items, .. } => {
                out.push(OwnedEvent::StartDefinitionList);
                for DefinitionItem { term, desc } in items {
                    out.push(OwnedEvent::StartDefinitionTerm);
                    ad_inline_events(term, out);
                    out.push(OwnedEvent::EndDefinitionTerm);
                    out.push(OwnedEvent::StartDefinitionDesc);
                    ad_inline_events(desc, out);
                    out.push(OwnedEvent::EndDefinitionDesc);
                }
                out.push(OwnedEvent::EndDefinitionList);
            }
            Block::HorizontalRule { .. } => out.push(OwnedEvent::HorizontalRule),
            Block::PageBreak { .. } => out.push(OwnedEvent::PageBreak),
            Block::Figure { image, .. } => out.push(OwnedEvent::Figure {
                url: image.url.clone(),
                alt: image.alt.clone(),
                title: ad_image_title(image),
            }),
            Block::Div {
                class,
                title,
                children,
                ..
            } => {
                out.push(OwnedEvent::StartDiv {
                    class: class.clone(),
                    title: title.clone(),
                });
                for c in children {
                    ad_block_events(c, out);
                }
                out.push(OwnedEvent::EndDiv);
            }
            Block::RawBlock {
                format, content, ..
            } => out.push(OwnedEvent::RawBlock {
                format: format.clone(),
                content: content.clone(),
            }),
            Block::MathBlock {
                content, flavor, ..
            } => out.push(OwnedEvent::MathBlock {
                content: content.clone(),
                flavor: flavor.clone(),
            }),
            Block::Table { rows, .. } => {
                out.push(OwnedEvent::StartTable);
                for row in rows {
                    out.push(OwnedEvent::StartTableRow {
                        is_header: row.is_header,
                    });
                    for cell in &row.cells {
                        out.push(OwnedEvent::StartTableCell);
                        ad_inline_events(cell, out);
                        out.push(OwnedEvent::EndTableCell);
                    }
                    out.push(OwnedEvent::EndTableRow);
                }
                out.push(OwnedEvent::EndTable);
            }
        }
    }

    fn ad_inline_events(inlines: &[Inline], out: &mut Vec<OwnedEvent>) {
        for i in inlines {
            match i {
                Inline::Text { text, .. } => out.push(OwnedEvent::Text(Cow::Owned(text.clone()))),
                Inline::Strong(children, _) => {
                    out.push(OwnedEvent::StartStrong);
                    ad_inline_events(children, out);
                    out.push(OwnedEvent::EndStrong);
                }
                Inline::Emphasis(children, _) => {
                    out.push(OwnedEvent::StartEmphasis);
                    ad_inline_events(children, out);
                    out.push(OwnedEvent::EndEmphasis);
                }
                Inline::Code(content, _) => out.push(OwnedEvent::Code(Cow::Owned(content.clone()))),
                Inline::Superscript(children, _) => {
                    out.push(OwnedEvent::StartSuperscript);
                    ad_inline_events(children, out);
                    out.push(OwnedEvent::EndSuperscript);
                }
                Inline::Subscript(children, _) => {
                    out.push(OwnedEvent::StartSubscript);
                    ad_inline_events(children, out);
                    out.push(OwnedEvent::EndSubscript);
                }
                Inline::Highlight(children, _) => {
                    out.push(OwnedEvent::StartHighlight);
                    ad_inline_events(children, out);
                    out.push(OwnedEvent::EndHighlight);
                }
                Inline::Strikeout(children, _) => {
                    out.push(OwnedEvent::StartStrikeout);
                    ad_inline_events(children, out);
                    out.push(OwnedEvent::EndStrikeout);
                }
                Inline::Underline(children, _) => {
                    out.push(OwnedEvent::StartUnderline);
                    ad_inline_events(children, out);
                    out.push(OwnedEvent::EndUnderline);
                }
                Inline::SmallCaps(children, _) => {
                    out.push(OwnedEvent::StartSmallCaps);
                    ad_inline_events(children, out);
                    out.push(OwnedEvent::EndSmallCaps);
                }
                Inline::Quoted {
                    quote_type,
                    children,
                    ..
                } => {
                    // `Event::StartQuoted.quote_type` is a `String` where the AST
                    // has a two-variant enum; the spelling is not derivable from
                    // the types. `events::handle_event` maps the string back with
                    // `if quote_type == "single" { Single } else { Double }`, which
                    // fixes "single"/"double" as the contract.
                    out.push(OwnedEvent::StartQuoted {
                        quote_type: match quote_type {
                            QuoteType::Single => "single".to_string(),
                            QuoteType::Double => "double".to_string(),
                        },
                    });
                    ad_inline_events(children, out);
                    out.push(OwnedEvent::EndQuoted);
                }
                Inline::Link {
                    url,
                    children,
                    target,
                    ..
                } => {
                    out.push(OwnedEvent::StartLink {
                        url: url.clone(),
                        target: target.clone(),
                    });
                    ad_inline_events(children, out);
                    out.push(OwnedEvent::EndLink);
                }
                Inline::Image(image, _) => out.push(OwnedEvent::InlineImage {
                    url: image.url.clone(),
                    alt: image.alt.clone(),
                    title: ad_image_title(image),
                }),
                Inline::LineBreak { .. } => out.push(OwnedEvent::LineBreak),
                Inline::SoftBreak { .. } => out.push(OwnedEvent::SoftBreak),
                Inline::FootnoteRef { label, .. } => out.push(OwnedEvent::FootnoteRef {
                    label: label.clone(),
                }),
                Inline::FootnoteDef {
                    label, children, ..
                } => {
                    out.push(OwnedEvent::StartFootnoteDef {
                        label: label.clone(),
                    });
                    ad_inline_events(children, out);
                    out.push(OwnedEvent::EndFootnoteDef);
                }
                Inline::MathInline {
                    content, flavor, ..
                } => out.push(OwnedEvent::MathInline {
                    content: content.clone(),
                    flavor: flavor.clone(),
                }),
                Inline::RawInline {
                    format, content, ..
                } => out.push(OwnedEvent::RawInline {
                    format: format.clone(),
                    content: content.clone(),
                }),
                Inline::Anchor { id, .. } => out.push(OwnedEvent::Anchor { id: id.clone() }),
            }
        }
    }

    // ---------------------------------------------------------------------------
    // The check
    // ---------------------------------------------------------------------------

    /// `events()` must equal the hand-written AST projection exactly, over
    /// every fixture in `fixtures/asciidoc/`.
    ///
    /// Projection coverage: all 12 `Block` and all 19 `Inline` variants are
    /// handled by exhaustive `match` with no `_` arm, so a new AST variant
    /// breaks the build rather than being silently skipped. The fixtures
    /// drive 61 of the 65 `Event` variants; the four never produced
    /// (`SoftBreak`, `StartQuoted`/`EndQuoted`, `InlineImage`) are a
    /// fixture-suite gap, not a projection gap — no fixture yields
    /// `Inline::SoftBreak`, `Inline::Quoted`, or `Inline::Image`
    /// (`fixtures/asciidoc/image/` produces a block `Figure`).
    #[test]
    fn asciidoc_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("asciidoc");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
            .expect("fixtures/asciidoc dir")
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();

        for path in dirs {
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            // `asciidoc::parse` is infallible (diagnostics, not errors), so unlike
            // the rst check there is no fixture to skip for failing to parse.
            let (doc, _diags) = asciidoc::parse(&input);
            let expected = ad_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> =
                asciidoc::events(&input).map(|e| e.into_owned()).collect();
            checked += 1;

            if expected != actual {
                let at = expected
                    .iter()
                    .zip(actual.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(expected.len().min(actual.len()));
                let lo = at.saturating_sub(2);
                failures.push(format!(
                    "fixture {}: first divergence at event index {at}\n  \
                     expected[{lo}..]: {:?}\n  actual[{lo}..]:   {:?}\n  \
                     (lengths: expected {}, actual {})",
                    path.file_name().unwrap().to_string_lossy(),
                    &expected[lo..expected.len().min(at + 4)],
                    &actual[lo..actual.len().min(at + 4)],
                    expected.len(),
                    actual.len(),
                ));
            }
        }

        assert!(
            checked > 50,
            "expected to check a substantial number of asciidoc fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {} of {checked} fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}
/// `StreamingParser` fed an asciidoc fixture under an adversarial chunking
/// must deliver the same event sequence `events()` delivers over the whole
/// input.
///
/// No sanctioned exception applies: `batch.rs`'s module docs and the
/// `StreamingParser` type docs make only a memory claim (`O(largest block)`),
/// and the crate's own `test_streaming_matches_bulk` asserts exact equality
/// with `events()` — i.e. the crate treats parity as the contract, so the
/// divergences this finds are bugs, not documented departures.
#[test]
fn asciidoc_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("asciidoc");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/asciidoc dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let Ok(input_str) = std::str::from_utf8(&input) else {
            continue;
        };
        let bulk: Vec<asciidoc::OwnedEvent> = asciidoc::events(input_str)
            .map(|e| e.into_owned())
            .collect();
        // Coverage floor, not a pass counter — see the rst equivalent.
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                asciidoc::StreamingParser::new(|e: asciidoc::OwnedEvent| streamed.push(e));
            for chunk in chunks {
                parser.feed(&chunk);
            }
            parser.finish();
            if bulk != streamed {
                if result.is_ok() {
                    result = Err(format!(
                        "StreamingParser diverged from events() for fixture {name} under \
                         chunking {chunking_name}:\n  events():         {bulk:?}\n  \
                         StreamingParser: {streamed:?}"
                    ));
                }
                break;
            }
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of asciidoc fixtures, got {checked}"
    );
    assert_or_known_failure("asciidoc", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(input)` must reproduce what
/// builder `build()` produces for the AST `parse(input)` returned.
///
/// asciidoc's `Writer` is not incrementally streaming — `writer.rs`'s module
/// docs say it "buffers all events, reconstructs the AST, then emits", and
/// `finish()` calls `emit::build`. The check is still meaningful: it drives
/// `events_to_doc`/`DocBuilder`, a substantial second reconstruction of the
/// AST from the event stream, against the AST `parse()` built directly. It
/// passes on all 85 fixtures.
#[test]
fn asciidoc_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("asciidoc");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/asciidoc dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = asciidoc::parse(&input);
        let built = asciidoc::build(&doc);

        let mut w = asciidoc::Writer::new(Vec::<u8>::new());
        for e in asciidoc::events(&input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        checked += 1;
        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {}:\n  build():  \
                 {built:?}\n  Writer(): {streamed:?}",
                path.display()
            ));
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of asciidoc fixtures, got {checked}"
    );
    assert_or_known_failure("asciidoc", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// djot-fmt: events() vs parse(), fully wired
// ---------------------------------------------------------------------------
//
// This check carries real signal for djot-fmt specifically, because the two
// paths are genuinely independent implementations: `parse()` is direct
// recursive descent (`parse_blocks_direct`/`parse_next_block_direct` in
// parse.rs), while `events()` is a line-driven frame-stack state machine
// (`EventIter::next`/`push_next_block_frames`, with `Frame::SubParser` for
// compound-block content). Neither is derived from the other.
//
// Note the doc comment on `events()` in djot-fmt/src/lib.rs claims it "parses
// the input first, then walks the AST yielding owned events" — that is stale
// and describes the hollow pattern CLAUDE.md rejects, not what the code does.
// `EventIter::next` pulls one top-level block at a time straight off the
// source lines. The doc comment should be corrected; tracked in TODO.md.
mod djot_events_check {
    use super::{find_input, fixtures_root};
    use djot_fmt::{Attr, Block, DjotDoc, Inline, OwnedEvent};
    use std::path::PathBuf;

    /// Unpack djot's `Attr` struct into the flattened `(id, classes, kv)` triple
    /// every attribute-carrying `Event` variant uses.
    fn dj_unpack(attr: &Attr) -> (Option<String>, Vec<String>, Vec<(String, String)>) {
        (attr.id.clone(), attr.classes.clone(), attr.kv.clone())
    }

    /// The exact event sequence `djot_fmt::events()` must produce for `doc`.
    ///
    /// CONTRACT (document level): `DjotDoc` carries `blocks`, `footnotes` and
    /// `link_defs` side by side, and the type definitions alone do not say where
    /// the latter two land in the stream. Resolved against `EventIter::next`'s
    /// `None` arm: once top-level blocks are exhausted, footnote defs are pushed
    /// in document order as `StartFootnoteDef`/blocks/`EndFootnoteDef`, i.e. they
    /// trail the body. `link_defs` never appear as events at all — there is no
    /// event variant for them, and `collect_doc_from_iter` recovers them from
    /// `EventIter::link_defs` rather than from the stream.
    fn dj_ast_to_events(doc: &DjotDoc) -> Vec<OwnedEvent> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            dj_block_events(b, &mut out);
        }
        for f in &doc.footnotes {
            out.push(OwnedEvent::StartFootnoteDef {
                label: f.label.clone(),
            });
            for b in &f.blocks {
                dj_block_events(b, &mut out);
            }
            out.push(OwnedEvent::EndFootnoteDef);
        }
        out
    }

    fn dj_block_events(b: &Block, out: &mut Vec<OwnedEvent>) {
        match b {
            Block::Paragraph { inlines, attr, .. } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartParagraph { id, classes, kv });
                dj_inline_events(inlines, out);
                out.push(OwnedEvent::EndParagraph);
            }
            Block::Heading {
                level,
                inlines,
                attr,
                ..
            } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartHeading {
                    level: *level,
                    id,
                    classes,
                    kv,
                });
                dj_inline_events(inlines, out);
                out.push(OwnedEvent::EndHeading);
            }
            Block::Blockquote { blocks, attr, .. } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartBlockquote { id, classes, kv });
                for c in blocks {
                    dj_block_events(c, out);
                }
                out.push(OwnedEvent::EndBlockquote);
            }
            Block::List {
                kind,
                items,
                tight,
                attr,
                ..
            } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartList {
                    kind: kind.clone(),
                    tight: *tight,
                    id,
                    classes,
                    kv,
                });
                for item in items {
                    out.push(OwnedEvent::StartListItem {
                        checked: item.checked,
                    });
                    for c in &item.blocks {
                        dj_block_events(c, out);
                    }
                    out.push(OwnedEvent::EndListItem);
                }
                out.push(OwnedEvent::EndList);
            }
            Block::CodeBlock {
                language,
                content,
                attr,
                ..
            } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartCodeBlock {
                    language: language.clone(),
                    id,
                    classes,
                    kv,
                });
                // CONTRACT: `CodeBlockContent` is emitted unconditionally, even for
                // an empty body — `handle_event`'s `StartCodeBlock` arm seeds the
                // frame with `String::new()` and only `CodeBlockContent` ever writes
                // it, so an omitted event and an empty one are indistinguishable to
                // the tree builder. Confirmed unconditional in `expand_block_frames`.
                out.push(OwnedEvent::CodeBlockContent(content.clone().into()));
                out.push(OwnedEvent::EndCodeBlock);
            }
            Block::RawBlock {
                format, content, ..
            } => {
                // `Event::RawBlock` has no attribute fields, so `Block::RawBlock`'s
                // `attr` has no representation in the stream. That is a lossy point
                // in the Event type itself, not a projection choice.
                out.push(OwnedEvent::RawBlock {
                    format: format.clone(),
                    content: content.clone(),
                });
            }
            Block::Div {
                class,
                blocks,
                attr,
                ..
            } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartDiv {
                    class: class.clone(),
                    id,
                    classes,
                    kv,
                });
                for c in blocks {
                    dj_block_events(c, out);
                }
                out.push(OwnedEvent::EndDiv);
            }
            Block::Table { caption, rows, .. } => {
                // CONTRACT: the caption is carried as a single `TableCaption(Vec<Inline>)`
                // event that *precedes* `StartTable`, not as inline events inside the
                // table. Pinned by `handle_event`: `TableCaption` pushes a
                // `TablePendingCaption` frame which the following `StartTable` pops.
                if let Some(cap) = caption {
                    out.push(OwnedEvent::TableCaption(cap.clone()));
                }
                out.push(OwnedEvent::StartTable);
                for row in rows {
                    out.push(OwnedEvent::StartTableRow {
                        is_header: row.is_header,
                    });
                    for cell in &row.cells {
                        out.push(OwnedEvent::StartTableCell {
                            alignment: cell.alignment.clone(),
                        });
                        dj_inline_events(&cell.inlines, out);
                        out.push(OwnedEvent::EndTableCell);
                    }
                    out.push(OwnedEvent::EndTableRow);
                }
                out.push(OwnedEvent::EndTable);
            }
            Block::ThematicBreak { attr, .. } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::ThematicBreak { id, classes, kv });
            }
            Block::DefinitionList { items, attr, .. } => {
                let (id, classes, kv) = dj_unpack(attr);
                out.push(OwnedEvent::StartDefinitionList { id, classes, kv });
                for item in items {
                    out.push(OwnedEvent::StartDefinitionTerm);
                    dj_inline_events(&item.term, out);
                    out.push(OwnedEvent::EndDefinitionTerm);
                    out.push(OwnedEvent::StartDefinitionDesc);
                    // `DefItem::definitions` is `Vec<Block>` (unlike rst's inline
                    // desc), so the desc body is a block sequence.
                    for c in &item.definitions {
                        dj_block_events(c, out);
                    }
                    out.push(OwnedEvent::EndDefinitionDesc);
                }
                out.push(OwnedEvent::EndDefinitionList);
            }
        }
    }

    fn dj_inline_events(inlines: &[Inline], out: &mut Vec<OwnedEvent>) {
        for i in inlines {
            match i {
                Inline::Text { content, .. } => out.push(OwnedEvent::Text(content.clone().into())),
                Inline::SoftBreak { .. } => out.push(OwnedEvent::SoftBreak),
                Inline::HardBreak { .. } => out.push(OwnedEvent::HardBreak),
                Inline::Emphasis { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartEmphasis { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndEmphasis);
                }
                Inline::Strong { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartStrong { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndStrong);
                }
                Inline::Delete { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartDelete { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndDelete);
                }
                Inline::Insert { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartInsert { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndInsert);
                }
                Inline::Highlight { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartHighlight { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndHighlight);
                }
                Inline::Subscript { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartSubscript { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndSubscript);
                }
                Inline::Superscript { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartSuperscript { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndSuperscript);
                }
                Inline::Verbatim { content, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::Verbatim {
                        content: content.clone().into(),
                        id,
                        classes,
                        kv,
                    });
                }
                Inline::MathInline { content, .. } => {
                    out.push(OwnedEvent::MathInline(content.clone().into()))
                }
                Inline::MathDisplay { content, .. } => {
                    out.push(OwnedEvent::MathDisplay(content.clone().into()))
                }
                Inline::RawInline {
                    format, content, ..
                } => out.push(OwnedEvent::RawInline {
                    format: format.clone(),
                    content: content.clone(),
                }),
                Inline::Link {
                    inlines,
                    url,
                    title,
                    attr,
                    ..
                } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartLink {
                        url: url.clone(),
                        title: title.clone(),
                        id,
                        classes,
                        kv,
                    });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndLink);
                }
                Inline::Image {
                    inlines,
                    url,
                    title,
                    attr,
                    ..
                } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    // Unlike rst's leaf `InlineImage`, djot's image is a container
                    // pair: the alt text is the child inline sequence.
                    out.push(OwnedEvent::StartImage {
                        url: url.clone(),
                        title: title.clone(),
                        id,
                        classes,
                        kv,
                    });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndImage);
                }
                Inline::Span { inlines, attr, .. } => {
                    let (id, classes, kv) = dj_unpack(attr);
                    out.push(OwnedEvent::StartSpan { id, classes, kv });
                    dj_inline_events(inlines, out);
                    out.push(OwnedEvent::EndSpan);
                }
                Inline::FootnoteRef { label, .. } => {
                    out.push(OwnedEvent::FootnoteRef(label.clone()))
                }
                Inline::Symbol { name, .. } => out.push(OwnedEvent::Symbol(name.clone())),
                Inline::Autolink { url, is_email, .. } => out.push(OwnedEvent::Autolink {
                    url: url.clone(),
                    is_email: *is_email,
                }),
            }
        }
    }

    /// `events()` must equal the hand-written AST projection exactly, over
    /// every fixture in `fixtures/djot/`.
    #[test]
    fn djot_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("djot");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
            .expect("fixtures/djot dir")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();

        for path in dirs {
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            // `parse()` is infallible and returns diagnostics alongside the doc;
            // diagnostics are not this check's concern.
            let (doc, _diags) = djot_fmt::parse(&input);
            let expected = dj_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> =
                djot_fmt::events(&input).map(|e| e.into_owned()).collect();
            checked += 1;

            if expected != actual {
                let at = expected
                    .iter()
                    .zip(actual.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(expected.len().min(actual.len()));
                let lo = at.saturating_sub(2);
                failures.push(format!(
                    "{}: first divergence at event #{at} (expected {} events, got {})\n  \
                     expected[{lo}..]: {:?}\n  actual[{lo}..]:   {:?}",
                    path.file_name().unwrap().to_string_lossy(),
                    expected.len(),
                    actual.len(),
                    &expected[lo..expected.len().min(at + 4)],
                    &actual[lo..actual.len().min(at + 4)],
                ));
            }
        }

        assert!(
            checked > 50,
            "expected to check a substantial number of djot fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {}/{checked} djot fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}
/// `StreamingParser` fed a djot fixture under an adversarial chunking must
/// deliver the same event sequence `events()` delivers over the whole input.
///
/// No sanctioned exception applies: `batch.rs`'s doc comments make only a
/// memory claim (`O(largest block)`) plus a nesting claim that one of the
/// bugs below actually violates, and CLAUDE.md names commonmark-fmt as the
/// only streaming exemption.
#[test]
fn djot_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("djot");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/djot dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let Ok(input_str) = std::str::from_utf8(&input) else {
            continue;
        };
        let bulk: Vec<djot_fmt::OwnedEvent> = djot_fmt::events(input_str)
            .map(|e| e.into_owned())
            .collect();
        // Coverage floor, not a pass counter — see the rst equivalent.
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                djot_fmt::StreamingParser::new(|e: djot_fmt::OwnedEvent| streamed.push(e));
            for chunk in chunks {
                parser.feed(&chunk);
            }
            parser.finish();
            if bulk != streamed {
                if result.is_ok() {
                    result = Err(format!(
                        "StreamingParser diverged from events() for fixture {name} under \
                         chunking {chunking_name}:\n  events():         {bulk:?}\n  \
                         StreamingParser: {streamed:?}"
                    ));
                }
                break;
            }
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of djot fixtures, got {checked}"
    );
    assert_or_known_failure("djot", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(input)` must reproduce what
/// builder `emit()` produces for the AST `parse(input)` returned.
///
/// djot's `Writer` is not incrementally streaming — `writer.rs`'s module docs
/// say it "buffers all events, reconstructs the AST, then emits", and
/// `finish()` calls `emit::emit`. The check still exercises `events_to_doc`/
/// `DocBuilder`, a substantial second AST reconstruction, against `parse()`'s
/// AST — and that is exactly where it finds the tracked defect.
#[test]
fn djot_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("djot");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/djot dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = djot_fmt::parse(&input);
        let built = djot_fmt::emit(&doc);

        let mut w = djot_fmt::Writer::new(Vec::<u8>::new());
        for e in djot_fmt::events(&input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        checked += 1;
        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from emit() for fixture {}:\n  emit():   \
                 {built:?}\n  Writer(): {streamed:?}",
                path.display()
            ));
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of djot fixtures, got {checked}"
    );
    assert_or_known_failure("djot", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// html-fmt: events()/StreamingParser are documented tree-walk projections;
// the streaming Writer is independent code and gets a real byte-identical check
// ---------------------------------------------------------------------------
//
// html-fmt is html5ever-backed, and CLAUDE.md puts third-party-library-backed
// formats (pulldown-cmark, html5ever) out of scope for the "three independently
// optimal reader APIs" mandate. The crate does not merely fail that mandate
// silently — it documents the reason at module and crate level:
//
//   crates/formats/html-fmt/src/batch.rs (module docs):
//     "The HTML5 parsing algorithm requires tree construction for correctness —
//      the spec mandates operations like foster parenting, implied element
//      insertion, and adoption agency that can rearrange previously-seen nodes.
//      This means truly incremental event delivery (events emitted during
//      `feed()`) is not possible without building the full tree first."
//
//   crates/formats/html-fmt/src/lib.rs (crate docs):
//     "All three reader APIs build the full parse tree internally. `events()`
//      and `StreamingParser` walk the tree to produce events after
//      construction. This is a fundamental limitation of the HTML5 spec, not a
//      library choice."
//
// Concretely: `html_fmt::events()` is `events_from_doc(&parse(input).0)` — a
// depth-first walk of the finished tree into a `Vec<OwnedEvent>` — and
// `StreamingParser::feed()` is a bare `buf.extend_from_slice(chunk)` with all
// parsing and handler dispatch deferred to `finish()`. An
// "events() == ast_to_events(parse())" equivalence check would therefore be
// tautological (both sides are literally the same tree walk) and carry zero
// signal, which is why those two APIs are declared `NotApplicable` with the
// citations above rather than given a check that would pass by construction and
// misrepresent html-fmt as having independent streaming readers.
//
// Two checks below still carry real signal:
//
//  * the streaming `Writer` (`writer.rs`) writes bytes to its sink directly and
//    shares nothing with `emit.rs`'s `Emitter` except the two escaping helpers,
//    so byte-identity against `emit()` is a genuine cross-implementation check;
//  * `StreamingParser`'s chunk buffering is checked for byte-boundary
//    correctness (a mid-UTF-8-character split must not corrupt the buffer),
//    which is the one property `feed()` can actually get wrong.

/// The streaming `Writer` must produce byte-identical output to builder
/// `emit()` over every html fixture.
///
/// `emit()`'s default `EmitOptions` is non-pretty, which is the mode `Writer`
/// implements (it has no pretty-printing path at all), so this compares the
/// two independent serializers on equal terms.
#[test]
fn html_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("html");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/html dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _) = html_fmt::parse(&input);
        let built = html_fmt::emit(&doc);

        let mut w = html_fmt::Writer::new(Vec::<u8>::new());
        for e in html_fmt::events(&input) {
            w.write_event(e);
        }
        let streamed = w.finish();

        // Count as checked as soon as both serializations exist, not only when
        // they match — `checked` is a coverage floor, not a pass counter (see
        // the rst StreamingParser test for why gating it on success makes the
        // floor depend on `read_dir` ordering).
        checked += 1;
        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from emit() for fixture {}:\n  emit():  {}\n  \
                 Writer: {}",
                path.display(),
                String::from_utf8_lossy(&built),
                String::from_utf8_lossy(&streamed),
            ));
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of html fixtures, got {checked}"
    );
    assert_or_known_failure("html", "streaming_writer", result);
}

/// `StreamingParser` buffers all input and dispatches at `finish()` (see the
/// module comment above). The one property that buffering can still get wrong
/// is chunk-boundary handling, so this feeds every html fixture under the
/// adversarial chunkings — including a split landing inside a multi-byte UTF-8
/// character — and requires the delivered event sequence to equal `events()`
/// over the whole input at once.
///
/// This is deliberately *not* claimed as a `Wired` `streaming_parser`
/// capability: it verifies buffering integrity, not the incremental event
/// delivery html-fmt documents it cannot provide.
#[test]
fn html_streaming_parser_buffering_survives_adversarial_chunking() {
    let root = fixtures_root().join("html");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/html dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let bulk: Vec<html_fmt::OwnedEvent> = html_fmt::events(&input).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                html_fmt::StreamingParser::new(|e: html_fmt::OwnedEvent| streamed.push(e));
            for chunk in chunks {
                parser.feed(&chunk);
            }
            parser.finish();
            assert_eq!(
                bulk,
                streamed,
                "StreamingParser chunk buffering corrupted input for fixture {} under \
                 chunking {chunking_name}",
                path.display()
            );
        }
    }
    assert!(
        checked > 50,
        "expected to check a substantial number of html fixtures, got {checked}"
    );
}

// ---------------------------------------------------------------------------
// ooxml-wml (docx) events(): known-failure — Text-drop / End-tag reversal
// ---------------------------------------------------------------------------

/// Minimal, realistic WML fragment: a paragraph containing a single run with
/// no `<w:pPr>` before the run — the most common shape in real DOCX body
/// content. Wrapped in `<w:document><w:body>…</w:body></w:document>` since
/// `events()` takes the raw `word/document.xml` content.
const WML_SIMPLE_PARAGRAPH: &[u8] = br#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body><w:p><w:r><w:t>Hello world</w:t></w:r></w:p></w:body>
</w:document>"#;

#[test]
fn wml_events_reaches_and_correctly_orders_paragraph_text() {
    let events: Vec<_> = ooxml_wml::events::events(WML_SIMPLE_PARAGRAPH).collect();

    let has_text = events
        .iter()
        .any(|e| matches!(e, ooxml_wml::WmlEvent::Text(t) if t.contains("Hello world")));

    let well_nested = {
        // A minimal well-nestedness check: EndRun must come before EndParagraph
        // (the run opened after, and inside, the paragraph).
        let end_run_idx = events
            .iter()
            .position(|e| matches!(e, ooxml_wml::WmlEvent::EndRun));
        let end_para_idx = events
            .iter()
            .position(|e| matches!(e, ooxml_wml::WmlEvent::EndParagraph));
        matches!((end_run_idx, end_para_idx), (Some(r), Some(p)) if r < p)
    };

    let result = if has_text && well_nested {
        Ok(())
    } else {
        Err(format!(
            "expected a Text(\"Hello world\") event and EndRun before EndParagraph; got {events:?}"
        ))
    };
    assert_or_known_failure("docx", "events", result);
}

// ---------------------------------------------------------------------------
// ooxml-pml (pptx) events(): known-failure — txBody unreachable
// ---------------------------------------------------------------------------

const PML_SIMPLE_SLIDE: &[u8] = br#"<?xml version="1.0"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
<p:cSld><p:spTree>
<p:sp><p:txBody><a:p><a:r><a:t>Hello world</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld>
</p:sld>"#;

#[test]
fn pml_events_reaches_slide_text() {
    let events: Vec<_> = ooxml_pml::events::events(PML_SIMPLE_SLIDE).collect();
    let has_text = events
        .iter()
        .any(|e| format!("{e:?}").contains("Hello world"));
    let result = if has_text {
        Ok(())
    } else {
        Err(format!(
            "expected an event carrying \"Hello world\" text from the slide's txBody; got {} \
             events: {events:?}",
            events.len()
        ))
    };
    assert_or_known_failure("pptx", "events", result);
}

// ---------------------------------------------------------------------------
// ooxml-sml (xlsx) events(): real, passing check
// ---------------------------------------------------------------------------

const SML_SIMPLE_SHEET: &[u8] = br#"<?xml version="1.0"?>
<worksheet><sheetData>
<row r="1"><c r="A1" t="inlineStr"><is><t>hi</t></is></c></row>
</sheetData></worksheet>"#;

#[test]
fn sml_events_reaches_cell_text() {
    let events: Vec<_> = ooxml_sml::events::events(SML_SIMPLE_SHEET).collect();
    let has_text = events
        .iter()
        .any(|e| matches!(e, ooxml_sml::SmlEvent::StringFragment(t) if t.contains("hi")));
    assert!(
        has_text,
        "expected a StringFragment(\"hi\") event from the inline string cell; got {events:?}"
    );
}

// ---------------------------------------------------------------------------
// ooxml-sml (xlsx) streaming writer: real, passing fidelity check
// ---------------------------------------------------------------------------
//
// This mirrors the fix already pinned by
// crates/formats/ooxml-sml/tests/streaming_writer.rs
// (`row_and_cell_attributes_pass_through`) — reproduced here, independently,
// as part of the fixture harness so the property is checked from this
// suite's vantage point too, not only the crate's own test file.

struct SharedSink(std::rc::Rc<std::cell::RefCell<Vec<u8>>>, u64);

impl std::io::Write for SharedSink {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let mut v = self.0.borrow_mut();
        let pos = self.1 as usize;
        if v.len() < pos + data.len() {
            v.resize(pos + data.len(), 0);
        }
        v[pos..pos + data.len()].copy_from_slice(data);
        self.1 += data.len() as u64;
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::io::Seek for SharedSink {
    fn seek(&mut self, from: std::io::SeekFrom) -> std::io::Result<u64> {
        let len = self.0.borrow().len() as u64;
        self.1 = match from {
            std::io::SeekFrom::Start(n) => n,
            std::io::SeekFrom::End(n) => (len as i64 + n) as u64,
            std::io::SeekFrom::Current(n) => (self.1 as i64 + n) as u64,
        };
        Ok(self.1)
    }
}

/// `SmlWriter::finish()` produces a complete XLSX zip package (content
/// types, rels, workbook part, worksheet part — not a bare XML fragment),
/// so unlike rst's `Writer` this cannot be compared byte-for-byte against a
/// builder; instead this extracts `xl/worksheets/sheet1.xml` and checks the
/// row/cell attributes survived.
#[test]
fn sml_streaming_writer_preserves_row_and_cell_attributes() {
    use ooxml_sml::generated::{Cell, CellType, Row};
    use ooxml_sml::{SmlEvent, SmlWriter};
    use std::io::Read;

    let row = Row {
        reference: Some(7),
        height: Some(30.0),
        ..Default::default()
    };
    let cell = Cell {
        reference: Some("A7".to_string()),
        cell_type: Some(CellType::String),
        style_index: Some(3),
        ..Default::default()
    };

    let buf = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
    let mut writer = SmlWriter::new(SharedSink(buf.clone(), 0));
    for e in [
        SmlEvent::StartWorkbook,
        SmlEvent::StartWorksheet,
        SmlEvent::StartSheetData,
        SmlEvent::StartRow {
            props: Box::new(row),
        },
        SmlEvent::StartCell {
            props: Box::new(cell),
        },
        SmlEvent::CellValue("hello".into()),
        SmlEvent::EndCell,
        SmlEvent::EndRow,
        SmlEvent::EndSheetData,
        SmlEvent::EndWorksheet,
        SmlEvent::EndWorkbook,
    ] {
        writer.write_event(e);
    }
    writer.finish().expect("SmlWriter::finish");

    let xlsx = buf.borrow().clone();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(xlsx)).expect("valid zip package");
    let mut sheet_xml = String::new();
    zip.by_name("xl/worksheets/sheet1.xml")
        .expect("worksheet part present")
        .read_to_string(&mut sheet_xml)
        .expect("read worksheet part");

    assert!(
        sheet_xml.contains(r#"r="7""#),
        "streaming writer dropped row number: {sheet_xml}"
    );
    assert!(
        sheet_xml.contains(r#"s="3""#),
        "streaming writer dropped cell style_index: {sheet_xml}"
    );
}

// ---------------------------------------------------------------------------
// texinfo: events() vs parse(), real and passing. StreamingParser and the
// streaming Writer are both buffer-until-finish wrappers (see their own
// module docs in crates/formats/texinfo/src/batch.rs and writer.rs, which
// self-report "Memory usage is O(full input)" / "buffers all events,
// reconstructs the AST, then emits") rather than genuine incremental
// implementations; tracked as KnownFailure entries.
// ---------------------------------------------------------------------------

fn texinfo_ast_to_events(doc: &texinfo::TexinfoDoc) -> Vec<texinfo::events::Event<'static>> {
    let mut out = Vec::new();
    for b in &doc.blocks {
        texinfo_block_events(b, &mut out);
    }
    out
}

fn texinfo_block_events(b: &texinfo::Block, out: &mut Vec<texinfo::events::Event<'static>>) {
    use std::borrow::Cow;
    use texinfo::Block;
    use texinfo::events::Event;
    match b {
        Block::Heading {
            level,
            kind,
            inlines,
            ..
        } => {
            out.push(Event::StartHeading {
                level: *level,
                kind: kind.clone(),
            });
            for i in inlines {
                texinfo_inline_events(i, out);
            }
            out.push(Event::EndHeading);
        }
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for i in inlines {
                texinfo_inline_events(i, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::CodeBlock {
            variant, content, ..
        } => {
            out.push(Event::CodeBlock {
                variant: variant.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for c in children {
                texinfo_block_events(c, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for i in item {
                    texinfo_inline_events(i, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for (term, desc) in items {
                out.push(Event::StartDefinitionTerm);
                for i in term {
                    texinfo_inline_events(i, out);
                }
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                for db in desc {
                    texinfo_block_events(db, out);
                }
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow {
                    is_header: row.is_header,
                });
                for cell in &row.cells {
                    out.push(Event::StartTableCell);
                    for i in cell {
                        texinfo_inline_events(i, out);
                    }
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::Menu { entries, .. } => {
            out.push(Event::StartMenu);
            for e in entries {
                out.push(Event::MenuEntry {
                    node: e.node.clone(),
                    description: e.description.clone(),
                });
            }
            out.push(Event::EndMenu);
        }
        Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
        Block::RawBlock {
            environment,
            content,
            ..
        } => out.push(Event::RawBlock {
            environment: environment.clone(),
            content: content.clone(),
        }),
        Block::Float {
            float_type,
            label,
            children,
            ..
        } => {
            out.push(Event::StartFloat {
                float_type: float_type.clone(),
                label: label.clone(),
            });
            for c in children {
                texinfo_block_events(c, out);
            }
            out.push(Event::EndFloat);
        }
        Block::NoIndent { .. } => out.push(Event::NoIndent),
    }
}

fn texinfo_inline_events(i: &texinfo::Inline, out: &mut Vec<texinfo::events::Event<'static>>) {
    use std::borrow::Cow;
    use texinfo::Inline;
    use texinfo::events::Event;
    match i {
        Inline::Text(s, _) => out.push(Event::Text(Cow::Owned(s.clone()))),
        Inline::Strong(c, _) => {
            out.push(Event::StartStrong);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndStrong);
        }
        Inline::Emphasis(c, _) => {
            out.push(Event::StartEmphasis);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndEmphasis);
        }
        Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
        Inline::Var(c, _) => {
            out.push(Event::StartVar);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndVar);
        }
        Inline::File(s, _) => out.push(Event::File(Cow::Owned(s.clone()))),
        Inline::Command(s, _) => out.push(Event::Command(Cow::Owned(s.clone()))),
        Inline::Option(s, _) => out.push(Event::Option(Cow::Owned(s.clone()))),
        Inline::Env(s, _) => out.push(Event::Env(Cow::Owned(s.clone()))),
        Inline::Samp(s, _) => out.push(Event::Samp(Cow::Owned(s.clone()))),
        Inline::Kbd(s, _) => out.push(Event::Kbd(Cow::Owned(s.clone()))),
        Inline::Key(s, _) => out.push(Event::Key(Cow::Owned(s.clone()))),
        Inline::Dfn(c, _) => {
            out.push(Event::StartDfn);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndDfn);
        }
        Inline::Cite(s, _) => out.push(Event::Cite(Cow::Owned(s.clone()))),
        Inline::Acronym {
            abbrev, expansion, ..
        } => out.push(Event::Acronym {
            abbrev: abbrev.clone(),
            expansion: expansion.clone(),
        }),
        Inline::Abbr {
            abbrev, expansion, ..
        } => out.push(Event::Abbr {
            abbrev: abbrev.clone(),
            expansion: expansion.clone(),
        }),
        Inline::Roman(s, _) => out.push(Event::Roman(Cow::Owned(s.clone()))),
        Inline::SmallCaps(s, _) => out.push(Event::SmallCaps(Cow::Owned(s.clone()))),
        Inline::DirectItalic(c, _) => {
            out.push(Event::StartDirectItalic);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndDirectItalic);
        }
        Inline::DirectBold(c, _) => {
            out.push(Event::StartDirectBold);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndDirectBold);
        }
        Inline::DirectTypewriter(s, _) => {
            out.push(Event::DirectTypewriter(Cow::Owned(s.clone())));
        }
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink { url: url.clone() });
            for x in children {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image {
            file,
            width,
            height,
            alt,
            extension,
            ..
        } => out.push(Event::Image {
            file: file.clone(),
            width: width.clone(),
            height: height.clone(),
            alt: alt.clone(),
            extension: extension.clone(),
        }),
        Inline::Superscript(c, _) => {
            out.push(Event::StartSuperscript);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndSuperscript);
        }
        Inline::Subscript(c, _) => {
            out.push(Event::StartSubscript);
            for x in c {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndSubscript);
        }
        Inline::LineBreak { .. } => out.push(Event::LineBreak),
        Inline::SoftBreak { .. } => out.push(Event::SoftBreak),
        Inline::FootnoteDef { content, .. } => {
            out.push(Event::StartFootnoteDef);
            for x in content {
                texinfo_inline_events(x, out);
            }
            out.push(Event::EndFootnoteDef);
        }
        Inline::CrossRef {
            kind, node, text, ..
        } => out.push(Event::CrossRef {
            kind: kind.clone(),
            node: node.clone(),
            text: text.clone(),
        }),
        Inline::Anchor { name, .. } => out.push(Event::Anchor { name: name.clone() }),
        Inline::NoBreak(s, _) => out.push(Event::NoBreak(Cow::Owned(s.clone()))),
        Inline::Email { address, text, .. } => out.push(Event::Email {
            address: address.clone(),
            text: text.clone(),
        }),
        Inline::Symbol(kind, _) => out.push(Event::Symbol(kind.clone())),
    }
}

#[test]
fn texinfo_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("texinfo");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/texinfo dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = texinfo::parse(&input);
        let expected = texinfo_ast_to_events(&doc);
        let actual: Vec<_> = texinfo::events::events(&input).collect();
        assert_eq!(
            expected,
            actual,
            "events() diverged from the AST projection for fixture {}",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of texinfo fixtures, got {checked}"
    );
}

/// `StreamingParser` buffers all fed bytes and only parses+delivers events
/// inside `finish()` (see `crates/formats/texinfo/src/batch.rs`), so this
/// check verifies two things: (1) the final event sequence still matches
/// `events()` under adversarial chunking (expected to hold, since finish()
/// just calls `events()` on the reassembled buffer), and (2) `feed()` alone,
/// without `finish()`, actually delivers events incrementally as input
/// arrives — which it does not, because all parsing is deferred to
/// `finish()`. (2) is the real, previously-undocumented defect this check
/// surfaces.
#[test]
fn texinfo_streaming_parser_matches_events_and_is_incremental() {
    let root = fixtures_root().join("texinfo");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/texinfo dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let Ok(input_str) = std::str::from_utf8(&input) else {
            continue;
        };
        let bulk: Vec<texinfo::OwnedEvent> = texinfo::events::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                texinfo::StreamingParser::new(|e: texinfo::OwnedEvent| streamed.push(e));
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}"
                ));
            }
        }

        if input.len() > 32 && !bulk.is_empty() {
            let mid = input.len() / 2;
            let mut delivered: Vec<texinfo::OwnedEvent> = Vec::new();
            let mut parser = texinfo::StreamingParser::new(|e| delivered.push(e));
            parser.feed(&input[..mid]);
            if delivered.is_empty() && result.is_ok() {
                result = Err(format!(
                    "StreamingParser delivered zero events to the handler after feed() with \
                     half of fixture {name} ({mid} bytes) and before finish() — \
                     texinfo::batch::StreamingParser buffers all input into a Vec<u8> (see \
                     crates/formats/texinfo/src/batch.rs's own module doc, \"Memory usage is \
                     O(full input)\") and only parses and delivers events inside finish(), so \
                     feed() never advances real incremental parser state"
                ));
            }
            // `parser` is intentionally dropped here without calling finish(): this
            // probe only needs to observe pre-finish handler state.
        }
    }
    assert!(
        checked > 5,
        "expected to check several texinfo fixtures, got {checked}"
    );
    assert_or_known_failure("texinfo", "streaming_parser", result);
}

/// `Writer` buffers all fed events into a `Vec<OwnedEvent>` and only
/// reconstructs the AST and calls `emit()` inside `finish()` (see
/// `crates/formats/texinfo/src/writer.rs`'s own module doc, "This
/// implementation buffers all events, reconstructs the AST, then emits").
/// Content-wise this still round-trips correctly for most fixtures (since
/// finish() ends up calling the same `emit()` the builder path uses) with one
/// exception this check found: the `Event` enum has no representation for
/// `TexinfoDoc::title` (no `@settitle`-carrying event exists), so
/// `events_to_doc()` always reconstructs a document with `title: None`,
/// silently dropping `@settitle` for any fixture that has one.
#[test]
fn texinfo_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("texinfo");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/texinfo dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = texinfo::parse(&input);
        let built = texinfo::emit(&doc);

        let mut w = texinfo::Writer::new(Vec::<u8>::new());
        for e in texinfo::events::events(&input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");
        checked += 1;

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}:\n  build():  \
                 {built:?}\n  streamed: {streamed:?}"
            ));
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of texinfo fixtures, got {checked}"
    );

    // Incrementality probe: a byte-identical final result (checked above)
    // only proves the *content* is right, not that the writer is genuinely
    // streaming. Feed several complete events (well short of finish()) and
    // check whether any bytes have already reached the sink.
    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = texinfo::Writer::new(ObservableSink(observed.clone()));
        w.write_event(texinfo::OwnedEvent::StartHeading {
            level: 1,
            kind: texinfo::HeadingKind::Numbered,
        });
        w.write_event(texinfo::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(texinfo::OwnedEvent::EndHeading);
        w.write_event(texinfo::OwnedEvent::StartParagraph);
        w.write_event(texinfo::OwnedEvent::Text("World".to_string().into()));
        w.write_event(texinfo::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after 6 complete write_event() calls (a \
                 full heading + paragraph) and before finish() — texinfo::writer::Writer \
                 buffers all events into a Vec<OwnedEvent> and only reconstructs the AST + \
                 calls emit() inside finish() (crates/formats/texinfo/src/writer.rs), so it is \
                 not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("texinfo", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// fb2-fmt: events() is a genuine incremental quick_xml pull parser (real,
// passing). The streaming Writer is likewise genuine — write_event() writes
// straight to the underlying quick_xml::Writer<W>, no buffering (real,
// passing). StreamingParser, however, just accumulates fed bytes into a
// Vec<u8> and defers all parsing to finish() (see
// crates/formats/fb2-fmt/src/events.rs's StreamingParser::finish); tracked
// as a KnownFailure.
// ---------------------------------------------------------------------------

fn fb2_ast_to_events(fb: &fb2_fmt::FictionBook) -> Vec<fb2_fmt::Event> {
    let mut out = Vec::new();
    out.push(fb2_fmt::Event::StartFictionBook);
    out.push(fb2_fmt::Event::Metadata(Box::new(fb.description.clone())));
    for body in &fb.bodies {
        out.push(fb2_fmt::Event::StartBody {
            name: body.name.clone(),
            lang: body.lang.clone(),
        });
        if let Some(title) = &body.title {
            fb2_title_events(title, &mut out);
        }
        for epigraph in &body.epigraph {
            fb2_epigraph_events(epigraph, &mut out);
        }
        for section in &body.section {
            fb2_section_events(section, &mut out);
        }
        out.push(fb2_fmt::Event::EndBody);
    }
    for binary in &fb.binaries {
        out.push(fb2_fmt::Event::Binary(binary.clone()));
    }
    out.push(fb2_fmt::Event::EndFictionBook);
    out
}

fn fb2_section_events(section: &fb2_fmt::Section, out: &mut Vec<fb2_fmt::Event>) {
    out.push(fb2_fmt::Event::StartSection {
        id: section.id.clone(),
        lang: section.lang.clone(),
    });
    if let Some(title) = &section.title {
        fb2_title_events(title, out);
    }
    for epigraph in &section.epigraph {
        fb2_epigraph_events(epigraph, out);
    }
    for content in &section.content {
        fb2_section_content_events(content, out);
    }
    for nested in &section.section {
        fb2_section_events(nested, out);
    }
    out.push(fb2_fmt::Event::EndSection);
}

fn fb2_title_events(title: &fb2_fmt::Title, out: &mut Vec<fb2_fmt::Event>) {
    use fb2_fmt::TitlePara;
    out.push(fb2_fmt::Event::StartTitle);
    for para in &title.para {
        match para {
            TitlePara::Para(il) => out.push(fb2_fmt::Event::TitleParagraph(il.clone())),
            TitlePara::EmptyLine => out.push(fb2_fmt::Event::EmptyLine),
        }
    }
    out.push(fb2_fmt::Event::EndTitle);
}

fn fb2_section_content_events(content: &fb2_fmt::SectionContent, out: &mut Vec<fb2_fmt::Event>) {
    use fb2_fmt::SectionContent;
    match content {
        SectionContent::Para(il) => {
            out.push(fb2_fmt::Event::StartParagraph);
            out.push(fb2_fmt::Event::Inline(il.clone()));
            out.push(fb2_fmt::Event::EndParagraph);
        }
        SectionContent::EmptyLine => out.push(fb2_fmt::Event::EmptyLine),
        SectionContent::Subtitle(il) => out.push(fb2_fmt::Event::Subtitle(il.clone())),
        SectionContent::Image(img) => out.push(fb2_fmt::Event::Image(img.clone())),
        SectionContent::Poem(p) => fb2_poem_events(p, out),
        SectionContent::Cite(c) => fb2_cite_events(c, out),
        SectionContent::Table(t) => out.push(fb2_fmt::Event::Table(t.clone())),
    }
}

fn fb2_poem_events(poem: &fb2_fmt::Poem, out: &mut Vec<fb2_fmt::Event>) {
    out.push(fb2_fmt::Event::StartPoem);
    if let Some(title) = &poem.title {
        fb2_title_events(title, out);
    }
    for epigraph in &poem.epigraph {
        fb2_epigraph_events(epigraph, out);
    }
    for stanza in &poem.stanza {
        out.push(fb2_fmt::Event::StartStanza);
        for v in &stanza.v {
            out.push(fb2_fmt::Event::VerseLine(v.clone()));
        }
        out.push(fb2_fmt::Event::EndStanza);
    }
    for ta in &poem.text_author {
        out.push(fb2_fmt::Event::TextAuthor(ta.clone()));
    }
    out.push(fb2_fmt::Event::EndPoem);
}

fn fb2_cite_events(cite: &fb2_fmt::Cite, out: &mut Vec<fb2_fmt::Event>) {
    use fb2_fmt::CiteContent;
    out.push(fb2_fmt::Event::StartCite {
        id: cite.id.clone(),
    });
    for content in &cite.content {
        match content {
            CiteContent::Para(il) => {
                out.push(fb2_fmt::Event::StartParagraph);
                out.push(fb2_fmt::Event::Inline(il.clone()));
                out.push(fb2_fmt::Event::EndParagraph);
            }
            CiteContent::EmptyLine => out.push(fb2_fmt::Event::EmptyLine),
            CiteContent::Poem(p) => fb2_poem_events(p, out),
            CiteContent::Table(t) => out.push(fb2_fmt::Event::Table(t.clone())),
        }
    }
    for ta in &cite.text_author {
        out.push(fb2_fmt::Event::TextAuthor(ta.clone()));
    }
    out.push(fb2_fmt::Event::EndCite);
}

fn fb2_epigraph_events(epigraph: &fb2_fmt::Epigraph, out: &mut Vec<fb2_fmt::Event>) {
    use fb2_fmt::EpigraphContent;
    out.push(fb2_fmt::Event::StartEpigraph {
        id: epigraph.id.clone(),
    });
    for content in &epigraph.content {
        match content {
            EpigraphContent::Para(il) => {
                out.push(fb2_fmt::Event::StartParagraph);
                out.push(fb2_fmt::Event::Inline(il.clone()));
                out.push(fb2_fmt::Event::EndParagraph);
            }
            EpigraphContent::EmptyLine => out.push(fb2_fmt::Event::EmptyLine),
            EpigraphContent::Poem(p) => fb2_poem_events(p, out),
            EpigraphContent::Cite(c) => fb2_cite_events(c, out),
        }
    }
    for ta in &epigraph.text_author {
        out.push(fb2_fmt::Event::TextAuthor(ta.clone()));
    }
    out.push(fb2_fmt::Event::EndEpigraph);
}

/// Found via this check (not previously known): `events()`/`EventIter`
/// silently drops the `Event::Metadata` event entirely whenever the input
/// has no literal `<description>` element — `finalize_description()`
/// (`crates/formats/fb2-fmt/src/events.rs`) only fires on a `</description>`
/// close tag, so if that tag never appears, no `Metadata` event is ever
/// queued. `parse()`'s AST, by contrast, always carries a
/// `FictionBook.description` field (defaulted when absent), so the
/// projection built from the AST always includes `Metadata` while the real
/// event stream sometimes doesn't. This is not a corner case: the majority
/// of single-construct fb2 fixtures omit `<description>` for brevity, so
/// this affects most of the fixture suite, not just the `adv-*` ones.
#[test]
fn fb2_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("fb2");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/fb2 dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (fb, _diags) = fb2_fmt::parse(&input);
        let expected = fb2_ast_to_events(&fb);
        let actual: Vec<_> = fb2_fmt::events(&input).collect();
        checked += 1;
        if expected != actual && result.is_ok() {
            result = Err(format!(
                "events() diverged from the AST projection for fixture {name}:\n  ast-derived: \
                 {expected:?}\n  events():    {actual:?}"
            ));
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of fb2 fixtures, got {checked}"
    );
    assert_or_known_failure("fb2", "events", result);
}

/// `StreamingParser` buffers all fed bytes into a `Vec<u8>` and only builds
/// the pull iterator over the reassembled buffer inside `finish()` (see
/// `crates/formats/fb2-fmt/src/events.rs`'s `StreamingParser::finish`), even
/// though `events()`/`EventIter` is a genuine incremental `quick_xml` pull
/// parser underneath. So: (1) the final sequence still matches `events()`
/// under adversarial chunking (expected, since finish() re-derives it from
/// the reassembled buffer), but (2) `feed()` alone never delivers events —
/// the real, previously-undocumented defect this check surfaces.
#[test]
fn fb2_streaming_parser_matches_events_and_is_incremental() {
    let root = fixtures_root().join("fb2");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/fb2 dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let bulk: Vec<fb2_fmt::Event> = fb2_fmt::events(&input).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser = fb2_fmt::StreamingParser::new(|e: fb2_fmt::Event| streamed.push(e));
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}"
                ));
            }
        }

        if input.len() > 32 && !bulk.is_empty() {
            let mid = input.len() / 2;
            let mut delivered: Vec<fb2_fmt::Event> = Vec::new();
            let mut parser = fb2_fmt::StreamingParser::new(|e| delivered.push(e));
            parser.feed(&input[..mid]);
            if delivered.is_empty() && result.is_ok() {
                result = Err(format!(
                    "StreamingParser delivered zero events to the handler after feed() with \
                     half of fixture {name} ({mid} bytes) and before finish() — \
                     fb2_fmt::StreamingParser accumulates fed bytes into a Vec<u8> and only \
                     constructs the pull iterator over the reassembled buffer inside finish() \
                     (crates/formats/fb2-fmt/src/events.rs), so feed() never advances real \
                     incremental parser state despite events()/EventIter being genuinely \
                     incremental"
                ));
            }
        }
    }
    assert!(
        checked > 5,
        "expected to check several fb2 fixtures, got {checked}"
    );
    assert_or_known_failure("fb2", "streaming_parser", result);
}

/// The streaming `Writer` itself is a genuine incremental implementation
/// (`write_event()` writes straight to the underlying `quick_xml::Writer<W>`,
/// no buffering) — but it is fed by `events()`, which (see the
/// `fb2/events` `KnownFailure` above) never delivers a `Metadata` event for
/// input lacking a literal `<description>` element. So the streaming
/// `Writer` never calls `write_description()` for those fixtures, while the
/// AST builder path (`emit()`) unconditionally writes a `<description>`
/// element for every document (even an empty/default one) — a downstream
/// consequence of the same events() gap, not an independent Writer bug.
#[test]
fn fb2_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("fb2");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/fb2 dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (fb, _diags) = fb2_fmt::parse(&input);
        let built = fb2_fmt::emit(&fb);

        let mut w = fb2_fmt::writer::Writer::new(Vec::<u8>::new());
        for e in fb2_fmt::events(&input) {
            w.write_event(e).expect("write_event");
        }
        let streamed = w.finish().expect("Writer::finish");

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}"
            ));
        }
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of fb2 fixtures, got {checked}"
    );
    assert_or_known_failure("fb2", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// textile-fmt: events() vs parse(), real and passing. StreamingParser and
// the streaming Writer are both buffer-until-finish wrappers (see their own
// module docs in crates/formats/textile-fmt/src/batch.rs and writer.rs,
// which self-report "buffers all input" / "buffers all events,
// reconstructs the AST, then emits") rather than genuine incremental
// implementations; tracked as KnownFailure entries.
// ---------------------------------------------------------------------------

fn textile_ast_to_events(doc: &textile_fmt::TextileDoc) -> Vec<textile_fmt::TextileEvent> {
    let mut out = Vec::new();
    for b in &doc.blocks {
        textile_block_events(b, &mut out);
    }
    out
}

fn textile_block_events(b: &textile_fmt::Block, out: &mut Vec<textile_fmt::TextileEvent>) {
    use textile_fmt::Block;
    use textile_fmt::TextileEvent;
    match b {
        Block::Paragraph {
            inlines,
            align,
            attrs,
            ..
        } => {
            out.push(TextileEvent::StartParagraph {
                align: align.clone(),
                attrs: attrs.clone(),
            });
            for i in inlines {
                textile_inline_events(i, out);
            }
            out.push(TextileEvent::EndParagraph);
        }
        Block::Heading {
            level,
            inlines,
            attrs,
            ..
        } => {
            out.push(TextileEvent::StartHeading {
                level: *level,
                attrs: attrs.clone(),
            });
            for i in inlines {
                textile_inline_events(i, out);
            }
            out.push(TextileEvent::EndHeading);
        }
        Block::CodeBlock {
            content, language, ..
        } => {
            out.push(TextileEvent::CodeBlock {
                content: content.clone(),
                language: language.clone(),
            });
        }
        Block::Blockquote { blocks, attrs, .. } => {
            out.push(TextileEvent::StartBlockquote {
                attrs: attrs.clone(),
            });
            for b in blocks {
                textile_block_events(b, out);
            }
            out.push(TextileEvent::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(TextileEvent::StartList { ordered: *ordered });
            for item_blocks in items {
                out.push(TextileEvent::StartListItem);
                for b in item_blocks {
                    textile_block_events(b, out);
                }
                out.push(TextileEvent::EndListItem);
            }
            out.push(TextileEvent::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(TextileEvent::StartTable);
            for row in rows {
                out.push(TextileEvent::StartTableRow {
                    attrs: row.attrs.clone(),
                });
                for cell in &row.cells {
                    out.push(TextileEvent::StartTableCell {
                        is_header: cell.is_header,
                        align: cell.align.clone(),
                    });
                    for i in &cell.inlines {
                        textile_inline_events(i, out);
                    }
                    out.push(TextileEvent::EndTableCell);
                }
                out.push(TextileEvent::EndTableRow);
            }
            out.push(TextileEvent::EndTable);
        }
        Block::HorizontalRule { .. } => out.push(TextileEvent::HorizontalRule),
        Block::FootnoteDef { label, inlines, .. } => {
            out.push(TextileEvent::StartFootnoteDef {
                label: label.clone(),
            });
            for i in inlines {
                textile_inline_events(i, out);
            }
            out.push(TextileEvent::EndFootnoteDef);
        }
        Block::DefinitionList { items, .. } => {
            out.push(TextileEvent::StartDefinitionList);
            for (term, def) in items {
                out.push(TextileEvent::StartDefinitionTerm);
                for i in term {
                    textile_inline_events(i, out);
                }
                out.push(TextileEvent::EndDefinitionTerm);
                out.push(TextileEvent::StartDefinitionDesc);
                for i in def {
                    textile_inline_events(i, out);
                }
                out.push(TextileEvent::EndDefinitionDesc);
            }
            out.push(TextileEvent::EndDefinitionList);
        }
        Block::Raw { content, .. } => out.push(TextileEvent::RawBlock {
            content: content.clone(),
        }),
    }
}

fn textile_inline_events(i: &textile_fmt::Inline, out: &mut Vec<textile_fmt::TextileEvent>) {
    use textile_fmt::Inline;
    use textile_fmt::TextileEvent;
    match i {
        Inline::Text(s, _) => out.push(TextileEvent::Text(s.clone())),
        Inline::Bold(c, _) => {
            out.push(TextileEvent::StartBold);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndBold);
        }
        Inline::Italic(c, _) => {
            out.push(TextileEvent::StartItalic);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndItalic);
        }
        Inline::Underline(c, _) => {
            out.push(TextileEvent::StartUnderline);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndUnderline);
        }
        Inline::Strikethrough(c, _) => {
            out.push(TextileEvent::StartStrikethrough);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndStrikethrough);
        }
        Inline::Code(s, _) => out.push(TextileEvent::InlineCode(s.clone())),
        Inline::Link {
            url,
            title,
            children,
            ..
        } => {
            out.push(TextileEvent::StartLink {
                url: url.clone(),
                title: title.clone(),
            });
            for x in children {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndLink);
        }
        Inline::Image { url, alt, .. } => out.push(TextileEvent::InlineImage {
            url: url.clone(),
            alt: alt.clone(),
        }),
        Inline::Superscript(c, _) => {
            out.push(TextileEvent::StartSuperscript);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndSuperscript);
        }
        Inline::Subscript(c, _) => {
            out.push(TextileEvent::StartSubscript);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndSubscript);
        }
        Inline::FootnoteRef { label, .. } => out.push(TextileEvent::FootnoteRef {
            label: label.clone(),
        }),
        Inline::LineBreak(_) => out.push(TextileEvent::LineBreak),
        Inline::Raw(s, _) => out.push(TextileEvent::RawInline { content: s.clone() }),
        Inline::Citation(c, _) => {
            out.push(TextileEvent::StartCitation);
            for x in c {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndCitation);
        }
        Inline::GenericSpan {
            attrs, children, ..
        } => {
            out.push(TextileEvent::StartGenericSpan {
                attrs: attrs.clone(),
            });
            for x in children {
                textile_inline_events(x, out);
            }
            out.push(TextileEvent::EndGenericSpan);
        }
        Inline::Acronym { text, title, .. } => out.push(TextileEvent::Acronym {
            text: text.clone(),
            title: title.clone(),
        }),
    }
}

#[test]
fn textile_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("textile");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/textile dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = textile_fmt::parse::parse(&input);
        let expected = textile_ast_to_events(&doc);
        let actual: Vec<_> = textile_fmt::events(&input).collect();
        assert_eq!(
            expected,
            actual,
            "events() diverged from the AST projection for fixture {}",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of textile fixtures, got {checked}"
    );
}

/// `StreamingParser` buffers all fed bytes and only parses + delivers events
/// inside `finish()` (see `crates/formats/textile-fmt/src/batch.rs`'s own
/// module doc). Checks (1) equivalence with `events()` under adversarial
/// chunking (expected to hold) and (2) incremental delivery (feed() alone,
/// before finish(), should deliver some events for large-enough input) —
/// (2) fails, the real defect this check surfaces.
#[test]
fn textile_streaming_parser_matches_events_and_is_incremental() {
    let root = fixtures_root().join("textile");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/textile dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let Ok(input_str) = std::str::from_utf8(&input) else {
            continue;
        };
        let bulk: Vec<textile_fmt::TextileEvent> = textile_fmt::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                textile_fmt::batch::StreamingParser::new(|e: textile_fmt::TextileEvent| {
                    streamed.push(e)
                });
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}"
                ));
            }
        }

        if input.len() > 32 && !bulk.is_empty() {
            let mid = input.len() / 2;
            let mut delivered: Vec<textile_fmt::TextileEvent> = Vec::new();
            let mut parser = textile_fmt::batch::StreamingParser::new(|e| delivered.push(e));
            parser.feed(&input[..mid]);
            if delivered.is_empty() && result.is_ok() {
                result = Err(format!(
                    "StreamingParser delivered zero events to the handler after feed() with \
                     half of fixture {name} ({mid} bytes) and before finish() — \
                     textile_fmt::batch::StreamingParser buffers all input into a Vec<u8> (see \
                     crates/formats/textile-fmt/src/batch.rs) and only parses and delivers \
                     events inside finish()"
                ));
            }
        }
    }
    assert!(
        checked > 5,
        "expected to check several textile fixtures, got {checked}"
    );
    assert_or_known_failure("textile", "streaming_parser", result);
}

/// `Writer` buffers all fed events into a `Vec<TextileEvent>` and only
/// reconstructs the AST + calls `emit()` inside `finish()` (see
/// `crates/formats/textile-fmt/src/writer.rs`'s own module doc). Checked via
/// byte-identical comparison against the builder path across all fixtures.
#[test]
fn textile_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("textile");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/textile dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = textile_fmt::parse::parse(&input);
        let built = textile_fmt::emit::emit(&doc);

        let mut w = textile_fmt::writer::Writer::new(Vec::<u8>::new());
        for e in textile_fmt::events(&input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}:\n  build():  \
                 {built:?}\n  streamed: {streamed:?}"
            ));
        }
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of textile fixtures, got {checked}"
    );

    // Incrementality probe: byte-identical final content (checked above)
    // only proves correctness, not genuine streaming. Feed a full paragraph
    // (well short of finish()) and check whether any bytes already reached
    // the sink.
    if result.is_ok() {
        use textile_fmt::TextileEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = textile_fmt::writer::Writer::new(ObservableSink(observed.clone()));
        w.write_event(TextileEvent::StartParagraph {
            align: None,
            attrs: Default::default(),
        });
        w.write_event(TextileEvent::Text("Hello world".to_string()));
        w.write_event(TextileEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after a full StartParagraph/Text/\
                 EndParagraph sequence and before finish() — textile_fmt::writer::Writer \
                 buffers all events into a Vec<TextileEvent> and only reconstructs the AST + \
                 calls emit() inside finish() (crates/formats/textile-fmt/src/writer.rs), so it \
                 is not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("textile", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// commonmark-fmt: shared by the "commonmark", "gfm", and "markdown" fixture
// formats (all three are wrappers over the same commonmark-fmt crate). This
// section wires each API exactly once against the "commonmark" fixture set
// (the superset — gfm/markdown fixtures exercise the same crate code paths)
// and the CAPABILITIES/KNOWN_FAILURES entries for gfm/markdown just cite
// these findings, per the task's "shares one crate audit" instruction.
//
// events() wraps pulldown-cmark's OffsetIter and is genuinely lazy/pull-based
// (real, passing). StreamingParser buffering all input before parsing is the
// sole CLAUDE.md-sanctioned pulldown-cmark exemption (documented in
// crates/formats/commonmark-fmt/src/lib.rs and src/batch.rs) — NotApplicable,
// no check needed. The streaming Writer, by contrast, self-admits in its own
// module doc that it "buffers all events, reconstructs the AST, then emits"
// — unrelated to the sanctioned reader exemption (the writer never touches
// pulldown-cmark) and a fake streaming writer per CLAUDE.md; tracked as a
// KnownFailure via the same incrementality-probe pattern used for
// texinfo/textile above.
// ---------------------------------------------------------------------------

fn commonmark_ast_to_events(
    doc: &commonmark_fmt::CmDoc,
) -> Vec<commonmark_fmt::events::Event<'static>> {
    use commonmark_fmt::events::Event;
    let mut out = vec![Event::StartDocument];
    if let Some(fm) = &doc.frontmatter {
        out.push(Event::FrontMatter {
            kind: fm.kind,
            content: std::borrow::Cow::Owned(fm.content.clone()),
        });
    }
    for b in &doc.blocks {
        commonmark_block_events(b, &mut out);
    }
    out.push(Event::EndDocument);
    out
}

fn commonmark_block_events(
    b: &commonmark_fmt::ast::Block,
    out: &mut Vec<commonmark_fmt::events::Event<'static>>,
) {
    use commonmark_fmt::ast::Block;
    use commonmark_fmt::events::Event;
    use std::borrow::Cow;
    match b {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for i in inlines {
                commonmark_inline_events(i, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            for i in inlines {
                commonmark_inline_events(i, out);
            }
            out.push(Event::EndHeading { level: *level });
        }
        Block::CodeBlock {
            language, content, ..
        } => {
            out.push(Event::CodeBlock {
                language: language.clone().map(Cow::Owned),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::HtmlBlock { content, .. } => {
            out.push(Event::HtmlBlock(Cow::Owned(content.clone())));
        }
        Block::Blockquote { blocks, .. } => {
            out.push(Event::StartBlockquote);
            for c in blocks {
                commonmark_block_events(c, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List {
            kind, items, tight, ..
        } => {
            use commonmark_fmt::ast::ListKind;
            let (ordered, start) = match kind {
                ListKind::Unordered { .. } => (false, 1),
                ListKind::Ordered { start, .. } => (true, *start),
            };
            out.push(Event::StartList {
                ordered,
                start,
                tight: *tight,
            });
            for item in items {
                out.push(Event::StartItem {
                    checked: item.checked,
                });
                for c in &item.blocks {
                    commonmark_block_events(c, out);
                }
                out.push(Event::EndItem);
            }
            out.push(Event::EndList);
        }
        Block::ThematicBreak { .. } => out.push(Event::ThematicBreak),
        Block::Table {
            alignments,
            head,
            rows,
            ..
        } => {
            out.push(Event::StartTable {
                alignments: alignments.clone(),
            });
            out.push(Event::StartTableHead);
            out.push(Event::StartTableRow);
            for cell in &head.cells {
                out.push(Event::StartTableCell);
                for i in &cell.inlines {
                    commonmark_inline_events(i, out);
                }
                out.push(Event::EndTableCell);
            }
            out.push(Event::EndTableRow);
            out.push(Event::EndTableHead);
            for row in rows {
                out.push(Event::StartTableRow);
                for cell in &row.cells {
                    out.push(Event::StartTableCell);
                    for i in &cell.inlines {
                        commonmark_inline_events(i, out);
                    }
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
    }
}

fn commonmark_inline_events(
    i: &commonmark_fmt::ast::Inline,
    out: &mut Vec<commonmark_fmt::events::Event<'static>>,
) {
    use commonmark_fmt::ast::Inline;
    use commonmark_fmt::events::Event;
    use std::borrow::Cow;
    match i {
        Inline::Text { content, .. } => out.push(Event::Text(Cow::Owned(content.clone()))),
        Inline::SoftBreak { .. } => out.push(Event::SoftBreak),
        Inline::HardBreak { .. } => out.push(Event::HardBreak),
        Inline::Emphasis { inlines, .. } => {
            out.push(Event::StartEmphasis);
            for c in inlines {
                commonmark_inline_events(c, out);
            }
            out.push(Event::EndEmphasis);
        }
        Inline::Strong { inlines, .. } => {
            out.push(Event::StartStrong);
            for c in inlines {
                commonmark_inline_events(c, out);
            }
            out.push(Event::EndStrong);
        }
        Inline::Strikethrough { inlines, .. } => {
            out.push(Event::StartStrikethrough);
            for c in inlines {
                commonmark_inline_events(c, out);
            }
            out.push(Event::EndStrikethrough);
        }
        Inline::Code { content, .. } => out.push(Event::Code(Cow::Owned(content.clone()))),
        Inline::HtmlInline { content, .. } => {
            out.push(Event::HtmlInline(Cow::Owned(content.clone())));
        }
        Inline::Link {
            inlines,
            url,
            title,
            ..
        } => {
            out.push(Event::StartLink {
                url: Cow::Owned(url.clone()),
                title: title.clone().map(Cow::Owned),
            });
            for c in inlines {
                commonmark_inline_events(c, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image {
            alt, url, title, ..
        } => {
            out.push(Event::StartImage {
                url: Cow::Owned(url.clone()),
                title: title.clone().map(Cow::Owned),
                alt: Cow::Owned(alt.clone()),
            });
            if !alt.is_empty() {
                out.push(Event::Text(Cow::Owned(alt.clone())));
            }
            out.push(Event::EndImage);
        }
    }
}

#[test]
fn commonmark_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("commonmark");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/commonmark dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = commonmark_fmt::parse::parse(&input);
        let expected = commonmark_ast_to_events(&doc);
        let Some(actual_iter) = commonmark_fmt::events::events(&input) else {
            continue; // non-UTF8 input: events() returns None, not this check's concern
        };
        let actual: Vec<_> = actual_iter.map(|e| e.into_owned()).collect();
        checked += 1;
        if expected != actual && result.is_ok() {
            result = Err(format!(
                "events() diverged from the AST projection for fixture {name}:\n  ast-derived: \
                 {expected:?}\n  events():    {actual:?}"
            ));
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of commonmark fixtures, got {checked}"
    );
    assert_or_known_failure("commonmark", "events", result);
}

/// `commonmark_fmt::writer::Writer` self-admits (see its own module doc) that
/// it buffers all fed events into a `Vec<OwnedEvent>` and only reconstructs
/// the AST + calls `emit()` inside `finish()`. Checked the same way as
/// texinfo/textile above: byte-identical-to-builder content correctness,
/// plus an incrementality probe (write a full paragraph, check whether any
/// bytes reached the sink before `finish()`).
#[test]
fn commonmark_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("commonmark");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/commonmark dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = commonmark_fmt::parse::parse(&input);
        let built = commonmark_fmt::emit::emit(&doc);

        let Some(events_iter) = commonmark_fmt::events::events(&input) else {
            continue;
        };
        let mut w = commonmark_fmt::Writer::new(Vec::<u8>::new());
        for e in events_iter {
            w.write_event(e);
        }
        let streamed = w.finish().expect("Writer::finish");

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}"
            ));
        }
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of commonmark fixtures, got {checked}"
    );

    if result.is_ok() {
        use commonmark_fmt::events::Event;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = commonmark_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(Event::StartDocument);
        w.write_event(Event::StartParagraph);
        w.write_event(Event::Text("Hello world".into()));
        w.write_event(Event::EndParagraph);
        w.write_event(Event::EndDocument);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartDocument/StartParagraph/Text/EndParagraph/EndDocument sequence and before \
                 finish() — commonmark_fmt::writer::Writer buffers all events into a \
                 Vec<OwnedEvent> and only reconstructs the AST + calls emit() inside finish() \
                 (crates/formats/commonmark-fmt/src/writer.rs, self-admitted in its own module \
                 doc), so it is not a genuine incremental streaming writer despite content \
                 round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("commonmark", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// t2t: events() is `EventIter::new(parse(input).0)` (src/events.rs's public
// `events()` fn) — a lazy frame-stack walk of the AST `parse()` already
// built, not an independently-implemented reader. Per the asciidoc precedent
// above, the ast_to_events-vs-events() check below is real and passes, but
// it validates the AST->event expansion layer (push_block/push_inline's
// frame-stack walk), not two independent parsers — the check would pass by
// construction even if `events()`'s only bug were "doesn't match its own
// AST", since both sides start from the same `T2tDoc`.
//
// StreamingParser is a genuine per-block incremental parser (batch.rs's
// feed_line/BlockState machine flushes each accumulated block via
// emit_block() as soon as a blank line or fence boundary is seen, not only
// at finish()) but emit_block() re-parses each block's text in isolation via
// crate::events::events(&text) — the same "re-parse each block alone, lose
// cross-block context" root cause already documented for org-fmt/asciidoc's
// StreamingParsers. Two distinct fixtures expose it here: the definition-list
// fixture, where the blank line between items ends the accumulated block
// (see feed_line's blank-line branch, batch.rs:143-150), splitting one
// multi-item DefinitionList into two DefinitionList event pairs (the same bug
// class already tracked for rst/org); and document-header, where the
// isolated re-parse of the header's own three lines re-triggers
// `try_parse_header()` (parse.rs:70), which requires >=3 lines and a
// non-heading/list/table/comment first line — a condition any 3+ line
// document-header-shaped block satisfies purely by looking like one out of
// context, producing a spurious extra StartDocument/EndDocument pair (since
// Event has no metadata variant to carry the consumed header text) that
// events() over the whole document never produces.
//
// Writer buffers all events into a Vec<OwnedEvent> and only reconstructs the
// AST + calls emit() inside finish() (writer.rs's own module doc: "This
// implementation buffers all events, reconstructs the AST, then emits") —
// the same fake-streaming-writer pattern as textile/commonmark/org/texinfo.
// It also drops doc.title/author/date on every fixture with a document
// header: emit::emit() always writes the 3-line header verbatim from
// T2tDoc.title/author/date (emit.rs:9-16), but t2t::Event has no variant
// carrying those fields, so writer.rs's DocBuilder::finish (T2tDoc {
// blocks, ..Default::default() }, writer.rs:400-404) always reconstructs
// title: None/author: None/date: None — an Event-enum expressiveness gap,
// not a one-line logic bug, exposed by the document-header fixture.
// ---------------------------------------------------------------------------

fn t2t_ast_to_events(doc: &t2t::T2tDoc) -> Vec<t2t::Event<'static>> {
    let mut out = vec![t2t::Event::StartDocument];
    for b in &doc.blocks {
        t2t_block_events(b, &mut out);
    }
    out.push(t2t::Event::EndDocument);
    out
}

fn t2t_block_events(b: &t2t::Block, out: &mut Vec<t2t::Event<'static>>) {
    use std::borrow::Cow;
    use t2t::{Block, Event};
    match b {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for i in inlines {
                t2t_inline_events(i, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::Heading {
            level,
            numbered,
            inlines,
            ..
        } => {
            out.push(Event::StartHeading {
                level: *level,
                numbered: *numbered,
            });
            for i in inlines {
                t2t_inline_events(i, out);
            }
            out.push(Event::EndHeading);
        }
        Block::CodeBlock { content, .. } => out.push(Event::CodeBlock {
            content: Cow::Owned(content.clone()),
        }),
        Block::RawBlock { content, .. } => out.push(Event::RawBlock {
            content: Cow::Owned(content.clone()),
        }),
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for c in children {
                t2t_block_events(c, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item_blocks in items {
                out.push(Event::StartListItem);
                for b in item_blocks {
                    t2t_block_events(b, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow {
                    header: row.is_header,
                });
                for cell in &row.cells {
                    out.push(Event::StartTableCell);
                    for i in cell {
                        t2t_inline_events(i, out);
                    }
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for (term, desc) in items {
                out.push(Event::StartDefinitionTerm);
                for i in term {
                    t2t_inline_events(i, out);
                }
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                for b in desc {
                    t2t_block_events(b, out);
                }
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
    }
}

fn t2t_inline_events(i: &t2t::Inline, out: &mut Vec<t2t::Event<'static>>) {
    use std::borrow::Cow;
    use t2t::{Event, Inline};
    match i {
        Inline::Text(s, _) => out.push(Event::Text(Cow::Owned(s.clone()))),
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            for c in children {
                t2t_inline_events(c, out);
            }
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            for c in children {
                t2t_inline_events(c, out);
            }
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            for c in children {
                t2t_inline_events(c, out);
            }
            out.push(Event::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            out.push(Event::StartStrikethrough);
            for c in children {
                t2t_inline_events(c, out);
            }
            out.push(Event::EndStrikethrough);
        }
        Inline::Code(s, _) => out.push(Event::Code(Cow::Owned(s.clone()))),
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink {
                url: Cow::Owned(url.clone()),
            });
            for c in children {
                t2t_inline_events(c, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image { url, .. } => out.push(Event::Image {
            src: Cow::Owned(url.clone()),
        }),
        Inline::LineBreak(_) => out.push(Event::LineBreak),
        Inline::SoftBreak(_) => out.push(Event::SoftBreak),
        Inline::Verbatim(s, _) => out.push(Event::Verbatim(Cow::Owned(s.clone()))),
        Inline::Tagged(s, _) => out.push(Event::Tagged(Cow::Owned(s.clone()))),
    }
}

#[test]
fn t2t_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("t2t");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/t2t dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = t2t::parse::parse(&input);
        let expected = t2t_ast_to_events(&doc);
        let actual: Vec<_> = t2t::events(&input).collect();
        assert_eq!(
            expected,
            actual,
            "events() diverged from the AST projection for fixture {}",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of t2t fixtures, got {checked}"
    );
}

/// `StreamingParser` genuinely flushes events per accumulated block as it's
/// fed (not only at `finish()`) but re-parses each block's text in isolation
/// via `crate::events::events()`, losing cross-block context. Checked via
/// adversarial-chunking equivalence against `events()` over the whole input.
#[test]
fn t2t_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("t2t");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/t2t dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let Ok(input_str) = std::str::from_utf8(&input) else {
            continue;
        };
        let bulk: Vec<t2t::OwnedEvent> = t2t::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser = t2t::batch::StreamingParser::new(|e: t2t::OwnedEvent| {
                streamed.push(e);
            });
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}"
                ));
            }
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of t2t fixtures, got {checked}"
    );
    assert_or_known_failure("t2t", "streaming_parser", result);
}

/// `Writer` buffers all fed events into a `Vec<OwnedEvent>` and only
/// reconstructs the AST + calls `emit()` inside `finish()` (see
/// `crates/formats/t2t/src/writer.rs`'s own module doc). Checked via
/// byte-identical comparison against the builder path, plus an
/// incrementality probe.
#[test]
fn t2t_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("t2t");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/t2t dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = t2t::parse::parse(&input);
        let built = t2t::emit::emit(&doc);

        let mut w = t2t::writer::Writer::new(Vec::<u8>::new());
        for e in t2t::events(&input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}:\n  build():  \
                 {built:?}\n  streamed: {streamed:?}"
            ));
        }
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of t2t fixtures, got {checked}"
    );

    // Incrementality probe: byte-identical final content (checked above)
    // only proves correctness, not genuine streaming.
    if result.is_ok() {
        use t2t::Event;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = t2t::writer::Writer::new(ObservableSink(observed.clone()));
        w.write_event(Event::StartParagraph);
        w.write_event(Event::Text("Hello world".to_string().into()));
        w.write_event(Event::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after a full StartParagraph/Text/\
                 EndParagraph sequence and before finish() — t2t::writer::Writer buffers all \
                 events into a Vec<OwnedEvent> and only reconstructs the AST + calls emit() \
                 inside finish() (crates/formats/t2t/src/writer.rs), so it is not a genuine \
                 incremental streaming writer despite content round-tripping correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("t2t", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// pod-fmt: top-level `pod_fmt::events()` (src/lib.rs) is `parse(input)` then
// an eager `.collect()` of `events::EventIter::new(&doc)` — a lazy
// frame-stack walk of the AST parse() already built (the same
// events()-is-parse()+AST-walk pattern already documented for t2t/asciidoc
// above), not an independently-implemented reader. The ast_to_events-vs-
// events() check below is real and passes, but validates the AST->event
// expansion layer, not two independent parsers.
//
// StreamingParser is explicitly self-documented buffer-then-finish: its own
// doc comment reads "POD documents are always small enough to buffer fully,
// so this implementation accumulates all input and parses on finish(). The
// chunk API is provided for interface consistency with other format
// crates." feed() only does `self.buf.extend_from_slice(chunk)`; all
// parsing and event delivery happens in finish() via
// `crate::events::EventIter::new(&doc)` over the *whole* buffered input (not
// per-isolated-block), so — unlike t2t/org/asciidoc — there is no
// re-parse-in-isolation divergence from events(): the adversarial-chunking
// equivalence check is expected to (and does) pass. The real defect is
// purely architectural non-incrementality, per CLAUDE.md's "the 'buffer all
// input until finish()' stub is explicitly rejected for hand-rolled
// parsers" — pod-fmt's own docstring rationale does not make this a
// sanctioned exemption (only commonmark-fmt's pulldown-cmark wrapping is);
// pinned via the incrementality probe.
//
// Writer buffers all fed events into a Vec<OwnedEvent> and only reconstructs
// the AST + calls emit::build() inside finish() (writer.rs's `finish()`:
// `events_to_doc(...)` then `crate::emit::build(&doc)`) — the same
// fake-streaming-writer pattern as t2t/textile/commonmark/org/texinfo. Since
// PodDoc has no document-level metadata field pod::Event could plausibly be
// missing (unlike t2t's title/author/date), the byte-identical-to-builder
// check is expected to (and does) pass; only the incrementality probe fails.
// ---------------------------------------------------------------------------

fn pod_ast_to_events(doc: &pod_fmt::PodDoc) -> Vec<pod_fmt::OwnedEvent> {
    let mut out = Vec::new();
    for b in &doc.blocks {
        pod_block_events(b, &mut out);
    }
    out
}

fn pod_block_events(b: &pod_fmt::Block, out: &mut Vec<pod_fmt::OwnedEvent>) {
    use pod_fmt::{Block, Event};
    match b {
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            for i in inlines {
                pod_inline_events(i, out);
            }
            out.push(Event::EndHeading);
        }
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for i in inlines {
                pod_inline_events(i, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::CodeBlock { content, .. } => out.push(Event::CodeBlock {
            content: content.clone().into(),
        }),
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item_blocks in items {
                out.push(Event::StartListItem);
                for b in item_blocks {
                    pod_block_events(b, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for item in items {
                out.push(Event::StartDefinitionTerm);
                for i in &item.term {
                    pod_inline_events(i, out);
                }
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                for b in &item.desc {
                    pod_block_events(b, out);
                }
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::RawBlock {
            format, content, ..
        } => out.push(Event::RawBlock {
            format: format.clone(),
            content: content.clone(),
        }),
        Block::ForBlock {
            format, content, ..
        } => out.push(Event::ForBlock {
            format: format.clone(),
            content: content.clone(),
        }),
        Block::Encoding { encoding, .. } => out.push(Event::Encoding {
            encoding: encoding.clone(),
        }),
    }
}

fn pod_inline_events(i: &pod_fmt::Inline, out: &mut Vec<pod_fmt::OwnedEvent>) {
    use pod_fmt::{Event, Inline};
    match i {
        Inline::Text(s, _) => out.push(Event::Text(s.clone().into())),
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            for c in children {
                pod_inline_events(c, out);
            }
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            for c in children {
                pod_inline_events(c, out);
            }
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            for c in children {
                pod_inline_events(c, out);
            }
            out.push(Event::EndUnderline);
        }
        Inline::Code(s, _) => out.push(Event::InlineCode(s.clone().into())),
        Inline::Link { url, label, .. } => {
            out.push(Event::StartLink {
                url: url.clone(),
                label: label.clone(),
            });
            out.push(Event::EndLink);
        }
        Inline::Filename(children, _) => {
            out.push(Event::StartFilename);
            for c in children {
                pod_inline_events(c, out);
            }
            out.push(Event::EndFilename);
        }
        Inline::NonBreaking(children, _) => {
            out.push(Event::StartNonBreaking);
            for c in children {
                pod_inline_events(c, out);
            }
            out.push(Event::EndNonBreaking);
        }
        Inline::IndexEntry(s, _) => out.push(Event::IndexEntry(s.clone())),
        Inline::Null(_) => out.push(Event::Null),
        Inline::Entity(s, _) => out.push(Event::Entity(s.clone())),
    }
}

#[test]
fn pod_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("pod");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/pod dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = pod_fmt::parse(&input);
        let expected = pod_ast_to_events(&doc);
        let actual: Vec<_> = pod_fmt::events(&input).collect();
        assert_eq!(
            expected,
            actual,
            "events() diverged from the AST projection for fixture {}",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of pod fixtures, got {checked}"
    );
}

/// `StreamingParser` is explicitly self-documented buffer-then-finish (see
/// `crates/formats/pod-fmt/src/batch.rs`'s own module doc). Checks (1)
/// equivalence with `events()` under adversarial chunking (expected to hold,
/// since finish() parses the whole buffered input the same way bulk
/// `events()` does — no per-block re-parse to diverge) and (2) incremental
/// delivery (feed() alone, before finish(), should deliver some events for
/// large-enough input) — (2) fails, the real defect this check surfaces.
#[test]
fn pod_streaming_parser_matches_events_and_is_incremental() {
    let root = fixtures_root().join("pod");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/pod dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let Ok(input_str) = std::str::from_utf8(&input) else {
            continue;
        };
        let bulk: Vec<pod_fmt::OwnedEvent> = pod_fmt::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser = pod_fmt::batch::StreamingParser::new(|e: pod_fmt::OwnedEvent| {
                streamed.push(e);
            });
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}"
                ));
            }
        }

        if input.len() > 32 && !bulk.is_empty() {
            let mid = input.len() / 2;
            let mut delivered: Vec<pod_fmt::OwnedEvent> = Vec::new();
            let mut parser = pod_fmt::batch::StreamingParser::new(|e| delivered.push(e));
            parser.feed(&input[..mid]);
            if delivered.is_empty() && result.is_ok() {
                result = Err(format!(
                    "StreamingParser delivered zero events to the handler after feed() with \
                     half of fixture {name} ({mid} bytes) and before finish() — \
                     pod_fmt::batch::StreamingParser buffers all input into a Vec<u8> (self- \
                     documented in crates/formats/pod-fmt/src/batch.rs's module doc) and only \
                     parses and delivers events inside finish()"
                ));
            }
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of pod fixtures, got {checked}"
    );
    assert_or_known_failure("pod", "streaming_parser", result);
}

/// `Writer` buffers all fed events into a `Vec<OwnedEvent>` and only
/// reconstructs the AST + calls `emit::build()` inside `finish()` (see
/// `crates/formats/pod-fmt/src/writer.rs`). Checked via byte-identical
/// comparison against the builder path, plus an incrementality probe.
#[test]
fn pod_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("pod");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/pod dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = pod_fmt::parse(&input);
        let built = pod_fmt::build(&doc);

        let mut w = pod_fmt::Writer::new(Vec::<u8>::new());
        for e in pod_fmt::events(&input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}:\n  build():  \
                 {built:?}\n  streamed: {streamed:?}"
            ));
        }
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of pod fixtures, got {checked}"
    );

    // Incrementality probe: byte-identical final content (checked above)
    // only proves correctness, not genuine streaming.
    if result.is_ok() {
        use pod_fmt::Event;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = pod_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(Event::StartParagraph);
        w.write_event(Event::Text("Hello world".to_string().into()));
        w.write_event(Event::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after a full StartParagraph/Text/\
                 EndParagraph sequence and before finish() — pod_fmt::writer::Writer buffers all \
                 events into a Vec<OwnedEvent> and only reconstructs the AST + calls \
                 emit::build() inside finish() (crates/formats/pod-fmt/src/writer.rs), so it is \
                 not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("pod", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// haddock-fmt: `haddock_fmt::events()` (src/lib.rs) is `events::events(input)`
// which is `parse(input)` then a lazy frame-stack `EventIter::expand_block`
// walk of the AST parse() already built — the same events()-is-parse()+
// AST-walk pattern already documented for t2t/pod/asciidoc above, not an
// independently-implemented reader. The ast_to_events-vs-events() check
// below is real and passes, but validates the AST->event expansion layer.
//
// StreamingParser (batch.rs) genuinely flushes events per accumulated block
// as fed (blank line or EOF triggers emit_block(), which re-parses just
// that block's text via crate::events::events()) — architecturally the same
// "re-parse each block alone" shape as t2t/org/asciidoc's StreamingParsers.
// Unlike those, every haddock block-termination rule in parse.rs (heading,
// paragraph, code block, @-code block, doctest, lists, definition list,
// property) depends only on the content of lines within the block being
// scanned — never on cross-block state or document position (no
// document-start-only special case the way t2t's 3-line header lookahead
// is) — so re-parsing an isolated block's text from scratch recovers
// exactly the same block boundaries parse() would find inline. This harness
// found no fixture where StreamingParser disagrees with events() under
// adversarial chunking, so streaming_parser is Wired, not KnownFailure.
//
// Writer buffers all fed events into a Vec<OwnedEvent> and only
// reconstructs the AST + calls emit::build() inside finish() (writer.rs's
// own module doc: "This implementation buffers all events, reconstructs the
// AST, then emits") — the same fake-streaming-writer pattern as
// t2t/pod/textile/commonmark/org/texinfo.
// ---------------------------------------------------------------------------

fn haddock_ast_to_events(doc: &haddock_fmt::HaddockDoc) -> Vec<haddock_fmt::OwnedEvent> {
    let mut out = Vec::new();
    for b in &doc.blocks {
        haddock_block_events(b, &mut out);
    }
    out
}

fn haddock_block_events(b: &haddock_fmt::Block, out: &mut Vec<haddock_fmt::OwnedEvent>) {
    use haddock_fmt::{Block, Event};
    match b {
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            for i in inlines {
                haddock_inline_events(i, out);
            }
            out.push(Event::EndHeading);
        }
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for i in inlines {
                haddock_inline_events(i, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::CodeBlock { content, .. } => out.push(Event::CodeBlock {
            content: content.clone().into(),
        }),
        Block::AtCodeBlock { content, .. } => out.push(Event::AtCodeBlock {
            content: content.clone().into(),
        }),
        Block::UnorderedList { items, .. } => {
            out.push(Event::StartUnorderedList);
            for item in items {
                out.push(Event::StartListItem);
                for i in item {
                    haddock_inline_events(i, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndUnorderedList);
        }
        Block::OrderedList { items, .. } => {
            out.push(Event::StartOrderedList);
            for item in items {
                out.push(Event::StartListItem);
                for i in item {
                    haddock_inline_events(i, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndOrderedList);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for (term, desc) in items {
                out.push(Event::StartDefinitionTerm);
                for i in term {
                    haddock_inline_events(i, out);
                }
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                for i in desc {
                    haddock_inline_events(i, out);
                }
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::DocTest {
            expression, result, ..
        } => out.push(Event::DocTest {
            expression: expression.clone().into(),
            result: result.clone().map(Into::into),
        }),
        Block::Blockquote { inlines, .. } => {
            out.push(Event::StartBlockquote);
            for i in inlines {
                haddock_inline_events(i, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::Property {
            key,
            name,
            description,
            ..
        } => {
            out.push(Event::Property {
                key: key.clone().into(),
                name: name.clone().map(Into::into),
            });
            for i in description {
                haddock_inline_events(i, out);
            }
            out.push(Event::EndProperty);
        }
    }
}

fn haddock_inline_events(i: &haddock_fmt::Inline, out: &mut Vec<haddock_fmt::OwnedEvent>) {
    use haddock_fmt::{Event, Inline};
    match i {
        Inline::Text(s, _) => out.push(Event::Text(s.clone().into())),
        Inline::Code(s, _) => out.push(Event::InlineCode(s.clone().into())),
        Inline::Strong(children, _) => {
            out.push(Event::StartStrong);
            for c in children {
                haddock_inline_events(c, out);
            }
            out.push(Event::EndStrong);
        }
        Inline::Emphasis(children, _) => {
            out.push(Event::StartEmphasis);
            for c in children {
                haddock_inline_events(c, out);
            }
            out.push(Event::EndEmphasis);
        }
        Inline::Link { url, text, .. } => {
            out.push(Event::StartLink {
                url: url.clone(),
                text: text.clone(),
            });
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndLink);
        }
        Inline::ModuleLink { module, .. } => out.push(Event::ModuleLink {
            module: module.clone(),
        }),
    }
}

#[test]
fn haddock_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("haddock");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/haddock dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = haddock_fmt::parse(&input);
        let expected = haddock_ast_to_events(&doc);
        let actual: Vec<_> = haddock_fmt::events(&input)
            .map(|e| e.into_owned())
            .collect();
        assert_eq!(
            expected,
            actual,
            "events() diverged from the AST projection for fixture {}",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of haddock fixtures, got {checked}"
    );
}

/// `StreamingParser` flushes events per accumulated block as fed, re-parsing
/// each block's text in isolation via `crate::events::events()`. Checked via
/// adversarial-chunking equivalence against `events()` over the whole input.
#[test]
fn haddock_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("haddock");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/haddock dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let Ok(input_str) = std::str::from_utf8(&input) else {
            continue;
        };
        let bulk: Vec<haddock_fmt::OwnedEvent> = haddock_fmt::events(input_str)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                haddock_fmt::batch::StreamingParser::new(|e: haddock_fmt::OwnedEvent| {
                    streamed.push(e);
                });
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}"
                ));
            }
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of haddock fixtures, got {checked}"
    );
    assert_or_known_failure("haddock", "streaming_parser", result);
}

/// `Writer` buffers all fed events into a `Vec<OwnedEvent>` and only
/// reconstructs the AST + calls `emit::build()` inside `finish()` (see
/// `crates/formats/haddock-fmt/src/writer.rs`'s own module doc). Checked via
/// byte-identical comparison against the builder path, plus an
/// incrementality probe.
#[test]
fn haddock_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("haddock");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/haddock dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = haddock_fmt::parse(&input);
        let built = haddock_fmt::build(&doc);

        let mut w = haddock_fmt::Writer::new(Vec::<u8>::new());
        for e in haddock_fmt::events(&input) {
            w.write_event(e.into_owned());
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}:\n  build():  \
                 {built:?}\n  streamed: {streamed:?}"
            ));
        }
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of haddock fixtures, got {checked}"
    );

    // Incrementality probe: byte-identical final content (checked above)
    // only proves correctness, not genuine streaming.
    if result.is_ok() {
        use haddock_fmt::Event;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = haddock_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(Event::StartParagraph);
        w.write_event(Event::Text("Hello world".to_string().into()));
        w.write_event(Event::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after a full StartParagraph/Text/\
                 EndParagraph sequence and before finish() — haddock_fmt::writer::Writer \
                 buffers all events into a Vec<OwnedEvent> and only reconstructs the AST + \
                 calls emit::build() inside finish() (crates/formats/haddock-fmt/src/writer.rs, \
                 self-admitted in its own module doc), so it is not a genuine incremental \
                 streaming writer despite content round-tripping correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("haddock", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// fountain-fmt: `fountain_fmt::events()` (src/lib.rs) returns
// `events::OwnedEventIter`, which is `parse(input)` then a lazy walk of the
// AST already built — the same events()-is-parse()+AST-walk pattern already
// documented for t2t/pod/haddock/asciidoc above. (Note: events.rs also
// defines a second, *borrowed* `EventIter<'a>` with its own `pub fn
// new(doc: &'a FountainDoc)` — but it is not re-exported from lib.rs and is
// not what `events()` returns, so it is out of scope for this harness; it
// independently appears to double-emit `Event::PageBreak` and never emit a
// `Text` event for any non-Character/Dialogue/Parenthetical block, per a
// direct reading of its `Blocks`-phase match arms, which is worth a
// follow-up look but is not part of the `events()` API this harness checks.)
// The ast_to_events-vs-events() check below is real and passes, but
// validates the AST->event expansion layer, not two independent parsers.
//
// StreamingParser (batch.rs) flushes events per accumulated block as fed
// (blank line, boneyard close, or EOF triggers emit_block()), but
// emit_block() re-parses the block's text via `crate::events::events(&text)`
// and forwards *every* event it yields — including that call's own
// StartDocument/EndDocument pair — straight to the handler with no
// filtering (batch.rs: `for event in crate::events::events(&text) {
// self.handler.handle(event); }`). Since bulk `events()` over the whole
// input emits exactly one StartDocument/EndDocument pair spanning the
// document, but StreamingParser emits one such pair *per accumulated
// block*, this diverges on every fixture with more than one
// blank-line-separated block — the majority of the suite, not an edge case
// the way t2t's header-lookahead bug was. A second, narrower defect shares
// the same root cause: `parse_title_page()` (parse.rs:81) runs
// unconditionally at the start of every `parse()` call with no "is this
// really the first block of the document" guard, so a body block that
// happens to match `key: value` for one of the 9 recognized title-page
// field names (title/credit/author/authors/source/draft date/contact/
// copyright/notes) gets misread as metadata when it is re-parsed in
// isolation, the same class of bug already tracked for t2t's
// try_parse_header().
//
// Writer buffers all fed events into a Vec<OwnedEvent> and only
// reconstructs the AST + calls emit() inside finish() (writer.rs's own
// module doc: "This implementation buffers all events, reconstructs the
// AST, then emits") — the same fake-streaming-writer pattern as
// t2t/pod/haddock/textile/commonmark/org/texinfo.
// ---------------------------------------------------------------------------

fn fountain_ast_to_events(doc: &fountain_fmt::FountainDoc) -> Vec<fountain_fmt::OwnedEvent> {
    use fountain_fmt::Block;
    use fountain_fmt::events::Event;

    let mut out = vec![Event::StartDocument];
    for (key, value) in &doc.metadata {
        out.push(Event::Metadata {
            key: key.clone().into(),
            value: value.clone().into(),
        });
    }

    let blocks = &doc.blocks;
    let mut i = 0;
    while i < blocks.len() {
        if let Block::Character { name, dual, .. } = &blocks[i] {
            out.push(Event::StartDialogueBlock);
            out.push(Event::StartCharacter { dual: *dual });
            out.push(Event::Text(name.clone().into()));
            out.push(Event::EndCharacter);
            i += 1;
            while i < blocks.len()
                && matches!(
                    blocks[i],
                    Block::Dialogue { .. } | Block::Parenthetical { .. }
                )
            {
                fountain_leaf_block_events(&blocks[i], &mut out);
                i += 1;
            }
            out.push(Event::EndDialogueBlock);
        } else {
            fountain_leaf_block_events(&blocks[i], &mut out);
            i += 1;
        }
    }

    out.push(Event::EndDocument);
    out
}

fn fountain_leaf_block_events(b: &fountain_fmt::Block, out: &mut Vec<fountain_fmt::OwnedEvent>) {
    use fountain_fmt::Block;
    use fountain_fmt::events::Event;
    match b {
        Block::SceneHeading { text, .. } => {
            out.push(Event::StartSceneHeading);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndSceneHeading);
        }
        Block::Action { text, .. } => {
            out.push(Event::StartAction);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndAction);
        }
        Block::Character { name, dual, .. } => {
            // Only reached for a Character with no following dialogue at
            // document end, since the caller special-cases the common path.
            out.push(Event::StartCharacter { dual: *dual });
            out.push(Event::Text(name.clone().into()));
            out.push(Event::EndCharacter);
        }
        Block::Dialogue { text, .. } => {
            out.push(Event::StartDialogue);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndDialogue);
        }
        Block::Parenthetical { text, .. } => {
            out.push(Event::StartParenthetical);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndParenthetical);
        }
        Block::Transition { text, .. } => {
            out.push(Event::StartTransition);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndTransition);
        }
        Block::Centered { text, .. } => {
            out.push(Event::StartCentered);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndCentered);
        }
        Block::Lyric { text, .. } => {
            out.push(Event::StartLyric);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndLyric);
        }
        Block::Note { text, .. } => {
            out.push(Event::StartNote);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndNote);
        }
        Block::Synopsis { text, .. } => {
            out.push(Event::StartSynopsis);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndSynopsis);
        }
        Block::Section { level, text, .. } => {
            out.push(Event::StartSection { level: *level });
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndSection);
        }
        Block::PageBreak { .. } => out.push(Event::PageBreak),
        Block::Boneyard { text, .. } => {
            out.push(Event::StartBoneyard);
            out.push(Event::Text(text.clone().into()));
            out.push(Event::EndBoneyard);
        }
    }
}

#[test]
fn fountain_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("fountain");
    let mut checked = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures/fountain dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = fountain_fmt::parse(&input);
        let expected = fountain_ast_to_events(&doc);
        let actual: Vec<_> = fountain_fmt::events(&input).collect();
        assert_eq!(
            expected,
            actual,
            "events() diverged from the AST projection for fixture {}",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of fountain fixtures, got {checked}"
    );
}

/// `StreamingParser` re-parses each accumulated block via
/// `crate::events::events()` and forwards that call's events verbatim,
/// including its own StartDocument/EndDocument pair — so bulk `events()`'s
/// single document-boundary pair vs. one pair per block is expected to (and
/// does) diverge on any fixture with more than one block. Checked via
/// adversarial-chunking equivalence against `events()` over the whole input.
#[test]
fn fountain_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("fountain");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/fountain dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let Ok(input_str) = std::str::from_utf8(&input) else {
            continue;
        };
        let bulk: Vec<fountain_fmt::OwnedEvent> = fountain_fmt::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                fountain_fmt::batch::StreamingParser::new(|e: fountain_fmt::OwnedEvent| {
                    streamed.push(e);
                });
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}"
                ));
            }
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of fountain fixtures, got {checked}"
    );
    assert_or_known_failure("fountain", "streaming_parser", result);
}

/// `Writer` buffers all fed events into a `Vec<OwnedEvent>` and only
/// reconstructs the AST + calls `emit()` inside `finish()` (see
/// `crates/formats/fountain-fmt/src/writer.rs`'s own module doc). Checked
/// via byte-identical comparison against the builder path, plus an
/// incrementality probe.
#[test]
fn fountain_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("fountain");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/fountain dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = fountain_fmt::parse(&input);
        let built = fountain_fmt::build(&doc);

        let mut w = fountain_fmt::Writer::new(Vec::<u8>::new());
        for e in fountain_fmt::events(&input) {
            w.write_event(e);
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from build() for fixture {name}:\n  build():  \
                 {built:?}\n  streamed: {streamed:?}"
            ));
        }
        checked += 1;
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of fountain fixtures, got {checked}"
    );

    // Incrementality probe: byte-identical final content (checked above)
    // only proves correctness, not genuine streaming.
    if result.is_ok() {
        use fountain_fmt::OwnedEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = fountain_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(OwnedEvent::StartDocument);
        w.write_event(OwnedEvent::StartAction);
        w.write_event(OwnedEvent::Text("Hello world".to_string().into()));
        w.write_event(OwnedEvent::EndAction);
        w.write_event(OwnedEvent::EndDocument);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after a full StartDocument/StartAction/\
                 Text/EndAction/EndDocument sequence and before finish() — \
                 fountain_fmt::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
                 reconstructs the AST + calls emit() inside finish() \
                 (crates/formats/fountain-fmt/src/writer.rs, self-admitted in its own module \
                 doc), so it is not a genuine incremental streaming writer despite content \
                 round-tripping correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("fountain", "streaming_writer", result);
}
