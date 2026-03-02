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

    // ── para_spacing ──────────────────────────────────────────────────────────
    {
        let mut builder = DocumentBuilder::new();
        let para = builder.body_mut().add_paragraph();
        para.set_space_before(240); // 12pt before
        para.set_space_after(120); // 6pt after
        para.add_run().set_text("Spaced paragraph.");
        write_docx(builder, fixtures_dir.join("para_spacing/input.docx"));
    }

    // ── para_indent ───────────────────────────────────────────────────────────
    {
        let mut builder = DocumentBuilder::new();
        let para = builder.body_mut().add_paragraph();
        para.set_indent_left(720); // half inch left indent
        para.add_run().set_text("Indented paragraph.");
        write_docx(builder, fixtures_dir.join("para_indent/input.docx"));
    }

    // ── image ─────────────────────────────────────────────────────────────────
    {
        // A minimal 1×1 white PNG (67 bytes): used to test image embedding
        // without depending on a real image file.
        #[rustfmt::skip]
        let png_bytes: &[u8] = &[
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // PNG magic
            0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, // IHDR chunk length + type
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // width=1, height=1
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // bit depth=8, color=RGB
            0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, // IHDR CRC, IDAT chunk
            0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, // IDAT type + data
            0x00, 0x00, 0x02, 0x00, 0x01, 0xe2, 0x21, 0xbc, // IDAT data + CRC
            0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, // IEND chunk
            0x44, 0xae, 0x42, 0x60, 0x82,                   // IEND data + CRC
        ];
        let mut builder = DocumentBuilder::new();
        let rel_id = builder.add_image(png_bytes.to_vec(), "image/png");
        let mut drawing = ooxml_wml::writer::Drawing::new();
        drawing.add_image(&rel_id);
        let ct_drawing = drawing.build(&mut 1usize);
        let para = builder.body_mut().add_paragraph();
        let run = para.add_run();
        run.add_drawing(ct_drawing);
        write_docx(builder, fixtures_dir.join("image/input.docx"));
    }

    println!("Done.");
}
