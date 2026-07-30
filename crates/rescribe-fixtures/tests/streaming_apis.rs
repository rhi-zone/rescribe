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
// bbcode-fmt: events() is `parse::parse(input)` followed by a tree walk (see
// crates/formats/bbcode-fmt/src/events.rs's `events()`, which literally
// calls `crate::parse::parse(input)` before walking the resulting
// `BbcodeDoc`) — the same "walk the tree parse() already built" shape as
// html-fmt's `events_from_doc(&parse(input).0)`, not an independent
// incremental reader. Unlike html-fmt, there is no format-spec reason
// (foster parenting, adoption agency, etc.) forcing that shape here — it's
// an implementation choice, not a structural absence — so per this task's
// brief the check is wired (like asciidoc's honestly-scoped entry) rather
// than declared `NotApplicable`: it still pins the current AST<->Event
// correspondence and would catch a field silently dropped or reordered by
// the walk, even though it cannot demonstrate two independent parsers.
// `StreamingParser` (batch.rs), by contrast, *is* a genuine incremental
// line-buffered state machine — feed() advances real parser state and calls
// emit_block() (and therefore the handler) as soon as a block boundary
// (blank line, or a recognized block tag's close line) is recognized, not
// only inside finish(). Both the incrementality probe and the
// adversarial-chunking equivalence check against events() pass for real,
// over all 53 bbcode fixtures plus several hand-built adversarial cases
// tried while auditing this (same-line-closed block tag immediately
// followed by more content with no blank line; a blank line inside an
// InBlock quote; nested same-tag quotes) — every case converges because
// StreamingParser's own block-boundary detection only ever needs to be
// *coarser than or equal to* parse()'s, never finer: whatever text it
// accumulates into one flushed chunk gets handed to `crate::events::events()`
// (i.e. a fresh `parse::parse()` call), which re-derives the exact same
// fine-grained block/inline structure the bulk parser would have for that
// span. The streaming `Writer` self-admits (module doc, writer.rs:3) it
// buffers all events and only reconstructs the AST + calls emit() inside
// finish() — the same hollow pattern as texinfo/commonmark's writers; its
// *content* still matches build() exactly (same reason: finish() ends up
// calling the same emit()), so only the incrementality probe fails.
// ---------------------------------------------------------------------------

/// Reconstruct the exact [`bbcode_fmt::events::Event`] sequence `events()`
/// must produce for `doc`, directly from the AST `parse()` returned. Mirrors
/// `bbcode_fmt::events::{emit_block_events, emit_inline_events}` structurally
/// (unavoidable, since bbcode-fmt's `Event` enum is a direct 1:1 mirror of
/// `Block`/`Inline` — see the module comment above for why that means this
/// check pins the AST<->Event correspondence rather than proving two
/// independent implementations agree).
fn bbcode_ast_to_events(doc: &bbcode_fmt::BbcodeDoc) -> Vec<bbcode_fmt::OwnedEvent> {
    let mut out = Vec::new();
    for block in &doc.blocks {
        bbcode_block_events(block, &mut out);
    }
    out
}

fn bbcode_block_events(block: &bbcode_fmt::ast::Block, out: &mut Vec<bbcode_fmt::OwnedEvent>) {
    use bbcode_fmt::Event;
    use bbcode_fmt::ast::Block;
    use std::borrow::Cow;
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for inline in inlines {
                bbcode_inline_events(inline, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::CodeBlock {
            language, content, ..
        } => {
            out.push(Event::CodeBlock {
                language: language.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote {
            author, children, ..
        } => {
            out.push(Event::StartBlockquote {
                author: author.clone(),
            });
            for child in children {
                bbcode_block_events(child, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for inline in item {
                    bbcode_inline_events(inline, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow);
                for (is_header, inlines) in &row.cells {
                    out.push(Event::StartTableCell {
                        is_header: *is_header,
                    });
                    for inline in inlines {
                        bbcode_inline_events(inline, out);
                    }
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::HorizontalRule { .. } => {
            out.push(Event::HorizontalRule);
        }
        Block::Heading {
            level, children, ..
        } => {
            out.push(Event::StartHeading { level: *level });
            for inline in children {
                bbcode_inline_events(inline, out);
            }
            out.push(Event::EndHeading);
        }
        Block::Alignment { kind, children, .. } => {
            out.push(Event::StartAlignment { kind: *kind });
            for child in children {
                bbcode_block_events(child, out);
            }
            out.push(Event::EndAlignment);
        }
        Block::Spoiler { children, .. } => {
            out.push(Event::StartSpoiler);
            for child in children {
                bbcode_block_events(child, out);
            }
            out.push(Event::EndSpoiler);
        }
        Block::Preformatted { content, .. } => {
            out.push(Event::Preformatted {
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Indent { children, .. } => {
            out.push(Event::StartIndent);
            for child in children {
                bbcode_block_events(child, out);
            }
            out.push(Event::EndIndent);
        }
    }
}

fn bbcode_inline_events(inline: &bbcode_fmt::ast::Inline, out: &mut Vec<bbcode_fmt::OwnedEvent>) {
    use bbcode_fmt::Event;
    use bbcode_fmt::ast::Inline;
    use std::borrow::Cow;
    match inline {
        Inline::Text(s, _) => {
            out.push(Event::Text(Cow::Owned(s.clone())));
        }
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            out.push(Event::StartStrikethrough);
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndStrikethrough);
        }
        Inline::Code(s, _) => {
            out.push(Event::InlineCode(Cow::Owned(s.clone())));
        }
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink { url: url.clone() });
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image {
            url, width, height, ..
        } => {
            out.push(Event::InlineImage {
                url: url.clone(),
                width: *width,
                height: *height,
            });
        }
        Inline::Subscript(children, _) => {
            out.push(Event::StartSubscript);
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndSubscript);
        }
        Inline::Superscript(children, _) => {
            out.push(Event::StartSuperscript);
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndSuperscript);
        }
        Inline::Color {
            value, children, ..
        } => {
            out.push(Event::StartColor {
                value: value.clone(),
            });
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndColor);
        }
        Inline::Size {
            value, children, ..
        } => {
            out.push(Event::StartSize {
                value: value.clone(),
            });
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndSize);
        }
        Inline::Font { name, children, .. } => {
            out.push(Event::StartFont { name: name.clone() });
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndFont);
        }
        Inline::Email { addr, children, .. } => {
            out.push(Event::StartEmail { addr: addr.clone() });
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndEmail);
        }
        Inline::Noparse(s, _) => {
            out.push(Event::Noparse(Cow::Owned(s.clone())));
        }
        Inline::Span {
            attr,
            value,
            children,
            ..
        } => {
            out.push(Event::StartSpan {
                attr: attr.clone(),
                value: value.clone(),
            });
            for child in children {
                bbcode_inline_events(child, out);
            }
            out.push(Event::EndSpan);
        }
    }
}

#[test]
fn bbcode_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("bbcode");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/bbcode dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = bbcode_fmt::parse(&input);
        let expected = bbcode_ast_to_events(&doc);
        let actual: Vec<_> = bbcode_fmt::events(&input)
            .map(bbcode_fmt::Event::into_owned)
            .collect();
        checked += 1;
        if expected != actual && result.is_ok() {
            result = Err(format!(
                "events() diverged from the AST projection for fixture {name}:\n  \
                 ast-derived: {expected:?}\n  events():    {actual:?}"
            ));
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of bbcode fixtures, got {checked}"
    );
    assert_or_known_failure("bbcode", "events", result);
}

/// `bbcode_fmt::batch::StreamingParser` accumulates lines into blocks and
/// calls `emit_block()` (which re-parses just the accumulated block text via
/// `crate::events::events()`) as soon as a block boundary is recognized —
/// see `crates/formats/bbcode-fmt/src/batch.rs`'s `feed_line`/`emit_block` —
/// so unlike texinfo/fb2/textile's `StreamingParser` it is not a hollow
/// buffer-then-`finish()` stub. Both halves of this check pass for real:
/// the adversarial-chunking equivalence check against `events()` holds over
/// every bbcode fixture, and the incrementality probe below confirms
/// `feed()` alone delivers events before `finish()` is ever called.
/// `detect_block_tag` (batch.rs:200-224) is coarser than `parse.rs`'s
/// `is_block_start` — it is missing heading/`[hr]` tags entirely and
/// returns `None` (no boundary at all) whenever a recognized block tag's
/// close appears on the same line (batch.rs:217-219) — but this never
/// causes a *visible* divergence: everywhere the streaming splitter is
/// coarser, it only accumulates more text into one flushed chunk, and that
/// chunk is handed to a fresh `crate::events::events()` call, which
/// re-derives the identical fine-grained block/inline structure a bulk
/// `parse()` over that span would have produced. Confirmed by hand against
/// several adversarial cases beyond the fixture suite (same-line-closed
/// tag immediately followed by more content, a blank line inside an
/// `InBlock` quote, nested same-tag quotes) in addition to all 53 fixtures
/// under every chunking in [`adversarial_chunkings`].
#[test]
fn bbcode_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("bbcode");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/bbcode dir") {
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
        let bulk: Vec<bbcode_fmt::OwnedEvent> = bbcode_fmt::events(input_str)
            .map(bbcode_fmt::Event::into_owned)
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                bbcode_fmt::StreamingParser::new(|e: bbcode_fmt::OwnedEvent| streamed.push(e));
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}:\n  events():        {bulk:?}\n  StreamingParser: \
                     {streamed:?}"
                ));
            }
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of bbcode fixtures, got {checked}"
    );

    // Incrementality probe: most individual fixtures are a single block (no
    // internal blank-line boundary), so nothing would legitimately flush
    // before finish() even under a fully incremental implementation — that
    // is not evidence of hollowness (see fixture adv-deeply-nested-unclosed,
    // one unterminated paragraph, found while first drafting this probe as
    // a per-fixture check). Use one hand-built input with an internal block
    // boundary (a completed bold paragraph, a blank line, then unterminated
    // trailing content) instead, and confirm the completed block's events
    // reach the handler before finish() is ever called.
    if result.is_ok() {
        let probe_input = b"[b]Hello[/b]\n\nUnterminated tail with no blank line after it";
        let mut delivered: Vec<bbcode_fmt::OwnedEvent> = Vec::new();
        let mut parser = bbcode_fmt::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        if delivered.is_empty() {
            result = Err(
                "StreamingParser delivered zero events to the handler after feed() with a \
                 complete `[b]Hello[/b]` paragraph followed by a blank line and unterminated \
                 trailing text, and before finish() was called — expected the completed first \
                 block to have been flushed as soon as its terminating blank line arrived"
                    .to_string(),
            );
        }
        // `parser` intentionally dropped without `finish()`: this probe only
        // needs to observe pre-finish handler state.
    }
    assert_or_known_failure("bbcode", "streaming_parser", result);
}

/// `bbcode_fmt::writer::Writer` self-admits (module doc, writer.rs:3) that
/// "this implementation buffers all events, reconstructs the AST, then
/// emits" — `write_event()` (writer.rs:42-44) only pushes onto an internal
/// `Vec<OwnedEvent>`, and all real work (`events_to_doc` + `emit::emit`)
/// happens inside `finish()`. Checked the same way as texinfo/commonmark's
/// writers: byte-identical-to-builder content correctness (expected to
/// pass, since `finish()` ultimately drives the same `emit()` the builder
/// path uses) plus an incrementality probe.
#[test]
fn bbcode_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("bbcode");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/bbcode dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = bbcode_fmt::parse(&input);
        let built = bbcode_fmt::emit(&doc);

        let mut w = bbcode_fmt::Writer::new(Vec::<u8>::new());
        for e in bbcode_fmt::events(&input).map(bbcode_fmt::Event::into_owned) {
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
        "expected to check a substantial number of bbcode fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = bbcode_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(bbcode_fmt::OwnedEvent::StartParagraph);
        w.write_event(bbcode_fmt::OwnedEvent::StartBold);
        w.write_event(bbcode_fmt::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(bbcode_fmt::OwnedEvent::EndBold);
        w.write_event(bbcode_fmt::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/StartBold/Text/EndBold/EndParagraph sequence and before \
                 finish() — bbcode_fmt::writer::Writer buffers all events into a \
                 Vec<OwnedEvent> and only reconstructs the AST + calls emit() inside finish() \
                 (crates/formats/bbcode-fmt/src/writer.rs, self-admitted in its own module doc), \
                 so it is not a genuine incremental streaming writer despite content \
                 round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("bbcode", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// creole: events() vs. an AST projection, StreamingParser adversarial
// chunking, streaming writer vs. builder
// ---------------------------------------------------------------------------

/// Reconstruct the exact [`creole::Event`] sequence `events()` must produce
/// for `doc`, directly from the AST `parse()` returned. Mirrors
/// `creole::events::{collect_block_events, collect_inline_events}`
/// structurally (unavoidable: `creole::events::EventIter::new` is literally
/// `crate::parse::parse(input)` followed by `collect_events(&doc)`, a
/// depth-first walk over the AST — see `crates/formats/creole/src/events.rs`
/// lines 123-127 — the same non-independent shape as bbcode-fmt's and
/// html-fmt's `events()`. Per the bbcode/asciidoc precedent this is still
/// wired as `Wired` rather than `NotApplicable`, since nothing in the
/// Creole format itself forces the coupling (unlike html5ever's tree
/// construction) — it is an implementation choice, not a structural
/// necessity. This check therefore pins the AST<->Event correspondence
/// (and would catch a `collect_events` that dropped or reordered a field)
/// rather than proving two independent implementations agree; the `Event`
/// enum's own `PartialEq` gives exact equality, not merely a lossy shape
/// comparison.
fn creole_ast_to_events(doc: &creole::CreoleDoc) -> Vec<creole::OwnedEvent> {
    let mut out = Vec::new();
    for block in &doc.blocks {
        creole_block_events(block, &mut out);
    }
    out
}

fn creole_block_events(block: &creole::Block, out: &mut Vec<creole::OwnedEvent>) {
    use creole::Block;
    use creole::Event;
    use std::borrow::Cow;
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            creole_inline_events(inlines, out);
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            creole_inline_events(inlines, out);
            out.push(Event::EndHeading);
        }
        Block::CodeBlock { content, .. } => {
            out.push(Event::CodeBlock {
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for child in children {
                creole_block_events(child, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for child in item {
                    creole_block_events(child, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow);
                for cell in &row.cells {
                    out.push(Event::StartTableCell {
                        is_header: cell.is_header,
                    });
                    creole_inline_events(&cell.inlines, out);
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for item in items {
                out.push(Event::StartDefinitionTerm);
                creole_inline_events(&item.term, out);
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                creole_inline_events(&item.desc, out);
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::HorizontalRule(_) => {
            out.push(Event::HorizontalRule);
        }
    }
}

fn creole_inline_events(inlines: &[creole::Inline], out: &mut Vec<creole::OwnedEvent>) {
    use creole::Event;
    use creole::Inline;
    use std::borrow::Cow;
    for inline in inlines {
        match inline {
            Inline::Text(s, _) => {
                out.push(Event::Text(Cow::Owned(s.clone())));
            }
            Inline::LineBreak(_) => {
                out.push(Event::LineBreak);
            }
            Inline::Code(s, _) => {
                out.push(Event::InlineCode(Cow::Owned(s.clone())));
            }
            Inline::Bold(children, _) => {
                out.push(Event::StartBold);
                creole_inline_events(children, out);
                out.push(Event::EndBold);
            }
            Inline::Italic(children, _) => {
                out.push(Event::StartItalic);
                creole_inline_events(children, out);
                out.push(Event::EndItalic);
            }
            Inline::Link { url, children, .. } => {
                out.push(Event::StartLink { url: url.clone() });
                creole_inline_events(children, out);
                out.push(Event::EndLink);
            }
            Inline::Image { url, alt, .. } => {
                out.push(Event::InlineImage {
                    url: url.clone(),
                    alt: alt.clone(),
                });
            }
        }
    }
}

#[test]
fn creole_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("creole");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/creole dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = creole::parse(&input);
        let expected = creole_ast_to_events(&doc);
        let actual: Vec<_> = creole::events(&input).collect();
        checked += 1;
        if expected != actual && result.is_ok() {
            result = Err(format!(
                "events() diverged from the AST projection for fixture {name}:\n  \
                 ast-derived: {expected:?}\n  events():    {actual:?}"
            ));
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of creole fixtures, got {checked}"
    );
    assert_or_known_failure("creole", "events", result);
}

/// `creole::batch::StreamingParser` accumulates lines into blocks and calls
/// `emit_block()` (which re-parses just the accumulated block text via
/// `crate::events::events()`) as soon as a block boundary is recognized —
/// see `crates/formats/creole/src/batch.rs`'s `feed_line`/`emit_block` — so
/// unlike texinfo/fb2/textile's `StreamingParser` it is not a hollow
/// buffer-then-`finish()` stub. Both halves of this check pass for real:
/// the adversarial-chunking equivalence check against `events()` holds over
/// every creole fixture, and a hand-built probe (see the incrementality
/// check inline below) confirms `feed()` alone delivers events before
/// `finish()` is ever called. One inspected-but-unobserved edge case:
/// `feed_line`'s in-nowiki close test (batch.rs, `is_end = line.trim() ==
/// "}}}"`) requires the closing marker to be the *entire* trimmed line,
/// while `parse.rs`'s `parse_nowiki_block` finds `"}}}"` anywhere in the
/// line (dropping any trailing text after it) — so a nowiki block closed by
/// a line like `"tail}}}"` never trips the streaming splitter's boundary
/// and everything from that opener onward is swept into one oversized
/// block, delivered only at `finish()`. Verified by hand
/// (`{{{\ncode\nsome}}}\nmore\n`) that this degrades *incrementality*, not
/// *correctness*: the oversized block is still handed whole to
/// `crate::events::events()`, which re-derives the identical block split a
/// bulk `parse()` over that span would produce, so the final event sequence
/// still matches `events()` exactly — not a tracked `KnownFailure`, since
/// nothing observable diverges.
#[test]
fn creole_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("creole");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/creole dir") {
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
        let bulk: Vec<creole::OwnedEvent> = creole::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                creole::batch::StreamingParser::new(|e: creole::OwnedEvent| streamed.push(e));
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}:\n  events():        {bulk:?}\n  StreamingParser: \
                     {streamed:?}"
                ));
            }
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of creole fixtures, got {checked}"
    );

    // Incrementality probe: confirm the completed first block's events reach
    // the handler before finish() is ever called, for a multi-block input
    // (a completed heading, a blank line, then unterminated trailing text).
    if result.is_ok() {
        let probe_input = b"= Hello\n\nUnterminated tail with no blank line after it";
        let mut delivered: Vec<creole::OwnedEvent> = Vec::new();
        let mut parser = creole::batch::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        if delivered.is_empty() {
            result = Err(
                "StreamingParser delivered zero events to the handler after feed() with a \
                 complete `= Hello` heading followed by a blank line and unterminated trailing \
                 text, and before finish() was called — expected the completed first block to \
                 have been flushed as soon as its terminating blank line arrived"
                    .to_string(),
            );
        }
        // `parser` intentionally dropped without `finish()`: this probe only
        // needs to observe pre-finish handler state.
    }
    assert_or_known_failure("creole", "streaming_parser", result);
}

/// `creole::writer::Writer` buffers all fed events into an internal
/// `Vec<OwnedEvent>` (`write_event()`, writer.rs:38-40, only pushes) and
/// only reconstructs the AST (`events_to_doc`) + calls `crate::emit::build`
/// inside `finish()` (writer.rs:43-48) — a hollow buffer-then-finish
/// implementation, not a genuine incremental streaming writer. Checked the
/// same way as bbcode/textile/commonmark's writers: byte-identical-to-builder
/// content correctness (expected to pass, since `finish()` ultimately drives
/// the same `build()` the builder path uses) plus an incrementality probe.
#[test]
fn creole_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("creole");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/creole dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = creole::parse(&input);
        let built = creole::build(&doc);

        let mut w = creole::writer::Writer::new(Vec::<u8>::new());
        for e in creole::events(&input) {
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
        "expected to check a substantial number of creole fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = creole::writer::Writer::new(ObservableSink(observed.clone()));
        w.write_event(creole::OwnedEvent::StartParagraph);
        w.write_event(creole::OwnedEvent::StartBold);
        w.write_event(creole::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(creole::OwnedEvent::EndBold);
        w.write_event(creole::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/StartBold/Text/EndBold/EndParagraph sequence and before \
                 finish() — creole::writer::Writer buffers all events into a Vec<OwnedEvent> \
                 and only reconstructs the AST + calls build() inside finish() \
                 (crates/formats/creole/src/writer.rs), so it is not a genuine incremental \
                 streaming writer despite content round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("creole", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// dokuwiki: events() vs. an AST projection, StreamingParser adversarial
// chunking, streaming writer vs. builder
// ---------------------------------------------------------------------------

// dokuwiki's `events()` (`crate::events::events`, re-exported at the crate
// root, `lib.rs:33-35`) is `InputEventIter::new`, which calls
// `crate::parse::parse(input)` to build a `DokuwikiDoc`, walks it with the
// crate's own lazy `EventIter` (a genuine O(depth) stack-machine walker over
// the already-owned tree — see `events.rs`'s `Frame`/`Iterator::next`), and
// eagerly collects the result into an owned `Vec` before returning
// (`events.rs:705-731`, self-documented: "not a genuine streaming parser...
// Memory use is therefore O(full document), not O(depth)"). That is the same
// "parse() then walk the tree" shape as bbcode-fmt's and creole's `events()`
// (and html-fmt's `events_from_doc`), not two independent implementations —
// but as with those two, nothing in the DokuWiki format forces this shape,
// so per the bbcode/creole/asciidoc precedent it is wired here rather than
// declared `NotApplicable`: the check still pins the exact AST<->Event
// correspondence and would catch a field silently dropped or reordered by
// the walk. `StreamingParser` (`batch.rs`), by contrast, is a genuine
// incremental line-buffered state machine: `feed()` advances real per-line
// state (`BlockState::{Between,Accumulating,InSpecialBlock}`) and calls
// `emit_block()` (which re-parses just the accumulated block text via
// `crate::events::events()`, i.e. a fresh `parse::parse()` call) as soon as
// a blank line or a recognized `<code>/<file>/<html>/<php>` block boundary
// is seen — not only inside `finish()`. Unlike org-fmt/rst-fmt/djot-fmt's
// batch parsers, dokuwiki's `Parser` (`parse.rs`) has *no* cross-block
// state at all (no loose-list joining across blank lines — `parse_list_items`
// already stops at a non-`"  "`-prefixed line, which includes blank lines —
// and no forward/backward reference resolution), so every block boundary
// `StreamingParser` can pick is one `parse.rs`'s own top-level dispatch loop
// would also treat as a valid block split point; re-parsing each flushed
// chunk in isolation re-derives the identical block/inline structure a bulk
// `parse()` over the whole input would. Confirmed here over every dokuwiki
// fixture under every chunking in [`adversarial_chunkings`] plus the
// incrementality probe below. The streaming `Writer` (`writer.rs`)
// self-admits (module doc, `writer.rs:3`, "Buffers all events, reconstructs
// the AST, then emits") that `write_event()` only pushes onto an internal
// `Vec<OwnedEvent>` (`writer.rs:27-29`) and all real work happens inside
// `finish()` (`writer.rs:32-37`) — the same hollow pattern as
// bbcode/creole/texinfo/commonmark's writers. Its *content* still matches
// `build()` exactly (same reason: `finish()` ends up calling the same
// `crate::emit::build`), so only the incrementality probe fails.
//
// Event-enum expressiveness: every `Block`/`Inline` variant and field in
// `ast.rs` has a corresponding `Event` variant/field in `events.rs` (block
// metadata such as `FileBlock`'s `filename`, `RawBlock`'s `format`,
// `Macro`'s `name`, and `Image`'s `alt` all round-trip) — no expressiveness
// gap was found for this crate, unlike org-fmt (no metadata variant) or
// djot-fmt (no `LinkDef` variant).
// ---------------------------------------------------------------------------

/// Reconstruct the exact [`dokuwiki::Event`] sequence `events()` must produce
/// for `doc`, directly from the AST `parse()` returned. Mirrors
/// `dokuwiki::events::{EventIter, emit_inline}` structurally (unavoidable:
/// `events()` is `parse()` + a walk over the resulting `DokuwikiDoc` — see
/// the module comment above for why that means this check pins the
/// AST<->Event correspondence rather than proving two independent
/// implementations agree).
fn dokuwiki_ast_to_events(doc: &dokuwiki::DokuwikiDoc) -> Vec<dokuwiki::OwnedEvent> {
    let mut out = Vec::new();
    for block in &doc.blocks {
        dokuwiki_block_events(block, &mut out);
    }
    out
}

fn dokuwiki_block_events(block: &dokuwiki::Block, out: &mut Vec<dokuwiki::OwnedEvent>) {
    use dokuwiki::Block;
    use dokuwiki::Event;
    use std::borrow::Cow;
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            for inline in inlines {
                dokuwiki_inline_events(inline, out);
            }
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            for inline in inlines {
                dokuwiki_inline_events(inline, out);
            }
            out.push(Event::EndHeading);
        }
        Block::CodeBlock {
            language, content, ..
        } => {
            out.push(Event::CodeBlock {
                language: language.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::FileBlock {
            language,
            filename,
            content,
            ..
        } => {
            out.push(Event::FileBlock {
                language: language.clone(),
                filename: filename.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for child in children {
                dokuwiki_block_events(child, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for inline in &item.inlines {
                    dokuwiki_inline_events(inline, out);
                }
                for child in &item.children {
                    dokuwiki_block_events(child, out);
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow {
                    is_header: row.is_header,
                });
                for cell in &row.cells {
                    out.push(Event::StartTableCell);
                    for inline in &cell.inlines {
                        dokuwiki_inline_events(inline, out);
                    }
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::DefinitionList { items, .. } => {
            out.push(Event::StartDefinitionList);
            for item in items {
                out.push(Event::StartDefinitionTerm);
                for inline in &item.term {
                    dokuwiki_inline_events(inline, out);
                }
                out.push(Event::EndDefinitionTerm);
                out.push(Event::StartDefinitionDesc);
                for inline in &item.desc {
                    dokuwiki_inline_events(inline, out);
                }
                out.push(Event::EndDefinitionDesc);
            }
            out.push(Event::EndDefinitionList);
        }
        Block::HorizontalRule(_) => {
            out.push(Event::HorizontalRule);
        }
        Block::RawBlock {
            format, content, ..
        } => {
            out.push(Event::RawBlock {
                format: format.clone(),
                content: content.clone(),
            });
        }
        Block::Macro { name, .. } => {
            out.push(Event::Macro { name: name.clone() });
        }
    }
}

fn dokuwiki_inline_events(inline: &dokuwiki::Inline, out: &mut Vec<dokuwiki::OwnedEvent>) {
    use dokuwiki::Event;
    use dokuwiki::Inline;
    use std::borrow::Cow;
    match inline {
        Inline::Text(s, _) => {
            out.push(Event::Text(Cow::Owned(s.clone())));
        }
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            out.push(Event::StartStrikethrough);
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndStrikethrough);
        }
        Inline::Superscript(children, _) => {
            out.push(Event::StartSuperscript);
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndSuperscript);
        }
        Inline::Subscript(children, _) => {
            out.push(Event::StartSubscript);
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndSubscript);
        }
        Inline::Code(s, _) => {
            out.push(Event::InlineCode(Cow::Owned(s.clone())));
        }
        Inline::Nowiki(s, _) => {
            out.push(Event::Nowiki(Cow::Owned(s.clone())));
        }
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink { url: url.clone() });
            for child in children {
                dokuwiki_inline_events(child, out);
            }
            out.push(Event::EndLink);
        }
        Inline::Image { url, alt, .. } => {
            out.push(Event::InlineImage {
                url: url.clone(),
                alt: alt.clone(),
            });
        }
        Inline::FootnoteRef { content, .. } => {
            out.push(Event::FootnoteRef {
                content: content.clone(),
            });
        }
        Inline::LineBreak(_) => {
            out.push(Event::LineBreak);
        }
        Inline::SoftBreak(_) => {
            out.push(Event::SoftBreak);
        }
    }
}

#[test]
fn dokuwiki_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("dokuwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/dokuwiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = dokuwiki::parse(&input);
        let expected = dokuwiki_ast_to_events(&doc);
        let actual: Vec<_> = dokuwiki::events(&input)
            .map(dokuwiki::Event::into_owned)
            .collect();
        checked += 1;
        if expected != actual && result.is_ok() {
            result = Err(format!(
                "events() diverged from the AST projection for fixture {name}:\n  \
                 ast-derived: {expected:?}\n  events():    {actual:?}"
            ));
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of dokuwiki fixtures, got {checked}"
    );
    assert_or_known_failure("dokuwiki", "events", result);
}

/// `dokuwiki::StreamingParser` accumulates lines into blocks and calls
/// `emit_block()` (which re-parses just the accumulated block text via
/// `crate::events::events()`) as soon as a blank line or a recognized
/// `<code>/<file>/<html>/<php>` block boundary is seen — see `batch.rs`'s
/// `feed_line`/`emit_block` — so unlike texinfo/fb2/textile's
/// `StreamingParser` it is not a hollow buffer-then-`finish()` stub. Both
/// halves of this check pass for real: the adversarial-chunking equivalence
/// check against `events()` holds over every dokuwiki fixture, and the
/// incrementality probe below confirms `feed()` alone delivers events before
/// `finish()` is ever called. This holds cleanly (no coarser-boundary caveat
/// needed, unlike bbcode/creole's `detect_block_tag`) because `parse.rs`'s
/// `Parser` has no cross-block state: every block type's own consumption
/// loop (`parse_list_items`, `parse_table`, `parse_definition_list`,
/// `parse_blockquote`, `parse_paragraph`) already stops at the same
/// boundaries `StreamingParser::feed_line` flushes on (a blank line, or a
/// `<code>/<file>/<html>/<php>` tag), so re-parsing a flushed chunk in
/// isolation always reproduces the identical block/inline structure a bulk
/// `parse()` over that span would.
#[test]
fn dokuwiki_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("dokuwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/dokuwiki dir") {
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
        let bulk: Vec<dokuwiki::OwnedEvent> = dokuwiki::events(input_str)
            .map(dokuwiki::Event::into_owned)
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                dokuwiki::StreamingParser::new(|e: dokuwiki::OwnedEvent| streamed.push(e));
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}:\n  events():        {bulk:?}\n  StreamingParser: \
                     {streamed:?}"
                ));
            }
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of dokuwiki fixtures, got {checked}"
    );

    // Incrementality probe: confirm a completed block's events reach the
    // handler as soon as its terminating blank line arrives, before
    // finish() is ever called.
    if result.is_ok() {
        let probe_input = b"**Hello**\n\nUnterminated tail with no blank line after it";
        let mut delivered: Vec<dokuwiki::OwnedEvent> = Vec::new();
        let mut parser = dokuwiki::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        if delivered.is_empty() {
            result = Err(
                "StreamingParser delivered zero events to the handler after feed() with a \
                 complete `**Hello**` paragraph followed by a blank line and unterminated \
                 trailing text, and before finish() was called — expected the completed first \
                 block to have been flushed as soon as its terminating blank line arrived"
                    .to_string(),
            );
        }
        // `parser` intentionally dropped without `finish()`: this probe only
        // needs to observe pre-finish handler state.
    }
    assert_or_known_failure("dokuwiki", "streaming_parser", result);
}

/// `dokuwiki::writer::Writer` self-admits (module doc, `writer.rs:3`) that
/// "Buffers all events, reconstructs the AST, then emits" — `write_event()`
/// (`writer.rs:27-29`) only pushes onto an internal `Vec<OwnedEvent>`, and
/// all real work (`events_to_doc` + `crate::emit::build`) happens inside
/// `finish()` (`writer.rs:32-37`). Checked the same way as
/// bbcode/creole/texinfo/commonmark's writers: byte-identical-to-builder
/// content correctness (expected to pass, since `finish()` ultimately drives
/// the same `build()` the builder path uses) plus an incrementality probe.
#[test]
fn dokuwiki_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("dokuwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/dokuwiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = dokuwiki::parse(&input);
        let built = dokuwiki::build(&doc);

        let mut w = dokuwiki::Writer::new(Vec::<u8>::new());
        for e in dokuwiki::events(&input).map(dokuwiki::Event::into_owned) {
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
        "expected to check a substantial number of dokuwiki fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = dokuwiki::Writer::new(ObservableSink(observed.clone()));
        w.write_event(dokuwiki::OwnedEvent::StartParagraph);
        w.write_event(dokuwiki::OwnedEvent::StartBold);
        w.write_event(dokuwiki::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(dokuwiki::OwnedEvent::EndBold);
        w.write_event(dokuwiki::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/StartBold/Text/EndBold/EndParagraph sequence and before \
                 finish() — dokuwiki::writer::Writer buffers all events into a Vec<OwnedEvent> \
                 and only reconstructs the AST + calls crate::emit::build inside finish() \
                 (crates/formats/dokuwiki/src/writer.rs, self-admitted in its own module doc), \
                 so it is not a genuine incremental streaming writer despite content \
                 round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("dokuwiki", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// jira-fmt: events() vs AST projection, StreamingParser vs events(),
// streaming Writer vs build()
// ---------------------------------------------------------------------------
//
// jira-fmt's `events()` (`crates/formats/jira-fmt/src/events.rs::events`) is
// `crate::parse::parse(input)` followed by a full walk of the resulting
// `JiraDoc` into a `Vec<OwnedEvent>` (`emit_doc_events`/`emit_block_events`/
// `emit_inline_events`) — the same "parse() then walk the tree" shape as
// bbcode-fmt's, creole's, and dokuwiki's `events()`, not two independent
// implementations. Nothing in the Jira wiki markup format forces this shape,
// but per the bbcode/creole/dokuwiki precedent established earlier in this
// file it is still wired as `Wired` rather than `NotApplicable` — the check
// below still pins the real AST<->Event correspondence (a hand-written
// projection built independently from `ast.rs`, not by calling the crate's
// own private `emit_*_events` helpers). `jira_fmt::Event` has a variant
// carrying every field every `Block`/`Inline` variant holds (checked by
// exhaustive match below) — no expressiveness gap was found for this crate.
//
// `jira_fmt::batch::StreamingParser` (`batch.rs`) is a genuine incremental
// line-buffered state machine, not a hollow buffer-then-`finish()` stub:
// `feed_line` dispatches per line into `{code:.../{quote}/{noformat}/{panel`
// delimited-block accumulation or blank-line-terminated block accumulation,
// and `emit_block()` re-parses just the accumulated block text via
// `crate::events::events()` as soon as a boundary is seen — real `Wired`,
// confirmed below by an adversarial-chunking equivalence check against
// `events()` over every jira fixture plus an incrementality probe. This
// holds with no coarser-boundary caveat (unlike bbcode/creole's
// `detect_block_tag`): `parse.rs`'s `Parser` has no state that spans a blank
// line or a delimited-block boundary (no loose-list joining, no reference
// resolution, no title/attribute line preceding a fence — the `{code:lang}`
// language and `{panel:title=...}` title are both encoded on the fence line
// itself, so there is no "flush a decorator line away from its target"
// construct for this format's grammar to trigger the class of bug found in
// org-fmt/asciidoc/djot-fmt), so every boundary `feed_line` flushes on is
// one `parse.rs`'s own `parse_paragraph`/`parse_list_at_depth`/`parse_table`
// stop conditions would also treat as a block boundary — re-parsing a
// flushed chunk in isolation always reproduces the identical block/inline
// structure a bulk `parse()` over that span would.
//
// `jira_fmt::writer::Writer` self-admits (module doc, `writer.rs:1-3`)
// "This implementation buffers all events, reconstructs the AST, then
// emits" — `write_event()` (`writer.rs:40-42`) only pushes onto an internal
// `Vec<OwnedEvent>`, and all real work (`events_to_doc` + `crate::emit::
// build`) happens inside `finish()` (`writer.rs:45-50`). Checked the same
// way as bbcode/creole/dokuwiki's writers: byte-identical-to-builder content
// correctness (expected to pass, since `finish()` ultimately drives the same
// `build()` path the builder uses) plus an incrementality probe that is
// expected to fail (zero bytes reach the sink before `finish()`).
// ---------------------------------------------------------------------------

/// Reconstruct the exact [`jira_fmt::OwnedEvent`] sequence `events()` must
/// produce for `doc`, directly from the AST `parse()` returned. Mirrors
/// `jira_fmt::events::{emit_block_events, emit_inline_events}` structurally
/// (unavoidable: `events()` is `parse()` + a walk over the resulting
/// `JiraDoc` — see the module comment above for why that means this check
/// pins the AST<->Event correspondence rather than proving two independent
/// implementations agree), but built independently from `jira_fmt::ast`
/// rather than by calling those private crate-internal helpers.
fn jira_ast_to_events(doc: &jira_fmt::JiraDoc) -> Vec<jira_fmt::OwnedEvent> {
    let mut out = Vec::new();
    for block in &doc.blocks {
        jira_block_events(block, &mut out);
    }
    out
}

fn jira_block_events(block: &jira_fmt::Block, out: &mut Vec<jira_fmt::OwnedEvent>) {
    use jira_fmt::Block;
    use jira_fmt::Event;
    use jira_fmt::ListItemContent;
    use std::borrow::Cow;
    match block {
        Block::Paragraph { inlines, .. } => {
            out.push(Event::StartParagraph);
            jira_inline_events_all(inlines, out);
            out.push(Event::EndParagraph);
        }
        Block::Heading { level, inlines, .. } => {
            out.push(Event::StartHeading { level: *level });
            jira_inline_events_all(inlines, out);
            out.push(Event::EndHeading);
        }
        Block::CodeBlock {
            content, language, ..
        } => {
            out.push(Event::CodeBlock {
                language: language.clone(),
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Noformat { content, .. } => {
            out.push(Event::Noformat {
                content: Cow::Owned(content.clone()),
            });
        }
        Block::Blockquote { children, .. } => {
            out.push(Event::StartBlockquote);
            for child in children {
                jira_block_events(child, out);
            }
            out.push(Event::EndBlockquote);
        }
        Block::Panel {
            title, children, ..
        } => {
            out.push(Event::StartPanel {
                title: title.clone(),
            });
            for child in children {
                jira_block_events(child, out);
            }
            out.push(Event::EndPanel);
        }
        Block::List { ordered, items, .. } => {
            out.push(Event::StartList { ordered: *ordered });
            for item in items {
                out.push(Event::StartListItem);
                for content in &item.children {
                    match content {
                        ListItemContent::Inline(inlines) => {
                            out.push(Event::StartParagraph);
                            jira_inline_events_all(inlines, out);
                            out.push(Event::EndParagraph);
                        }
                        ListItemContent::NestedList(nested) => {
                            jira_block_events(nested, out);
                        }
                    }
                }
                out.push(Event::EndListItem);
            }
            out.push(Event::EndList);
        }
        Block::Table { rows, .. } => {
            out.push(Event::StartTable);
            for row in rows {
                out.push(Event::StartTableRow);
                for cell in &row.cells {
                    out.push(Event::StartTableCell {
                        is_header: cell.is_header,
                    });
                    jira_inline_events_all(&cell.inlines, out);
                    out.push(Event::EndTableCell);
                }
                out.push(Event::EndTableRow);
            }
            out.push(Event::EndTable);
        }
        Block::HorizontalRule { .. } => {
            out.push(Event::HorizontalRule);
        }
    }
}

fn jira_inline_events_all(inlines: &[jira_fmt::Inline], out: &mut Vec<jira_fmt::OwnedEvent>) {
    for inline in inlines {
        jira_inline_events(inline, out);
    }
}

fn jira_inline_events(inline: &jira_fmt::Inline, out: &mut Vec<jira_fmt::OwnedEvent>) {
    use jira_fmt::Event;
    use jira_fmt::Inline;
    use std::borrow::Cow;
    match inline {
        Inline::Text(s, _) => {
            out.push(Event::Text(Cow::Owned(s.clone())));
        }
        Inline::Bold(children, _) => {
            out.push(Event::StartBold);
            jira_inline_events_all(children, out);
            out.push(Event::EndBold);
        }
        Inline::Italic(children, _) => {
            out.push(Event::StartItalic);
            jira_inline_events_all(children, out);
            out.push(Event::EndItalic);
        }
        Inline::Underline(children, _) => {
            out.push(Event::StartUnderline);
            jira_inline_events_all(children, out);
            out.push(Event::EndUnderline);
        }
        Inline::Strikethrough(children, _) => {
            out.push(Event::StartStrikethrough);
            jira_inline_events_all(children, out);
            out.push(Event::EndStrikethrough);
        }
        Inline::Code(s, _) => {
            out.push(Event::InlineCode(Cow::Owned(s.clone())));
        }
        Inline::Link { url, children, .. } => {
            out.push(Event::StartLink { url: url.clone() });
            jira_inline_events_all(children, out);
            out.push(Event::EndLink);
        }
        Inline::Image { url, alt, .. } => {
            out.push(Event::InlineImage {
                url: url.clone(),
                alt: alt.clone(),
            });
        }
        Inline::Superscript(children, _) => {
            out.push(Event::StartSuperscript);
            jira_inline_events_all(children, out);
            out.push(Event::EndSuperscript);
        }
        Inline::Subscript(children, _) => {
            out.push(Event::StartSubscript);
            jira_inline_events_all(children, out);
            out.push(Event::EndSubscript);
        }
        Inline::ColorSpan {
            color, children, ..
        } => {
            out.push(Event::StartColorSpan {
                color: color.clone(),
            });
            jira_inline_events_all(children, out);
            out.push(Event::EndColorSpan);
        }
        Inline::Mention(name, _) => {
            out.push(Event::Mention(Cow::Owned(name.clone())));
        }
    }
}

#[test]
fn jira_events_equals_ast_projection_over_all_fixtures() {
    let root = fixtures_root().join("jira");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/jira dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = jira_fmt::parse(&input);
        let expected = jira_ast_to_events(&doc);
        let actual: Vec<_> = jira_fmt::events(&input).collect();
        checked += 1;
        if expected != actual && result.is_ok() {
            result = Err(format!(
                "events() diverged from the AST projection for fixture {name}:\n  \
                 ast-derived: {expected:?}\n  events():    {actual:?}"
            ));
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of jira fixtures, got {checked}"
    );
    assert_or_known_failure("jira", "events", result);
}

#[test]
fn jira_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("jira");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/jira dir") {
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
        let bulk: Vec<jira_fmt::OwnedEvent> = jira_fmt::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                jira_fmt::StreamingParser::new(|e: jira_fmt::OwnedEvent| streamed.push(e));
            for chunk in &chunks {
                parser.feed(chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}:\n  events():        {bulk:?}\n  StreamingParser: \
                     {streamed:?}"
                ));
            }
        }
    }
    assert!(
        checked > 10,
        "expected to check a substantial number of jira fixtures, got {checked}"
    );

    // Incrementality probe: confirm a completed block's events reach the
    // handler as soon as its terminating blank line arrives, before
    // finish() is ever called.
    if result.is_ok() {
        let probe_input = b"*Hello*\n\nUnterminated tail with no blank line after it";
        let mut delivered: Vec<jira_fmt::OwnedEvent> = Vec::new();
        let mut parser = jira_fmt::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        if delivered.is_empty() {
            result = Err(
                "StreamingParser delivered zero events to the handler after feed() with a \
                 complete `*Hello*` paragraph followed by a blank line and unterminated \
                 trailing text, and before finish() was called — expected the completed first \
                 block to have been flushed as soon as its terminating blank line arrived"
                    .to_string(),
            );
        }
        // `parser` intentionally dropped without `finish()`: this probe only
        // needs to observe pre-finish handler state.
    }
    assert_or_known_failure("jira", "streaming_parser", result);
}

/// `jira_fmt::writer::Writer` self-admits (module doc, `writer.rs:1-3`) that
/// "this implementation buffers all events, reconstructs the AST, then
/// emits" — `write_event()` (`writer.rs:40-42`) only pushes onto an internal
/// `Vec<OwnedEvent>`, and all real work (`events_to_doc` + `crate::emit::
/// build`) happens inside `finish()` (`writer.rs:45-50`). Checked the same
/// way as bbcode/creole/dokuwiki's writers: byte-identical-to-builder
/// content correctness (expected to pass, since `finish()` ultimately drives
/// the same `build()` path the builder uses) plus an incrementality probe.
#[test]
fn jira_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("jira");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/jira dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = jira_fmt::parse(&input);
        let built = jira_fmt::build(&doc);

        let mut w = jira_fmt::Writer::new(Vec::<u8>::new());
        for e in jira_fmt::events(&input) {
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
        "expected to check a substantial number of jira fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = jira_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(jira_fmt::OwnedEvent::StartParagraph);
        w.write_event(jira_fmt::OwnedEvent::StartBold);
        w.write_event(jira_fmt::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(jira_fmt::OwnedEvent::EndBold);
        w.write_event(jira_fmt::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/StartBold/Text/EndBold/EndParagraph sequence and before \
                 finish() — jira_fmt::writer::Writer buffers all events into a Vec<OwnedEvent> \
                 and only reconstructs the AST + calls crate::emit::build inside finish() \
                 (crates/formats/jira-fmt/src/writer.rs, self-admitted in its own module doc), \
                 so it is not a genuine incremental streaming writer despite content \
                 round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("jira", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// mediawiki-fmt: events() vs parse(), StreamingParser vs events(), streaming
// writer vs build() -- all fully wired
// ---------------------------------------------------------------------------
//
// mediawiki-fmt's `events()` (`EventIter::new`, events.rs) is architecturally
// parse()-then-walk: it calls `crate::parse::parse(input)` and then walks the
// resulting tree with `emit_doc_events`/`emit_block_events`/
// `emit_inline_events`. Unlike html-fmt's `events_from_doc` (a generic,
// structure-free depth-first walk over an html5ever DOM -- see the `html`
// `CAPABILITIES` entry), mediawiki-fmt's walk makes real per-variant semantic
// decisions (one arm per `Block`/`Inline` variant, e.g. `Inline::Link`
// unpacks into `StartLink`/`Text`/`EndLink`), so an independently-derived
// projection from the AST can and does diverge from the walk when the walk
// has a real mapping bug -- it is not guaranteed to pass by construction the
// way html's would be. This mirrors asciidoc's narrower-than-rst "Wired"
// claim (see the comment above `asciidoc_events_check`): it validates the
// AST->event projection layer, not two independent parsers, because both
// `events()` and this check's `mw_ast_to_events` start from the same
// `parse()` output.
mod mediawiki_events_check {
    use super::{assert_or_known_failure, find_input, fixtures_root};
    use mediawiki_fmt::ast::{Block, Inline, MediawikiDoc};
    use mediawiki_fmt::events::OwnedEvent;
    use std::borrow::Cow;
    type Event = OwnedEvent;

    fn mw_ast_to_events(doc: &MediawikiDoc) -> Vec<Event> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            mw_block_events(b, &mut out);
        }
        out
    }

    fn mw_block_events(b: &Block, out: &mut Vec<Event>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(Event::StartParagraph);
                mw_inline_events(inlines, out);
                out.push(Event::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(Event::StartHeading { level: *level });
                mw_inline_events(inlines, out);
                out.push(Event::EndHeading);
            }
            Block::CodeBlock {
                language, content, ..
            } => {
                out.push(Event::CodeBlock {
                    language: language.clone(),
                    content: Cow::Owned(content.clone()),
                });
            }
            Block::List { ordered, items, .. } => {
                out.push(Event::StartList { ordered: *ordered });
                for item_blocks in items {
                    out.push(Event::StartListItem);
                    for b in item_blocks {
                        mw_block_events(b, out);
                    }
                    out.push(Event::EndListItem);
                }
                out.push(Event::EndList);
            }
            Block::DefinitionList { items, .. } => {
                out.push(Event::StartDefinitionList);
                for item in items {
                    out.push(Event::StartDefinitionTerm);
                    mw_inline_events(&item.term, out);
                    out.push(Event::EndDefinitionTerm);
                    out.push(Event::StartDefinitionDesc);
                    mw_inline_events(&item.desc, out);
                    out.push(Event::EndDefinitionDesc);
                }
                out.push(Event::EndDefinitionList);
            }
            Block::HorizontalRule => out.push(Event::HorizontalRule),
            Block::Table { rows, caption, .. } => {
                out.push(Event::StartTable {
                    caption: caption.clone(),
                });
                for row in rows {
                    out.push(Event::StartTableRow);
                    for cell in &row.cells {
                        out.push(Event::StartTableCell {
                            is_header: cell.is_header,
                        });
                        mw_inline_events(&cell.inlines, out);
                        out.push(Event::EndTableCell);
                    }
                    out.push(Event::EndTableRow);
                }
                out.push(Event::EndTable);
            }
            Block::Blockquote { children, .. } => {
                out.push(Event::StartBlockquote);
                for child in children {
                    mw_block_events(child, out);
                }
                out.push(Event::EndBlockquote);
            }
            Block::PreBlock { content, .. } => {
                out.push(Event::PreBlock {
                    content: Cow::Owned(content.clone()),
                });
            }
            Block::RawBlock { content, .. } => {
                out.push(Event::RawBlock {
                    content: Cow::Owned(content.clone()),
                });
            }
        }
    }

    fn mw_inline_events(inlines: &[Inline], out: &mut Vec<Event>) {
        for i in inlines {
            match i {
                Inline::Text(s) => out.push(Event::Text(Cow::Owned(s.clone()))),
                Inline::Bold(children) => {
                    out.push(Event::StartBold);
                    mw_inline_events(children, out);
                    out.push(Event::EndBold);
                }
                Inline::Italic(children) => {
                    out.push(Event::StartItalic);
                    mw_inline_events(children, out);
                    out.push(Event::EndItalic);
                }
                Inline::Code(s) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
                Inline::Link { url, text } => {
                    out.push(Event::StartLink { url: url.clone() });
                    out.push(Event::Text(Cow::Owned(text.clone())));
                    out.push(Event::EndLink);
                }
                Inline::Image { url, alt } => out.push(Event::InlineImage {
                    url: url.clone(),
                    alt: alt.clone(),
                }),
                Inline::LineBreak => out.push(Event::LineBreak),
                Inline::Strikeout(children) => {
                    out.push(Event::StartStrikethrough);
                    mw_inline_events(children, out);
                    out.push(Event::EndStrikethrough);
                }
                Inline::Underline(children) => {
                    out.push(Event::StartUnderline);
                    mw_inline_events(children, out);
                    out.push(Event::EndUnderline);
                }
                Inline::Subscript(children) => {
                    out.push(Event::StartSubscript);
                    mw_inline_events(children, out);
                    out.push(Event::EndSubscript);
                }
                Inline::Superscript(children) => {
                    out.push(Event::StartSuperscript);
                    mw_inline_events(children, out);
                    out.push(Event::EndSuperscript);
                }
                Inline::FootnoteRef { label, content } => out.push(Event::FootnoteRef {
                    label: label.clone(),
                    content: content.clone(),
                }),
                Inline::MathInline { source } => out.push(Event::MathInline {
                    source: source.clone(),
                }),
                Inline::Template { content } => out.push(Event::Template {
                    content: content.clone(),
                }),
                Inline::Nowiki { content } => out.push(Event::Nowiki {
                    content: content.clone(),
                }),
            }
        }
    }

    #[test]
    fn mediawiki_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("mediawiki");
        let mut checked = 0;
        let mut result: Result<(), String> = Ok(());
        for entry in std::fs::read_dir(&root).expect("fixtures/mediawiki dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = mediawiki_fmt::parse::parse(&input);
            let expected = mw_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> = mediawiki_fmt::events(&input)
                .map(|e| e.into_owned())
                .collect();
            checked += 1;
            if expected != actual && result.is_ok() {
                result = Err(format!(
                    "events() diverged from the AST projection for fixture {name}:\n  \
                     ast-derived: {expected:?}\n  events():    {actual:?}"
                ));
            }
        }
        assert!(
            checked > 20,
            "expected to check a substantial number of mediawiki fixtures, got {checked}"
        );
        assert_or_known_failure("mediawiki", "events", result);
    }
}

/// `StreamingParser` fed a mediawiki fixture under an adversarial chunking
/// must deliver the same event sequence `events()` delivers over the whole
/// input.
///
/// `mediawiki_fmt::batch::StreamingParser::emit_block` re-parses each
/// accumulated block in isolation via `crate::events::events(&text)`
/// (batch.rs) -- the same "re-parse each block" architecture already found to
/// split cross-block constructs for rst/org/asciidoc's `StreamingParser`s.
#[test]
fn mediawiki_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("mediawiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/mediawiki dir") {
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
        let bulk: Vec<mediawiki_fmt::OwnedEvent> = mediawiki_fmt::events(input_str)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser = mediawiki_fmt::StreamingParser::new(|e: mediawiki_fmt::OwnedEvent| {
                streamed.push(e)
            });
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
        checked > 20,
        "expected to check a substantial number of mediawiki fixtures, got {checked}"
    );
    assert_or_known_failure("mediawiki", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(input)` must reproduce what
/// `emit()` produces for the AST `parse(input)` returned. Also probes for
/// genuine incrementality: `Writer::write_event` (writer.rs) only pushes onto
/// an internal `Vec<OwnedEvent>`; `finish()` reconstructs the AST via
/// `events_to_doc` and calls `crate::emit::emit` -- a buffer-then-emit
/// architecture, not incremental streaming, per CLAUDE.md.
#[test]
fn mediawiki_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("mediawiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/mediawiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = mediawiki_fmt::parse(&input);
        let built = mediawiki_fmt::emit(&doc);

        let mut w = mediawiki_fmt::Writer::new(Vec::<u8>::new());
        for e in mediawiki_fmt::events(&input) {
            w.write_event(e.into_owned());
        }
        let streamed = String::from_utf8(w.finish()).expect("streaming writer output is UTF-8");

        checked += 1;
        if built != streamed && result.is_ok() {
            result = Err(format!(
                "streaming Writer diverged from emit() for fixture {}:\n  emit():  {built:?}\n  \
                 Writer(): {streamed:?}",
                path.display()
            ));
        }
    }
    assert!(
        checked > 20,
        "expected to check a substantial number of mediawiki fixtures, got {checked}"
    );

    if result.is_ok() {
        use mediawiki_fmt::OwnedEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = mediawiki_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text("Hello world".to_string().into()));
        w.write_event(OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/Text/EndParagraph sequence and before finish() -- \
                 mediawiki_fmt::writer::Writer buffers all events into a Vec<OwnedEvent> and \
                 only reconstructs the AST + calls emit() inside finish(), so it is not a \
                 genuine incremental streaming writer despite content round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("mediawiki", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// tikiwiki: events() vs parse(), StreamingParser vs events(), streaming
// writer vs build() -- all fully wired
// ---------------------------------------------------------------------------
//
// Same architecture and same narrower-Wired-claim caveat as mediawiki-fmt
// above: `tikiwiki::tikiwiki_events` (`EventIter::new`, events.rs) calls
// `crate::parse::parse(input)` then walks the tree with `emit_block`/
// `emit_inlines`, so this check validates the AST->event projection layer,
// not two independent parsers.
mod tikiwiki_events_check {
    use super::{assert_or_known_failure, find_input, fixtures_root};
    use std::borrow::Cow;
    use tikiwiki::ast::{Block, Inline, TikiwikiDoc};
    use tikiwiki::events::OwnedEvent;
    type Event = OwnedEvent;

    fn tw_ast_to_events(doc: &TikiwikiDoc) -> Vec<Event> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            tw_block_events(b, &mut out);
        }
        out
    }

    fn tw_block_events(b: &Block, out: &mut Vec<Event>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(Event::StartParagraph);
                tw_inline_events(inlines, out);
                out.push(Event::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(Event::StartHeading { level: *level });
                tw_inline_events(inlines, out);
                out.push(Event::EndHeading);
            }
            Block::CodeBlock {
                content, language, ..
            } => {
                out.push(Event::CodeBlock {
                    language: language.clone(),
                    content: Cow::Owned(content.clone()),
                });
            }
            Block::Blockquote { blocks, .. } => {
                out.push(Event::StartBlockquote);
                for b in blocks {
                    tw_block_events(b, out);
                }
                out.push(Event::EndBlockquote);
            }
            Block::List { ordered, items, .. } => {
                out.push(Event::StartList { ordered: *ordered });
                for item in items {
                    out.push(Event::StartListItem);
                    tw_inline_events(&item.inlines, out);
                    for child in &item.children {
                        tw_block_events(child, out);
                    }
                    out.push(Event::EndListItem);
                }
                out.push(Event::EndList);
            }
            Block::Table { rows, .. } => {
                out.push(Event::StartTable);
                for row in rows {
                    out.push(Event::StartTableRow {
                        is_header: row.is_header,
                    });
                    for cell in &row.cells {
                        out.push(Event::StartTableCell);
                        tw_inline_events(&cell.inlines, out);
                        out.push(Event::EndTableCell);
                    }
                    out.push(Event::EndTableRow);
                }
                out.push(Event::EndTable);
            }
            Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
        }
    }

    fn tw_inline_events(inlines: &[Inline], out: &mut Vec<Event>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(Event::Text(Cow::Owned(s.clone()))),
                Inline::Bold(c, _) => {
                    out.push(Event::StartBold);
                    tw_inline_events(c, out);
                    out.push(Event::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(Event::StartItalic);
                    tw_inline_events(c, out);
                    out.push(Event::EndItalic);
                }
                Inline::Underline(c, _) => {
                    out.push(Event::StartUnderline);
                    tw_inline_events(c, out);
                    out.push(Event::EndUnderline);
                }
                Inline::Strikethrough(c, _) => {
                    out.push(Event::StartStrikethrough);
                    tw_inline_events(c, out);
                    out.push(Event::EndStrikethrough);
                }
                Inline::Superscript(c, _) => {
                    out.push(Event::StartSuperscript);
                    tw_inline_events(c, out);
                    out.push(Event::EndSuperscript);
                }
                Inline::Subscript(c, _) => {
                    out.push(Event::StartSubscript);
                    tw_inline_events(c, out);
                    out.push(Event::EndSubscript);
                }
                Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
                Inline::Nowiki(s, _) => out.push(Event::Nowiki(Cow::Owned(s.clone()))),
                Inline::Link { url, children, .. } => {
                    out.push(Event::StartLink { url: url.clone() });
                    tw_inline_events(children, out);
                    out.push(Event::EndLink);
                }
                Inline::WikiLink { page, children, .. } => {
                    out.push(Event::StartWikiLink { page: page.clone() });
                    tw_inline_events(children, out);
                    out.push(Event::EndWikiLink);
                }
                Inline::Image { url, alt, .. } => out.push(Event::InlineImage {
                    url: url.clone(),
                    alt: alt.clone(),
                }),
                Inline::LineBreak { .. } => out.push(Event::LineBreak),
            }
        }
    }

    #[test]
    fn tikiwiki_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("tikiwiki");
        let mut checked = 0;
        let mut result: Result<(), String> = Ok(());
        for entry in std::fs::read_dir(&root).expect("fixtures/tikiwiki dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = tikiwiki::parse::parse(&input);
            let expected = tw_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> = tikiwiki::tikiwiki_events(&input)
                .map(|e| e.into_owned())
                .collect();
            checked += 1;
            if expected != actual && result.is_ok() {
                result = Err(format!(
                    "events() diverged from the AST projection for fixture {name}:\n  \
                     ast-derived: {expected:?}\n  events():    {actual:?}"
                ));
            }
        }
        assert!(
            checked > 15,
            "expected to check a substantial number of tikiwiki fixtures, got {checked}"
        );
        assert_or_known_failure("tikiwiki", "events", result);
    }
}

/// `StreamingParser` fed a tikiwiki fixture under an adversarial chunking
/// must deliver the same event sequence `events()` delivers over the whole
/// input. `tikiwiki::batch::StreamingParser::emit_block` re-parses each
/// accumulated block in isolation via `crate::events::events(&text)`.
#[test]
fn tikiwiki_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("tikiwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/tikiwiki dir") {
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
        let bulk: Vec<tikiwiki::OwnedEvent> = tikiwiki::tikiwiki_events(input_str)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                tikiwiki::StreamingParser::new(|e: tikiwiki::OwnedEvent| streamed.push(e));
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
        checked > 15,
        "expected to check a substantial number of tikiwiki fixtures, got {checked}"
    );
    assert_or_known_failure("tikiwiki", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(input)` must reproduce what
/// `build()` produces for the AST `parse(input)` returned, plus an
/// incrementality probe (`Writer::write_event` only pushes onto an internal
/// `Vec`; `finish()` reconstructs the AST and calls `crate::emit::build`).
#[test]
fn tikiwiki_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("tikiwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/tikiwiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = tikiwiki::parse(&input);
        let built = tikiwiki::build(&doc);

        let mut w = tikiwiki::Writer::new(Vec::<u8>::new());
        for e in tikiwiki::tikiwiki_events(&input) {
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
        checked > 15,
        "expected to check a substantial number of tikiwiki fixtures, got {checked}"
    );

    if result.is_ok() {
        use tikiwiki::OwnedEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = tikiwiki::Writer::new(ObservableSink(observed.clone()));
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text("Hello world".to_string().into()));
        w.write_event(OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/Text/EndParagraph sequence and before finish() -- \
                 tikiwiki::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
                 reconstructs the AST + calls build() inside finish(), so it is not a genuine \
                 incremental streaming writer despite content round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("tikiwiki", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// twiki: events() vs parse(), StreamingParser vs events(), streaming
// writer vs build() -- all fully wired
// ---------------------------------------------------------------------------
//
// twiki's `events()` (`twiki::events::events`) has a narrower signature than
// every other format checked in this file: `fn events(doc: &TwikiDoc) ->
// EventIter<'_>` takes an already-parsed AST, not raw input -- a caller must
// call `parse()` first. This is a real deviation from the vertical
// completion checklist's `events(input: &[u8]) -> impl Iterator<Item =
// Event>` contract (CLAUDE.md), tracked as a follow-up in TODO.md, not fixed
// here. It does not block wiring this check: `EventIter::new(doc)` still
// walks the tree with `emit_block`/`emit_inlines`, making real per-variant
// mapping decisions, so an independently-derived projection can and does
// diverge from the walk on a genuine bug (same narrower-Wired-claim caveat
// as mediawiki-fmt/tikiwiki above).
mod twiki_events_check {
    use super::{assert_or_known_failure, find_input, fixtures_root};
    use std::borrow::Cow;
    use twiki::ast::{Block, Inline, TwikiDoc};
    use twiki::events::OwnedEvent;
    type Event = OwnedEvent;

    fn tw_ast_to_events(doc: &TwikiDoc) -> Vec<Event> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            tw_block_events(b, &mut out);
        }
        out
    }

    fn tw_block_events(b: &Block, out: &mut Vec<Event>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(Event::StartParagraph);
                tw_inline_events(inlines, out);
                out.push(Event::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(Event::StartHeading { level: *level });
                tw_inline_events(inlines, out);
                out.push(Event::EndHeading);
            }
            Block::CodeBlock { content, .. } => {
                out.push(Event::CodeBlock {
                    content: Cow::Owned(content.clone()),
                });
            }
            Block::List { ordered, items, .. } => {
                out.push(Event::StartList { ordered: *ordered });
                for item in items {
                    out.push(Event::StartListItem);
                    tw_inline_events(&item.inlines, out);
                    for child in &item.children {
                        tw_block_events(child, out);
                    }
                    out.push(Event::EndListItem);
                }
                out.push(Event::EndList);
            }
            Block::Table { rows, .. } => {
                out.push(Event::StartTable);
                for row in rows {
                    out.push(Event::StartTableRow);
                    for cell in &row.cells {
                        out.push(Event::StartTableCell {
                            is_header: cell.is_header,
                        });
                        tw_inline_events(&cell.inlines, out);
                        out.push(Event::EndTableCell);
                    }
                    out.push(Event::EndTableRow);
                }
                out.push(Event::EndTable);
            }
            Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
            Block::RawBlock { content, .. } => {
                out.push(Event::RawBlock {
                    content: Cow::Owned(content.clone()),
                });
            }
            Block::DefinitionList { items, .. } => {
                out.push(Event::StartDefinitionList);
                for item in items {
                    out.push(Event::StartDefinitionTerm);
                    tw_inline_events(&item.term, out);
                    out.push(Event::EndDefinitionTerm);
                    out.push(Event::StartDefinitionDesc);
                    tw_inline_events(&item.desc, out);
                    out.push(Event::EndDefinitionDesc);
                }
                out.push(Event::EndDefinitionList);
            }
            Block::Blockquote { children, .. } => {
                out.push(Event::StartBlockquote);
                for child in children {
                    tw_block_events(child, out);
                }
                out.push(Event::EndBlockquote);
            }
        }
    }

    fn tw_inline_events(inlines: &[Inline], out: &mut Vec<Event>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(Event::Text(Cow::Owned(s.clone()))),
                Inline::Bold(c, _) => {
                    out.push(Event::StartBold);
                    tw_inline_events(c, out);
                    out.push(Event::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(Event::StartItalic);
                    tw_inline_events(c, out);
                    out.push(Event::EndItalic);
                }
                Inline::BoldItalic(c, _) => {
                    out.push(Event::StartBoldItalic);
                    tw_inline_events(c, out);
                    out.push(Event::EndBoldItalic);
                }
                Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
                Inline::BoldCode(c, _) => {
                    out.push(Event::StartBoldCode);
                    tw_inline_events(c, out);
                    out.push(Event::EndBoldCode);
                }
                Inline::Link { url, label, .. } => {
                    out.push(Event::StartLink { url: url.clone() });
                    out.push(Event::Text(Cow::Owned(label.clone())));
                    out.push(Event::EndLink);
                }
                Inline::LineBreak { .. } => out.push(Event::LineBreak),
                Inline::Strikethrough(c, _) => {
                    out.push(Event::StartStrikethrough);
                    tw_inline_events(c, out);
                    out.push(Event::EndStrikethrough);
                }
                Inline::Superscript(c, _) => {
                    out.push(Event::StartSuperscript);
                    tw_inline_events(c, out);
                    out.push(Event::EndSuperscript);
                }
                Inline::Subscript(c, _) => {
                    out.push(Event::StartSubscript);
                    tw_inline_events(c, out);
                    out.push(Event::EndSubscript);
                }
                Inline::Underline(c, _) => {
                    out.push(Event::StartUnderline);
                    tw_inline_events(c, out);
                    out.push(Event::EndUnderline);
                }
                Inline::Image { url, alt, .. } => out.push(Event::Image {
                    url: url.clone(),
                    alt: alt.clone(),
                }),
                Inline::RawInline { content, .. } => out.push(Event::RawInline {
                    content: content.clone(),
                }),
                Inline::WikiWord { word, .. } => out.push(Event::WikiWord { word: word.clone() }),
            }
        }
    }

    #[test]
    fn twiki_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("twiki");
        let mut checked = 0;
        let mut result: Result<(), String> = Ok(());
        for entry in std::fs::read_dir(&root).expect("fixtures/twiki dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = twiki::parse::parse(&input);
            let expected = tw_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> = twiki::events::events(&doc)
                .map(|e| e.into_owned())
                .collect();
            checked += 1;
            if expected != actual && result.is_ok() {
                result = Err(format!(
                    "events() diverged from the AST projection for fixture {name}:\n  \
                     ast-derived: {expected:?}\n  events():    {actual:?}"
                ));
            }
        }
        assert!(
            checked > 15,
            "expected to check a substantial number of twiki fixtures, got {checked}"
        );
        assert_or_known_failure("twiki", "events", result);
    }
}

/// `StreamingParser` fed a twiki fixture under an adversarial chunking must
/// deliver the same event sequence `events()` delivers over the whole input.
/// `twiki::batch::StreamingParser::emit_block` re-parses each accumulated
/// block in isolation via `crate::parse::parse(&text)` followed by
/// `crate::events::events(&doc)`.
#[test]
fn twiki_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("twiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/twiki dir") {
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
        let (bulk_doc, _) = twiki::parse::parse(input_str);
        let bulk: Vec<twiki::OwnedEvent> = twiki::events::events(&bulk_doc)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser = twiki::StreamingParser::new(|e: twiki::OwnedEvent| streamed.push(e));
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
        checked > 15,
        "expected to check a substantial number of twiki fixtures, got {checked}"
    );
    assert_or_known_failure("twiki", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(&doc)` must reproduce what
/// `build()` produces for the same AST, plus an incrementality probe.
#[test]
fn twiki_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("twiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/twiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = twiki::parse(&input);
        let built = twiki::build(&doc);

        let mut w = twiki::Writer::new(Vec::<u8>::new());
        for e in twiki::events::events(&doc) {
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
        checked > 15,
        "expected to check a substantial number of twiki fixtures, got {checked}"
    );

    if result.is_ok() {
        use twiki::OwnedEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = twiki::Writer::new(ObservableSink(observed.clone()));
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text("Hello world".to_string().into()));
        w.write_event(OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/Text/EndParagraph sequence and before finish() -- \
                 twiki::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
                 reconstructs the AST + calls build() inside finish(), so it is not a genuine \
                 incremental streaming writer despite content round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("twiki", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// vimwiki-fmt: events() vs parse(), StreamingParser vs events(), streaming
// writer vs build() -- all fully wired
// ---------------------------------------------------------------------------
//
// Same architecture and narrower-Wired-claim caveat as mediawiki-fmt/
// tikiwiki/twiki above: `vimwiki_fmt::events::EventIter::new` calls
// `crate::parse::parse(input)` then walks the tree with `emit_doc_events`/
// `emit_block_events`/`emit_inline_events`.
mod vimwiki_events_check {
    use super::{assert_or_known_failure, find_input, fixtures_root};
    use std::borrow::Cow;
    use vimwiki_fmt::ast::{Block, Inline, VimwikiDoc};
    use vimwiki_fmt::events::OwnedEvent;
    type Event = OwnedEvent;

    fn vw_ast_to_events(doc: &VimwikiDoc) -> Vec<Event> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            vw_block_events(b, &mut out);
        }
        out
    }

    fn vw_block_events(b: &Block, out: &mut Vec<Event>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(Event::StartParagraph);
                vw_inline_events(inlines, out);
                out.push(Event::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(Event::StartHeading { level: *level });
                vw_inline_events(inlines, out);
                out.push(Event::EndHeading);
            }
            Block::CodeBlock {
                language, content, ..
            } => {
                out.push(Event::CodeBlock {
                    language: language.clone(),
                    content: Cow::Owned(content.clone()),
                });
            }
            // events.rs's `emit_block_events` wraps a vimwiki blockquote's flat
            // `inlines` in a synthetic StartParagraph/EndParagraph pair inside
            // the blockquote (blockquotes hold inlines directly in the AST, not
            // a nested paragraph block) -- mirrored here, not simplified away,
            // since the projection must match what events() actually emits.
            Block::Blockquote { inlines, .. } => {
                out.push(Event::StartBlockquote);
                out.push(Event::StartParagraph);
                vw_inline_events(inlines, out);
                out.push(Event::EndParagraph);
                out.push(Event::EndBlockquote);
            }
            Block::List { ordered, items, .. } => {
                out.push(Event::StartList { ordered: *ordered });
                for item in items {
                    out.push(Event::StartListItem {
                        checked: item.checked,
                    });
                    vw_inline_events(&item.inlines, out);
                    out.push(Event::EndListItem);
                }
                out.push(Event::EndList);
            }
            Block::Table { rows, .. } => {
                out.push(Event::StartTable);
                for row in rows {
                    out.push(Event::StartTableRow);
                    for cell in &row.cells {
                        out.push(Event::StartTableCell);
                        vw_inline_events(cell, out);
                        out.push(Event::EndTableCell);
                    }
                    out.push(Event::EndTableRow);
                }
                out.push(Event::EndTable);
            }
            Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
            Block::DefinitionList { items, .. } => {
                out.push(Event::StartDefinitionList);
                for item in items {
                    out.push(Event::StartDefinitionTerm);
                    vw_inline_events(&item.term, out);
                    out.push(Event::EndDefinitionTerm);
                    out.push(Event::StartDefinitionDesc);
                    vw_inline_events(&item.desc, out);
                    out.push(Event::EndDefinitionDesc);
                }
                out.push(Event::EndDefinitionList);
            }
        }
    }

    fn vw_inline_events(inlines: &[Inline], out: &mut Vec<Event>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(Event::Text(Cow::Owned(s.clone()))),
                Inline::Bold(c, _) => {
                    out.push(Event::StartBold);
                    vw_inline_events(c, out);
                    out.push(Event::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(Event::StartItalic);
                    vw_inline_events(c, out);
                    out.push(Event::EndItalic);
                }
                Inline::Strikethrough(c, _) => {
                    out.push(Event::StartStrikethrough);
                    vw_inline_events(c, out);
                    out.push(Event::EndStrikethrough);
                }
                Inline::Superscript(c, _) => {
                    out.push(Event::StartSuperscript);
                    vw_inline_events(c, out);
                    out.push(Event::EndSuperscript);
                }
                Inline::Subscript(c, _) => {
                    out.push(Event::StartSubscript);
                    vw_inline_events(c, out);
                    out.push(Event::EndSubscript);
                }
                Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Owned(s.clone()))),
                Inline::Link { url, label, .. } => {
                    out.push(Event::StartLink { url: url.clone() });
                    out.push(Event::Text(Cow::Owned(label.clone())));
                    out.push(Event::EndLink);
                }
                Inline::Image {
                    url, alt, style, ..
                } => out.push(Event::InlineImage {
                    url: url.clone(),
                    alt: alt.clone(),
                    style: style.clone(),
                }),
            }
        }
    }

    #[test]
    fn vimwiki_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("vimwiki");
        let mut checked = 0;
        let mut result: Result<(), String> = Ok(());
        for entry in std::fs::read_dir(&root).expect("fixtures/vimwiki dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = vimwiki_fmt::parse::parse(&input);
            let expected = vw_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> = vimwiki_fmt::events(&input)
                .map(|e| e.into_owned())
                .collect();
            checked += 1;
            if expected != actual && result.is_ok() {
                result = Err(format!(
                    "events() diverged from the AST projection for fixture {name}:\n  \
                     ast-derived: {expected:?}\n  events():    {actual:?}"
                ));
            }
        }
        assert!(
            checked > 15,
            "expected to check a substantial number of vimwiki fixtures, got {checked}"
        );
        assert_or_known_failure("vimwiki", "events", result);
    }
}

/// `StreamingParser` fed a vimwiki fixture under an adversarial chunking must
/// deliver the same event sequence `events()` delivers over the whole input.
/// `vimwiki_fmt::batch::StreamingParser::emit_block` re-parses each
/// accumulated block in isolation via `crate::events::events(&text)`. The
/// crate's own `test_streaming_matches_bulk` (batch.rs) already exercises
/// one hand-picked heading+2-paragraph input under 7-byte chunking; this
/// generalizes that self-check to the full adversarial-chunking suite (whole
/// input, single-byte, 3/7/13-byte chunks, mid-UTF-8-char split) over every
/// `fixtures/vimwiki/` fixture.
#[test]
fn vimwiki_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("vimwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/vimwiki dir") {
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
        let bulk: Vec<vimwiki_fmt::OwnedEvent> = vimwiki_fmt::events(input_str)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                vimwiki_fmt::StreamingParser::new(|e: vimwiki_fmt::OwnedEvent| streamed.push(e));
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
        checked > 15,
        "expected to check a substantial number of vimwiki fixtures, got {checked}"
    );
    assert_or_known_failure("vimwiki", "streaming_parser", result);
}

/// The streaming `Writer` driven with `events(input)` must reproduce what
/// `build()` produces for the AST `parse(input)` returned, plus an
/// incrementality probe. `Writer::write_event` (writer.rs) only pushes onto
/// an internal `Vec<OwnedEvent>`; `finish()` calls
/// `crate::events::collect_doc_from_events` then `crate::emit::build`.
#[test]
fn vimwiki_streaming_writer_matches_builder_over_all_fixtures() {
    let root = fixtures_root().join("vimwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/vimwiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _) = vimwiki_fmt::parse(&input);
        let built = vimwiki_fmt::build(&doc);

        let mut w = vimwiki_fmt::Writer::new(Vec::<u8>::new());
        for e in vimwiki_fmt::events(&input) {
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
        checked > 15,
        "expected to check a substantial number of vimwiki fixtures, got {checked}"
    );

    if result.is_ok() {
        use vimwiki_fmt::OwnedEvent;
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = vimwiki_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(OwnedEvent::StartParagraph);
        w.write_event(OwnedEvent::Text("Hello world".to_string().into()));
        w.write_event(OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err("Writer wrote zero bytes to the sink after a complete \
                 StartParagraph/Text/EndParagraph sequence and before finish() -- \
                 vimwiki_fmt::writer::Writer buffers all events into a Vec<OwnedEvent> and only \
                 reconstructs the AST + calls build() inside finish(), so it is not a genuine \
                 incremental streaming writer despite content round-tripping correctly"
                .to_string());
        }
    }
    assert_or_known_failure("vimwiki", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// xwiki: events() is a genuine lazy pull-iterator over &XwikiDoc (unlike
// zimwiki/markua/muse-fmt below, which eagerly materialize a Vec/VecDeque of
// events before iteration begins). StreamingParser and Writer are both
// confirmed-fake buffer-then-finish wrappers.
// ---------------------------------------------------------------------------
//
// xwiki::events::events() takes `&XwikiDoc`, not `&str` — EventIter::next()
// (crates/formats/xwiki/src/events.rs:168-385) is a true frame-stack walker
// pulled on demand, so this check validates that walk directly against an
// independently hand-written projection.
mod xwiki_events_check {
    use super::{find_input, fixtures_root};
    use std::borrow::Cow;
    use xwiki::{Block, Event, Inline, XwikiDoc};

    /// Reconstruct the exact [`xwiki::Event`] sequence `events()` must produce
    /// for `doc`.
    ///
    /// One non-obvious mapping: `Inline::Link { url, label, .. }` stores `label`
    /// as a plain `String` (not nested inlines), but the event vocabulary only
    /// has `StartLink`/`EndLink` with no leaf "link text" event — confirmed by
    /// reading `EventIter::next()`'s `Inline::Link` arm (events.rs:361-368),
    /// which emits `StartLink`, queues a single `Text(label)` as `self.pending`,
    /// and closes with `EndLink`. The projection below mirrors that exactly.
    fn xwiki_ast_to_events(doc: &XwikiDoc) -> Vec<Event<'_>> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            xwiki_block_events(b, &mut out);
        }
        out
    }

    fn xwiki_block_events<'a>(b: &'a Block, out: &mut Vec<Event<'a>>) {
        match b {
            Block::Heading { level, inlines, .. } => {
                out.push(Event::StartHeading { level: *level });
                xwiki_inline_events(inlines, out);
                out.push(Event::EndHeading);
            }
            Block::Paragraph { inlines, .. } => {
                out.push(Event::StartParagraph);
                xwiki_inline_events(inlines, out);
                out.push(Event::EndParagraph);
            }
            Block::CodeBlock {
                content, language, ..
            } => out.push(Event::CodeBlock {
                language: language.clone(),
                content: Cow::Borrowed(content),
            }),
            Block::Table { rows, .. } => {
                out.push(Event::StartTable);
                for row in rows {
                    out.push(Event::StartTableRow);
                    for cell in &row.cells {
                        out.push(Event::StartTableCell {
                            is_header: cell.is_header,
                        });
                        xwiki_inline_events(&cell.inlines, out);
                        out.push(Event::EndTableCell);
                    }
                    out.push(Event::EndTableRow);
                }
                out.push(Event::EndTable);
            }
            Block::List { ordered, items, .. } => {
                out.push(Event::StartList { ordered: *ordered });
                for item in items {
                    out.push(Event::StartListItem);
                    for c in item {
                        xwiki_block_events(c, out);
                    }
                    out.push(Event::EndListItem);
                }
                out.push(Event::EndList);
            }
            Block::HorizontalRule { .. } => out.push(Event::HorizontalRule),
            Block::Blockquote { children, .. } => {
                out.push(Event::StartBlockquote);
                for c in children {
                    xwiki_block_events(c, out);
                }
                out.push(Event::EndBlockquote);
            }
            Block::MacroBlock {
                name,
                params,
                content,
                ..
            } => out.push(Event::MacroBlock {
                name: name.clone(),
                params: params.clone(),
                content: content.clone(),
            }),
            Block::MacroInline { name, params, .. } => out.push(Event::MacroInline {
                name: name.clone(),
                params: params.clone(),
            }),
        }
    }

    fn xwiki_inline_events<'a>(inlines: &'a [Inline], out: &mut Vec<Event<'a>>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(Event::Text(Cow::Borrowed(s))),
                Inline::Bold(c, _) => {
                    out.push(Event::StartBold);
                    xwiki_inline_events(c, out);
                    out.push(Event::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(Event::StartItalic);
                    xwiki_inline_events(c, out);
                    out.push(Event::EndItalic);
                }
                Inline::Underline(c, _) => {
                    out.push(Event::StartUnderline);
                    xwiki_inline_events(c, out);
                    out.push(Event::EndUnderline);
                }
                Inline::Strikeout(c, _) => {
                    out.push(Event::StartStrikeout);
                    xwiki_inline_events(c, out);
                    out.push(Event::EndStrikeout);
                }
                Inline::Superscript(c, _) => {
                    out.push(Event::StartSuperscript);
                    xwiki_inline_events(c, out);
                    out.push(Event::EndSuperscript);
                }
                Inline::Subscript(c, _) => {
                    out.push(Event::StartSubscript);
                    xwiki_inline_events(c, out);
                    out.push(Event::EndSubscript);
                }
                Inline::Code(s, _) => out.push(Event::InlineCode(Cow::Borrowed(s))),
                Inline::Link { url, label, .. } => {
                    out.push(Event::StartLink { url: url.clone() });
                    out.push(Event::Text(Cow::Borrowed(label)));
                    out.push(Event::EndLink);
                }
                Inline::Image {
                    url, alt, params, ..
                } => out.push(Event::InlineImage {
                    url: url.clone(),
                    alt: alt.clone(),
                    params: params.clone(),
                }),
                Inline::LineBreak { .. } => out.push(Event::LineBreak),
                Inline::SoftBreak { .. } => out.push(Event::SoftBreak),
            }
        }
    }

    #[test]
    fn xwiki_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("xwiki");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&root).expect("fixtures/xwiki dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = xwiki::parse(&input);
            let expected = xwiki_ast_to_events(&doc);
            let actual: Vec<_> = xwiki::events::events(&doc).collect();
            checked += 1;
            if expected != actual {
                let at = expected
                    .iter()
                    .zip(actual.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(expected.len().min(actual.len()));
                failures.push(format!(
                    "{}: first divergence at event #{at} (expected len {}, actual len {})",
                    path.file_name().unwrap().to_string_lossy(),
                    expected.len(),
                    actual.len(),
                ));
            }
        }
        assert!(
            checked > 20,
            "expected to check a substantial number of xwiki fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {}/{checked} xwiki fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

/// `xwiki::batch::StreamingParser::feed()` is a bare `buf.extend_from_slice`
/// (crates/formats/xwiki/src/batch.rs:61-63); all parsing happens in
/// `finish()` (batch.rs:66-72), which calls `parse::parse` then walks the
/// result with `events::events`. So the adversarial-chunking equivalence
/// check below is expected to pass trivially (finish() always reproduces
/// exactly what `events()` computes over the reassembled buffer) — the real
/// defect this check is built to catch is architectural, not a content
/// mismatch: does `feed()` deliver any events before `finish()` is called?
/// It does not.
#[test]
fn xwiki_streaming_parser_matches_events_and_is_incremental() {
    let root = fixtures_root().join("xwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/xwiki dir") {
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
        let (doc, _diags) = xwiki::parse(input_str);
        let bulk: Vec<xwiki::OwnedEvent> = xwiki::events::events(&doc)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                xwiki::batch::StreamingParser::new(|e: xwiki::OwnedEvent| streamed.push(e));
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
            let mut delivered: Vec<xwiki::OwnedEvent> = Vec::new();
            let mut parser = xwiki::batch::StreamingParser::new(|e| delivered.push(e));
            parser.feed(&input[..mid]);
            if delivered.is_empty() && result.is_ok() {
                result = Err(format!(
                    "StreamingParser delivered zero events to the handler after feed() with \
                     half of fixture {name} ({mid} bytes) and before finish() — \
                     xwiki::batch::StreamingParser buffers all input into a Vec<u8> \
                     (crates/formats/xwiki/src/batch.rs:61-63) and only parses and delivers \
                     events inside finish() (batch.rs:66-72), so feed() never advances real \
                     incremental parser state"
                ));
            }
            // `parser` intentionally dropped without calling finish(): this probe
            // only needs to observe pre-finish handler state.
        }
    }
    assert!(
        checked > 20,
        "expected to check a substantial number of xwiki fixtures, got {checked}"
    );
    assert_or_known_failure("xwiki", "streaming_parser", result);
}

/// `xwiki::writer::Writer::write_event()` only pushes to a `Vec`
/// (crates/formats/xwiki/src/writer.rs:39-41); `finish()` reconstructs the
/// AST via `collect_doc_from_events` and calls `emit::build` once
/// (writer.rs:44-49). Content-wise this round-trips correctly (checked
/// below), but an incrementality probe shows zero bytes reach the sink
/// before `finish()`.
#[test]
fn xwiki_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("xwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/xwiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = xwiki::parse(&input);
        let built = xwiki::build(&doc);

        let mut w = xwiki::Writer::new(Vec::<u8>::new());
        for e in xwiki::events::events(&doc) {
            w.write_event(e.into_owned());
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
        checked > 20,
        "expected to check a substantial number of xwiki fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = xwiki::Writer::new(ObservableSink(observed.clone()));
        w.write_event(xwiki::OwnedEvent::StartHeading { level: 1 });
        w.write_event(xwiki::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(xwiki::OwnedEvent::EndHeading);
        w.write_event(xwiki::OwnedEvent::StartParagraph);
        w.write_event(xwiki::OwnedEvent::Text("World".to_string().into()));
        w.write_event(xwiki::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after 6 complete write_event() calls (a \
                 full heading + paragraph) and before finish() — xwiki::writer::Writer buffers \
                 all events into a Vec<OwnedEvent> and only reconstructs the AST + calls \
                 emit::build() inside finish() (crates/formats/xwiki/src/writer.rs:39-49), so it \
                 is not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("xwiki", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// zimwiki: events() is parse()+eager-materialize-then-walk (EventIter::new
// calls parse::parse(input), then walks the resulting tree into a Vec before
// any event is returned — see events.rs:94-102) — a narrower claim than
// xwiki's genuinely lazy walker, in the same spirit as asciidoc's narrower
// "Wired" claim: the equivalence check validates the AST->event expansion
// layer (emit_block/emit_inline), not two independent parsers.
// StreamingParser here, unlike xwiki/muse-fmt, is REAL incremental: feed_line
// tracks verbatim-block boundaries and blank-line block termination and calls
// emit_block() during feed(), not deferred to finish() (batch.rs:93-152).
// ---------------------------------------------------------------------------
mod zimwiki_events_check {
    use super::{find_input, fixtures_root};
    use std::borrow::Cow;
    use zimwiki::{Block, Inline, OwnedEvent, ZimwikiDoc};

    /// Reconstruct the exact [`zimwiki::OwnedEvent`] sequence `events()` must
    /// produce for `doc`.
    fn zimwiki_ast_to_events(doc: &ZimwikiDoc) -> Vec<OwnedEvent> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            zimwiki_block_events(b, &mut out);
        }
        out
    }

    fn zimwiki_block_events(b: &Block, out: &mut Vec<OwnedEvent>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(OwnedEvent::StartParagraph);
                zimwiki_inline_events(inlines, out);
                out.push(OwnedEvent::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(OwnedEvent::StartHeading { level: *level });
                zimwiki_inline_events(inlines, out);
                out.push(OwnedEvent::EndHeading);
            }
            Block::CodeBlock { content, .. } => out.push(OwnedEvent::CodeBlock {
                content: Cow::Owned(content.clone()),
            }),
            Block::Blockquote { children, .. } => {
                out.push(OwnedEvent::StartBlockquote);
                for c in children {
                    zimwiki_block_events(c, out);
                }
                out.push(OwnedEvent::EndBlockquote);
            }
            Block::List { ordered, items, .. } => {
                out.push(OwnedEvent::StartList { ordered: *ordered });
                for item in items {
                    out.push(OwnedEvent::StartListItem {
                        checked: item.checked,
                    });
                    for c in &item.children {
                        zimwiki_block_events(c, out);
                    }
                    out.push(OwnedEvent::EndListItem);
                }
                out.push(OwnedEvent::EndList);
            }
            Block::Table { rows, .. } => {
                out.push(OwnedEvent::StartTable);
                for row in rows {
                    out.push(OwnedEvent::StartTableRow);
                    for cell in &row.cells {
                        out.push(OwnedEvent::StartTableCell);
                        zimwiki_inline_events(cell, out);
                        out.push(OwnedEvent::EndTableCell);
                    }
                    out.push(OwnedEvent::EndTableRow);
                }
                out.push(OwnedEvent::EndTable);
            }
            Block::HorizontalRule { .. } => out.push(OwnedEvent::HorizontalRule),
        }
    }

    fn zimwiki_inline_events(inlines: &[Inline], out: &mut Vec<OwnedEvent>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(OwnedEvent::Text(Cow::Owned(s.clone()))),
                Inline::Bold(c, _) => {
                    out.push(OwnedEvent::StartBold);
                    zimwiki_inline_events(c, out);
                    out.push(OwnedEvent::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(OwnedEvent::StartItalic);
                    zimwiki_inline_events(c, out);
                    out.push(OwnedEvent::EndItalic);
                }
                Inline::Underline(c, _) => {
                    out.push(OwnedEvent::StartUnderline);
                    zimwiki_inline_events(c, out);
                    out.push(OwnedEvent::EndUnderline);
                }
                Inline::Strikethrough(c, _) => {
                    out.push(OwnedEvent::StartStrikethrough);
                    zimwiki_inline_events(c, out);
                    out.push(OwnedEvent::EndStrikethrough);
                }
                Inline::Subscript(c, _) => {
                    out.push(OwnedEvent::StartSubscript);
                    zimwiki_inline_events(c, out);
                    out.push(OwnedEvent::EndSubscript);
                }
                Inline::Superscript(c, _) => {
                    out.push(OwnedEvent::StartSuperscript);
                    zimwiki_inline_events(c, out);
                    out.push(OwnedEvent::EndSuperscript);
                }
                Inline::Code(s, _) => out.push(OwnedEvent::InlineCode(Cow::Owned(s.clone()))),
                Inline::Link { url, children, .. } => {
                    out.push(OwnedEvent::StartLink { url: url.clone() });
                    zimwiki_inline_events(children, out);
                    out.push(OwnedEvent::EndLink);
                }
                Inline::Image { url, .. } => out.push(OwnedEvent::InlineImage { url: url.clone() }),
                Inline::LineBreak { .. } => out.push(OwnedEvent::LineBreak),
                Inline::SoftBreak { .. } => out.push(OwnedEvent::SoftBreak),
            }
        }
    }

    #[test]
    fn zimwiki_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("zimwiki");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&root).expect("fixtures/zimwiki dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = zimwiki::parse(&input);
            let expected = zimwiki_ast_to_events(&doc);
            let actual: Vec<OwnedEvent> = zimwiki::events(&input).collect();
            checked += 1;
            if expected != actual {
                let at = expected
                    .iter()
                    .zip(actual.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(expected.len().min(actual.len()));
                failures.push(format!(
                    "{}: first divergence at event #{at} (expected len {}, actual len {})",
                    path.file_name().unwrap().to_string_lossy(),
                    expected.len(),
                    actual.len(),
                ));
            }
        }
        assert!(
            checked > 20,
            "expected to check a substantial number of zimwiki fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {}/{checked} zimwiki fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

/// `StreamingParser` fed a zimwiki fixture under an adversarial chunking must
/// deliver the same event sequence `events()` delivers over the whole input.
/// Unlike xwiki/muse-fmt, `zimwiki::batch::StreamingParser::feed()` really is
/// incremental — it tracks verbatim-block (`'''`) boundaries and blank-line
/// block termination line-by-line and calls `emit_block()` during `feed()`
/// (batch.rs:93-152) — so divergences found here are genuine block-boundary
/// bugs, the same bug class already tracked for org/rst/asciidoc
/// (`emit_block()` re-parses each accumulated block in isolation via
/// `crate::events::events()`, so cross-block context such as a loose list's
/// blank-line-separated items is lost).
#[test]
fn zimwiki_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("zimwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/zimwiki dir") {
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
        let bulk: Vec<zimwiki::OwnedEvent> = zimwiki::events(input_str).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                zimwiki::batch::StreamingParser::new(|e: zimwiki::OwnedEvent| streamed.push(e));
            for chunk in chunks {
                parser.feed(&chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}:\n  events():         {bulk:?}\n  StreamingParser: \
                     {streamed:?}"
                ));
            }
        }
    }
    assert!(
        checked > 20,
        "expected to check a substantial number of zimwiki fixtures, got {checked}"
    );
    assert_or_known_failure("zimwiki", "streaming_parser", result);
}

/// `zimwiki::writer::Writer::write_event()` only pushes to a `Vec`
/// (crates/formats/zimwiki/src/writer.rs:24-26); `finish()` reconstructs the
/// AST via `collect_doc_from_events` and calls `emit::build` once
/// (writer.rs:29-34). Content round-trips correctly (checked below), but an
/// incrementality probe shows zero bytes reach the sink before `finish()`.
#[test]
fn zimwiki_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("zimwiki");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/zimwiki dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = zimwiki::parse(&input);
        let built = zimwiki::build(&doc);

        let mut w = zimwiki::Writer::new(Vec::<u8>::new());
        for e in zimwiki::events(&input) {
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
        checked > 20,
        "expected to check a substantial number of zimwiki fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = zimwiki::Writer::new(ObservableSink(observed.clone()));
        w.write_event(zimwiki::OwnedEvent::StartHeading { level: 1 });
        w.write_event(zimwiki::OwnedEvent::Text("Hello".to_string().into()));
        w.write_event(zimwiki::OwnedEvent::EndHeading);
        w.write_event(zimwiki::OwnedEvent::StartParagraph);
        w.write_event(zimwiki::OwnedEvent::Text("World".to_string().into()));
        w.write_event(zimwiki::OwnedEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after 6 complete write_event() calls (a \
                 full heading + paragraph) and before finish() — zimwiki::writer::Writer buffers \
                 all events into a Vec<OwnedEvent> and only reconstructs the AST + calls \
                 emit::build() inside finish() (crates/formats/zimwiki/src/writer.rs:24-34), so \
                 it is not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("zimwiki", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// markua: events() is parse()+eager-tree-build-then-walk. `EventIter::new`
// (re-exported from parse.rs, not events.rs) runs the full recursive-descent
// `Parser::parse()` before any event is returned (parse.rs:969-985); only the
// subsequent `Iterator::next()` pull over `expand_block`/`expand_inline` is
// lazy. So this is the same narrower "Wired" claim as asciidoc/zimwiki: the
// check validates the AST->event expansion layer.
// ---------------------------------------------------------------------------
mod markua_events_check {
    use super::{find_input, fixtures_root};
    use markua::{Block, Inline, MarkuaDoc, OwnedMarkuaEvent};
    use std::borrow::Cow;

    /// Reconstruct the exact [`markua::OwnedMarkuaEvent`] sequence `events()`
    /// must produce for `doc`.
    ///
    /// `Block::Figure`/`Inline::FootnoteRef` etc. are handled for
    /// completeness (an exhaustive match, so a new AST variant breaks the
    /// build), but `Block::Figure` is never constructed by `parse()` —
    /// confirmed by reading `crates/formats/markua/src/parse.rs` and
    /// `emit.rs`, neither of which has any code path building a `Figure`
    /// from Markua syntax — so it is unreachable via any fixture in this
    /// check and its ordering (caption before body, settled from
    /// `expand_block`'s `Block::Figure` arm, parse.rs:1071-1082) is untested
    /// here.
    fn markua_ast_to_events(doc: &MarkuaDoc) -> Vec<OwnedMarkuaEvent> {
        let mut out = Vec::new();
        for b in &doc.blocks {
            markua_block_events(b, &mut out);
        }
        out
    }

    fn markua_block_events(b: &Block, out: &mut Vec<OwnedMarkuaEvent>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(OwnedMarkuaEvent::StartParagraph);
                markua_inline_events(inlines, out);
                out.push(OwnedMarkuaEvent::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(OwnedMarkuaEvent::StartHeading { level: *level });
                markua_inline_events(inlines, out);
                out.push(OwnedMarkuaEvent::EndHeading);
            }
            Block::CodeBlock {
                content, language, ..
            } => out.push(OwnedMarkuaEvent::CodeBlock {
                language: language.clone(),
                content: Cow::Owned(content.clone()),
            }),
            Block::Blockquote { children, .. } => {
                out.push(OwnedMarkuaEvent::StartBlockquote);
                for c in children {
                    markua_block_events(c, out);
                }
                out.push(OwnedMarkuaEvent::EndBlockquote);
            }
            Block::List { ordered, items, .. } => {
                out.push(OwnedMarkuaEvent::StartList { ordered: *ordered });
                for item in items {
                    out.push(OwnedMarkuaEvent::StartListItem);
                    for c in item {
                        markua_block_events(c, out);
                    }
                    out.push(OwnedMarkuaEvent::EndListItem);
                }
                out.push(OwnedMarkuaEvent::EndList);
            }
            Block::Table { rows, .. } => {
                out.push(OwnedMarkuaEvent::StartTable);
                for row in rows {
                    out.push(OwnedMarkuaEvent::StartTableRow);
                    for cell in &row.cells {
                        out.push(OwnedMarkuaEvent::StartTableCell);
                        markua_inline_events(cell, out);
                        out.push(OwnedMarkuaEvent::EndTableCell);
                    }
                    out.push(OwnedMarkuaEvent::EndTableRow);
                }
                out.push(OwnedMarkuaEvent::EndTable);
            }
            Block::HorizontalRule { .. } => out.push(OwnedMarkuaEvent::HorizontalRule),
            Block::SpecialBlock {
                block_type,
                children,
                ..
            } => {
                out.push(OwnedMarkuaEvent::StartSpecialBlock {
                    kind: block_type.clone(),
                });
                for c in children {
                    markua_block_events(c, out);
                }
                out.push(OwnedMarkuaEvent::EndSpecialBlock);
            }
            Block::DefinitionList { items, .. } => {
                out.push(OwnedMarkuaEvent::StartDefinitionList);
                for (term, desc) in items {
                    out.push(OwnedMarkuaEvent::StartDefinitionTerm);
                    markua_inline_events(term, out);
                    out.push(OwnedMarkuaEvent::EndDefinitionTerm);
                    out.push(OwnedMarkuaEvent::StartDefinitionDesc);
                    for b in desc {
                        markua_block_events(b, out);
                    }
                    out.push(OwnedMarkuaEvent::EndDefinitionDesc);
                }
                out.push(OwnedMarkuaEvent::EndDefinitionList);
            }
            Block::PageBreak { .. } => out.push(OwnedMarkuaEvent::PageBreak),
            Block::Figure { caption, body, .. } => {
                out.push(OwnedMarkuaEvent::StartFigure);
                if !caption.is_empty() {
                    out.push(OwnedMarkuaEvent::StartCaption);
                    markua_inline_events(caption, out);
                    out.push(OwnedMarkuaEvent::EndCaption);
                }
                markua_block_events(body, out);
                out.push(OwnedMarkuaEvent::EndFigure);
            }
        }
    }

    fn markua_inline_events(inlines: &[Inline], out: &mut Vec<OwnedMarkuaEvent>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(OwnedMarkuaEvent::Text(Cow::Owned(s.clone()))),
                Inline::Strong(c, _) => {
                    out.push(OwnedMarkuaEvent::StartStrong);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndStrong);
                }
                Inline::Emphasis(c, _) => {
                    out.push(OwnedMarkuaEvent::StartEmphasis);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndEmphasis);
                }
                Inline::Strikethrough(c, _) => {
                    out.push(OwnedMarkuaEvent::StartStrikethrough);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndStrikethrough);
                }
                Inline::Subscript(c, _) => {
                    out.push(OwnedMarkuaEvent::StartSubscript);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndSubscript);
                }
                Inline::Superscript(c, _) => {
                    out.push(OwnedMarkuaEvent::StartSuperscript);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndSuperscript);
                }
                Inline::Underline(c, _) => {
                    out.push(OwnedMarkuaEvent::StartUnderline);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndUnderline);
                }
                Inline::SmallCaps(c, _) => {
                    out.push(OwnedMarkuaEvent::StartSmallCaps);
                    markua_inline_events(c, out);
                    out.push(OwnedMarkuaEvent::EndSmallCaps);
                }
                Inline::Code(s, _) => out.push(OwnedMarkuaEvent::InlineCode(Cow::Owned(s.clone()))),
                Inline::Link { url, children, .. } => {
                    out.push(OwnedMarkuaEvent::StartLink { url: url.clone() });
                    markua_inline_events(children, out);
                    out.push(OwnedMarkuaEvent::EndLink);
                }
                Inline::Image { url, alt, .. } => out.push(OwnedMarkuaEvent::Image {
                    url: url.clone(),
                    alt: alt.clone(),
                }),
                Inline::LineBreak(_) => out.push(OwnedMarkuaEvent::LineBreak),
                Inline::SoftBreak(_) => out.push(OwnedMarkuaEvent::SoftBreak),
                Inline::FootnoteRef { content, .. } => {
                    out.push(OwnedMarkuaEvent::StartFootnoteRef);
                    markua_inline_events(content, out);
                    out.push(OwnedMarkuaEvent::EndFootnoteRef);
                }
                Inline::IndexTerm { term, .. } => {
                    out.push(OwnedMarkuaEvent::IndexTerm { term: term.clone() })
                }
                Inline::MathInline { content, .. } => out.push(OwnedMarkuaEvent::MathInline {
                    content: content.clone(),
                }),
            }
        }
    }

    #[test]
    fn markua_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("markua");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&root).expect("fixtures/markua dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = markua::parse(&input);
            let expected = markua_ast_to_events(&doc);
            let actual: Vec<OwnedMarkuaEvent> =
                markua::events(&input).map(|e| e.into_owned()).collect();
            checked += 1;
            if expected != actual {
                let at = expected
                    .iter()
                    .zip(actual.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(expected.len().min(actual.len()));
                failures.push(format!(
                    "{}: first divergence at event #{at} (expected len {}, actual len {})",
                    path.file_name().unwrap().to_string_lossy(),
                    expected.len(),
                    actual.len(),
                ));
            }
        }
        assert!(
            checked > 20,
            "expected to check a substantial number of markua fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {}/{checked} markua fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

/// `StreamingParser` fed a markua fixture under an adversarial chunking must
/// deliver the same event sequence `events()` delivers over the whole input.
/// `markua::batch::StreamingParser::feed()` is REAL incremental
/// block-boundary segmentation (fenced-code-aware `feed_line`, batch.rs:108-
/// 152), unlike xwiki/muse-fmt — `emit_block()` re-parses each accumulated
/// block via `crate::events::events()`, the same architecture (and bug
/// class) already tracked for org/rst/asciidoc/zimwiki.
#[test]
fn markua_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("markua");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/markua dir") {
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
        let bulk: Vec<markua::OwnedMarkuaEvent> =
            markua::events(input_str).map(|e| e.into_owned()).collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                markua::batch::StreamingParser::new(|e: markua::OwnedMarkuaEvent| streamed.push(e));
            for chunk in chunks {
                parser.feed(&chunk);
            }
            parser.finish();
            if bulk != streamed && result.is_ok() {
                result = Err(format!(
                    "StreamingParser diverged from events() for fixture {name} under chunking \
                     {chunking_name}:\n  events():         {bulk:?}\n  StreamingParser: \
                     {streamed:?}"
                ));
            }
        }
    }
    assert!(
        checked > 20,
        "expected to check a substantial number of markua fixtures, got {checked}"
    );
    assert_or_known_failure("markua", "streaming_parser", result);
}

/// `markua::writer::Writer::write_event()` only pushes to a `Vec`
/// (crates/formats/markua/src/writer.rs:40-42); `finish()` reconstructs the
/// AST via `events_to_doc`/`DocBuilder` and calls `emit::emit` once
/// (writer.rs:45-50). Content round-trips correctly for every fixture
/// (checked below — `MarkuaDoc::title`/`author`/`description` are always
/// `None` regardless of path: `parse()` itself never populates them from any
/// Markua syntax, confirmed by reading `parse.rs`'s `pub fn parse`, which
/// hardcodes `title: None, author: None, description: None` unconditionally
/// — so the `DocBuilder::finish` hardcoding the same is not a reachable
/// divergence via any fixture). An incrementality probe shows zero bytes
/// reach the sink before `finish()`.
#[test]
fn markua_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("markua");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/markua dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = markua::parse(&input);
        let built = markua::build(&doc);

        let mut w = markua::Writer::new(Vec::<u8>::new());
        for e in markua::events(&input) {
            w.write_event(e.into_owned());
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
        checked > 20,
        "expected to check a substantial number of markua fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = markua::Writer::new(ObservableSink(observed.clone()));
        w.write_event(markua::OwnedMarkuaEvent::StartHeading { level: 1 });
        w.write_event(markua::OwnedMarkuaEvent::Text("Hello".to_string().into()));
        w.write_event(markua::OwnedMarkuaEvent::EndHeading);
        w.write_event(markua::OwnedMarkuaEvent::StartParagraph);
        w.write_event(markua::OwnedMarkuaEvent::Text("World".to_string().into()));
        w.write_event(markua::OwnedMarkuaEvent::EndParagraph);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after 6 complete write_event() calls (a \
                 full heading + paragraph) and before finish() — markua::writer::Writer buffers \
                 all events into a Vec<OwnedMarkuaEvent> and only reconstructs the AST + calls \
                 emit::emit() inside finish() (crates/formats/markua/src/writer.rs:40-50), so it \
                 is not a genuine incremental streaming writer despite content round-tripping \
                 correctly"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("markua", "streaming_writer", result);
}

// ---------------------------------------------------------------------------
// muse: events() takes &MuseDoc (like xwiki), but eagerly materializes a
// VecDeque in EventIter::new (events.rs:211-220) rather than pulling lazily —
// still a real, independently hand-checkable walk. StreamingParser and Writer
// are both confirmed-fake buffer-then-finish wrappers, and the crate's own
// module docs for StreamingParser admit it outright ("Muse's block-level
// structure makes true incremental parsing difficult without a dedicated
// state machine", batch.rs:11-13).
// ---------------------------------------------------------------------------
mod muse_events_check {
    use super::{find_input, fixtures_root};
    use muse_fmt::{Block, Inline, MuseDoc, OwnedMuseEvent};
    use std::borrow::Cow;

    /// Reconstruct the exact [`muse_fmt::OwnedMuseEvent`] sequence `events()`
    /// must produce for `doc`, including the `StartDocument`/`EndDocument`
    /// wrapper pair `EventIter::new` always emits (events.rs:213-219).
    fn muse_ast_to_events(doc: &MuseDoc) -> Vec<OwnedMuseEvent> {
        let mut out = vec![OwnedMuseEvent::StartDocument];
        for b in &doc.blocks {
            muse_block_events(b, &mut out);
        }
        out.push(OwnedMuseEvent::EndDocument);
        out
    }

    fn muse_block_events(b: &Block, out: &mut Vec<OwnedMuseEvent>) {
        match b {
            Block::Paragraph { inlines, .. } => {
                out.push(OwnedMuseEvent::StartParagraph);
                muse_inline_events(inlines, out);
                out.push(OwnedMuseEvent::EndParagraph);
            }
            Block::Heading { level, inlines, .. } => {
                out.push(OwnedMuseEvent::StartHeading { level: *level });
                muse_inline_events(inlines, out);
                out.push(OwnedMuseEvent::EndHeading);
            }
            Block::CodeBlock { content, .. } => out.push(OwnedMuseEvent::CodeBlock {
                content: Cow::Owned(content.clone()),
            }),
            Block::Blockquote { children, .. } => {
                out.push(OwnedMuseEvent::StartBlockquote);
                for c in children {
                    muse_block_events(c, out);
                }
                out.push(OwnedMuseEvent::EndBlockquote);
            }
            Block::List { ordered, items, .. } => {
                out.push(OwnedMuseEvent::StartList { ordered: *ordered });
                for item in items {
                    out.push(OwnedMuseEvent::StartListItem);
                    for c in item {
                        muse_block_events(c, out);
                    }
                    out.push(OwnedMuseEvent::EndListItem);
                }
                out.push(OwnedMuseEvent::EndList);
            }
            Block::DefinitionList { items, .. } => {
                out.push(OwnedMuseEvent::StartDefinitionList);
                for (term, desc) in items {
                    out.push(OwnedMuseEvent::StartDefinitionTerm);
                    muse_inline_events(term, out);
                    out.push(OwnedMuseEvent::EndDefinitionTerm);
                    out.push(OwnedMuseEvent::StartDefinitionDesc);
                    for b in desc {
                        muse_block_events(b, out);
                    }
                    out.push(OwnedMuseEvent::EndDefinitionDesc);
                }
                out.push(OwnedMuseEvent::EndDefinitionList);
            }
            Block::HorizontalRule { .. } => out.push(OwnedMuseEvent::HorizontalRule),
            Block::Verse { children, .. } => {
                out.push(OwnedMuseEvent::StartVerse);
                for c in children {
                    muse_block_events(c, out);
                }
                out.push(OwnedMuseEvent::EndVerse);
            }
            Block::CenteredBlock { children, .. } => {
                out.push(OwnedMuseEvent::StartCenteredBlock);
                for c in children {
                    muse_block_events(c, out);
                }
                out.push(OwnedMuseEvent::EndCenteredBlock);
            }
            Block::RightBlock { children, .. } => {
                out.push(OwnedMuseEvent::StartRightBlock);
                for c in children {
                    muse_block_events(c, out);
                }
                out.push(OwnedMuseEvent::EndRightBlock);
            }
            Block::LiteralBlock { content, .. } => out.push(OwnedMuseEvent::LiteralBlock {
                content: Cow::Owned(content.clone()),
            }),
            Block::SrcBlock { lang, content, .. } => out.push(OwnedMuseEvent::SrcBlock {
                lang: lang.clone().map(Cow::Owned),
                content: Cow::Owned(content.clone()),
            }),
            Block::Comment { content, .. } => out.push(OwnedMuseEvent::Comment {
                content: Cow::Owned(content.clone()),
            }),
            Block::Table { rows, .. } => {
                out.push(OwnedMuseEvent::StartTable);
                for row in rows {
                    out.push(OwnedMuseEvent::StartTableRow { header: row.header });
                    for cell in &row.cells {
                        out.push(OwnedMuseEvent::StartTableCell);
                        muse_inline_events(cell, out);
                        out.push(OwnedMuseEvent::EndTableCell);
                    }
                    out.push(OwnedMuseEvent::EndTableRow);
                }
                out.push(OwnedMuseEvent::EndTable);
            }
            Block::FootnoteDef { label, content, .. } => {
                out.push(OwnedMuseEvent::StartFootnoteDef {
                    label: Cow::Owned(label.clone()),
                });
                muse_inline_events(content, out);
                out.push(OwnedMuseEvent::EndFootnoteDef);
            }
        }
    }

    fn muse_inline_events(inlines: &[Inline], out: &mut Vec<OwnedMuseEvent>) {
        for i in inlines {
            match i {
                Inline::Text(s, _) => out.push(OwnedMuseEvent::Text(Cow::Owned(s.clone()))),
                Inline::Bold(c, _) => {
                    out.push(OwnedMuseEvent::StartBold);
                    muse_inline_events(c, out);
                    out.push(OwnedMuseEvent::EndBold);
                }
                Inline::Italic(c, _) => {
                    out.push(OwnedMuseEvent::StartItalic);
                    muse_inline_events(c, out);
                    out.push(OwnedMuseEvent::EndItalic);
                }
                Inline::Code(s, _) => out.push(OwnedMuseEvent::Code(Cow::Owned(s.clone()))),
                Inline::Link { url, children, .. } => {
                    out.push(OwnedMuseEvent::StartLink {
                        url: Cow::Owned(url.clone()),
                    });
                    muse_inline_events(children, out);
                    out.push(OwnedMuseEvent::EndLink);
                }
                Inline::Underline(c, _) => {
                    out.push(OwnedMuseEvent::StartUnderline);
                    muse_inline_events(c, out);
                    out.push(OwnedMuseEvent::EndUnderline);
                }
                Inline::Strikethrough(c, _) => {
                    out.push(OwnedMuseEvent::StartStrikethrough);
                    muse_inline_events(c, out);
                    out.push(OwnedMuseEvent::EndStrikethrough);
                }
                Inline::Superscript(c, _) => {
                    out.push(OwnedMuseEvent::StartSuperscript);
                    muse_inline_events(c, out);
                    out.push(OwnedMuseEvent::EndSuperscript);
                }
                Inline::Subscript(c, _) => {
                    out.push(OwnedMuseEvent::StartSubscript);
                    muse_inline_events(c, out);
                    out.push(OwnedMuseEvent::EndSubscript);
                }
                Inline::FootnoteRef { label, .. } => out.push(OwnedMuseEvent::FootnoteRef {
                    label: Cow::Owned(label.clone()),
                }),
                Inline::LineBreak(_) => out.push(OwnedMuseEvent::LineBreak),
                Inline::Anchor { name, .. } => out.push(OwnedMuseEvent::Anchor {
                    name: Cow::Owned(name.clone()),
                }),
                Inline::Image { src, alt, .. } => out.push(OwnedMuseEvent::Image {
                    src: Cow::Owned(src.clone()),
                    alt: alt.clone().map(Cow::Owned),
                }),
            }
        }
    }

    #[test]
    fn muse_events_equals_ast_projection_over_all_fixtures() {
        let root = fixtures_root().join("muse");
        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&root).expect("fixtures/muse dir") {
            let path = entry.unwrap().path();
            if !path.is_dir() {
                continue;
            }
            let Some(input_path) = find_input(&path) else {
                continue;
            };
            let input = std::fs::read_to_string(&input_path).expect("read fixture input");
            let (doc, _diags) = muse_fmt::parse(&input);
            let expected = muse_ast_to_events(&doc);
            let actual: Vec<OwnedMuseEvent> = muse_fmt::events::events(&doc)
                .map(|e| e.into_owned())
                .collect();
            checked += 1;
            if expected != actual {
                let at = expected
                    .iter()
                    .zip(actual.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or(expected.len().min(actual.len()));
                failures.push(format!(
                    "{}: first divergence at event #{at} (expected len {}, actual len {})",
                    path.file_name().unwrap().to_string_lossy(),
                    expected.len(),
                    actual.len(),
                ));
            }
        }
        assert!(
            checked > 20,
            "expected to check a substantial number of muse fixtures, got {checked}"
        );
        assert!(
            failures.is_empty(),
            "events() diverged from the AST projection for {}/{checked} muse fixtures:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
    }
}

/// `muse_fmt::batch::StreamingParser::feed()` is a bare `buf.extend_from_slice`
/// (crates/formats/muse-fmt/src/batch.rs:94-96); all parsing happens in
/// `finish()` (batch.rs:98-105), which calls `parse::parse` then walks the
/// result with `events::events`. The crate's own module docs admit this
/// outright ("Muse's block-level structure makes true incremental parsing
/// difficult without a dedicated state machine", batch.rs:11-13). So the
/// content-equivalence half of this check is expected to pass trivially; the
/// incrementality probe is the real test.
#[test]
fn muse_streaming_parser_matches_events_and_is_incremental() {
    let root = fixtures_root().join("muse");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/muse dir") {
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
        let (doc, _diags) = muse_fmt::parse(input_str);
        let bulk: Vec<muse_fmt::OwnedMuseEvent> = muse_fmt::events::events(&doc)
            .map(|e| e.into_owned())
            .collect();
        checked += 1;

        for (chunking_name, chunks) in adversarial_chunkings(&input) {
            let mut streamed = Vec::new();
            let mut parser =
                muse_fmt::batch::StreamingParser::new(|e: muse_fmt::OwnedMuseEvent| {
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
            let mut delivered: Vec<muse_fmt::OwnedMuseEvent> = Vec::new();
            let mut parser = muse_fmt::batch::StreamingParser::new(|e| delivered.push(e));
            parser.feed(&input[..mid]);
            if delivered.is_empty() && result.is_ok() {
                result = Err(format!(
                    "StreamingParser delivered zero events to the handler after feed() with \
                     half of fixture {name} ({mid} bytes) and before finish() — \
                     muse_fmt::batch::StreamingParser buffers all input into a Vec<u8> \
                     (crates/formats/muse-fmt/src/batch.rs:94-96) and only parses and delivers \
                     events inside finish() (batch.rs:98-105), so feed() never advances real \
                     incremental parser state"
                ));
            }
            // `parser` intentionally dropped without calling finish(): this probe
            // only needs to observe pre-finish handler state.
        }
    }
    assert!(
        checked > 20,
        "expected to check a substantial number of muse fixtures, got {checked}"
    );
    assert_or_known_failure("muse", "streaming_parser", result);
}

/// `muse_fmt::writer::Writer::write_event()` only pushes to a `Vec`
/// (crates/formats/muse-fmt/src/writer.rs:42-44); `finish()` reconstructs the
/// AST via `events_to_doc`/`DocBuilder` and calls `emit::build` once
/// (writer.rs:47-52). Unlike xwiki/zimwiki/markua, this is NOT purely an
/// architectural finding: `DocBuilder::finish` builds `MuseDoc { blocks,
/// span: Span::NONE, ..Default::default() }` (writer.rs:494-504), so
/// `title`/`author`/`date`/`description`/`keywords` always come back `None`
/// — and unlike markua, muse-fmt's `parse()` genuinely does populate these
/// fields from `#title`/`#author`/`#date`/`#desc`/`#keywords` directives
/// (parse.rs:240-249), reachable via the `document-header` fixture. The
/// `MuseEvent` enum has no variant carrying document metadata at all
/// (confirmed by reading the full enum, events.rs:27-114), so this is the
/// same expressiveness-gap bug class already tracked for org-fmt/texinfo.
#[test]
fn muse_streaming_writer_byte_identical_to_builder_over_all_fixtures() {
    let root = fixtures_root().join("muse");
    let mut checked = 0;
    let mut result: Result<(), String> = Ok(());
    for entry in std::fs::read_dir(&root).expect("fixtures/muse dir") {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(&path) else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");
        let (doc, _diags) = muse_fmt::parse(&input);
        let built = muse_fmt::build(&doc);

        let mut w = muse_fmt::Writer::new(Vec::<u8>::new());
        for e in muse_fmt::events::events(&doc) {
            w.write_event(e.into_owned());
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
        checked > 20,
        "expected to check a substantial number of muse fixtures, got {checked}"
    );

    if result.is_ok() {
        let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let mut w = muse_fmt::Writer::new(ObservableSink(observed.clone()));
        w.write_event(muse_fmt::OwnedMuseEvent::StartDocument);
        w.write_event(muse_fmt::OwnedMuseEvent::StartHeading { level: 1 });
        w.write_event(muse_fmt::OwnedMuseEvent::Text("Hello".to_string().into()));
        w.write_event(muse_fmt::OwnedMuseEvent::EndHeading);
        w.write_event(muse_fmt::OwnedMuseEvent::StartParagraph);
        w.write_event(muse_fmt::OwnedMuseEvent::Text("World".to_string().into()));
        w.write_event(muse_fmt::OwnedMuseEvent::EndParagraph);
        w.write_event(muse_fmt::OwnedMuseEvent::EndDocument);
        let pre_finish = observed.borrow().len();
        let _ = w.finish();
        if pre_finish == 0 {
            result = Err(
                "Writer wrote zero bytes to the sink after 8 complete write_event() calls (a \
                 full heading + paragraph) and before finish() — muse_fmt::writer::Writer \
                 buffers all events into a Vec<OwnedMuseEvent> and only reconstructs the AST + \
                 calls emit::build() inside finish() (crates/formats/muse-fmt/src/writer.rs:42- \
                 52), so it is not a genuine incremental streaming writer despite content \
                 round-tripping correctly for fixtures without document metadata"
                    .to_string(),
            );
        }
    }
    assert_or_known_failure("muse", "streaming_writer", result);
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
