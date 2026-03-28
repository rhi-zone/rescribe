/// Generate ODT fixture files for the rescribe-read-odt test suite.
///
/// Run with: cargo run -p rescribe-read-odt --bin gen_fixtures
use std::io::{Cursor, Write};
use std::path::Path;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

fn make_odt(content_xml: &str) -> Vec<u8> {
    make_odt_full(content_xml, None)
}

fn make_odt_with_meta(content_xml: &str, meta_xml: &str) -> Vec<u8> {
    make_odt_full(content_xml, Some(meta_xml))
}

fn make_odt_full(content_xml: &str, meta_xml: Option<&str>) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buf);
    let opts = SimpleFileOptions::default();
    zip.start_file("mimetype", opts).unwrap();
    zip.write_all(b"application/vnd.oasis.opendocument.text").unwrap();
    zip.start_file("content.xml", opts).unwrap();
    zip.write_all(content_xml.as_bytes()).unwrap();
    if let Some(meta) = meta_xml {
        zip.start_file("meta.xml", opts).unwrap();
        zip.write_all(meta.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
    buf.into_inner()
}

fn write_fixture_meta(name: &str, content_xml: &str, meta_xml: &str, expected_json: &str) {
    let dir = format!("fixtures/odt/{name}");
    std::fs::create_dir_all(&dir).unwrap();
    let odt = make_odt_with_meta(content_xml, meta_xml);
    std::fs::write(format!("{dir}/input.odt"), &odt).unwrap();
    std::fs::write(format!("{dir}/expected.json"), expected_json).unwrap();
    println!("wrote {dir}/");
}

fn write_fixture(name: &str, content_xml: &str, expected_json: &str) {
    // fixtures/odt/{name}/
    let dir = format!("fixtures/odt/{name}");
    std::fs::create_dir_all(&dir).unwrap();
    let odt = make_odt(content_xml);
    std::fs::write(format!("{dir}/input.odt"), &odt).unwrap();
    std::fs::write(format!("{dir}/expected.json"), expected_json).unwrap();
    println!("wrote {dir}/");
}

fn write_fixture_raw(name: &str, raw_bytes: Vec<u8>, expected_json: &str) {
    let dir = format!("fixtures/odt/{name}");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(format!("{dir}/input.odt"), &raw_bytes).unwrap();
    std::fs::write(format!("{dir}/expected.json"), expected_json).unwrap();
    println!("wrote {dir}/");
}

fn make_odt_with_styles(content_xml: &str, styles_xml: &str) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buf);
    let opts = SimpleFileOptions::default();
    zip.start_file("mimetype", opts).unwrap();
    zip.write_all(b"application/vnd.oasis.opendocument.text").unwrap();
    zip.start_file("content.xml", opts).unwrap();
    zip.write_all(content_xml.as_bytes()).unwrap();
    zip.start_file("styles.xml", opts).unwrap();
    zip.write_all(styles_xml.as_bytes()).unwrap();
    zip.finish().unwrap();
    buf.into_inner()
}

fn write_fixture_styles(name: &str, content_xml: &str, styles_xml: &str, expected_json: &str) {
    let dir = format!("fixtures/odt/{name}");
    std::fs::create_dir_all(&dir).unwrap();
    let odt = make_odt_with_styles(content_xml, styles_xml);
    std::fs::write(format!("{dir}/input.odt"), &odt).unwrap();
    std::fs::write(format!("{dir}/expected.json"), expected_json).unwrap();
    println!("wrote {dir}/");
}

/// Minimal 1×1 PNG (binary).
fn tiny_png() -> Vec<u8> {
    // A minimal valid 1×1 white PNG
    vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // PNG signature
        0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, // IHDR length + type
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1×1
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // 8-bit RGB + CRC start
        0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, // IDAT length + type
        0x54, 0x08, 0xd7, 0x63, 0xf8, 0xcf, 0xc0, 0x00, // IDAT data
        0x00, 0x00, 0x02, 0x00, 0x01, 0xe2, 0x21, 0xbc, // IDAT CRC
        0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, // IEND length + type
        0x44, 0xae, 0x42, 0x60, 0x82,                   // IEND CRC
    ]
}

fn make_odt_with_image(content_xml: &str, image_name: &str, image_bytes: Vec<u8>) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buf);
    let opts = SimpleFileOptions::default();
    zip.start_file("mimetype", opts).unwrap();
    zip.write_all(b"application/vnd.oasis.opendocument.text").unwrap();
    zip.start_file("content.xml", opts).unwrap();
    zip.write_all(content_xml.as_bytes()).unwrap();
    zip.start_file(image_name, opts).unwrap();
    zip.write_all(&image_bytes).unwrap();
    zip.finish().unwrap();
    buf.into_inner()
}

fn write_fixture_image(name: &str, content_xml: &str, image_name: &str, image_bytes: Vec<u8>, expected_json: &str) {
    let dir = format!("fixtures/odt/{name}");
    std::fs::create_dir_all(&dir).unwrap();
    let odt = make_odt_with_image(content_xml, image_name, image_bytes);
    std::fs::write(format!("{dir}/input.odt"), &odt).unwrap();
    std::fs::write(format!("{dir}/expected.json"), expected_json).unwrap();
    println!("wrote {dir}/");
}

fn make_odt_no_content() -> Vec<u8> {
    // Valid zip but missing content.xml
    let mut buf = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buf);
    let opts = SimpleFileOptions::default();
    zip.start_file("mimetype", opts).unwrap();
    zip.write_all(b"application/vnd.oasis.opendocument.text").unwrap();
    zip.finish().unwrap();
    buf.into_inner()
}

fn make_odt_wrong_mimetype() -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buf);
    let opts = SimpleFileOptions::default();
    zip.start_file("mimetype", opts).unwrap();
    zip.write_all(b"application/vnd.oasis.opendocument.spreadsheet").unwrap();
    zip.start_file("content.xml", opts).unwrap();
    zip.write_all(b"<root/>").unwrap();
    zip.finish().unwrap();
    buf.into_inner()
}

fn make_odt_corrupt_styles() -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buf);
    let opts = SimpleFileOptions::default();
    zip.start_file("mimetype", opts).unwrap();
    zip.write_all(b"application/vnd.oasis.opendocument.text").unwrap();
    zip.start_file("styles.xml", opts).unwrap();
    zip.write_all(b"<not valid xml at all <<<").unwrap();
    zip.start_file("content.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Body text.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#).unwrap();
    zip.finish().unwrap();
    buf.into_inner()
}

fn main() {
    // Change to workspace root so we write fixtures/ in the right place.
    let manifest = env!("CARGO_MANIFEST_DIR");
    let workspace_root = Path::new(manifest)
        .parent().unwrap() // readers/
        .parent().unwrap() // crates/
        .parent().unwrap(); // workspace root
    std::env::set_current_dir(workspace_root).unwrap();

    // ── bold (automatic style with fo:font-weight="bold") ──────────────────────
    write_fixture("bold",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:font-weight="bold"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>Before <text:span text:style-name="T1">bold text</text:span> after.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT bold inline (automatic style with fo:font-weight=bold)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document" },
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Before " } },
    { "path": "/0/1", "kind": "strong" },
    { "path": "/0/1/0", "kind": "text", "props": { "content": "bold text" } },
    { "path": "/0/2", "kind": "text", "props": { "content": " after." } }
  ]
}"#,
    );

    // ── italic ────────────────────────────────────────────────────────────────
    write_fixture("italic",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:font-style="italic"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>Before <text:span text:style-name="T1">italic text</text:span> after.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT italic inline (automatic style with fo:font-style=italic)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document" },
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Before " } },
    { "path": "/0/1", "kind": "emphasis" },
    { "path": "/0/1/0", "kind": "text", "props": { "content": "italic text" } },
    { "path": "/0/2", "kind": "text", "props": { "content": " after." } }
  ]
}"#,
    );

    // ── underline ─────────────────────────────────────────────────────────────
    write_fixture("underline",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties style:text-underline-style="solid" style:text-underline-width="auto" style:text-underline-color="font-color"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>Before <text:span text:style-name="T1">underlined text</text:span> after.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT underline inline",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document" },
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Before " } },
    { "path": "/0/1", "kind": "underline" },
    { "path": "/0/1/0", "kind": "text", "props": { "content": "underlined text" } },
    { "path": "/0/2", "kind": "text", "props": { "content": " after." } }
  ]
}"#,
    );

    // ── strikeout ─────────────────────────────────────────────────────────────
    write_fixture("strikeout",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties style:text-line-through-style="solid"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>Before <text:span text:style-name="T1">struck text</text:span> after.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT strikeout inline",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document" },
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Before " } },
    { "path": "/0/1", "kind": "strikeout" },
    { "path": "/0/1/0", "kind": "text", "props": { "content": "struck text" } },
    { "path": "/0/2", "kind": "text", "props": { "content": " after." } }
  ]
}"#,
    );

    // ── hyperlink ─────────────────────────────────────────────────────────────
    write_fixture("hyperlink",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:xlink="http://www.w3.org/1999/xlink">
  <office:body>
    <office:text>
      <text:p>See <text:a xlink:href="https://example.com" xlink:type="simple">this link</text:a> for details.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT hyperlink (text:a)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document" },
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "See " } },
    { "path": "/0/1", "kind": "link", "props": { "url": "https://example.com" } },
    { "path": "/0/1/0", "kind": "text", "props": { "content": "this link" } },
    { "path": "/0/2", "kind": "text", "props": { "content": " for details." } }
  ]
}"#,
    );

    // ── ordered-list ─────────────────────────────────────────────────────────
    write_fixture("ordered-list",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0">
  <office:automatic-styles>
    <text:list-style style:name="L1">
      <text:list-level-style-number text:level="1" text:style-name="Numbering_20_Symbols"
        style:num-suffix="." style:num-format="1"/>
    </text:list-style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:list text:style-name="L1">
        <text:list-item><text:p>First item</text:p></text:list-item>
        <text:list-item><text:p>Second item</text:p></text:list-item>
      </text:list>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT ordered list (numbered list style)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document" },
    { "path": "/0", "kind": "list", "props": { "ordered": true } },
    { "path": "/0/0", "kind": "list_item" },
    { "path": "/0/1", "kind": "list_item" }
  ]
}"#,
    );

    // ── table ─────────────────────────────────────────────────────────────────
    write_fixture("table",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0">
  <office:body>
    <office:text>
      <table:table>
        <table:table-row>
          <table:table-cell><text:p>A</text:p></table:table-cell>
          <table:table-cell><text:p>B</text:p></table:table-cell>
        </table:table-row>
        <table:table-row>
          <table:table-cell><text:p>C</text:p></table:table-cell>
          <table:table-cell><text:p>D</text:p></table:table-cell>
        </table:table-row>
      </table:table>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT 2x2 table",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document" },
    { "path": "/0", "kind": "table" },
    { "path": "/0/0", "kind": "table_row" },
    { "path": "/0/0/0", "kind": "table_cell" },
    { "path": "/0/0/1", "kind": "table_cell" },
    { "path": "/0/1", "kind": "table_row" }
  ]
}"#,
    );

    // ── code-block ────────────────────────────────────────────────────────────
    write_fixture("code-block",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p text:style-name="Preformatted Text">let x = 1;</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT code block (Preformatted Text style)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document" },
    { "path": "/0", "kind": "code_block", "props": { "content": "let x = 1;" } }
  ]
}"#,
    );

    // ── blockquote ────────────────────────────────────────────────────────────
    write_fixture("blockquote",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p text:style-name="Quotations">Quoted text here.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT blockquote (Quotations paragraph style)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document" },
    { "path": "/0", "kind": "blockquote" },
    { "path": "/0/0", "kind": "paragraph" },
    { "path": "/0/0/0", "kind": "text", "props": { "content": "Quoted text here." } }
  ]
}"#,
    );

    // ── subscript ─────────────────────────────────────────────────────────────
    write_fixture("subscript",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties style:text-position="sub 58%"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>H<text:span text:style-name="T1">2</text:span>O</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT subscript inline (style:text-position=sub)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document" },
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "H" } },
    { "path": "/0/1", "kind": "subscript" },
    { "path": "/0/1/0", "kind": "text", "props": { "content": "2" } },
    { "path": "/0/2", "kind": "text", "props": { "content": "O" } }
  ]
}"#,
    );

    // ── superscript ───────────────────────────────────────────────────────────
    write_fixture("superscript",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties style:text-position="super 58%"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>E=mc<text:span text:style-name="T1">2</text:span></text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT superscript inline (style:text-position=super)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document" },
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "E=mc" } },
    { "path": "/0/1", "kind": "superscript" },
    { "path": "/0/1/0", "kind": "text", "props": { "content": "2" } }
  ]
}"#,
    );

    // ── tab ───────────────────────────────────────────────────────────────────
    write_fixture("tab",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>before<text:tab/>after</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT tab stop (text:tab)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document" },
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "before\tafter" } }
  ]
}"#,
    );

    // ── heading-levels ───────────────────────────────────────────────────────
    write_fixture("heading-levels",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:h text:outline-level="1">Level One</text:h>
      <text:h text:outline-level="2">Level Two</text:h>
      <text:h text:outline-level="3">Level Three</text:h>
      <text:h text:outline-level="4">Level Four</text:h>
      <text:h text:outline-level="5">Level Five</text:h>
      <text:h text:outline-level="6">Level Six</text:h>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT heading levels 1 through 6",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "heading", "props": { "level": 1 } },
    { "path": "/1", "kind": "heading", "props": { "level": 2 } },
    { "path": "/2", "kind": "heading", "props": { "level": 3 } },
    { "path": "/3", "kind": "heading", "props": { "level": 4 } },
    { "path": "/4", "kind": "heading", "props": { "level": 5 } },
    { "path": "/5", "kind": "heading", "props": { "level": 6 } }
  ]
}"#,
    );

    // ── table-header ─────────────────────────────────────────────────────────
    write_fixture("table-header",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0">
  <office:body>
    <office:text>
      <table:table>
        <table:table-header-rows>
          <table:table-row>
            <table:table-cell><text:p>Header A</text:p></table:table-cell>
            <table:table-cell><text:p>Header B</text:p></table:table-cell>
          </table:table-row>
        </table:table-header-rows>
        <table:table-row>
          <table:table-cell><text:p>Data 1</text:p></table:table-cell>
          <table:table-cell><text:p>Data 2</text:p></table:table-cell>
        </table:table-row>
      </table:table>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT table with header row (table:table-header-rows)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "table" },
    { "path": "/0/0", "kind": "table_row" },
    { "path": "/0/0/0", "kind": "table_cell" },
    { "path": "/0/1", "kind": "table_row" },
    { "path": "/0/1/0", "kind": "table_cell" }
  ]
}"#,
    );

    // ── nested-list ──────────────────────────────────────────────────────────
    write_fixture("nested-list",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:list>
        <text:list-item>
          <text:p>Item one</text:p>
          <text:list>
            <text:list-item><text:p>Nested item</text:p></text:list-item>
          </text:list>
        </text:list-item>
        <text:list-item><text:p>Item two</text:p></text:list-item>
      </text:list>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT nested list (list within list_item)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "list" },
    { "path": "/0/0", "kind": "list_item" },
    { "path": "/0/1", "kind": "list_item" }
  ]
}"#,
    );

    // ── font-color ────────────────────────────────────────────────────────────
    // Use r##"..."## because the XML contains "#ff0000" which would prematurely end r#"..."#
    write_fixture("font-color",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:color="#ff0000"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>Before <text:span text:style-name="T1">red text</text:span> after.</text:p>
    </office:text>
  </office:body>
</office:document-content>"##,
        r##"{
  "description": "ODT font color (fo:color)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Before " } },
    { "path": "/0/1", "kind": "span", "props": { "style:color": "#ff0000" } },
    { "path": "/0/1/0", "kind": "text", "props": { "content": "red text" } },
    { "path": "/0/2", "kind": "text", "props": { "content": " after." } }
  ]
}"##,
    );

    // ── font-size ─────────────────────────────────────────────────────────────
    write_fixture("font-size",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:font-size="24pt"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>Before <text:span text:style-name="T1">large text</text:span> after.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT font size (fo:font-size)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Before " } },
    { "path": "/0/1", "kind": "span", "props": { "style:size": "24pt" } },
    { "path": "/0/1/0", "kind": "text", "props": { "content": "large text" } },
    { "path": "/0/2", "kind": "text", "props": { "content": " after." } }
  ]
}"#,
    );

    // ── font-name ─────────────────────────────────────────────────────────────
    write_fixture("font-name",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:font-family="Arial"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>Before <text:span text:style-name="T1">Arial text</text:span> after.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT font name (fo:font-family)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Before " } },
    { "path": "/0/1", "kind": "span", "props": { "style:font": "Arial" } },
    { "path": "/0/1/0", "kind": "text", "props": { "content": "Arial text" } },
    { "path": "/0/2", "kind": "text", "props": { "content": " after." } }
  ]
}"#,
    );

    // ── small-caps ────────────────────────────────────────────────────────────
    write_fixture("small-caps",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:font-variant="small-caps"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>Before <text:span text:style-name="T1">small caps</text:span> after.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT small caps (fo:font-variant=small-caps)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Before " } },
    { "path": "/0/1", "kind": "span", "props": { "style:variant": "small-caps" } },
    { "path": "/0/1/0", "kind": "text", "props": { "content": "small caps" } },
    { "path": "/0/2", "kind": "text", "props": { "content": " after." } }
  ]
}"#,
    );

    // ── soft-hyphen ───────────────────────────────────────────────────────────
    write_fixture("soft-hyphen",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>hyph<text:soft-hyphen/>en</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        "{
  \"description\": \"ODT soft hyphen (text:soft-hyphen)\",
  \"category\": \"happy\",
  \"assertions\": [
    { \"path\": \"/0\", \"kind\": \"paragraph\" },
    { \"path\": \"/0/0\", \"kind\": \"text\", \"props\": { \"content\": \"hyph\\u00aden\" } }
  ]
}",
    );

    // ── para-align ────────────────────────────────────────────────────────────
    write_fixture("para-align",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="P1" style:family="paragraph">
      <style:paragraph-properties fo:text-align="center"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p text:style-name="P1">Centered text.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT paragraph alignment (fo:text-align=center)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph", "props": { "style:align": "center" } },
    { "path": "/0/0", "kind": "text", "props": { "content": "Centered text." } }
  ]
}"#,
    );

    // ── para-indent ───────────────────────────────────────────────────────────
    write_fixture("para-indent",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="P1" style:family="paragraph">
      <style:paragraph-properties fo:margin-left="1.5cm" fo:text-indent="0.5cm"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p text:style-name="P1">Indented paragraph.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT paragraph indent (fo:margin-left, fo:text-indent)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph", "props": { "style:margin-left": "1.5cm", "style:text-indent": "0.5cm" } },
    { "path": "/0/0", "kind": "text", "props": { "content": "Indented paragraph." } }
  ]
}"#,
    );

    // ── para-spacing ──────────────────────────────────────────────────────────
    write_fixture("para-spacing",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="P1" style:family="paragraph">
      <style:paragraph-properties fo:margin-top="0.5cm" fo:margin-bottom="0.5cm"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p text:style-name="P1">Spaced paragraph.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT paragraph spacing (fo:margin-top, fo:margin-bottom)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph", "props": { "style:margin-top": "0.5cm", "style:margin-bottom": "0.5cm" } },
    { "path": "/0/0", "kind": "text", "props": { "content": "Spaced paragraph." } }
  ]
}"#,
    );

    // ── para-background ───────────────────────────────────────────────────────
    // Use r##"..."## because the XML contains "#ffffcc" which would prematurely end r#"..."#
    write_fixture("para-background",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="P1" style:family="paragraph">
      <style:paragraph-properties fo:background-color="#ffffcc"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p text:style-name="P1">Yellow background.</text:p>
    </office:text>
  </office:body>
</office:document-content>"##,
        r##"{
  "description": "ODT paragraph background color (fo:background-color)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph", "props": { "style:background": "#ffffcc" } },
    { "path": "/0/0", "kind": "text", "props": { "content": "Yellow background." } }
  ]
}"##,
    );

    // ── para-border ───────────────────────────────────────────────────────────
    write_fixture("para-border",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="P1" style:family="paragraph">
      <style:paragraph-properties fo:border="0.5pt solid #000000"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p text:style-name="P1">Bordered paragraph.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT paragraph border (fo:border)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph", "props": { "style:border": "0.5pt solid #000000" } },
    { "path": "/0/0", "kind": "text", "props": { "content": "Bordered paragraph." } }
  ]
}"#,
    );

    // ── line-height ───────────────────────────────────────────────────────────
    write_fixture("line-height",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="P1" style:family="paragraph">
      <style:paragraph-properties fo:line-height="150%"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p text:style-name="P1">One-and-a-half line spacing.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT line height (fo:line-height)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph", "props": { "style:line-height": "150%" } },
    { "path": "/0/0", "kind": "text", "props": { "content": "One-and-a-half line spacing." } }
  ]
}"#,
    );

    // ── keep-together ─────────────────────────────────────────────────────────
    write_fixture("keep-together",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="P1" style:family="paragraph">
      <style:paragraph-properties fo:keep-together="always" fo:keep-with-next="always"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p text:style-name="P1">Keep together paragraph.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT keep-together / keep-with-next paragraph properties",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph", "props": { "style:keep-together": "always", "style:keep-with-next": "always" } },
    { "path": "/0/0", "kind": "text", "props": { "content": "Keep together paragraph." } }
  ]
}"#,
    );

    // ── meta-title ────────────────────────────────────────────────────────────
    write_fixture_meta("meta-title",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Document body.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:dc="http://purl.org/dc/elements/1.1/">
  <office:meta>
    <dc:title>My Document Title</dc:title>
  </office:meta>
</office:document-meta>"#,
        r#"{
  "description": "ODT document title from meta.xml (dc:title)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document", "metadata": { "title": "My Document Title" } }
  ]
}"#,
    );

    // ── meta-author ───────────────────────────────────────────────────────────
    write_fixture_meta("meta-author",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Document body.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:dc="http://purl.org/dc/elements/1.1/">
  <office:meta>
    <dc:creator>Jane Doe</dc:creator>
  </office:meta>
</office:document-meta>"#,
        r#"{
  "description": "ODT document author from meta.xml (dc:creator)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document", "metadata": { "author": "Jane Doe" } }
  ]
}"#,
    );

    // ── non-breaking-space ───────────────────────────────────────────────────
    // text:s represents a run of multiple spaces; here just the non-breaking concept
    write_fixture("non-breaking-space",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>before&#160;after</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        "{
  \"description\": \"ODT non-breaking space (&#160;)\",
  \"category\": \"happy\",
  \"assertions\": [
    { \"path\": \"/0\", \"kind\": \"paragraph\" },
    { \"path\": \"/0/0\", \"kind\": \"text\", \"props\": { \"content\": \"before\\u00a0after\" } }
  ]
}",
    );

    // ── meta-description ─────────────────────────────────────────────────────
    write_fixture_meta("meta-description",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Body.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:dc="http://purl.org/dc/elements/1.1/">
  <office:meta>
    <dc:description>A test document.</dc:description>
  </office:meta>
</office:document-meta>"#,
        r#"{
  "description": "ODT document description from meta.xml (dc:description)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document", "metadata": { "description": "A test document." } }
  ]
}"#,
    );

    // ── meta-date ─────────────────────────────────────────────────────────────
    write_fixture_meta("meta-date",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Body.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:dc="http://purl.org/dc/elements/1.1/">
  <office:meta>
    <dc:date>2024-01-15T10:30:00</dc:date>
  </office:meta>
</office:document-meta>"#,
        r#"{
  "description": "ODT document date from meta.xml (dc:date)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document", "metadata": { "date": "2024-01-15T10:30:00" } }
  ]
}"#,
    );

    // ── meta-language ─────────────────────────────────────────────────────────
    write_fixture_meta("meta-language",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Body.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:dc="http://purl.org/dc/elements/1.1/">
  <office:meta>
    <dc:language>fr-FR</dc:language>
  </office:meta>
</office:document-meta>"#,
        r#"{
  "description": "ODT document language from meta.xml (dc:language)",
  "category": "happy",
  "assertions": [
    { "path": "/", "kind": "document", "metadata": { "language": "fr-FR" } }
  ]
}"#,
    );

    // ── para-style-name ───────────────────────────────────────────────────────
    write_fixture("para-style-name",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p text:style-name="Text_20_Body">Styled paragraph.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT paragraph style name preserved as odt:style-name property",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph", "props": { "odt:style-name": "Text_20_Body" } },
    { "path": "/0/0", "kind": "text", "props": { "content": "Styled paragraph." } }
  ]
}"#,
    );

    // ── colspan-rowspan ───────────────────────────────────────────────────────
    write_fixture("colspan-rowspan",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0">
  <office:body>
    <office:text>
      <table:table>
        <table:table-row>
          <table:table-cell table:number-columns-spanned="2"><text:p>Wide</text:p></table:table-cell>
          <table:covered-table-cell/>
        </table:table-row>
        <table:table-row>
          <table:table-cell table:number-rows-spanned="2"><text:p>Tall</text:p></table:table-cell>
          <table:table-cell><text:p>B</text:p></table:table-cell>
        </table:table-row>
        <table:table-row>
          <table:covered-table-cell/>
          <table:table-cell><text:p>C</text:p></table:table-cell>
        </table:table-row>
      </table:table>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT table with colspan and rowspan",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "table" },
    { "path": "/0/0/0", "kind": "table_cell", "props": { "colspan": 2 } },
    { "path": "/0/1/0", "kind": "table_cell", "props": { "rowspan": 2 } }
  ]
}"#,
    );

    // ── footnote ──────────────────────────────────────────────────────────────
    write_fixture("footnote",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Body text<text:note text:id="ftn1" text:note-class="footnote">
        <text:note-citation>1</text:note-citation>
        <text:note-body>
          <text:p>Footnote content here.</text:p>
        </text:note-body>
      </text:note> continues.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT footnote (text:note with note-class=footnote)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Body text" } },
    { "path": "/0/1", "kind": "footnote_ref", "props": { "label": "ftn1" } },
    { "path": "/1", "kind": "footnote_def", "props": { "label": "ftn1" } }
  ]
}"#,
    );

    // ── endnote ───────────────────────────────────────────────────────────────
    write_fixture("endnote",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Body text<text:note text:id="edn1" text:note-class="endnote">
        <text:note-citation>i</text:note-citation>
        <text:note-body>
          <text:p>Endnote content.</text:p>
        </text:note-body>
      </text:note> more.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT endnote (text:note with note-class=endnote)",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/1", "kind": "footnote_ref", "props": { "label": "edn1" } },
    { "path": "/1", "kind": "footnote_def", "props": { "label": "edn1" } }
  ]
}"#,
    );

    // ── bookmark ──────────────────────────────────────────────────────────────
    write_fixture("bookmark",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Before <text:bookmark text:name="anchor1"/>anchor after.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT bookmark (text:bookmark) becomes span with id",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Before " } },
    { "path": "/0/1", "kind": "span", "props": { "id": "anchor1" } },
    { "path": "/0/2", "kind": "text", "props": { "content": "anchor after." } }
  ]
}"#,
    );

    // ── horizontal-rule ───────────────────────────────────────────────────────
    write_fixture("horizontal-rule",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Before rule.</text:p>
      <text:p text:style-name="Horizontal Line"/>
      <text:p>After rule.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT horizontal rule via Horizontal Line paragraph style",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/1", "kind": "horizontal_rule" },
    { "path": "/2", "kind": "paragraph" }
  ]
}"#,
    );

    // ── Composition ──────────────────────────────────────────────────────────

    // ── table-cells-formatted ────────────────────────────────────────────────
    write_fixture("table-cells-formatted",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:font-weight="bold"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <table:table>
        <table:table-row>
          <table:table-cell><text:p><text:span text:style-name="T1">Bold cell</text:span></text:p></table:table-cell>
          <table:table-cell><text:p>Plain cell</text:p></table:table-cell>
        </table:table-row>
      </table:table>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT table cells containing formatted inline content",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "table" },
    { "path": "/0/0/0", "kind": "table_cell" },
    { "path": "/0/0/0/0", "kind": "paragraph" },
    { "path": "/0/0/0/0/0", "kind": "strong" }
  ]
}"#,
    );

    // ── list-items-formatted ─────────────────────────────────────────────────
    write_fixture("list-items-formatted",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:font-style="italic"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:list>
        <text:list-item><text:p><text:span text:style-name="T1">Italic item</text:span></text:p></text:list-item>
        <text:list-item><text:p>Plain item</text:p></text:list-item>
      </text:list>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT list items with inline formatting",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "list" },
    { "path": "/0/0", "kind": "list_item" },
    { "path": "/0/1", "kind": "list_item" }
  ]
}"#,
    );

    // ── heading-formatted ─────────────────────────────────────────────────────
    write_fixture("heading-formatted",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:font-weight="bold"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:h text:outline-level="1">Plain <text:span text:style-name="T1">bold</text:span> heading</text:h>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT heading containing inline bold formatting",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "heading", "props": { "level": 1 } },
    { "path": "/0/0", "kind": "text", "props": { "content": "Plain " } },
    { "path": "/0/1", "kind": "strong" },
    { "path": "/0/2", "kind": "text", "props": { "content": " heading" } }
  ]
}"#,
    );

    // ── link-formatted ────────────────────────────────────────────────────────
    write_fixture("link-formatted",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:xlink="http://www.w3.org/1999/xlink"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:font-weight="bold"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>See <text:a xlink:href="https://example.com" xlink:type="simple"><text:span text:style-name="T1">bold link</text:span></text:a>.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT hyperlink containing formatted text",
  "category": "happy",
  "assertions": [
    { "path": "/0/1", "kind": "link", "props": { "url": "https://example.com" } },
    { "path": "/0/1/0", "kind": "strong" }
  ]
}"#,
    );

    // ── Adversarial ──────────────────────────────────────────────────────────

    // ── adv-empty ─────────────────────────────────────────────────────────────
    write_fixture("adv-empty",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text/>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT with empty body produces document with no children",
  "category": "adversarial",
  "assertions": [
    { "path": "", "kind": "document", "children_count": 0 }
  ]
}"#,
    );

    // ── adv-unknown-namespace ─────────────────────────────────────────────────
    write_fixture("adv-unknown-namespace",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:custom="http://example.com/custom/1.0">
  <office:body>
    <office:text>
      <text:p>Normal text.</text:p>
      <custom:unknown-element custom:attr="value"/>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT with unknown XML namespace parses body content normally",
  "category": "adversarial",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Normal text." } }
  ]
}"#,
    );

    // ── adv-corrupt-styles ────────────────────────────────────────────────────
    write_fixture_raw("adv-corrupt-styles",
        make_odt_corrupt_styles(),
        r#"{
  "description": "ODT with corrupt styles.xml still parses body content",
  "category": "adversarial",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Body text." } }
  ]
}"#,
    );

    // ── adv-missing-content ───────────────────────────────────────────────────
    write_fixture_raw("adv-missing-content",
        make_odt_no_content(),
        r#"{
  "description": "ODT zip missing content.xml returns a parse error",
  "category": "adversarial",
  "expect_error": true,
  "assertions": []
}"#,
    );

    // ── adv-malformed-zip ─────────────────────────────────────────────────────
    write_fixture_raw("adv-malformed-zip",
        b"this is not a zip file at all".to_vec(),
        r#"{
  "description": "Non-zip input returns a parse error",
  "category": "adversarial",
  "expect_error": true,
  "assertions": []
}"#,
    );

    // ── adv-wrong-mimetype ────────────────────────────────────────────────────
    write_fixture_raw("adv-wrong-mimetype",
        make_odt_wrong_mimetype(),
        r#"{
  "description": "ODT zip with wrong mimetype (spreadsheet) returns parse error",
  "category": "adversarial",
  "expect_error": true,
  "assertions": []
}"#,
    );

    // ── annotation ───────────────────────────────────────────────────────────
    write_fixture("annotation",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Annotated<office:annotation><dc:creator xmlns:dc="http://purl.org/dc/elements/1.1/">Alice</dc:creator><text:p>A comment.</text:p></office:annotation> word.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT annotation (office:annotation) produces span with odt:annotation prop",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Annotated" } },
    { "path": "/0/1", "kind": "span", "props": { "odt:annotation": "A comment." } },
    { "path": "/0/2", "kind": "text", "props": { "content": " word." } }
  ]
}"#,
    );

    // ── text-box ─────────────────────────────────────────────────────────────
    write_fixture("text-box",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
  xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0">
  <office:body>
    <office:text>
      <draw:frame draw:name="TextBox1" svg:width="10cm" svg:height="3cm">
        <draw:text-box>
          <text:p>Text box content.</text:p>
        </draw:text-box>
      </draw:frame>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "ODT draw:text-box produces a div with paragraph content",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "div" },
    { "path": "/0/0", "kind": "paragraph" },
    { "path": "/0/0/0", "kind": "text", "props": { "content": "Text box content." } }
  ]
}"#,
    );

    // ── image ────────────────────────────────────────────────────────────────
    write_fixture_image("image",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
  xmlns:xlink="http://www.w3.org/1999/xlink"
  xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0">
  <office:body>
    <office:text>
      <text:p>Before <draw:frame draw:name="img1" svg:width="2cm" svg:height="2cm">
        <draw:image xlink:href="Pictures/test.png" xlink:type="simple" xlink:show="embed"/>
      </draw:frame> after.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        "Pictures/test.png",
        tiny_png(),
        r#"{
  "description": "ODT draw:frame/draw:image embeds image node inline",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "text", "props": { "content": "Before " } },
    { "path": "/0/1", "kind": "image" },
    { "path": "/0/2", "kind": "text", "props": { "content": " after." } }
  ]
}"#,
    );

    // ── meta-custom ───────────────────────────────────────────────────────────
    write_fixture_meta("meta-custom",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Content.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0">
  <office:meta>
    <meta:user-defined meta:name="Project">RescribeDemo</meta:user-defined>
    <meta:user-defined meta:name="Version">1.0</meta:user-defined>
  </office:meta>
</office:document-meta>"#,
        r#"{
  "description": "ODT custom user-defined metadata preserved with meta: prefix",
  "category": "happy",
  "assertions": [],
  "metadata": {
    "meta:Project": "RescribeDemo",
    "meta:Version": "1.0"
  }
}"#,
    );

    // ── page-layout ───────────────────────────────────────────────────────────
    write_fixture_styles("page-layout",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>Content.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:page-layout style:name="Mpm1">
      <style:page-layout-properties
        fo:page-width="21cm"
        fo:page-height="29.7cm"
        fo:margin-top="2cm"
        fo:margin-bottom="2cm"
        fo:margin-left="2.5cm"
        fo:margin-right="2.5cm"/>
    </style:page-layout>
  </office:automatic-styles>
</office:document-styles>"#,
        r#"{
  "description": "ODT page size and margins preserved in document metadata",
  "category": "happy",
  "assertions": [],
  "metadata": {
    "page-width": "21cm",
    "page-height": "29.7cm",
    "margin-top": "2cm",
    "margin-bottom": "2cm",
    "margin-left": "2.5cm",
    "margin-right": "2.5cm"
  }
}"#,
    );

    // ── footnote-formatted ────────────────────────────────────────────────────
    write_fixture("footnote-formatted",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
  <office:automatic-styles>
    <style:style style:name="T1" style:family="text">
      <style:text-properties fo:font-weight="bold"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:text>
      <text:p>Body<text:note text:id="ftn1" text:note-class="footnote">
        <text:note-citation>1</text:note-citation>
        <text:note-body>
          <text:p>See <text:span text:style-name="T1">important</text:span> ref.</text:p>
        </text:note-body>
      </text:note>.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "Footnote body with bold inline formatting preserved",
  "category": "composition",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/1", "kind": "footnote_ref", "props": { "label": "ftn1" } },
    { "path": "/1", "kind": "footnote_def", "props": { "label": "ftn1" } },
    { "path": "/1/0", "kind": "paragraph" },
    { "path": "/1/0/0", "kind": "text", "props": { "content": "See " } },
    { "path": "/1/0/1", "kind": "strong" },
    { "path": "/1/0/1/0", "kind": "text", "props": { "content": "important" } },
    { "path": "/1/0/2", "kind": "text", "props": { "content": " ref." } }
  ]
}"#,
    );

    // ── nested-blockquote ─────────────────────────────────────────────────────
    // ODT has no native nested blockquote; consecutive Quotations paragraphs merge into one.
    // This fixture verifies that behavior.
    write_fixture("nested-blockquote",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p text:style-name="Quotations">First quote.</text:p>
      <text:p text:style-name="Quotations">Second quote.</text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        r#"{
  "description": "Consecutive Quotations paragraphs merge into a single blockquote",
  "category": "happy",
  "assertions": [
    { "path": "/0", "kind": "blockquote", "children_count": 2 },
    { "path": "/0/0", "kind": "paragraph" },
    { "path": "/0/0/0", "kind": "text", "props": { "content": "First quote." } },
    { "path": "/0/1", "kind": "paragraph" },
    { "path": "/0/1/0", "kind": "text", "props": { "content": "Second quote." } }
  ]
}"#,
    );

    // ── image-caption ─────────────────────────────────────────────────────────
    write_fixture_image("image-caption",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
  xmlns:xlink="http://www.w3.org/1999/xlink"
  xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0">
  <office:body>
    <office:text>
      <draw:frame draw:name="fig1" svg:width="5cm" svg:height="4cm">
        <draw:image xlink:href="Pictures/photo.png" xlink:type="simple"/>
        <draw:text-box>
          <text:p>Figure 1: A photo.</text:p>
        </draw:text-box>
      </draw:frame>
    </office:text>
  </office:body>
</office:document-content>"#,
        "Pictures/photo.png",
        tiny_png(),
        r#"{
  "description": "ODT draw:frame with image and caption text-box; image takes priority",
  "category": "composition",
  "assertions": [
    { "path": "/0", "kind": "image" }
  ]
}"#,
    );

    // ── adv-corrupt-image ─────────────────────────────────────────────────────
    write_fixture_image("adv-corrupt-image",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
  xmlns:xlink="http://www.w3.org/1999/xlink"
  xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0">
  <office:body>
    <office:text>
      <text:p><draw:frame draw:name="bad" svg:width="2cm" svg:height="2cm">
        <draw:image xlink:href="Pictures/corrupt.png" xlink:type="simple"/>
      </draw:frame></text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
        "Pictures/corrupt.png",
        b"NOT A VALID PNG\x00\xff\xfe".to_vec(),
        r#"{
  "description": "ODT with corrupt image binary still parses; image node present with src",
  "category": "adversarial",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/0/0", "kind": "image" }
  ]
}"#,
    );

    // ── Pathological ──────────────────────────────────────────────────────────

    // ── path-many-paragraphs ──────────────────────────────────────────────────
    {
        let paras: String = (0..1000)
            .map(|i| format!("      <text:p>Paragraph number {i}.</text:p>\n"))
            .collect();
        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
{paras}    </office:text>
  </office:body>
</office:document-content>"#
        );
        write_fixture("path-many-paragraphs", &content,
            r#"{
  "description": "Document with 1000 paragraphs parses without error",
  "category": "pathological",
  "assertions": [
    { "path": "/0", "kind": "paragraph" },
    { "path": "/999", "kind": "paragraph" }
  ]
}"#,
        );
    }

    // ── path-many-char-runs ───────────────────────────────────────────────────
    {
        let spans: String = (0..200)
            .map(|i| {
                if i % 2 == 0 {
                    format!(r#"run{i}"#)
                } else {
                    format!(r#"<text:span>run{i}</text:span>"#)
                }
            })
            .collect::<Vec<_>>()
            .join("");
        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
      <text:p>{spans}</text:p>
    </office:text>
  </office:body>
</office:document-content>"#
        );
        write_fixture("path-many-char-runs", &content,
            r#"{
  "description": "Paragraph with 200 interleaved text runs parses without error",
  "category": "pathological",
  "assertions": [
    { "path": "/0", "kind": "paragraph" }
  ]
}"#,
        );
    }

    // ── path-deeply-nested-list ───────────────────────────────────────────────
    {
        // Build a list nested 6 levels deep
        let item_open: String = (0..6).map(|_|
            "      <text:list><text:list-item>\n".to_owned()
        ).collect();
        let item_close: String = (0..6).map(|_|
            "      </text:list-item></text:list>\n".to_owned()
        ).collect();
        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>
{item_open}        <text:p>Deep item.</text:p>
{item_close}    </office:text>
  </office:body>
</office:document-content>"#
        );
        write_fixture("path-deeply-nested-list", &content,
            r#"{
  "description": "List nested 6 levels deep parses without error",
  "category": "pathological",
  "assertions": [
    { "path": "/0", "kind": "list" }
  ]
}"#,
        );
    }

    // ── path-deeply-nested-table ──────────────────────────────────────────────
    {
        // 3 levels of table nesting
        let inner = r#"<table:table xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"><table:table-row><table:table-cell><text:p>inner</text:p></table:table-cell></table:table-row></table:table>"#;
        let mid = format!(
            r#"<table:table xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"><table:table-row><table:table-cell><text:p>{inner}</text:p></table:table-cell></table:table-row></table:table>"#
        );
        let content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0">
  <office:body>
    <office:text>
      <table:table><table:table-row><table:table-cell><text:p>{mid}</text:p></table:table-cell></table:table-row></table:table>
    </office:text>
  </office:body>
</office:document-content>"#
        );
        write_fixture("path-deeply-nested-table", &content,
            r#"{
  "description": "Table nested 3 levels deep parses without error",
  "category": "pathological",
  "assertions": [
    { "path": "/0", "kind": "table" }
  ]
}"#,
        );
    }

    // ── path-large-image ─────────────────────────────────────────────────────
    // 100 KB of "image" data (not valid PNG but should not panic)
    {
        let large_bytes = vec![0xffu8; 100_000];
        write_fixture_image("path-large-image",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
  xmlns:xlink="http://www.w3.org/1999/xlink"
  xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0">
  <office:body>
    <office:text>
      <text:p><draw:frame draw:name="big" svg:width="20cm" svg:height="20cm">
        <draw:image xlink:href="Pictures/large.bin" xlink:type="simple"/>
      </draw:frame></text:p>
    </office:text>
  </office:body>
</office:document-content>"#,
            "Pictures/large.bin",
            large_bytes,
            r#"{
  "description": "Document with 100KB embedded image parses without error",
  "category": "pathological",
  "assertions": [
    { "path": "/0/0", "kind": "image" }
  ]
}"#,
        );
    }
}
