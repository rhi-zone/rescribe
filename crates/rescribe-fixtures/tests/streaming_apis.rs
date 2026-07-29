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
    CAPABILITIES, NOT_YET_AUDITED, adversarial_chunkings, assert_or_known_failure,
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
