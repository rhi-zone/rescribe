//! FictionBook 2 (FB2) reader for rescribe.
//!
//! Thin adapter over `fb2-fmt`. Converts native FB2 AST → rescribe IR.

use fb2_fmt::{
    AnnotationContent, Body, Cite, CiteContent, Epigraph, EpigraphContent, FictionBook,
    InlineElement, Poem, Section, SectionContent, Stanza, Table, TitlePara,
};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, ParseOptions, Properties,
    Resource, ResourceId, ResourceMap, Severity, WarningKind,
};
use rescribe_std::{node, prop};

/// Parse FB2 XML into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse FB2 XML into a document with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (fb, diags) = fb2_fmt::parse_str(input);

    let warnings: Vec<FidelityWarning> = diags
        .iter()
        .map(|d| {
            FidelityWarning::new(
                Severity::Minor,
                WarningKind::FeatureLost(d.message.clone()),
                d.message.clone(),
            )
        })
        .collect();

    let metadata = build_metadata(&fb);
    let content_nodes = convert_fb(&fb);
    let resources = convert_resources(&fb);

    let document = Document {
        content: Node::new(node::DOCUMENT).children(content_nodes),
        resources,
        metadata,
        source: None,
    };

    Ok(ConversionResult::with_warnings(document, warnings))
}

fn build_metadata(fb: &FictionBook) -> Properties {
    let mut meta = Properties::new();
    let ti = &fb.description.title_info;
    if !ti.book_title.is_empty() {
        meta.set("title", ti.book_title.clone());
    }
    if let Some(author) = ti.author.first() {
        let name = author.display_name();
        if !name.is_empty() {
            meta.set("author", name);
        }
    }
    if let Some(genre) = ti.genre.first()
        && !genre.is_empty()
    {
        meta.set("genre", genre.clone());
    }
    if !ti.lang.is_empty() {
        meta.set("lang", ti.lang.clone());
    }
    if let Some(kw) = &ti.keywords {
        meta.set("keywords", kw.clone());
    }
    if let Some(ann) = &ti.annotation {
        let text = extract_annotation_text(ann);
        if !text.is_empty() {
            meta.set("meta:annotation", text);
        }
    }
    meta
}

/// Extract plain text from an Annotation by concatenating text content of all paragraphs.
fn extract_annotation_text(ann: &fb2_fmt::Annotation) -> String {
    let mut parts: Vec<String> = Vec::new();
    for item in &ann.content {
        match item {
            AnnotationContent::Para(inlines) => {
                parts.push(extract_inline_text(inlines));
            }
            AnnotationContent::Subtitle(inlines) => {
                parts.push(extract_inline_text(inlines));
            }
            _ => {}
        }
    }
    parts.join(" ")
}

fn extract_inline_text(inlines: &[InlineElement]) -> String {
    let mut s = String::new();
    for el in inlines {
        match el {
            InlineElement::Text(t) => s.push_str(t),
            InlineElement::Strong(ch)
            | InlineElement::Emphasis(ch)
            | InlineElement::Strikethrough(ch)
            | InlineElement::Sub(ch)
            | InlineElement::Sup(ch) => s.push_str(&extract_inline_text(ch)),
            InlineElement::Code(t) => s.push_str(t),
            InlineElement::Link { children, .. } => s.push_str(&extract_inline_text(children)),
            InlineElement::FootnoteRef { children, .. } => {
                s.push_str(&extract_inline_text(children));
            }
            InlineElement::Image(_) => {}
        }
    }
    s
}

fn convert_fb(fb: &FictionBook) -> Vec<Node> {
    fb.bodies.iter().flat_map(convert_body).collect()
}

fn convert_body(body: &Body) -> Vec<Node> {
    if body.name.as_deref() == Some("notes") {
        // Notes body: each section is a footnote definition
        return body
            .section
            .iter()
            .map(convert_footnote_def)
            .collect();
    }
    let sections: Vec<Node> = body.section.iter().map(|s| convert_section(s, 1)).collect();
    vec![Node::new(node::DIV).children(sections)]
}

fn convert_footnote_def(section: &Section) -> Node {
    let mut children: Vec<Node> = Vec::new();
    for item in &section.content {
        children.extend(convert_section_content(item));
    }
    for nested in &section.section {
        children.push(convert_section(nested, 1));
    }
    let mut n = Node::new(node::FOOTNOTE_DEF).children(children);
    if let Some(id) = &section.id {
        n = n.prop(prop::ID, id.clone());
    }
    n
}

fn convert_section(section: &Section, depth: usize) -> Node {
    let mut children: Vec<Node> = Vec::new();
    let level = depth.clamp(1, 6) as i64;

    if let Some(title) = &section.title {
        let mut inlines = Vec::new();
        for para in &title.para {
            if let TitlePara::Para(il) = para {
                inlines.extend(convert_inlines(il));
            }
        }
        if !inlines.is_empty() {
            children.push(Node::new(node::HEADING).prop(prop::LEVEL, level).children(inlines));
        }
    }

    for epigraph in &section.epigraph {
        children.push(convert_epigraph(epigraph));
    }

    for item in &section.content {
        children.extend(convert_section_content(item));
    }

    for nested in &section.section {
        children.push(convert_section(nested, depth + 1));
    }

    let mut n = Node::new(node::DIV).children(children);
    if let Some(id) = &section.id {
        n = n.prop("id", id.clone());
    }
    n
}

fn convert_section_content(item: &SectionContent) -> Vec<Node> {
    match item {
        SectionContent::Para(inlines) => {
            vec![Node::new(node::PARAGRAPH).children(convert_inlines(inlines))]
        }
        SectionContent::EmptyLine => vec![Node::new(node::PARAGRAPH)],
        SectionContent::Image(img) => {
            let url = img.href.strip_prefix('#').unwrap_or(&img.href);
            vec![Node::new(node::IMAGE).prop(prop::URL, url.to_string())]
        }
        SectionContent::Poem(poem) => vec![convert_poem(poem)],
        SectionContent::Subtitle(inlines) => {
            let il = convert_inlines(inlines);
            vec![Node::new(node::HEADING).prop(prop::LEVEL, 4i64).children(il)]
        }
        SectionContent::Cite(cite) => vec![convert_cite(cite)],
        SectionContent::Table(table) => vec![convert_table(table)],
    }
}

fn convert_epigraph(epigraph: &Epigraph) -> Node {
    let mut children = Vec::new();
    for item in &epigraph.content {
        match item {
            EpigraphContent::Para(inlines) => {
                children.push(Node::new(node::PARAGRAPH).children(convert_inlines(inlines)));
            }
            EpigraphContent::EmptyLine => {
                children.push(Node::new(node::PARAGRAPH));
            }
            EpigraphContent::Poem(poem) => {
                children.push(convert_poem(poem));
            }
            EpigraphContent::Cite(cite) => {
                children.push(convert_cite(cite));
            }
        }
    }
    for ta in &epigraph.text_author {
        let il = convert_inlines(ta);
        children.push(
            Node::new(node::PARAGRAPH)
                .prop("html:class", "text-author")
                .children(il),
        );
    }
    Node::new(node::BLOCKQUOTE)
        .prop("fb2:type", "epigraph")
        .children(children)
}

fn convert_cite(cite: &Cite) -> Node {
    let mut children = Vec::new();
    for item in &cite.content {
        match item {
            CiteContent::Para(inlines) => {
                children.push(Node::new(node::PARAGRAPH).children(convert_inlines(inlines)));
            }
            CiteContent::EmptyLine => {
                children.push(Node::new(node::PARAGRAPH));
            }
            CiteContent::Poem(poem) => {
                children.push(convert_poem(poem));
            }
            CiteContent::Table(table) => {
                children.push(convert_table(table));
            }
        }
    }
    for ta in &cite.text_author {
        let il = convert_inlines(ta);
        children.push(
            Node::new(node::PARAGRAPH)
                .prop("html:class", "text-author")
                .children(il),
        );
    }
    Node::new(node::BLOCKQUOTE).children(children)
}

fn convert_poem(poem: &Poem) -> Node {
    let children: Vec<Node> = poem.stanza.iter().map(convert_stanza).collect();
    Node::new(node::DIV).prop("html:class", "poem").children(children)
}

fn convert_stanza(stanza: &Stanza) -> Node {
    let mut children = Vec::new();
    for v in &stanza.v {
        let mut line_children = convert_inlines(v);
        line_children.push(Node::new(node::LINE_BREAK));
        children.push(Node::new(node::SPAN).children(line_children));
    }
    Node::new(node::DIV).prop("html:class", "stanza").children(children)
}

fn convert_table(table: &Table) -> Node {
    let rows: Vec<Node> = table
        .row
        .iter()
        .map(|row| {
            let cells: Vec<Node> = row
                .cell
                .iter()
                .map(|cell| {
                    let kind = if cell.is_header {
                        node::TABLE_HEADER
                    } else {
                        node::TABLE_CELL
                    };
                    let mut n = Node::new(kind).children(convert_inlines(&cell.content));
                    if let Some(align) = &cell.align {
                        n = n.prop("style:align", align.clone());
                    }
                    n
                })
                .collect();
            Node::new(node::TABLE_ROW).children(cells)
        })
        .collect();
    Node::new(node::TABLE).children(rows)
}

fn convert_inlines(inlines: &[InlineElement]) -> Vec<Node> {
    inlines.iter().map(convert_inline).collect()
}

fn convert_inline(el: &InlineElement) -> Node {
    match el {
        InlineElement::Text(s) => Node::new(node::TEXT).prop(prop::CONTENT, s.clone()),
        InlineElement::Strong(ch) => {
            Node::new(node::STRONG).children(convert_inlines(ch))
        }
        InlineElement::Emphasis(ch) => {
            Node::new(node::EMPHASIS).children(convert_inlines(ch))
        }
        InlineElement::Strikethrough(ch) => {
            Node::new(node::STRIKEOUT).children(convert_inlines(ch))
        }
        InlineElement::Sub(ch) => Node::new(node::SUBSCRIPT).children(convert_inlines(ch)),
        InlineElement::Sup(ch) => Node::new(node::SUPERSCRIPT).children(convert_inlines(ch)),
        InlineElement::Code(s) => Node::new(node::CODE).prop(prop::CONTENT, s.clone()),
        InlineElement::Image(img) => {
            let url = img.href.strip_prefix('#').unwrap_or(&img.href);
            Node::new(node::IMAGE).prop(prop::URL, url.to_string())
        }
        InlineElement::Link { href, kind, children } => {
            let mut n = Node::new(node::LINK)
                .prop(prop::URL, href.clone())
                .children(convert_inlines(children));
            if let Some(k) = kind {
                n = n.prop("fb2:link-type", k.clone());
            }
            n
        }
        InlineElement::FootnoteRef { href, children } => Node::new(node::FOOTNOTE_REF)
            .prop(prop::URL, href.clone())
            .children(convert_inlines(children)),
    }
}

fn convert_resources(fb: &FictionBook) -> ResourceMap {
    let mut resources = ResourceMap::default();
    for binary in &fb.binaries {
        resources.insert(
            ResourceId::from_string(binary.id.clone()),
            Resource {
                name: Some(binary.id.clone()),
                mime_type: binary.content_type.clone(),
                data: binary.data.clone(),
                metadata: Properties::new(),
            },
        );
    }
    resources
}

#[cfg(test)]
mod fixture_tests {
    use rescribe_fixtures::run_format_fixtures;
    use std::path::PathBuf;

    fn fixtures_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap() // crates/readers/
            .parent()
            .unwrap() // crates/
            .parent()
            .unwrap() // workspace root
            .join("fixtures")
    }

    #[test]
    fn fb2_fixtures() {
        run_format_fixtures(&fixtures_root(), "fb2", |input| {
            let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
            super::parse(s)
                .map(|r| r.value)
                .map_err(|e| e.to_string())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let fb2 = r#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description>
    <title-info><book-title>Test Book</book-title><lang>en</lang></title-info>
  </description>
  <body>
    <section><p>Hello, world!</p></section>
  </body>
</FictionBook>"#;

        let result = parse(fb2).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(doc.metadata.get_str("title"), Some("Test Book"));
    }

    #[test]
    fn test_parse_with_sections() {
        let fb2 = r#"<?xml version="1.0"?>
<FictionBook>
  <body>
    <section>
      <title><p>Chapter 1</p></title>
      <p>Content here.</p>
    </section>
    <section>
      <title><p>Chapter 2</p></title>
      <p>More content.</p>
    </section>
  </body>
</FictionBook>"#;

        let result = parse(fb2).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_inline_formatting() {
        let fb2 = r#"<?xml version="1.0"?>
<FictionBook>
  <body>
    <section>
      <p>This is <emphasis>italic</emphasis> and <strong>bold</strong> text.</p>
    </section>
  </body>
</FictionBook>"#;

        let result = parse(fb2).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_links() {
        let fb2 = r#"<?xml version="1.0"?>
<FictionBook xmlns:l="http://www.w3.org/1999/xlink">
  <body>
    <section>
      <p>Visit <a l:href="http://example.com">example</a>.</p>
    </section>
  </body>
</FictionBook>"#;

        let result = parse(fb2).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_cite() {
        let fb2 = r#"<?xml version="1.0"?>
<FictionBook>
  <body>
    <section>
      <cite>
        <p>A famous quote.</p>
        <text-author>Someone Famous</text-author>
      </cite>
    </section>
  </body>
</FictionBook>"#;

        let result = parse(fb2).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_poem() {
        let fb2 = r#"<?xml version="1.0"?>
<FictionBook>
  <body>
    <section>
      <poem>
        <stanza>
          <v>Line one</v>
          <v>Line two</v>
        </stanza>
      </poem>
    </section>
  </body>
</FictionBook>"#;

        let result = parse(fb2).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_empty_line() {
        let fb2 = r#"<?xml version="1.0"?>
<FictionBook>
  <body>
    <section>
      <p>Before</p>
      <empty-line/>
      <p>After</p>
    </section>
  </body>
</FictionBook>"#;

        let result = parse(fb2).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }
}
