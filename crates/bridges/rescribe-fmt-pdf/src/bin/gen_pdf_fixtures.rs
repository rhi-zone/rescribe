//! Generates real PDF fixture files for `fixtures/pdf/` using lopdf's
//! low-level Document/Content API to write genuine PDF object/stream
//! syntax (following the pattern in lopdf's own `examples/create.rs`).
//!
//! This is a `[[bin]]` fixture-generation tool, not library code -- per
//! CLAUDE.md's adapter rules, format-parsing/writing logic belongs in the
//! `-fmt` crate (or here, the upstream library), never in production
//! `rescribe-fmt-pdf` code. `pdf_extract`/`lopdf` don't ship a PDF writer
//! usable for constructing arbitrary layouts, so this tool talks to lopdf
//! directly to build real, structurally valid PDF bytes -- not
//! hand-authored/guessed byte sequences.
//!
//! Run with `cargo run -p rescribe-fmt-pdf --bin gen_pdf_fixtures`.

use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, Stream, dictionary};
use std::path::Path;

/// One line of text to place on a page: absolute position (PDF user-space
/// points, origin bottom-left), font size in points, and the text itself.
struct Line {
    x: f64,
    y: f64,
    font_size: f64,
    text: &'static str,
}

fn build_heading_font_size_fixture() -> Vec<u8> {
    // Layout rationale (see rescribe-fmt-pdf/src/lib.rs's block/line-break
    // heuristics): body text is 12pt with ~14pt line spacing (ratio 1.17,
    // well under the 1.5x line-break factor and the 2.5x block-break
    // factor, so wrapped lines merge into one paragraph block). Block
    // transitions (heading <-> paragraph) use a 70pt vertical gap, which
    // exceeds 2.5x even the largest font size used here (24pt -> 60pt), so
    // every transition reliably reads as a new block.
    let page1: Vec<Line> = vec![
        Line {
            x: 72.0,
            y: 750.0,
            font_size: 24.0,
            text: "Introduction",
        },
        Line {
            x: 72.0,
            y: 680.0,
            font_size: 12.0,
            text: "This is the first paragraph of body text.",
        },
        Line {
            x: 72.0,
            y: 666.0,
            font_size: 12.0,
            text: "It continues onto a second line.",
        },
        Line {
            x: 72.0,
            y: 596.0,
            font_size: 18.0,
            text: "Background",
        },
        Line {
            x: 72.0,
            y: 526.0,
            font_size: 12.0,
            text: "More body text at the same body size follows here.",
        },
    ];
    let page2: Vec<Line> = vec![Line {
        x: 72.0,
        y: 750.0,
        font_size: 12.0,
        text: "Text continues on the second page.",
    }];

    build_pdf(&[page1, page2])
}

/// Build a minimal but real multi-page PDF: one Type1/Helvetica font
/// resource shared across pages, one content stream per page built from
/// `Tf`/`Td`/`Tj` operations.
fn build_pdf(pages: &[Vec<Line>]) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });

    let pages_id = doc.new_object_id();
    let mut page_ids = Vec::new();

    for lines in pages {
        let mut operations = vec![Operation::new("BT", vec![])];
        let (mut last_x, mut last_y) = (0.0_f64, 0.0_f64);
        for line in lines {
            operations.push(Operation::new(
                "Tf",
                vec!["F1".into(), line.font_size.into()],
            ));
            let (dx, dy) = (line.x - last_x, line.y - last_y);
            operations.push(Operation::new("Td", vec![dx.into(), dy.into()]));
            operations.push(Operation::new(
                "Tj",
                vec![Object::string_literal(line.text)],
            ));
            last_x = line.x;
            last_y = line.y;
        }
        operations.push(Operation::new("ET", vec![]));

        let content = Content { operations };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));

        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => resources_id,
            "Contents" => content_id,
        });
        page_ids.push(page_id);
    }

    let pages_dict = dictionary! {
        "Type" => "Pages",
        "Kids" => page_ids.into_iter().map(Object::Reference).collect::<Vec<_>>(),
        "Count" => pages.len() as i64,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.compress();

    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

fn main() {
    let fixtures_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/pdf")
        .canonicalize()
        .expect("fixtures/pdf must exist");

    let heading_dir = fixtures_root.join("heading-font-size");
    std::fs::create_dir_all(&heading_dir).unwrap();
    std::fs::write(
        heading_dir.join("input.pdf"),
        build_heading_font_size_fixture(),
    )
    .unwrap();

    println!("Wrote {}", heading_dir.join("input.pdf").display());
}
