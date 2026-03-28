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
}
