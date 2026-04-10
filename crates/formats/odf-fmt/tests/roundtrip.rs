//! Roundtrip tests for odf-fmt: parse(emit(doc)) == doc.

use odf_fmt::*;

/// Build a minimal valid ODF text document programmatically.
fn minimal_text_doc() -> OdfDocument {
    OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Paragraph(Paragraph {
                style_name: Some("Text_20_Body".to_string()),
                content: vec![Inline::Text("Hello, world!".to_string())],
                ..Default::default()
            }),
            TextBlock::Heading(Heading {
                style_name: Some("Heading_20_1".to_string()),
                outline_level: Some(1),
                content: vec![Inline::Text("Introduction".to_string())],
                ..Default::default()
            }),
            TextBlock::Paragraph(Paragraph {
                content: vec![
                    Inline::Text("This is ".to_string()),
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
                ],
                ..Default::default()
            }),
        ]),
        meta: OdfMeta {
            title: Some("Test Document".to_string()),
            creator: Some("Test Author".to_string()),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn roundtrip_minimal() {
    let doc = minimal_text_doc();
    let bytes = emit(&doc).expect("emit failed");

    // Must be parseable as a ZIP
    assert!(!bytes.is_empty(), "emitted bytes should not be empty");

    // Parse back
    let result = parse(&bytes).expect("parse failed");
    let doc2 = result.value;

    assert_eq!(doc2.mimetype, "application/vnd.oasis.opendocument.text");
    assert_eq!(doc2.meta.title, Some("Test Document".to_string()));
    assert_eq!(doc2.meta.creator, Some("Test Author".to_string()));

    // Check body structure
    let OdfBody::Text(blocks) = &doc2.body else {
        panic!("expected Text body, got {:?}", doc2.body);
    };
    assert_eq!(blocks.len(), 3, "expected 3 blocks");

    // First block: paragraph with text
    let TextBlock::Paragraph(p) = &blocks[0] else {
        panic!("expected Paragraph, got {:?}", blocks[0]);
    };
    assert_eq!(p.style_name, Some("Text_20_Body".to_string()));
    assert_eq!(p.content.len(), 1);
    let Inline::Text(t) = &p.content[0] else { panic!("expected Text") };
    assert_eq!(t, "Hello, world!");

    // Second block: heading
    let TextBlock::Heading(h) = &blocks[1] else {
        panic!("expected Heading, got {:?}", blocks[1]);
    };
    assert_eq!(h.outline_level, Some(1));
    let Inline::Text(ht) = &h.content[0] else { panic!("expected Text") };
    assert_eq!(ht, "Introduction");

    // Third block: paragraph with spans
    let TextBlock::Paragraph(p3) = &blocks[2] else {
        panic!("expected Paragraph");
    };
    assert_eq!(p3.content.len(), 5);
    let Inline::Span(span) = &p3.content[1] else { panic!("expected Span") };
    assert_eq!(span.style_name, Some("Bold".to_string()));
}

#[test]
fn roundtrip_list() {
    let doc = OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::List(List {
                style_name: Some("List_20_1".to_string()),
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
                ],
                ..Default::default()
            }),
        ]),
        ..Default::default()
    };

    let bytes = emit(&doc).unwrap();
    let result = parse(&bytes).unwrap();
    let doc2 = result.value;

    let OdfBody::Text(blocks) = &doc2.body else { panic!("expected Text body") };
    assert_eq!(blocks.len(), 1);
    let TextBlock::List(l) = &blocks[0] else { panic!("expected List") };
    assert_eq!(l.items.len(), 2);
    let TextBlock::Paragraph(p) = &l.items[0].content[0] else { panic!("expected Paragraph") };
    let Inline::Text(t) = &p.content[0] else { panic!("expected Text") };
    assert_eq!(t, "First item");
}

#[test]
fn roundtrip_table() {
    let doc = OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.text".to_string(),
        body: OdfBody::Text(vec![
            TextBlock::Table(Table {
                name: Some("Table1".to_string()),
                rows: vec![
                    TableRow {
                        cells: vec![
                            TableCell {
                                content: vec![TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Cell A1".to_string())],
                                    ..Default::default()
                                })],
                                ..Default::default()
                            },
                            TableCell {
                                content: vec![TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Cell B1".to_string())],
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
    };

    let bytes = emit(&doc).unwrap();
    let result = parse(&bytes).unwrap();
    let doc2 = result.value;

    let OdfBody::Text(blocks) = &doc2.body else { panic!("expected Text body") };
    let TextBlock::Table(t) = &blocks[0] else { panic!("expected Table") };
    assert_eq!(t.rows.len(), 1);
    assert_eq!(t.rows[0].cells.len(), 2);
}

#[test]
fn events_basic() {
    let doc = minimal_text_doc();
    let bytes = emit(&doc).unwrap();

    let events: Vec<_> = odf_fmt::events(&bytes).collect();

    // Should see StartText, at least some paragraph/heading events, EndText
    let has_start_text = events.iter().any(|e| matches!(e, OdfEvent::StartText));
    let has_end_text = events.iter().any(|e| matches!(e, OdfEvent::EndText));
    let has_paragraph = events.iter().any(|e| matches!(e, OdfEvent::StartParagraph { .. }));
    let has_heading = events.iter().any(|e| matches!(e, OdfEvent::StartHeading { .. }));

    assert!(has_start_text, "missing StartText event");
    assert!(has_end_text, "missing EndText event");
    assert!(has_paragraph, "missing StartParagraph event");
    assert!(has_heading, "missing StartHeading event");
}

#[test]
fn parse_invalid_zip_returns_error() {
    let result = parse(b"not a zip file");
    assert!(result.is_err(), "should fail on non-ZIP input");
}

#[test]
fn emit_empty_doc() {
    let doc = OdfDocument::default();
    let bytes = emit(&doc).expect("emit should succeed for empty doc");
    assert!(!bytes.is_empty());
    // Can be parsed back
    let _ = parse(&bytes).expect("should parse back");
}

// ── ODS (spreadsheet) ─────────────────────────────────────────────────────────

#[test]
fn roundtrip_spreadsheet_basic() {
    let doc = OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.spreadsheet".to_string(),
        body: OdfBody::Spreadsheet(SpreadsheetBody {
            sheets: vec![Sheet {
                name: Some("Sheet1".to_string()),
                print: true,
                rows: vec![
                    SheetRow {
                        cells: vec![
                            SheetCell {
                                value_type: Some("string".to_string()),
                                value: Some("Name".to_string()),
                                content: vec![TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Name".to_string())],
                                    ..Default::default()
                                })],
                                ..Default::default()
                            },
                            SheetCell {
                                value_type: Some("string".to_string()),
                                value: Some("Score".to_string()),
                                content: vec![TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Score".to_string())],
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
                                value: Some("Alice".to_string()),
                                content: vec![TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Alice".to_string())],
                                    ..Default::default()
                                })],
                                ..Default::default()
                            },
                            SheetCell {
                                value_type: Some("float".to_string()),
                                value: Some("42".to_string()),
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
            }],
            named_ranges: vec![],
        }),
        meta: OdfMeta {
            title: Some("Test Spreadsheet".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let bytes = emit(&doc).expect("emit failed");
    assert!(!bytes.is_empty());

    let result = parse(&bytes).expect("parse failed");
    let doc2 = result.value;

    assert_eq!(doc2.mimetype, "application/vnd.oasis.opendocument.spreadsheet");
    assert_eq!(doc2.meta.title, Some("Test Spreadsheet".to_string()));

    let OdfBody::Spreadsheet(body) = &doc2.body else {
        panic!("expected Spreadsheet body, got {:?}", doc2.body);
    };
    assert_eq!(body.sheets.len(), 1);
    let sheet = &body.sheets[0];
    assert_eq!(sheet.name, Some("Sheet1".to_string()));
    assert!(sheet.print);
    assert_eq!(sheet.rows.len(), 2);

    // Header row
    assert_eq!(sheet.rows[0].cells.len(), 2);
    assert_eq!(sheet.rows[0].cells[0].value_type, Some("string".to_string()));
    assert_eq!(sheet.rows[0].cells[0].value, Some("Name".to_string()));

    // Data row
    assert_eq!(sheet.rows[1].cells[1].value_type, Some("float".to_string()));
    assert_eq!(sheet.rows[1].cells[1].value, Some("42".to_string()));
}

#[test]
fn roundtrip_spreadsheet_formula() {
    let doc = OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.spreadsheet".to_string(),
        body: OdfBody::Spreadsheet(SpreadsheetBody {
            sheets: vec![Sheet {
                name: Some("Calc".to_string()),
                rows: vec![SheetRow {
                    cells: vec![
                        SheetCell {
                            value_type: Some("float".to_string()),
                            value: Some("10".to_string()),
                            ..Default::default()
                        },
                        SheetCell {
                            value_type: Some("float".to_string()),
                            value: Some("20".to_string()),
                            ..Default::default()
                        },
                        SheetCell {
                            value_type: Some("float".to_string()),
                            value: Some("30".to_string()),
                            formula: Some("of:=[.A1]+[.B1]".to_string()),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                }],
                ..Default::default()
            }],
            named_ranges: vec![NamedRange {
                name: "TotalRange".to_string(),
                cell_range_address: Some("Calc.$A$1:.$C$1".to_string()),
                base_cell_address: Some("Calc.$A$1".to_string()),
            }],
        }),
        ..Default::default()
    };

    let bytes = emit(&doc).unwrap();
    let result = parse(&bytes).unwrap();
    let doc2 = result.value;

    let OdfBody::Spreadsheet(body) = &doc2.body else {
        panic!("expected Spreadsheet body");
    };
    let cell = &body.sheets[0].rows[0].cells[2];
    assert_eq!(cell.formula, Some("of:=[.A1]+[.B1]".to_string()));
    assert_eq!(cell.value, Some("30".to_string()));
    assert_eq!(body.named_ranges.len(), 1);
    assert_eq!(body.named_ranges[0].name, "TotalRange");
}

// ── ODP (presentation) ────────────────────────────────────────────────────────

#[test]
fn roundtrip_presentation_basic() {
    let doc = OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.presentation".to_string(),
        body: OdfBody::Presentation(PresentationBody {
            pages: vec![
                DrawPage {
                    name: Some("page1".to_string()),
                    master_page_name: Some("Default".to_string()),
                    shapes: vec![
                        DrawShape {
                            presentation_class: Some("title".to_string()),
                            x: Some("5.0cm".to_string()),
                            y: Some("4.0cm".to_string()),
                            width: Some("20.0cm".to_string()),
                            height: Some("3.0cm".to_string()),
                            content: DrawShapeContent::TextBox(vec![
                                TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Hello Presentation".to_string())],
                                    ..Default::default()
                                }),
                            ]),
                            ..Default::default()
                        },
                        DrawShape {
                            presentation_class: Some("subtitle".to_string()),
                            content: DrawShapeContent::TextBox(vec![
                                TextBlock::Paragraph(Paragraph {
                                    content: vec![Inline::Text("Subtitle text".to_string())],
                                    ..Default::default()
                                }),
                            ]),
                            ..Default::default()
                        },
                    ],
                    notes: None,
                    ..Default::default()
                },
            ],
        }),
        meta: OdfMeta {
            title: Some("My Presentation".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let bytes = emit(&doc).expect("emit failed");
    assert!(!bytes.is_empty());

    let result = parse(&bytes).expect("parse failed");
    let doc2 = result.value;

    assert_eq!(doc2.mimetype, "application/vnd.oasis.opendocument.presentation");
    assert_eq!(doc2.meta.title, Some("My Presentation".to_string()));

    let OdfBody::Presentation(body) = &doc2.body else {
        panic!("expected Presentation body, got {:?}", doc2.body);
    };
    assert_eq!(body.pages.len(), 1);

    let page = &body.pages[0];
    assert_eq!(page.name, Some("page1".to_string()));
    assert_eq!(page.master_page_name, Some("Default".to_string()));
    assert_eq!(page.shapes.len(), 2);

    // Title shape
    let title = &page.shapes[0];
    assert_eq!(title.presentation_class, Some("title".to_string()));
    assert_eq!(title.x, Some("5.0cm".to_string()));
    let DrawShapeContent::TextBox(blocks) = &title.content else {
        panic!("expected TextBox");
    };
    assert_eq!(blocks.len(), 1);
    let TextBlock::Paragraph(p) = &blocks[0] else { panic!("expected Paragraph") };
    let Inline::Text(t) = &p.content[0] else { panic!("expected Text") };
    assert_eq!(t, "Hello Presentation");
}

#[test]
fn roundtrip_presentation_notes() {
    let doc = OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.presentation".to_string(),
        body: OdfBody::Presentation(PresentationBody {
            pages: vec![DrawPage {
                name: Some("slide1".to_string()),
                shapes: vec![DrawShape {
                    presentation_class: Some("body".to_string()),
                    content: DrawShapeContent::TextBox(vec![
                        TextBlock::Paragraph(Paragraph {
                            content: vec![Inline::Text("Slide body".to_string())],
                            ..Default::default()
                        }),
                    ]),
                    ..Default::default()
                }],
                notes: Some(Box::new(NotesPage {
                    shapes: vec![DrawShape {
                        presentation_class: Some("notes".to_string()),
                        content: DrawShapeContent::TextBox(vec![
                            TextBlock::Paragraph(Paragraph {
                                content: vec![Inline::Text("Speaker notes here".to_string())],
                                ..Default::default()
                            }),
                        ]),
                        ..Default::default()
                    }],
                    ..Default::default()
                })),
                ..Default::default()
            }],
        }),
        ..Default::default()
    };

    let bytes = emit(&doc).unwrap();
    let result = parse(&bytes).unwrap();
    let doc2 = result.value;

    let OdfBody::Presentation(body) = &doc2.body else {
        panic!("expected Presentation body");
    };
    let page = &body.pages[0];
    let notes = page.notes.as_ref().expect("expected notes page");
    assert_eq!(notes.shapes.len(), 1);
    let DrawShapeContent::TextBox(blocks) = &notes.shapes[0].content else {
        panic!("expected TextBox in notes");
    };
    let TextBlock::Paragraph(p) = &blocks[0] else { panic!("expected Paragraph") };
    let Inline::Text(t) = &p.content[0] else { panic!("expected Text") };
    assert_eq!(t, "Speaker notes here");
}

// ── Events: ODS ───────────────────────────────────────────────────────────────

#[test]
fn events_spreadsheet() {
    let doc = OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.spreadsheet".to_string(),
        body: OdfBody::Spreadsheet(SpreadsheetBody {
            sheets: vec![Sheet {
                name: Some("Data".to_string()),
                rows: vec![SheetRow {
                    cells: vec![SheetCell {
                        value_type: Some("float".to_string()),
                        value: Some("99".to_string()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            }],
            named_ranges: vec![],
        }),
        ..Default::default()
    };

    let bytes = emit(&doc).unwrap();
    let evts: Vec<_> = odf_fmt::events(&bytes).collect();

    assert!(evts.iter().any(|e| matches!(e, OdfEvent::StartSpreadsheet)), "missing StartSpreadsheet");
    assert!(evts.iter().any(|e| matches!(e, OdfEvent::EndSpreadsheet)), "missing EndSpreadsheet");
    assert!(
        evts.iter().any(|e| matches!(e, OdfEvent::StartSheet { name, .. } if name.as_deref() == Some("Data"))),
        "missing StartSheet with name=Data"
    );
    assert!(evts.iter().any(|e| matches!(e, OdfEvent::EndSheet)), "missing EndSheet");
    assert!(evts.iter().any(|e| matches!(e, OdfEvent::StartSheetRow { .. })), "missing StartSheetRow");
    assert!(
        evts.iter().any(|e| matches!(e, OdfEvent::StartSheetCell { value_type, .. } if value_type.as_deref() == Some("float"))),
        "missing StartSheetCell with value_type=float"
    );
}

// ── Events: ODP ───────────────────────────────────────────────────────────────

#[test]
fn events_presentation() {
    let doc = OdfDocument {
        mimetype: "application/vnd.oasis.opendocument.presentation".to_string(),
        body: OdfBody::Presentation(PresentationBody {
            pages: vec![DrawPage {
                name: Some("slide1".to_string()),
                shapes: vec![DrawShape {
                    presentation_class: Some("title".to_string()),
                    content: DrawShapeContent::TextBox(vec![
                        TextBlock::Paragraph(Paragraph {
                            content: vec![Inline::Text("My Title".to_string())],
                            ..Default::default()
                        }),
                    ]),
                    ..Default::default()
                }],
                ..Default::default()
            }],
        }),
        ..Default::default()
    };

    let bytes = emit(&doc).unwrap();
    let evts: Vec<_> = odf_fmt::events(&bytes).collect();

    assert!(evts.iter().any(|e| matches!(e, OdfEvent::StartPresentation)), "missing StartPresentation");
    assert!(evts.iter().any(|e| matches!(e, OdfEvent::EndPresentation)), "missing EndPresentation");
    assert!(
        evts.iter().any(|e| matches!(e, OdfEvent::StartSlide { name, .. } if name.as_deref() == Some("slide1"))),
        "missing StartSlide with name=slide1"
    );
    assert!(evts.iter().any(|e| matches!(e, OdfEvent::EndSlide)), "missing EndSlide");
    assert!(
        evts.iter().any(|e| matches!(e, OdfEvent::StartShape { presentation_class, .. } if presentation_class.as_deref() == Some("title"))),
        "missing StartShape with presentation_class=title"
    );
    assert!(evts.iter().any(|e| matches!(e, OdfEvent::StartTextBox)), "missing StartTextBox");
    assert!(evts.iter().any(|e| matches!(e, OdfEvent::StartParagraph { .. })), "missing StartParagraph");
    assert!(
        evts.iter().any(|e| matches!(e, OdfEvent::Text(t) if t.contains("My Title"))),
        "missing Text 'My Title'"
    );
}
