//! Generate DOCX input files for the rescribe fixture suite.
//!
//! Run with:
//!   cargo run -p rescribe-fmt-ooxml --example gen_docx_fixtures --features docx
//!
//! Writes files to `fixtures/docx/{feature}/input.docx` relative to the
//! workspace root. Re-run whenever fixture input files need to be regenerated.

use ooxml_wml::types;
use ooxml_wml::types::STUnderline;
use ooxml_wml::writer::{DocumentBuilder, ListType};
use std::io::Cursor;
use std::path::Path;
use std::{env, fs};

fn write_docx(rel_path: &str, builder: DocumentBuilder) {
    // Locate workspace root (parent of the crate's `Cargo.toml`)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let workspace_root = Path::new(&manifest_dir)
        .parent() // formats/
        .and_then(|p| p.parent()) // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("cannot find workspace root");
    let path = workspace_root.join(rel_path);

    let mut bytes = Vec::new();
    builder
        .write(&mut Cursor::new(&mut bytes))
        .expect("write failed");
    fs::create_dir_all(path.parent().unwrap()).expect("create_dir_all failed");
    fs::write(&path, bytes).expect("write failed");
    println!("Written: {}", path.display());
}

fn main() {
    let base = "fixtures/docx";

    // --- inline_bold ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        let run = para.add_run();
        run.set_text("bold text");
        run.set_bold(true);
        write_docx(&format!("{}/inline_bold/input.docx", base), b);
    }

    // --- inline_italic ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        let run = para.add_run();
        run.set_text("italic text");
        run.set_italic(true);
        write_docx(&format!("{}/inline_italic/input.docx", base), b);
    }

    // --- inline_color ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        let run = para.add_run();
        run.set_text("red text");
        run.set_color("FF0000");
        write_docx(&format!("{}/inline_color/input.docx", base), b);
    }

    // --- inline_font_size ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        let run = para.add_run();
        run.set_text("big text");
        run.set_font_size(48); // 24pt = 48 half-points
        write_docx(&format!("{}/inline_font_size/input.docx", base), b);
    }

    // --- inline_underline ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        let run = para.add_run();
        run.set_text("underlined text");
        run.set_underline(STUnderline::Single);
        write_docx(&format!("{}/inline_underline/input.docx", base), b);
    }

    // --- alignment ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        para.set_alignment(ooxml_wml::types::STJc::Center);
        para.add_run().set_text("centered paragraph");
        write_docx(&format!("{}/alignment/input.docx", base), b);
    }

    // --- list (unordered) ---
    {
        let mut b = DocumentBuilder::new();
        let num_id = b.add_list(ListType::Bullet);
        let para = b.body_mut().add_paragraph();
        para.set_numbering(num_id, 0);
        para.add_run().set_text("first item");
        let para2 = b.body_mut().add_paragraph();
        para2.set_numbering(num_id, 0);
        para2.add_run().set_text("second item");
        write_docx(&format!("{}/list/input.docx", base), b);
    }

    // --- list_ordered ---
    {
        let mut b = DocumentBuilder::new();
        let num_id = b.add_list(ListType::Decimal);
        let para = b.body_mut().add_paragraph();
        para.set_numbering(num_id, 0);
        para.add_run().set_text("first item");
        let para2 = b.body_mut().add_paragraph();
        para2.set_numbering(num_id, 0);
        para2.add_run().set_text("second item");
        write_docx(&format!("{}/list_ordered/input.docx", base), b);
    }

    // --- table ---
    {
        let mut b = DocumentBuilder::new();
        let table = b.body_mut().add_table();
        let row = table.add_row();
        let cell1 = row.add_cell();
        cell1.add_paragraph().add_run().set_text("Cell A1");
        let cell2 = row.add_cell();
        cell2.add_paragraph().add_run().set_text("Cell B1");
        write_docx(&format!("{}/table/input.docx", base), b);
    }

    // --- footnote ---
    {
        let mut b = DocumentBuilder::new();
        let fn_id = {
            let mut fn_builder = b.add_footnote();
            fn_builder.add_paragraph("Footnote text.");
            fn_builder.id() as i64
        };
        let para = b.body_mut().add_paragraph();
        para.add_run().set_text("Body text.");
        let ref_run = para.add_run();
        ref_run.add_footnote_ref(fn_id);
        write_docx(&format!("{}/footnote/input.docx", base), b);
    }

    // --- inline_font_name ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        let run = para.add_run();
        run.set_text("Courier text");
        run.set_fonts(types::Fonts {
            ascii: Some("Courier New".to_string()),
            h_ansi: Some("Courier New".to_string()),
            ..Default::default()
        });
        write_docx(&format!("{}/inline_font_name/input.docx", base), b);
    }

    // --- inline_language ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        let run = para.add_run();
        run.set_text("Bonjour");
        run.set_properties(types::RunProperties {
            lang: Some(Box::new(types::LanguageElement {
                value: Some("fr-FR".to_string()),
                east_asia: None,
                bidi: None,
                extra_attrs: Default::default(),
            })),
            ..Default::default()
        });
        write_docx(&format!("{}/inline_language/input.docx", base), b);
    }

    // --- inline_line_break ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        para.add_run().set_text("before");
        let br_run = para.add_run();
        br_run
            .run_content
            .push(types::RunContent::Br(Box::new(types::CTBr {
                r#type: None,
                clear: None,
                extra_attrs: Default::default(),
            })));
        para.add_run().set_text("after");
        write_docx(&format!("{}/inline_line_break/input.docx", base), b);
    }

    // --- inline_page_break ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        para.add_run().set_text("before");
        para.add_page_break();
        para.add_run().set_text("after");
        write_docx(&format!("{}/inline_page_break/input.docx", base), b);
    }

    // --- inline_column_break ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        para.add_run().set_text("before");
        para.add_column_break();
        para.add_run().set_text("after");
        write_docx(&format!("{}/inline_column_break/input.docx", base), b);
    }

    // --- inline_tab_stop ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        let run = para.add_run();
        run.run_content
            .push(types::RunContent::T(Box::new(types::Text {
                text: Some("before".to_string()),
                extra_children: Vec::new(),
            })));
        run.run_content
            .push(types::RunContent::Tab(Box::new(types::CTEmpty)));
        run.run_content
            .push(types::RunContent::T(Box::new(types::Text {
                text: Some("after".to_string()),
                extra_children: Vec::new(),
            })));
        write_docx(&format!("{}/inline_tab_stop/input.docx", base), b);
    }

    // --- table_colspan ---
    {
        let mut b = DocumentBuilder::new();
        let table = b.body_mut().add_table();
        let row1 = table.add_row();
        let wide_cell = row1.add_cell();
        wide_cell.set_grid_span(2);
        wide_cell
            .add_paragraph()
            .add_run()
            .set_text("Spans two columns");
        let row2 = table.add_row();
        row2.add_cell().add_paragraph().add_run().set_text("A2");
        row2.add_cell().add_paragraph().add_run().set_text("B2");
        write_docx(&format!("{}/table_colspan/input.docx", base), b);
    }

    // --- table_rowspan ---
    {
        use ooxml_wml::convenience::VMergeType;
        let mut b = DocumentBuilder::new();
        let table = b.body_mut().add_table();
        let row1 = table.add_row();
        let merged = row1.add_cell();
        merged.set_vertical_merge(VMergeType::Restart);
        merged.add_paragraph().add_run().set_text("Spans two rows");
        row1.add_cell().add_paragraph().add_run().set_text("B1");
        let row2 = table.add_row();
        let cont = row2.add_cell();
        cont.set_vertical_merge(VMergeType::Continue);
        cont.add_paragraph();
        row2.add_cell().add_paragraph().add_run().set_text("B2");
        write_docx(&format!("{}/table_rowspan/input.docx", base), b);
    }

    // --- table_shading ---
    {
        let mut b = DocumentBuilder::new();
        let table = b.body_mut().add_table();
        let row = table.add_row();
        let cell = row.add_cell();
        cell.set_background_color("FFFF00");
        cell.add_paragraph().add_run().set_text("Shaded cell");
        write_docx(&format!("{}/table_shading/input.docx", base), b);
    }

    // --- table_borders ---
    {
        use ooxml_wml::convenience::BorderStyle;
        let mut b = DocumentBuilder::new();
        let table = b.body_mut().add_table();
        let row = table.add_row();
        let cell = row.add_cell();
        cell.set_borders(BorderStyle::Single, 8, "FF0000");
        cell.add_paragraph().add_run().set_text("Bordered cell");
        write_docx(&format!("{}/table_borders/input.docx", base), b);
    }

    // --- para_style ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        para.set_properties(types::ParagraphProperties {
            paragraph_style: Some(Box::new(types::CTString {
                value: "Quote".to_string(),
                extra_attrs: Default::default(),
            })),
            ..Default::default()
        });
        para.add_run().set_text("Quoted paragraph.");
        write_docx(&format!("{}/para_style/input.docx", base), b);
    }

    // --- para_keep ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        para.set_properties(types::ParagraphProperties {
            keep_next: Some(Box::new(types::OnOffElement {
                value: None,
                extra_attrs: Default::default(),
            })),
            keep_lines: Some(Box::new(types::OnOffElement {
                value: None,
                extra_attrs: Default::default(),
            })),
            ..Default::default()
        });
        para.add_run().set_text("Keep with next paragraph.");
        write_docx(&format!("{}/para_keep/input.docx", base), b);
    }

    // --- para_page_break_before ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        para.set_properties(types::ParagraphProperties {
            page_break_before: Some(Box::new(types::OnOffElement {
                value: None,
                extra_attrs: Default::default(),
            })),
            ..Default::default()
        });
        para.add_run().set_text("Starts a new page.");
        write_docx(&format!("{}/para_page_break_before/input.docx", base), b);
    }

    // --- para_border ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        let border = types::CTBorder {
            value: types::STBorder::Single,
            color: Some("0000FF".to_string()),
            theme_color: None,
            theme_tint: None,
            theme_shade: None,
            size: Some(8),
            space: None,
            shadow: None,
            frame: None,
            extra_attrs: Default::default(),
        };
        para.set_properties(types::ParagraphProperties {
            paragraph_border: Some(Box::new(types::CTPBdr {
                top: Some(Box::new(border.clone())),
                bottom: Some(Box::new(border.clone())),
                left: Some(Box::new(border.clone())),
                right: Some(Box::new(border)),
                ..Default::default()
            })),
            ..Default::default()
        });
        para.add_run().set_text("Bordered paragraph.");
        write_docx(&format!("{}/para_border/input.docx", base), b);
    }

    // --- para_shading ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        para.set_properties(types::ParagraphProperties {
            shading: Some(Box::new(types::CTShd {
                value: types::STShd::Clear,
                color: None,
                theme_color: None,
                theme_tint: None,
                theme_shade: None,
                fill: Some("CCFFCC".to_string()),
                theme_fill: None,
                theme_fill_tint: None,
                theme_fill_shade: None,
                extra_attrs: Default::default(),
            })),
            ..Default::default()
        });
        para.add_run().set_text("Shaded paragraph.");
        write_docx(&format!("{}/para_shading/input.docx", base), b);
    }

    // --- heading_levels: one heading per level 1-6, exercising pStyle-based detection ---
    {
        let mut b = DocumentBuilder::new();
        for level in 1..=6u8 {
            let para = b.body_mut().add_paragraph();
            para.set_properties(types::ParagraphProperties {
                paragraph_style: Some(Box::new(types::CTString {
                    value: format!("Heading{}", level),
                    extra_attrs: Default::default(),
                })),
                ..Default::default()
            });
            para.add_run().set_text(format!("Heading level {}", level));
        }
        write_docx(&format!("{}/heading_levels/input.docx", base), b);
    }

    // --- revision_ins ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        para.add_run().set_text("Unchanged text. ");
        para.add_tracked_insertion(
            1,
            "Jane Doe",
            Some("2026-01-01T00:00:00Z"),
            "Inserted text.",
        );
        write_docx(&format!("{}/revision_ins/input.docx", base), b);
    }

    // --- revision_del ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        para.add_run().set_text("Unchanged text. ");
        para.add_tracked_deletion(2, "Jane Doe", Some("2026-01-01T00:00:00Z"), "Deleted text.");
        write_docx(&format!("{}/revision_del/input.docx", base), b);
    }

    // --- doc_page_layout: section properties (page size/margins/orientation) ---
    {
        let mut b = DocumentBuilder::new();
        b.body_mut()
            .add_paragraph()
            .add_run()
            .set_text("Body text.");
        b.body_mut()
            .set_section_properties(types::SectionProperties {
                pg_sz: Some(Box::new(types::PageSize {
                    width: Some("12240".to_string()),
                    height: Some("15840".to_string()),
                    orient: Some(types::STPageOrientation::Portrait),
                    code: None,
                    extra_attrs: Default::default(),
                })),
                pg_mar: Some(Box::new(types::PageMargins {
                    top: "1440".to_string(),
                    right: "1800".to_string(),
                    bottom: "1440".to_string(),
                    left: "1800".to_string(),
                    header: "720".to_string(),
                    footer: "720".to_string(),
                    gutter: "0".to_string(),
                    extra_attrs: Default::default(),
                })),
                ..Default::default()
            });
        write_docx(&format!("{}/doc_page_layout/input.docx", base), b);
    }

    // --- doc_language: document-level default language (docDefaults/rPrDefault/rPr/lang) ---
    {
        let mut b = DocumentBuilder::new();
        b.body_mut()
            .add_paragraph()
            .add_run()
            .set_text("Body text.");
        b.set_styles(types::Styles {
            doc_defaults: Some(Box::new(types::DocumentDefaults {
                r_pr_default: Some(Box::new(types::RunPropertiesDefault {
                    r_pr: Some(Box::new(types::RunProperties {
                        lang: Some(Box::new(types::LanguageElement {
                            value: Some("de-DE".to_string()),
                            east_asia: None,
                            bidi: None,
                            extra_attrs: Default::default(),
                        })),
                        ..Default::default()
                    })),
                    extra_children: Vec::new(),
                })),
                p_pr_default: None,
                extra_children: Vec::new(),
            })),
            ..Default::default()
        });
        write_docx(&format!("{}/doc_language/input.docx", base), b);
    }

    // --- doc_core_properties ---
    {
        use ooxml_wml::CoreProperties;
        let mut b = DocumentBuilder::new();
        b.body_mut()
            .add_paragraph()
            .add_run()
            .set_text("Body text.");
        b.set_core_properties(CoreProperties {
            title: Some("Sample Document".to_string()),
            creator: Some("Jane Doe".to_string()),
            description: Some("A test document.".to_string()),
            created: Some("2026-01-01T00:00:00Z".to_string()),
            modified: Some("2026-01-02T00:00:00Z".to_string()),
            ..Default::default()
        });
        write_docx(&format!("{}/doc_core_properties/input.docx", base), b);
    }

    // --- integration: table cells with formatted runs ---
    {
        let mut b = DocumentBuilder::new();
        let table = b.body_mut().add_table();
        let row = table.add_row();
        let cell = row.add_cell();
        let para = cell.add_paragraph();
        let run = para.add_run();
        run.set_text("bold cell text");
        run.set_bold(true);
        write_docx(
            &format!("{}/integration_table_formatted_runs/input.docx", base),
            b,
        );
    }

    // --- integration: list items with inline formatting ---
    {
        let mut b = DocumentBuilder::new();
        let num_id = b.add_list(ListType::Bullet);
        let para = b.body_mut().add_paragraph();
        para.set_numbering(num_id, 0);
        let run = para.add_run();
        run.set_text("italic item");
        run.set_italic(true);
        write_docx(
            &format!("{}/integration_list_formatted/input.docx", base),
            b,
        );
    }

    // --- integration: footnote with formatted content ---
    {
        let mut b = DocumentBuilder::new();
        let fn_id = {
            let mut fn_builder = b.add_footnote();
            let para = fn_builder.body_mut().add_paragraph();
            let run = para.add_run();
            run.set_text("bold footnote text");
            run.set_bold(true);
            fn_builder.id() as i64
        };
        let para = b.body_mut().add_paragraph();
        para.add_run().set_text("Body text.");
        para.add_run().add_footnote_ref(fn_id);
        write_docx(
            &format!("{}/integration_footnote_formatted/input.docx", base),
            b,
        );
    }

    // --- integration: heading with inline formatting ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        para.set_properties(types::ParagraphProperties {
            paragraph_style: Some(Box::new(types::CTString {
                value: "Heading1".to_string(),
                extra_attrs: Default::default(),
            })),
            ..Default::default()
        });
        let run = para.add_run();
        run.set_text("Bold Heading");
        run.set_bold(true);
        write_docx(
            &format!("{}/integration_heading_formatted/input.docx", base),
            b,
        );
    }

    // --- integration: hyperlink containing formatted text ---
    {
        let mut b = DocumentBuilder::new();
        let rel_id = b.add_hyperlink("https://example.com");
        let para = b.body_mut().add_paragraph();
        let link = para.add_hyperlink();
        link.set_rel_id(&rel_id);
        let run = link.add_run();
        run.set_text("bold link text");
        run.set_bold(true);
        write_docx(
            &format!("{}/integration_hyperlink_formatted/input.docx", base),
            b,
        );
    }

    // --- adv-empty-document: a valid docx with zero paragraphs ---
    {
        let b = DocumentBuilder::new();
        write_docx(&format!("{}/adv-empty-document/input.docx", base), b);
    }

    // --- adv-long-style-name: pStyle value far beyond any realistic length ---
    {
        let mut b = DocumentBuilder::new();
        let para = b.body_mut().add_paragraph();
        para.set_properties(types::ParagraphProperties {
            paragraph_style: Some(Box::new(types::CTString {
                value: "X".repeat(100_000),
                extra_attrs: Default::default(),
            })),
            ..Default::default()
        });
        para.add_run()
            .set_text("Paragraph with a pathologically long style name.");
        write_docx(&format!("{}/adv-long-style-name/input.docx", base), b);
    }

    // Note: adv-corrupt-image, adv-missing-document-xml, adv-corrupt-rels,
    // adv-unknown-namespace, adv-circular-relationships, adv-malformed-zip, and
    // adv-empty-bytes are hand-corrupted zip/XML packages (not expressible via
    // DocumentBuilder, which always produces well-formed output) and are
    // committed as static input.docx files rather than generated here.

    println!("Done generating DOCX fixtures.");
}
