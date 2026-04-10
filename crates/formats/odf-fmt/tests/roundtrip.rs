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
