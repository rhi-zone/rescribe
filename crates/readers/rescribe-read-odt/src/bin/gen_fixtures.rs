/// Generate ODT fixture files for the rescribe-read-odt test suite.
///
/// Run with: cargo run -p rescribe-read-odt --bin gen_fixtures
use std::io::{Cursor, Write};
use std::path::Path;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

fn make_odt(content_xml: &str) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buf);
    let opts = SimpleFileOptions::default();
    zip.start_file("mimetype", opts).unwrap();
    zip.write_all(b"application/vnd.oasis.opendocument.text").unwrap();
    zip.start_file("content.xml", opts).unwrap();
    zip.write_all(content_xml.as_bytes()).unwrap();
    zip.finish().unwrap();
    buf.into_inner()
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
}
