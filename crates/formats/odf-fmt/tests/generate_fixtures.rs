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
    write_fixture_as(name, "odt", doc);
}

fn write_fixture_as(name: &str, ext: &str, doc: OdfDocument) {
    let dir = fixtures_dir().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let bytes = emit(&doc).expect("emit failed");
    std::fs::write(dir.join(format!("input.{ext}")), bytes).unwrap();
    eprintln!("Wrote fixtures/odf/{name}/input.{ext}");
}

fn write_raw_fixture(name: &str, ext: &str, bytes: Vec<u8>) {
    let dir = fixtures_dir().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("input.{ext}")), bytes).unwrap();
    eprintln!("Wrote fixtures/odf/{name}/input.{ext} (raw)");
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

// ── Remaining block constructs ────────────────────────────────────────────────

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_list_ordered() {
    write_fixture("list-ordered", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::List(List {
                style_name: Some("Ordered_20_List_20_1".to_string()),
                items: vec![
                    ListItem {
                        content: vec![TextBlock::Paragraph(Paragraph {
                            content: vec![Inline::Text("First step".to_string())],
                            ..Default::default()
                        })],
                        ..Default::default()
                    },
                    ListItem {
                        content: vec![TextBlock::Paragraph(Paragraph {
                            content: vec![Inline::Text("Second step".to_string())],
                            ..Default::default()
                        })],
                        ..Default::default()
                    },
                    ListItem {
                        content: vec![TextBlock::Paragraph(Paragraph {
                            content: vec![Inline::Text("Third step".to_string())],
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
fn generate_list_nested() {
    write_fixture("list-nested", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::List(List {
                items: vec![
                    ListItem {
                        content: vec![
                            TextBlock::Paragraph(Paragraph {
                                content: vec![Inline::Text("Outer item 1".to_string())],
                                ..Default::default()
                            }),
                            TextBlock::List(List {
                                items: vec![
                                    ListItem {
                                        content: vec![TextBlock::Paragraph(Paragraph {
                                            content: vec![Inline::Text("Inner item A".to_string())],
                                            ..Default::default()
                                        })],
                                        ..Default::default()
                                    },
                                    ListItem {
                                        content: vec![TextBlock::Paragraph(Paragraph {
                                            content: vec![Inline::Text("Inner item B".to_string())],
                                            ..Default::default()
                                        })],
                                        ..Default::default()
                                    },
                                ],
                                ..Default::default()
                            }),
                        ],
                        ..Default::default()
                    },
                    ListItem {
                        content: vec![TextBlock::Paragraph(Paragraph {
                            content: vec![Inline::Text("Outer item 2".to_string())],
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
fn generate_rare_table_spans() {
    write_fixture("rare-table-spans", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Table(Table {
                name: Some("SpanTable".to_string()),
                rows: vec![
                    // Row 1: first cell spans 2 columns
                    TableRow {
                        cells: vec![
                            TableCell {
                                col_span: Some(2),
                                content: vec![TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Wide cell".to_string())],
                                    ..Default::default()
                                })],
                                ..Default::default()
                            },
                            TableCell {
                                covered: true,
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    },
                    // Row 2: normal cells
                    TableRow {
                        cells: vec![
                            TableCell {
                                content: vec![TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("A".to_string())],
                                    ..Default::default()
                                })],
                                ..Default::default()
                            },
                            TableCell {
                                content: vec![TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("B".to_string())],
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
fn generate_section() {
    write_fixture("section", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Section(Section {
                name: Some("Introduction".to_string()),
                content: vec![
                    TextBlock::Paragraph(Paragraph {
                        content: vec![Inline::Text("Content inside a section.".to_string())],
                        ..Default::default()
                    }),
                ],
                ..Default::default()
            }),
            TextBlock::Paragraph(Paragraph {
                content: vec![Inline::Text("Content outside the section.".to_string())],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_frame_textbox() {
    write_fixture("frame-textbox", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![Inline::Text("Paragraph before frame.".to_string())],
                ..Default::default()
            }),
            TextBlock::Frame(Frame {
                name: Some("TextBox1".to_string()),
                anchor_type: Some("paragraph".to_string()),
                width: Some("8cm".to_string()),
                height: Some("3cm".to_string()),
                content: FrameContent::TextBox(vec![
                    TextBlock::Paragraph(Paragraph {
                        content: vec![Inline::Text("Text inside the text box.".to_string())],
                        ..Default::default()
                    }),
                ]),
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}

// ── Inline constructs ─────────────────────────────────────────────────────────

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_inline_image() {
    write_fixture("inline-image", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![
                    Inline::Text("See image: ".to_string()),
                    Inline::Frame(Frame {
                        name: Some("Image1".to_string()),
                        anchor_type: Some("as-char".to_string()),
                        width: Some("3cm".to_string()),
                        height: Some("2cm".to_string()),
                        content: FrameContent::Image {
                            href: "Pictures/test.png".to_string(),
                            mime_type: Some("image/png".to_string()),
                        },
                        ..Default::default()
                    }),
                    Inline::Text(" caption here.".to_string()),
                ],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_rare_tab() {
    write_fixture("rare-tab", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![
                    Inline::Text("Column A".to_string()),
                    Inline::Tab,
                    Inline::Text("Column B".to_string()),
                    Inline::Tab,
                    Inline::Text("Column C".to_string()),
                ],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_rare_space_run() {
    write_fixture("rare-space-run", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![
                    Inline::Text("Before".to_string()),
                    Inline::Space { count: 5 },
                    Inline::Text("After".to_string()),
                ],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_rare_endnote() {
    write_fixture("rare-endnote", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![
                    Inline::Text("Text with an endnote".to_string()),
                    Inline::Note(Note {
                        note_class: NoteClass::Endnote,
                        id: Some("edn1".to_string()),
                        citation: Some("i".to_string()),
                        body: vec![
                            TextBlock::Paragraph(Paragraph {
                                content: vec![Inline::Text("This is the endnote text.".to_string())],
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

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_rare_fields() {
    write_fixture("rare-fields", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![
                    Inline::Text("Page ".to_string()),
                    Inline::Field { name: "text:page-number".to_string(), value: "1".to_string() },
                    Inline::Text(" of ".to_string()),
                    Inline::Field { name: "text:page-count".to_string(), value: "10".to_string() },
                ],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    });
}

// ── Metadata constructs ───────────────────────────────────────────────────────

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_rare_doc_stats() {
    write_fixture("rare-doc-stats", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![Inline::Text("Document with statistics.".to_string())],
                ..Default::default()
            }),
        ]),
        meta: OdfMeta {
            title: Some("Stats Document".to_string()),
            document_statistics: Some(DocumentStatistics {
                page_count: Some(3),
                paragraph_count: Some(12),
                word_count: Some(500),
                character_count: Some(2800),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    });
}

// ── Styles ────────────────────────────────────────────────────────────────────

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_styles_text_props() {
    write_fixture("styles-text-props", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                style_name: Some("Default".to_string()),
                content: vec![
                    Inline::Span(Span {
                        style_name: Some("BoldStyle".to_string()),
                        content: vec![Inline::Text("Bold red text".to_string())],
                    }),
                ],
                ..Default::default()
            }),
        ]),
        automatic_styles: vec![
            StyleEntry {
                name: "BoldStyle".to_string(),
                family: Some("text".to_string()),
                text_props: TextProperties {
                    bold: true,
                    color: Some("#cc0000".to_string()),
                    font_size: Some("14pt".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        ],
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_styles_para_props() {
    write_fixture("styles-para-props", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                style_name: Some("CenteredPara".to_string()),
                content: vec![Inline::Text("Centered paragraph.".to_string())],
                ..Default::default()
            }),
        ]),
        automatic_styles: vec![
            StyleEntry {
                name: "CenteredPara".to_string(),
                family: Some("paragraph".to_string()),
                para_props: ParagraphProperties {
                    align: Some("center".to_string()),
                    margin_top: Some("0.2cm".to_string()),
                    margin_bottom: Some("0.2cm".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        ],
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_styles_named() {
    write_fixture("styles-named", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                style_name: Some("MyHeading".to_string()),
                content: vec![Inline::Text("Styled heading.".to_string())],
                ..Default::default()
            }),
        ]),
        named_styles: vec![
            StyleEntry {
                name: "MyHeading".to_string(),
                family: Some("paragraph".to_string()),
                display_name: Some("My Heading Style".to_string()),
                text_props: TextProperties {
                    bold: true,
                    font_size: Some("16pt".to_string()),
                    ..Default::default()
                },
                para_props: ParagraphProperties {
                    align: Some("left".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        ],
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_styles_page_layout() {
    write_fixture("styles-page-layout", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                content: vec![Inline::Text("Document with custom page layout.".to_string())],
                ..Default::default()
            }),
        ]),
        page_layouts: vec![
            PageLayout {
                name: "pm1".to_string(),
                page_width: Some("21cm".to_string()),
                page_height: Some("29.7cm".to_string()),
                margin_top: Some("2cm".to_string()),
                margin_bottom: Some("2cm".to_string()),
                margin_left: Some("2.5cm".to_string()),
                margin_right: Some("2.5cm".to_string()),
                print_orientation: Some("portrait".to_string()),
            },
        ],
        ..Default::default()
    });
}

// ── Other document types ──────────────────────────────────────────────────────

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_ods_body() {
    write_fixture_as("ods-body", "ods", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.spreadsheet".to_string(),
        body: OdfBody::Spreadsheet(SpreadsheetBody {
            sheets: vec![
                Sheet {
                    name: Some("Sales".to_string()),
                    rows: vec![
                        SheetRow {
                            cells: vec![
                                SheetCell {
                                    value_type: Some("string".to_string()),
                                    value: Some("Product".to_string()),
                                    content: vec![TextBlock::Paragraph(Paragraph {
                                        content: vec![Inline::Text("Product".to_string())],
                                        ..Default::default()
                                    })],
                                    ..Default::default()
                                },
                                SheetCell {
                                    value_type: Some("string".to_string()),
                                    value: Some("Revenue".to_string()),
                                    content: vec![TextBlock::Paragraph(Paragraph {
                                        content: vec![Inline::Text("Revenue".to_string())],
                                        ..Default::default()
                                    })],
                                    ..Default::default()
                                },
                            ],
                            ..Default::default()
                        },
                        SheetRow {
                            cells: vec![
                                SheetCell {
                                    value_type: Some("string".to_string()),
                                    value: Some("Widget".to_string()),
                                    content: vec![TextBlock::Paragraph(Paragraph {
                                        content: vec![Inline::Text("Widget".to_string())],
                                        ..Default::default()
                                    })],
                                    ..Default::default()
                                },
                                SheetCell {
                                    value_type: Some("float".to_string()),
                                    value: Some("1500".to_string()),
                                    ..Default::default()
                                },
                            ],
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
            ],
            named_ranges: vec![],
        }),
        meta: OdfMeta {
            title: Some("Sales Data".to_string()),
            ..Default::default()
        },
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_odp_body() {
    write_fixture_as("odp-body", "odp", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.presentation".to_string(),
        body: OdfBody::Presentation(PresentationBody {
            pages: vec![
                DrawPage {
                    name: Some("slide1".to_string()),
                    master_page_name: Some("Default".to_string()),
                    shapes: vec![
                        DrawShape {
                            presentation_class: Some("title".to_string()),
                            x: Some("5.01cm".to_string()),
                            y: Some("4.57cm".to_string()),
                            width: Some("19.84cm".to_string()),
                            height: Some("2.82cm".to_string()),
                            content: DrawShapeContent::TextBox(vec![
                                TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Slide Title".to_string())],
                                    ..Default::default()
                                }),
                            ]),
                            ..Default::default()
                        },
                        DrawShape {
                            presentation_class: Some("subtitle".to_string()),
                            content: DrawShapeContent::TextBox(vec![
                                TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Subtitle text here.".to_string())],
                                    ..Default::default()
                                }),
                            ]),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                },
            ],
        }),
        meta: OdfMeta {
            title: Some("Test Presentation".to_string()),
            ..Default::default()
        },
        ..Default::default()
    });
}

// ── Adversarial fixtures ──────────────────────────────────────────────────────

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_adv_empty() {
    write_fixture("adv-empty", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Empty,
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_adv_bad_zip() {
    // Not a ZIP archive at all — just garbage bytes.
    write_raw_fixture("adv-bad-zip", "odt", b"NOT A ZIP FILE \x00\x01\x02\x03".to_vec());
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_adv_missing_content() {
    // A valid ZIP archive that does not contain content.xml.
    use std::io::Write as _;
    let buf = Vec::new();
    let cursor = std::io::Cursor::new(buf);
    let mut zip = zip::ZipWriter::new(cursor);
    let opts = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Stored);
    zip.start_file("mimetype", opts).unwrap();
    zip.write_all(b"application/vnd.oasis.opendocument.text").unwrap();
    let finished = zip.finish().unwrap();
    write_raw_fixture("adv-missing-content", "odt", finished.into_inner());
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_adv_deep_list() {
    fn nested_list(depth: u32) -> TextBlock {
        if depth == 0 {
            TextBlock::Paragraph(Paragraph {
                content: vec![Inline::Text(format!("Leaf item at depth {depth}"))],
                ..Default::default()
            })
        } else {
            TextBlock::List(List {
                items: vec![ListItem {
                    content: vec![
                        TextBlock::Paragraph(Paragraph {
                            content: vec![Inline::Text(format!("Item at depth {depth}"))],
                            ..Default::default()
                        }),
                        nested_list(depth - 1),
                    ],
                    ..Default::default()
                }],
                ..Default::default()
            })
        }
    }

    write_fixture("adv-deep-list", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![nested_list(20)]),
        ..Default::default()
    });
}

#[test]
#[ignore = "run explicitly to regenerate fixture inputs"]
fn generate_adv_large() {
    let blocks: Vec<TextBlock> = (1u32..=500)
        .map(|i| TextBlock::Paragraph(Paragraph {
            content: vec![Inline::Text(format!("Paragraph number {i} with some filler text to make it non-trivial."))],
            ..Default::default()
        }))
        .collect();

    write_fixture("adv-large", OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(blocks),
        ..Default::default()
    });
}
