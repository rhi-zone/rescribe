//! Generate DOCX input files for the rescribe fixture suite.
//!
//! Run with:
//!   cargo run -p rescribe-write-docx --example gen_docx_fixtures
//!
//! Writes files to `fixtures/docx/{feature}/input.docx` relative to the
//! workspace root. Re-run whenever fixture input files need to be regenerated.

use ooxml_wml::types::STUnderline;
use ooxml_wml::writer::{DocumentBuilder, ListType};
use std::io::Cursor;
use std::path::Path;
use std::{env, fs};

fn write_docx(rel_path: &str, builder: DocumentBuilder) {
    // Locate workspace root (parent of the crate's `Cargo.toml`)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let workspace_root = Path::new(&manifest_dir)
        .parent() // writers/
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

    println!("Done generating DOCX fixtures.");
}
