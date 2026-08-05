//! Streaming-API cross-checks for fb2. Split out of the former monolithic
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
    if let Some(annotation) = &section.annotation {
        fb2_annotation_events(annotation, out);
    }
    for content in &section.content {
        fb2_section_content_events(content, out);
    }
    for nested in &section.section {
        fb2_section_events(nested, out);
    }
    out.push(fb2_fmt::Event::EndSection);
}

/// `Section.annotation` (distinct from `TitleInfo.annotation`, which is
/// packaged whole into `Event::Metadata` and never appears here) streams as
/// `StartAnnotation`/`EndAnnotation` wrapping the same per-construct events
/// section content already uses — mirrors `fb2_epigraph_events`'s shape,
/// since `Section.annotation` attaches to `Section` the same way
/// `Section.epigraph` does.
fn fb2_annotation_events(annotation: &fb2_fmt::Annotation, out: &mut Vec<fb2_fmt::Event>) {
    use fb2_fmt::AnnotationContent;
    out.push(fb2_fmt::Event::StartAnnotation {
        id: annotation.id.clone(),
    });
    for content in &annotation.content {
        match content {
            AnnotationContent::Para(il) => {
                out.push(fb2_fmt::Event::StartParagraph);
                out.push(fb2_fmt::Event::Inline(il.clone()));
                out.push(fb2_fmt::Event::EndParagraph);
            }
            AnnotationContent::EmptyLine => out.push(fb2_fmt::Event::EmptyLine),
            AnnotationContent::Subtitle(il) => out.push(fb2_fmt::Event::Subtitle(il.clone())),
            AnnotationContent::Poem(p) => fb2_poem_events(p, out),
            AnnotationContent::Cite(c) => fb2_cite_events(c, out),
            AnnotationContent::Table(t) => out.push(fb2_fmt::Event::Table(t.clone())),
        }
    }
    out.push(fb2_fmt::Event::EndAnnotation);
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
        let (fb, _diags) = fb2_fmt::FictionBook::parse(&input);
        let expected = fb2_ast_to_events(&fb);
        let actual: Vec<_> = fb2_fmt::FictionBook::events(&input).collect();
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

/// `StreamingParser` now drains incrementally (see events.rs's
/// `StreamingParser::drain`): each `feed()` call rebuilds a `quick_xml`
/// `Reader` over just the unconsumed tail and dispatches every event it can
/// prove complete to the handler immediately, reusing the same
/// `SemanticState` machine `events()`/`EventIter` already used internally.
/// This check confirms both (1) the final sequence matches `events()` under
/// adversarial chunking, and (2) `feed()` alone (before `finish()`) actually
/// delivers events for a half-fed fixture — the property that used to fail
/// when `feed()` only extended a `Vec<u8>`.
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
        let bulk: Vec<fb2_fmt::Event> = fb2_fmt::FictionBook::events(&input).collect();
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

        // Deliberately NOT probed here: an arbitrary 50%-byte split of each
        // real fixture. That was this check's original design and it is
        // fixture-shape-unaware in exactly the way jats-fmt's own
        // streaming_parser KnownFailure documents: a 50% split of
        // fixtures/fb2/adv-empty (138 bytes) lands mid-attribute-value
        // inside the still-open root `<FictionBook xmlns="...` start tag,
        // so zero events delivered at that exact split point is the
        // correct, spec-conforming answer, not a StreamingParser defect.
        // See the hand-built probe below instead, which guarantees an
        // unambiguous complete-prefix boundary.
    }
    assert!(
        checked > 5,
        "expected to check several fb2 fixtures, got {checked}"
    );

    // Incrementality probe: a hand-built input with a definite complete
    // prefix (StartFictionBook/StartBody/StartSection/StartParagraph/
    // Inline/EndParagraph are all provably complete tokens) followed by
    // deliberately unterminated trailing tags (no `</section>`, `</body>`,
    // `</FictionBook>`). Confirms events reach the handler after feed()
    // alone, before finish() is ever called — the property that failed
    // when StreamingParser only extended a Vec<u8> and parsed in finish().
    if result.is_ok() {
        let probe_input = b"<?xml version=\"1.0\"?>\
            <FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\">\
            <body><section><p>Hello</p>";
        let mut delivered: Vec<fb2_fmt::Event> = Vec::new();
        let mut parser = fb2_fmt::StreamingParser::new(|e| delivered.push(e));
        parser.feed(probe_input);
        result = assert_streaming_parser_is_incremental("fb2", !delivered.is_empty());
    }
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
        let (fb, _diags) = fb2_fmt::FictionBook::parse(&input);
        let built = fb.emit();

        let mut w = fb2_fmt::writer::Writer::new(Vec::<u8>::new());
        for e in fb2_fmt::FictionBook::events(&input) {
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
