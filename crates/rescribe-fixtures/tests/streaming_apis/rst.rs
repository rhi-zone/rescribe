//! Streaming-API cross-checks for rst. Split out of the former monolithic
//! `streaming_apis.rs` (see `crates/rescribe-fixtures/tests/streaming_apis.rs`
//! for the harness overview and `common.rs` for shared helpers) so concurrent
//! per-format edits stop colliding on one file.

#[allow(unused_imports)]
use crate::common::{assert_streaming_parser_is_incremental, find_input, fixtures_root};
#[allow(unused_imports)]
use rescribe_fixtures::streaming_harness::{
    CAPABILITIES, NOT_YET_AUDITED, ObservableSink, adversarial_chunkings, assert_or_known_failure,
};
#[allow(unused_imports)]
use rescribe_format_api::{Emit, Events, Handler, Parse, StreamingParse, StreamingWrite};
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

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
/// the whole input at once, with one documented, sanctioned exception class:
/// forward/cross-block-declared RST link targets and substitutions, which
/// `StreamingParser` does not resolve (each flushed block is re-parsed via a
/// fresh `EventIter`, so `link_targets`/`substitutions` collected in one
/// block aren't visible to a later block — see `crates/formats/rst-fmt/src/
/// batch.rs` module docs) while `events()`/`parse()` pre-scan the whole
/// input for them — fixtures exercising that are excluded rather than
/// flagged as a bug.
///
/// Wiring this check (previously nothing drove `StreamingParser` against
/// more than rst-fmt's own hand-picked 6 chunk-splitting cases) surfaced a
/// real bug, since fixed: multi-item RST definition lists got closed and
/// reopened as separate `StartDefinitionList`/`EndDefinitionList` pairs per
/// item in `StreamingParser`, instead of one list spanning all items the way
/// `events()` produces (`feed_line` now defers the blank-line flush with a
/// one-line lookahead mirroring `parse_definition_list`'s own `peek_line()`
/// check). Fixing it surfaced a second bug it had been masking (this check
/// only reports the first divergence per run): re-parsing each block in
/// isolation reset heading-level numbering per block instead of carrying it
/// across the whole document — also fixed, via
/// `EventIter::with_heading_levels`.
///
/// Fixing both surfaced a third bug of the same root cause (`emit_block()`'s
/// blank-line flush granularity), since also fixed: ordinary bullet/numbered
/// lists spanning blank lines, including blank-line-separated nested
/// sub-lists, also got split into multiple `StartList`/`EndList` pairs
/// instead of one. `feed_line` now defers the flush the same way for a
/// blank line following any bullet/numbered list-item marker (at any
/// indentation), confirming continuation from the next line rather than
/// replicating `parse_bullet_list`/`parse_numbered_list`'s full continuation
/// grammar — see the `rst` `CAPABILITIES` entry in `streaming_harness.rs`
/// for the mechanism and why the more permissive check is still correct.
/// Confirmed on the `nested-list` and `path-deep-list` fixtures. No longer
/// tracked in `KNOWN_FAILURES`; `streaming_parser` is `ApiState::Wired`.
#[test]
fn rst_streaming_parser_matches_events_under_adversarial_chunking() {
    let root = fixtures_root().join("rst");
    // anonymous-link/citation/link-target-url/rare-link-named: forward-declared
    // link targets. substitution/path-substitutions: forward/cross-block-declared
    // substitutions. All are the same sanctioned cross-block-resolution
    // limitation described above, not bugs.
    const SKIP: &[&str] = &[
        "anonymous-link",
        "citation",
        "link-target-url",
        "rare-link-named",
        "substitution",
        "path-substitutions",
    ];
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
        let built = String::from_utf8(doc.emit()).expect("rst-fmt emits valid UTF-8");

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
