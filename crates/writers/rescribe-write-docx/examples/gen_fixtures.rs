//! Generate missing DOCX fixture files for the test suite.
//!
//! Run with: cargo run --example gen_fixtures -p rescribe-write-docx
//! from the workspace root.

use ooxml_wml::types::{CTHighlight, OnOffElement, STHighlightColor, TableRowProperties};
use ooxml_wml::writer::DocumentBuilder;
use std::fs;
use std::io::Cursor;
use std::path::Path;

fn write_docx(builder: DocumentBuilder, path: impl AsRef<Path>) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = Vec::new();
    let w = builder;
    w.write(&mut Cursor::new(&mut bytes)).unwrap();
    fs::write(path, &bytes).unwrap();
    println!("Wrote {}", path.display());
}

fn main() {
    let fixtures_dir = Path::new("fixtures/docx");

    // ── hyperlink ─────────────────────────────────────────────────────────────
    {
        let mut builder = DocumentBuilder::new();
        let rel_id = builder.add_hyperlink("https://example.com");
        let para = builder.body_mut().add_paragraph();
        let link = para.add_hyperlink();
        link.set_rel_id(&rel_id);
        link.add_run().set_text("click here");
        write_docx(builder, fixtures_dir.join("hyperlink/input.docx"));
    }

    // ── inline_small_caps ─────────────────────────────────────────────────────
    {
        let mut builder = DocumentBuilder::new();
        let para = builder.body_mut().add_paragraph();
        let run = para.add_run();
        run.set_text("Small Caps Text");
        run.set_small_caps(true);
        write_docx(builder, fixtures_dir.join("inline_small_caps/input.docx"));
    }

    // ── inline_all_caps ───────────────────────────────────────────────────────
    {
        let mut builder = DocumentBuilder::new();
        let para = builder.body_mut().add_paragraph();
        let run = para.add_run();
        run.set_text("All Caps Text");
        run.set_all_caps(true);
        write_docx(builder, fixtures_dir.join("inline_all_caps/input.docx"));
    }

    // ── inline_hidden ─────────────────────────────────────────────────────────
    {
        let mut builder = DocumentBuilder::new();
        let para = builder.body_mut().add_paragraph();
        para.add_run().set_text("visible");
        let run_hidden = para.add_run();
        run_hidden.set_text("hidden");
        run_hidden.set_vanish(true);
        write_docx(builder, fixtures_dir.join("inline_hidden/input.docx"));
    }

    // ── inline_highlight ──────────────────────────────────────────────────────
    {
        let mut builder = DocumentBuilder::new();
        let para = builder.body_mut().add_paragraph();
        let run = para.add_run();
        run.set_text("highlighted");
        run.set_properties(ooxml_wml::types::RunProperties {
            highlight: Some(Box::new(CTHighlight {
                value: STHighlightColor::Yellow,
                extra_attrs: Default::default(),
            })),
            ..Default::default()
        });
        write_docx(builder, fixtures_dir.join("inline_highlight/input.docx"));
    }

    // ── endnote ───────────────────────────────────────────────────────────────
    {
        let mut builder = DocumentBuilder::new();
        let en_id = {
            let mut en_builder = builder.add_endnote();
            let id = en_builder.id() as i64;
            en_builder
                .body_mut()
                .add_paragraph()
                .add_run()
                .set_text("Endnote text.");
            id
        };
        let para = builder.body_mut().add_paragraph();
        para.add_run().set_text("Body text.");
        para.add_run().add_endnote_ref(en_id);
        write_docx(builder, fixtures_dir.join("endnote/input.docx"));
    }

    // ── table_header ──────────────────────────────────────────────────────────
    {
        let mut builder = DocumentBuilder::new();
        let table = builder.body_mut().add_table();
        // Header row: set tblHeader property on row properties
        let header_row = table.add_row();
        let row_pr = header_row
            .row_properties
            .get_or_insert_with(|| Box::new(TableRowProperties::default()));
        row_pr.tbl_header = Some(Box::new(OnOffElement {
            value: None, // presence = true in OOXML
            ..Default::default()
        }));
        header_row
            .add_cell()
            .add_paragraph()
            .add_run()
            .set_text("Name");
        header_row
            .add_cell()
            .add_paragraph()
            .add_run()
            .set_text("Value");
        // Data row
        let data_row = table.add_row();
        data_row
            .add_cell()
            .add_paragraph()
            .add_run()
            .set_text("foo");
        data_row.add_cell().add_paragraph().add_run().set_text("42");
        write_docx(builder, fixtures_dir.join("table_header/input.docx"));
    }

    println!("Done.");
}
