//! Streaming-API cross-checks for asciidoc. Split out of the former monolithic
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
    /// Note on `Event::Metadata` (document `:key: value` attributes): unlike
    /// every other projected event, this one cannot be positioned faithfully
    /// from `doc` alone. `AsciiDoc.attributes` is a `HashMap<String, String>`
    /// with no source-position or duplicate-declaration information — by the
    /// time `parse()` builds it, that's already gone (last declaration of a
    /// key wins, order is not tracked). `events()` itself is strictly more
    /// faithful here: `EventIter` emits one `Metadata` event at the exact
    /// source line each `:key: value` declaration is consumed (see
    /// `Event::Metadata`'s doc comment in `crates/formats/asciidoc/src/
    /// events.rs`), which the AST cannot reconstruct. So this projection
    /// emits a canonical, deterministic stand-in — one `Metadata` event per
    /// `doc.attributes` entry, sorted by key, right after `StartDocument` —
    /// and `asciidoc_events_equals_ast_projection_over_all_fixtures` compares
    /// `Metadata` events separately from the rest (as an order-independent
    /// set), not as part of the strict positional sequence, precisely
    /// because this one field is not something the AST is ground truth for.
    fn ad_ast_to_events(doc: &AsciiDoc) -> Vec<OwnedEvent> {
        let mut out = vec![OwnedEvent::StartDocument];
        let mut attrs: Vec<(&String, &String)> = doc.attributes.iter().collect();
        attrs.sort_by(|a, b| a.0.cmp(b.0));
        for (key, value) in attrs {
            out.push(OwnedEvent::Metadata {
                key: key.clone(),
                value: value.clone(),
            });
        }
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
        let mut metadata_failures: Vec<String> = Vec::new();
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

            // Metadata events are compared separately, as an order-independent
            // set — see ad_ast_to_events's doc comment on why the AST cannot be
            // ground truth for their stream position (events() places them at
            // their real source line; the AST's HashMap tracks none of that).
            fn split_metadata(evs: Vec<OwnedEvent>) -> (Vec<OwnedEvent>, Vec<(String, String)>) {
                let mut rest = Vec::new();
                let mut meta = Vec::new();
                for e in evs {
                    match e {
                        OwnedEvent::Metadata { key, value } => meta.push((key, value)),
                        other => rest.push(other),
                    }
                }
                (rest, meta)
            }
            let (expected, expected_meta_raw) = split_metadata(expected);
            let (actual, actual_meta_raw) = split_metadata(actual);
            // Duplicate declarations of the same key in source order all reach
            // events() (attribute_log records every declaration), but
            // doc.attributes keeps only the final value — dedupe actual's
            // Metadata events down to "final value per key" to match,
            // applying HashMap::insert semantics in declaration order (the
            // same rule DocBuilder::process uses in writer.rs).
            let mut final_actual: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for (k, v) in actual_meta_raw {
                final_actual.insert(k, v);
            }
            let mut actual_meta: Vec<(String, String)> = final_actual.into_iter().collect();
            actual_meta.sort();
            let mut expected_meta = expected_meta_raw;
            expected_meta.sort();
            if expected_meta != actual_meta {
                metadata_failures.push(format!(
                    "fixture {}: Metadata events diverged from doc.attributes\n  \
                     expected (from AST): {expected_meta:?}\n  actual (from events()): \
                     {actual_meta:?}",
                    path.file_name().unwrap().to_string_lossy(),
                ));
            }

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
        assert!(
            metadata_failures.is_empty(),
            "events() Metadata events diverged from doc.attributes for {} of {checked} \
             fixtures:\n\n{}",
            metadata_failures.len(),
            metadata_failures.join("\n\n")
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
