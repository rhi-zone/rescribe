/// Generate EPUB fixture files for the rescribe-read-epub test suite.
///
/// Run with: cargo run -p rescribe-read-epub --bin gen_fixtures
use rescribe_core::Node;
use std::io::Write;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

// ── EPUB construction helpers ──────────────────────────────────────────────

/// Full OPF package document with extended metadata fields.
fn opf_extended(
    title: &str,
    author: &str,
    lang: &str,
    publisher: Option<&str>,
    description: Option<&str>,
    date: Option<&str>,
    subject: Option<&str>,
    spine_items: &[(&str, &str)],
) -> String {
    let manifest_items: String = spine_items
        .iter()
        .map(|(id, href)| {
            format!(
                r#"    <item id="{id}" href="{href}" media-type="application/xhtml+xml"/>"#
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let spine_itemrefs: String = spine_items
        .iter()
        .map(|(id, _)| format!(r#"    <itemref idref="{id}"/>"#))
        .collect::<Vec<_>>()
        .join("\n");
    let extra: String = [
        publisher.map(|v| format!("    <dc:publisher>{v}</dc:publisher>")),
        description.map(|v| format!("    <dc:description>{v}</dc:description>")),
        date.map(|v| format!("    <dc:date>{v}</dc:date>")),
        subject.map(|v| format!("    <dc:subject>{v}</dc:subject>")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>{title}</dc:title>
    <dc:creator>{author}</dc:creator>
    <dc:language>{lang}</dc:language>
    <dc:identifier id="uid">urn:uuid:test-epub</dc:identifier>
{extra}
  </metadata>
  <manifest>
{manifest_items}
  </manifest>
  <spine>
{spine_itemrefs}
  </spine>
</package>"#
    )
}

/// Minimal OPF package document.
fn opf(title: &str, author: &str, lang: &str, spine_items: &[(&str, &str)]) -> String {
    let manifest_items: String = spine_items
        .iter()
        .map(|(id, href)| {
            format!(
                r#"    <item id="{id}" href="{href}" media-type="application/xhtml+xml"/>"#
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let spine_itemrefs: String = spine_items
        .iter()
        .map(|(id, _)| format!(r#"    <itemref idref="{id}"/>"#))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>{title}</dc:title>
    <dc:creator>{author}</dc:creator>
    <dc:language>{lang}</dc:language>
    <dc:identifier id="uid">urn:uuid:test-epub</dc:identifier>
  </metadata>
  <manifest>
{manifest_items}
  </manifest>
  <spine>
{spine_itemrefs}
  </spine>
</package>"#
    )
}

/// Minimal XHTML content document.
fn xhtml(title: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>{title}</title></head>
<body>
{body}
</body>
</html>"#
    )
}

/// Build a minimal EPUB zip from one or more (id, href, content) chapters.
fn make_epub_chapters(
    opf_str: &str,
    chapters: &[(&str, &str)], // (href, xhtml_content)
) -> Vec<u8> {
    let buf = std::io::Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(buf);
    // mimetype must be first and uncompressed
    let stored = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    zip.start_file("mimetype", stored).unwrap();
    zip.write_all(b"application/epub+zip").unwrap();

    let deflated = SimpleFileOptions::default();
    zip.start_file("META-INF/container.xml", deflated).unwrap();
    zip.write_all(
        br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
    )
    .unwrap();

    zip.start_file("OEBPS/content.opf", deflated).unwrap();
    zip.write_all(opf_str.as_bytes()).unwrap();

    for (href, content) in chapters {
        zip.start_file(format!("OEBPS/{href}"), deflated).unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }

    zip.finish().unwrap().into_inner()
}

/// Build a single-chapter EPUB.
fn make_epub(chapter_body: &str) -> Vec<u8> {
    let opf_str = opf("Test", "Author", "en", &[("ch1", "chapter1.xhtml")]);
    let ch = xhtml("Test", chapter_body);
    make_epub_chapters(&opf_str, &[("chapter1.xhtml", &ch)])
}

/// Build a single-chapter EPUB with custom OPF metadata.
fn make_epub_meta(title: &str, author: &str, lang: &str, body: &str) -> Vec<u8> {
    let opf_str = opf(title, author, lang, &[("ch1", "chapter1.xhtml")]);
    let ch = xhtml(title, body);
    make_epub_chapters(&opf_str, &[("chapter1.xhtml", &ch)])
}

/// Build a two-chapter EPUB.
fn make_epub_two_chapters(title1: &str, body1: &str, title2: &str, body2: &str) -> Vec<u8> {
    let opf_str = opf(
        "Book",
        "Author",
        "en",
        &[
            ("ch1", format!("{title1}.xhtml").as_str()),
            ("ch2", format!("{title2}.xhtml").as_str()),
        ],
    );
    // Use static names to avoid lifetime issues
    let ch1 = xhtml(title1, body1);
    let ch2 = xhtml(title2, body2);
    let entries: Vec<(&str, String)> = vec![
        (Box::leak(format!("{title1}.xhtml").into_boxed_str()), ch1),
        (Box::leak(format!("{title2}.xhtml").into_boxed_str()), ch2),
    ];
    let chapters: Vec<(&str, &str)> = entries.iter().map(|(h, c)| (*h, c.as_str())).collect();
    make_epub_chapters(&opf_str, &chapters)
}

// ── Expected JSON generation ───────────────────────────────────────────────

fn node_to_assertions(node: &Node, path: &str, out: &mut Vec<serde_json::Value>) {
    let kind = node.kind.as_str();
    let mut obj = serde_json::json!({
        "path": path,
        "kind": kind,
    });

    // Add important props
    let props_to_include = ["content", "level", "url", "title", "alt",
                            "ordered", "language", "style:bold", "style:italic",
                            "style:underline", "style:strikeout",
                            "style:subscript", "style:superscript", "style:code"];
    let mut props_map = serde_json::Map::new();
    for key in &props_to_include {
        if let Some(val) = node.props.get(key) {
            props_map.insert(key.to_string(), serde_json::Value::String(format!("{val:?}")));
        }
    }
    // Use raw string for string props (override the debug-format values above)
    for key in &["content", "url", "alt", "title", "language"] {
        if let Some(val) = node.props.get_str(key) {
            props_map.insert(key.to_string(), serde_json::Value::String(val.to_string()));
        }
    }
    if let Some(level) = node.props.get_int("level") {
        props_map.insert("level".to_string(), serde_json::Value::Number(level.into()));
    }
    if let Some(ordered) = node.props.get_bool("ordered") {
        props_map.insert("ordered".to_string(), serde_json::Value::Bool(ordered));
    }
    if !props_map.is_empty() {
        obj["props"] = serde_json::Value::Object(props_map);
    }

    out.push(obj);

    for (i, child) in node.children.iter().enumerate() {
        let child_path = if path == "/" {
            format!("/{i}")
        } else {
            format!("{path}/{i}")
        };
        node_to_assertions(child, &child_path, out);
    }
}

fn generate_expected_json(desc: &str, epub_bytes: &[u8]) -> String {
    let result = rescribe_read_epub::parse_bytes(epub_bytes)
        .expect("parse failed");
    let doc = result.value;

    let mut assertions: Vec<serde_json::Value> = Vec::new();

    // Document-level assertion
    let mut doc_obj = serde_json::json!({
        "path": "/",
        "kind": "document",
    });
    // Include non-empty metadata
    let important_meta = ["title", "author", "language", "publisher", "date", "identifier"];
    let mut meta_map = serde_json::Map::new();
    for key in &important_meta {
        if let Some(val) = doc.metadata.get_str(key) && !val.is_empty() {
            meta_map.insert(key.to_string(), serde_json::Value::String(val.to_string()));
        }
    }
    if !meta_map.is_empty() {
        doc_obj["metadata"] = serde_json::Value::Object(meta_map);
    }
    assertions.push(doc_obj);

    // Children
    for (i, child) in doc.content.children.iter().enumerate() {
        node_to_assertions(child, &format!("/{i}"), &mut assertions);
    }

    let obj = serde_json::json!({
        "description": desc,
        "assertions": assertions,
    });
    serde_json::to_string_pretty(&obj).unwrap()
}

// ── Fixture writing ────────────────────────────────────────────────────────

fn write_fixture(name: &str, epub_bytes: Vec<u8>, desc: &str) {
    let dir = format!("fixtures/epub/{name}");
    std::fs::create_dir_all(&dir).unwrap();
    let expected = generate_expected_json(desc, &epub_bytes);
    std::fs::write(format!("{dir}/input.epub"), &epub_bytes).unwrap();
    std::fs::write(format!("{dir}/expected.json"), &expected).unwrap();
    println!("wrote {dir}/");
}

// ── Main ───────────────────────────────────────────────────────────────────

fn main() {
    // ── Block constructs ──────────────────────────────────────────────────

    write_fixture(
        "heading",
        make_epub("<h1>Chapter One</h1><p>Some text.</p>"),
        "EPUB heading h1 mapped to heading node with level 1",
    );

    write_fixture(
        "heading-levels",
        make_epub(
            "<h1>H1</h1><h2>H2</h2><h3>H3</h3><h4>H4</h4><h5>H5</h5><h6>H6</h6>",
        ),
        "EPUB headings h1-h6 mapped to heading nodes with levels 1-6",
    );

    write_fixture(
        "unordered-list",
        make_epub("<ul><li>Alpha</li><li>Beta</li><li>Gamma</li></ul>"),
        "EPUB unordered list mapped to list node with list_item children",
    );

    write_fixture(
        "ordered-list",
        make_epub("<ol><li>First</li><li>Second</li><li>Third</li></ol>"),
        "EPUB ordered list mapped to list node with ordered=true",
    );

    write_fixture(
        "nested-list",
        make_epub(
            "<ul><li>Item 1<ul><li>Item 1a</li><li>Item 1b</li></ul></li><li>Item 2</li></ul>",
        ),
        "EPUB nested list: list_item contains a child list",
    );

    write_fixture(
        "blockquote",
        make_epub("<blockquote><p>To be or not to be.</p></blockquote>"),
        "EPUB blockquote mapped to blockquote node",
    );

    write_fixture(
        "code-block",
        make_epub("<pre><code>fn main() {\n    println!(\"hello\");\n}</code></pre>"),
        "EPUB code block (pre>code) mapped to code_block node",
    );

    write_fixture(
        "horizontal-rule",
        make_epub("<p>Before.</p><hr/><p>After.</p>"),
        "EPUB hr mapped to horizontal_rule node",
    );

    write_fixture(
        "table",
        make_epub(
            "<table><tr><td>A</td><td>B</td></tr><tr><td>C</td><td>D</td></tr></table>",
        ),
        "EPUB table mapped to table/table_row/table_cell nodes",
    );

    write_fixture(
        "table-header",
        make_epub(
            "<table><thead><tr><th>Name</th><th>Age</th></tr></thead><tbody><tr><td>Alice</td><td>30</td></tr></tbody></table>",
        ),
        "EPUB table with thead/th mapped to table_header cells",
    );

    // ── Inline constructs ─────────────────────────────────────────────────

    write_fixture(
        "emphasis",
        make_epub("<p>This is <em>emphasized</em> text.</p>"),
        "EPUB em element mapped to emphasis node",
    );

    write_fixture(
        "strong",
        make_epub("<p>This is <strong>strong</strong> text.</p>"),
        "EPUB strong element mapped to strong node",
    );

    write_fixture(
        "underline",
        make_epub(r#"<p>This is <span style="text-decoration:underline">underlined</span> text.</p>"#),
        "EPUB underline via CSS style mapped to underline node",
    );

    write_fixture(
        "strikeout",
        make_epub("<p>This is <s>struck out</s> text.</p>"),
        "EPUB s element mapped to strikeout node",
    );

    write_fixture(
        "subscript",
        make_epub("<p>H<sub>2</sub>O</p>"),
        "EPUB sub element mapped to subscript node",
    );

    write_fixture(
        "superscript",
        make_epub("<p>E=mc<sup>2</sup></p>"),
        "EPUB sup element mapped to superscript node",
    );

    write_fixture(
        "inline-code",
        make_epub("<p>Use <code>cargo build</code> to compile.</p>"),
        "EPUB inline code element mapped to code node",
    );

    write_fixture(
        "link",
        make_epub(r#"<p>Visit <a href="https://example.com">Example</a> for more.</p>"#),
        "EPUB anchor with href mapped to link node with url property",
    );

    write_fixture(
        "line-break",
        make_epub("<p>Line one.<br/>Line two.</p>"),
        "EPUB br element mapped to line_break node",
    );

    // ── Metadata ──────────────────────────────────────────────────────────

    write_fixture(
        "metadata-full",
        make_epub_meta(
            "Full Metadata Book",
            "Jane Author",
            "en",
            "<p>Content here.</p>",
        ),
        "EPUB metadata: title, author, language extracted to document properties",
    );

    // ── Multi-chapter ─────────────────────────────────────────────────────

    write_fixture(
        "two-chapters",
        make_epub_two_chapters(
            "Introduction",
            "<p>Welcome to the book.</p>",
            "Conclusion",
            "<p>Thanks for reading.</p>",
        ),
        "Two-chapter EPUB: each chapter generates a heading and paragraph",
    );

    // ── Composition ───────────────────────────────────────────────────────

    write_fixture(
        "mixed-content",
        make_epub(
            r#"<h1>Chapter</h1>
<p>A paragraph with <em>emphasis</em> and <strong>bold</strong> text.</p>
<ul><li>Item one</li><li>Item two</li></ul>
<blockquote><p>A blockquote.</p></blockquote>"#,
        ),
        "EPUB chapter with heading, paragraph with inline formatting, list, and blockquote",
    );

    write_fixture(
        "nested-inline",
        make_epub("<p>This is <strong><em>bold italic</em></strong> text.</p>"),
        "EPUB nested inline formatting: strong>em produces strong>emphasis nesting",
    );

    // ── Adversarial ───────────────────────────────────────────────────────

    write_fixture(
        "empty-chapter",
        make_epub(""),
        "EPUB with empty chapter body produces no content nodes",
    );

    write_fixture(
        "special-chars",
        make_epub("<p>Symbols: &lt;angle&gt; &amp; &quot;quotes&quot; \u{2014} em-dash.</p>"),
        "EPUB HTML entities and Unicode decoded correctly in text content",
    );

    // ── EPUB2-style (OPF 2.0) ─────────────────────────────────────────────

    let opf2 = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">
    <dc:title>EPUB2 Book</dc:title>
    <dc:creator opf:role="aut">Old Author</dc:creator>
    <dc:language>en</dc:language>
    <dc:identifier id="uid">urn:uuid:epub2-test</dc:identifier>
  </metadata>
  <manifest>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine toc="ncx">
    <itemref idref="ch1"/>
  </spine>
</package>"#;
    let ch2 = xhtml("EPUB2 Book", "<p>An EPUB 2.0 style document.</p>");
    write_fixture(
        "epub2-compat",
        make_epub_chapters(opf2, &[("chapter1.xhtml", &ch2)]),
        "EPUB2 OPF 2.0 document parsed successfully (no nav doc required)",
    );

    // ── Pathological ──────────────────────────────────────────────────────

    let many_paras: String = (1..=50)
        .map(|i| format!("<p>Paragraph {i} with some text content here.</p>"))
        .collect::<Vec<_>>()
        .join("\n");
    write_fixture(
        "path-many-paragraphs",
        make_epub(&many_paras),
        "Chapter with 50 paragraphs produces 50 paragraph nodes",
    );

    // ── Block constructs (additional) ─────────────────────────────────────

    write_fixture(
        "figure-with-caption",
        make_epub(
            r#"<figure><img src="" alt="A diagram"/><figcaption>Figure 1: A diagram caption</figcaption></figure>"#,
        ),
        "EPUB figure with figcaption mapped to figure node",
    );

    write_fixture(
        "definition-list",
        make_epub(
            "<dl><dt>Term 1</dt><dd>Definition of term 1.</dd><dt>Term 2</dt><dd>Definition of term 2.</dd></dl>",
        ),
        "EPUB definition list (dl/dt/dd) mapped to definition_list nodes",
    );

    write_fixture(
        "section-div",
        make_epub(
            r#"<section><p>Inside a section.</p></section><div><p>Inside a div.</p></div>"#,
        ),
        "EPUB section and div elements produce their contained block content",
    );

    // ── Inline constructs (additional) ────────────────────────────────────

    write_fixture(
        "span-style",
        make_epub(
            r#"<p>Text with <span style="color:red">red</span> and <span class="highlight">highlighted</span> spans.</p>"#,
        ),
        "EPUB span elements with style/class attributes produce span nodes",
    );

    write_fixture(
        "cross-document-link",
        {
            let opf_str = opf(
                "Cross-Links",
                "Author",
                "en",
                &[("ch1", "chapter1.xhtml"), ("ch2", "chapter2.xhtml")],
            );
            let ch1 = xhtml("Chapter 1", "<h1>Chapter 1</h1><p>First chapter content.</p>");
            let ch2 = xhtml(
                "Chapter 2",
                r#"<h1>Chapter 2</h1><p>See also <a href="chapter1.xhtml">Chapter 1</a>.</p>"#,
            );
            make_epub_chapters(&opf_str, &[("chapter1.xhtml", &ch1), ("chapter2.xhtml", &ch2)])
        },
        "EPUB cross-document link (href to another spine item) parsed as link node",
    );

    // ── Metadata (additional) ─────────────────────────────────────────────

    write_fixture(
        "metadata-extended",
        {
            let opf_str = opf_extended(
                "Extended Metadata Book",
                "Jane Author",
                "en",
                Some("Acme Publishing"),
                Some("A book about testing EPUB metadata"),
                Some("2024-01-15"),
                Some("Testing; EPUB; Metadata"),
                &[("ch1", "chapter1.xhtml")],
            );
            let ch = xhtml("Test", "<p>Content here.</p>");
            make_epub_chapters(&opf_str, &[("chapter1.xhtml", &ch)])
        },
        "EPUB extended metadata: publisher, description, date, subject extracted",
    );

    // ── Adversarial (additional) ──────────────────────────────────────────

    // Invalid XHTML (well-formed enough for the ZIP, but bad body content)
    write_fixture(
        "adv-invalid-xhtml",
        make_epub("<p>Valid paragraph.</p><p>Unclosed tag <em>emphasis without close</p>"),
        "EPUB chapter with unclosed tag is recovered by the HTML parser",
    );

    // Empty spine: OPF exists but spine has no items
    write_fixture(
        "adv-empty-spine",
        {
            let opf_str = r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Empty Spine</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="uid">urn:uuid:empty-spine</dc:identifier>
  </metadata>
  <manifest/>
  <spine/>
</package>"#;
            make_epub_chapters(opf_str, &[])
        },
        "EPUB with no spine items produces an empty document without panic",
    );

    // ── Pathological (additional) ─────────────────────────────────────────

    write_fixture(
        "path-many-chapters",
        {
            let num = 20u32;
            let spine_items: Vec<(String, String)> = (1..=num)
                .map(|i| (format!("ch{i}"), format!("chapter{i}.xhtml")))
                .collect();
            let spine_refs: Vec<(&str, &str)> = spine_items
                .iter()
                .map(|(id, href)| (id.as_str(), href.as_str()))
                .collect();
            let opf_str = opf("Many Chapters", "Author", "en", &spine_refs);
            let chapters: Vec<(String, String)> = (1..=num)
                .map(|i| {
                    let href = format!("chapter{i}.xhtml");
                    let content = xhtml(&format!("Chapter {i}"), &format!("<p>Content of chapter {i}.</p>"));
                    (href, content)
                })
                .collect();
            let chapter_refs: Vec<(&str, &str)> =
                chapters.iter().map(|(h, c)| (h.as_str(), c.as_str())).collect();
            make_epub_chapters(&opf_str, &chapter_refs)
        },
        "EPUB with 20 chapters all parsed without panic",
    );

    println!("Done.");
}
