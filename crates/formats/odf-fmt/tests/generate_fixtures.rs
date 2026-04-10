//! Fixture generator for ODF format tests.
//!
//! Run with: `cargo test -p odf-fmt --test generate_fixtures -- --ignored`
//!
//! Generates `fixtures/odf/**/*.odt` input files from programmatic ASTs.

use odf_fmt::*;
use std::path::Path;

fn fixtures_dir() -> std::path::PathBuf {
    // Walk up from the crate root to the workspace root
    let crate_root = env!("CARGO_MANIFEST_DIR");
    Path::new(crate_root)
        .join("../../../fixtures/odf")
}

fn write_fixture(name: &str, doc: OdfDocument) {
    let dir = fixtures_dir().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let bytes = emit(&doc).expect("emit failed");
    std::fs::write(dir.join("input.odt"), bytes).unwrap();
    eprintln!("Wrote fixtures/odf/{name}/input.odt");
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_paragraph() {
    write_fixture("paragraph", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![Inline::Text("Hello, world!".to_string())],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_heading() {
    write_fixture("heading", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Heading(Heading {
                outline_level: Some(1),
                content: vec![Inline::Text("Chapter One".to_string())],
                ..Default::default()
            }),
            TextBlock::Heading(Heading {
                outline_level: Some(2),
                content: vec![Inline::Text("Section 1.1".to_string())],
                ..Default::default()
            }),
            TextBlock::Paragraph(Paragraph {
                content: vec![Inline::Text("Paragraph text.".to_string())],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_list() {
    write_fixture("list", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::List(List {
                items: vec![
                    ListItem {
                        content: vec![TextBlock::Paragraph(Paragraph {
                            content: vec![Inline::Text("First item".to_string())],
                            ..Default::default()
                        })],
                        ..Default::default()
                    },
                    ListItem {
                        content: vec![TextBlock::Paragraph(Paragraph {
                            content: vec![Inline::Text("Second item".to_string())],
                            ..Default::default()
                        })],
                        ..Default::default()
                    },
                    ListItem {
                        content: vec![TextBlock::Paragraph(Paragraph {
                            content: vec![Inline::Text("Third item".to_string())],
                            ..Default::default()
                        })],
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_table() {
    write_fixture("table", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Table(Table {
                name: Some("Table1".to_string()),
                rows: vec![
                    TableRow {
                        cells: vec![
                            TableCell {
                                content: vec![TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Name".to_string())],
                                    ..Default::default()
                                })],
                                ..Default::default()
                            },
                            TableCell {
                                content: vec![TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Value".to_string())],
                                    ..Default::default()
                                })],
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    },
                    TableRow {
                        cells: vec![
                            TableCell {
                                content: vec![TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Foo".to_string())],
                                    ..Default::default()
                                })],
                                ..Default::default()
                            },
                            TableCell {
                                content: vec![TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("42".to_string())],
                                    ..Default::default()
                                })],
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_metadata() {
    write_fixture("metadata", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![Inline::Text("Document with metadata.".to_string())],
                ..Default::default()
            }),
        ]),
        meta: OdfMeta {
            title: Some("My Document Title".to_string()),
            creator: Some("Jane Author".to_string()),
            description: Some("A test document with metadata.".to_string()),
            subject: Some("Testing".to_string()),
            language: Some("en-US".to_string()),
            keywords: vec!["test".to_string(), "odf".to_string()],
            ..Default::default()
        },
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_inline_spans() {
    write_fixture("inline-spans", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![
                    Inline::Text("Normal ".to_string()),
                    Inline::Span(Span {
                        style_name: Some("Bold".to_string()),
                        content: vec![Inline::Text("bold".to_string())],
                    }),
                    Inline::Text(" and ".to_string()),
                    Inline::Span(Span {
                        style_name: Some("Italic".to_string()),
                        content: vec![Inline::Text("italic".to_string())],
                    }),
                    Inline::Text(" text.".to_string()),
                    Inline::LineBreak,
                    Inline::Text("After line break.".to_string()),
                ],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_inline_links() {
    write_fixture("inline-links", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![
                    Inline::Text("See ".to_string()),
                    Inline::Hyperlink(Hyperlink {
                        href: Some("https://example.com".to_string()),
                        title: Some("Example".to_string()),
                        content: vec![Inline::Text("this link".to_string())],
                        ..Default::default()
                    }),
                    Inline::Text(" for details.".to_string()),
                ],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_footnote() {
    write_fixture("footnote", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![
                    Inline::Text("Text with a footnote".to_string()),
                    Inline::Note(Note {
                        note_class: NoteClass::Footnote,
                        id: Some("ftn1".to_string()),
                        citation: Some("1".to_string()),
                        body: vec![
                            TextBlock::Paragraph(Paragraph {
                                content: vec![Inline::Text("This is the footnote text.".to_string())],
                                ..Default::default()
                            }),
                        ],
                    }),
                    Inline::Text(".".to_string()),
                ],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}
