//! AST↔`rescribe::Document` translation for FictionBook 2 (FB2).
//!
//! This module only translates between [`FictionBook`](crate::FictionBook)
//! and rescribe's `Document` IR — no XML tokenizing/parsing/emitting
//! happens here (that all lives in the rest of this crate; see
//! `crate::parse_str` and the `Emit` trait impl). Enabled by the
//! `rescribe` feature; each direction is additionally gated on the
//! reader/writer mode feature it depends on, so enabling `rescribe` alone
//! (with no mode feature) compiles nothing.
//!
//! # Mapping
//!
//! Body sections become `div` nodes; a `notes` body's sections become
//! `footnote_def` nodes. Section titles become the first `heading`;
//! subsequent titles/subtitles become level-4 headings. `epigraph` and
//! `annotation` become `blockquote` nodes tagged `fb2:type`; `cite` becomes
//! an untagged `blockquote`. Inline formatting (`strong`/`emphasis`/
//! `strikethrough`/`sub`/`sup`/`code`/links/footnote refs/images) maps
//! directly onto the corresponding IR inline node kinds. Binaries become
//! `Document::resources` entries keyed by their FB2 `id`.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{
        Annotation, AnnotationContent, Body, Cite, CiteContent, Epigraph, EpigraphContent,
        FictionBook, InlineElement, Poem, Section, SectionContent, Stanza, Table, TitlePara,
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
        let (fb, diags) = crate::parse_str(input);

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
    fn extract_annotation_text(ann: &crate::Annotation) -> String {
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
            return body.section.iter().map(convert_footnote_def).collect();
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
                children.push(
                    Node::new(node::HEADING)
                        .prop(prop::LEVEL, level)
                        .children(inlines),
                );
            }
        }

        for epigraph in &section.epigraph {
            children.push(convert_epigraph(epigraph));
        }

        if let Some(ann) = &section.annotation {
            children.push(convert_annotation(ann));
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
                vec![
                    Node::new(node::HEADING)
                        .prop(prop::LEVEL, 4i64)
                        .children(il),
                ]
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

    /// Convert a section-level `<section><annotation>` (`Section.annotation`, distinct from
    /// `TitleInfo.annotation` which is instead flattened into `meta:annotation` document
    /// metadata — a section-level annotation has no natural document-wide metadata slot since
    /// a document can have many sections, so it is modeled structurally instead) into a
    /// `blockquote` node tagged `fb2:type = "annotation"`, mirroring how `convert_epigraph`
    /// models `Section.epigraph` as a `blockquote` tagged `fb2:type = "epigraph"`.
    fn convert_annotation(ann: &Annotation) -> Node {
        let mut children = Vec::new();
        for item in &ann.content {
            match item {
                AnnotationContent::Para(inlines) => {
                    children.push(Node::new(node::PARAGRAPH).children(convert_inlines(inlines)));
                }
                AnnotationContent::EmptyLine => {
                    children.push(Node::new(node::PARAGRAPH));
                }
                AnnotationContent::Subtitle(inlines) => {
                    let il = convert_inlines(inlines);
                    children.push(
                        Node::new(node::HEADING)
                            .prop(prop::LEVEL, 4i64)
                            .children(il),
                    );
                }
                AnnotationContent::Poem(poem) => {
                    children.push(convert_poem(poem));
                }
                AnnotationContent::Cite(cite) => {
                    children.push(convert_cite(cite));
                }
                AnnotationContent::Table(table) => {
                    children.push(convert_table(table));
                }
            }
        }
        let mut n = Node::new(node::BLOCKQUOTE)
            .prop("fb2:type", "annotation")
            .children(children);
        if let Some(id) = &ann.id {
            n = n.prop(prop::ID, id.clone());
        }
        n
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
        Node::new(node::DIV)
            .prop("html:class", "poem")
            .children(children)
    }

    fn convert_stanza(stanza: &Stanza) -> Node {
        let mut children = Vec::new();
        for v in &stanza.v {
            let mut line_children = convert_inlines(v);
            line_children.push(Node::new(node::LINE_BREAK));
            children.push(Node::new(node::SPAN).children(line_children));
        }
        Node::new(node::DIV)
            .prop("html:class", "stanza")
            .children(children)
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
            InlineElement::Strong(ch) => Node::new(node::STRONG).children(convert_inlines(ch)),
            InlineElement::Emphasis(ch) => Node::new(node::EMPHASIS).children(convert_inlines(ch)),
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
            InlineElement::Link {
                href,
                kind,
                children,
            } => {
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
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use crate::{
        Annotation, AnnotationContent, Author, Binary, Body, Cite, CiteContent, Description,
        DocumentInfo, FictionBook, Image, InlineElement, Poem, Section, SectionContent, Stanza,
        Table, TableCell, TableRow, Title, TitleInfo, TitlePara,
    };
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document to FB2 XML.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document to FB2 XML with options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let fb = doc_to_fb(doc);
        let bytes = fb.emit();
        Ok(ConversionResult::ok(bytes))
    }

    fn doc_to_fb(doc: &Document) -> FictionBook {
        let genre = doc.metadata.get_str("genre").unwrap_or("prose").to_string();
        let author_str = doc.metadata.get_str("author").unwrap_or("").to_string();
        let book_title = doc
            .metadata
            .get_str("title")
            .unwrap_or("Untitled")
            .to_string();
        let lang = doc.metadata.get_str("lang").unwrap_or("en").to_string();

        let author = if !author_str.is_empty() {
            let parts: Vec<&str> = author_str.splitn(2, ' ').collect();
            vec![Author {
                first_name: parts.first().map(|s| s.to_string()),
                last_name: if parts.len() >= 2 {
                    Some(parts[1].to_string())
                } else {
                    None
                },
                ..Default::default()
            }]
        } else {
            vec![]
        };

        let title_info = TitleInfo {
            genre: vec![genre],
            author,
            book_title,
            lang,
            ..Default::default()
        };

        let description = Description {
            title_info,
            document_info: Some(DocumentInfo {
                program_used: Some("rescribe".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let body_section = nodes_to_section(&doc.content.children);
        let body = Body {
            section: vec![body_section],
            ..Default::default()
        };

        // Collect footnote_def nodes from document content into a notes body
        let footnote_defs = collect_footnote_defs(&doc.content.children);
        let mut bodies = vec![body];
        if !footnote_defs.is_empty() {
            let notes_body = Body {
                name: Some("notes".to_string()),
                section: footnote_defs,
                ..Default::default()
            };
            bodies.push(notes_body);
        }

        let binaries: Vec<Binary> = doc
            .resources
            .iter()
            .map(|(id, res)| Binary {
                id: id.as_str().to_string(),
                content_type: res.mime_type.clone(),
                data: res.data.clone(),
            })
            .collect();

        FictionBook {
            description,
            bodies,
            binaries,
        }
    }

    /// Recursively collect all `footnote_def` nodes and convert them to FB2 sections.
    fn collect_footnote_defs(nodes: &[Node]) -> Vec<Section> {
        let mut sections = Vec::new();
        for node in nodes {
            if node.kind.as_str() == node::FOOTNOTE_DEF {
                let id = node.props.get_str(prop::ID).map(|s| s.to_string());
                let section = nodes_to_section(&node.children);
                sections.push(Section {
                    id,
                    content: section.content,
                    section: section.section,
                    title: section.title,
                    ..Default::default()
                });
            } else {
                // Recurse into children
                sections.extend(collect_footnote_defs(&node.children));
            }
        }
        sections
    }

    /// Convert a flat list of rescribe nodes into a single FB2 section.
    fn nodes_to_section(nodes: &[Node]) -> Section {
        let mut section_title: Option<Title> = None;
        let mut section_annotation: Option<Annotation> = None;
        let mut content = Vec::new();
        for node in nodes {
            match node.kind.as_str() {
                node::DOCUMENT => {
                    let inner = nodes_to_section(&node.children);
                    return inner;
                }
                node::DIV => {
                    // Recursively convert nested divs as sub-sections
                    // (this mirrors write_node's DIV handling)
                    let class = node.props.get_str("html:class");
                    match class {
                        Some("poem") => {
                            content.push(SectionContent::Poem(div_to_poem(node)));
                        }
                        Some("stanza") => {} // handled inside poem
                        _ => {
                            // A div becomes a nested section content — flatten
                            let inner = nodes_to_section(&node.children);
                            if inner.title.is_some() && section_title.is_none() {
                                section_title = inner.title;
                            }
                            if inner.annotation.is_some() && section_annotation.is_none() {
                                section_annotation = inner.annotation;
                            }
                            content.extend(inner.content);
                        }
                    }
                }
                node::HEADING => {
                    // First heading becomes section title; subsequent ones become subtitle
                    let inlines = nodes_to_inlines(&node.children);
                    if section_title.is_none() {
                        section_title = Some(Title {
                            para: vec![TitlePara::Para(inlines)],
                            ..Default::default()
                        });
                    } else {
                        content.push(SectionContent::Subtitle(inlines));
                    }
                }
                node::PARAGRAPH => {
                    if node.children.is_empty() {
                        content.push(SectionContent::EmptyLine);
                    } else {
                        content.push(SectionContent::Para(nodes_to_inlines(&node.children)));
                    }
                }
                node::BLOCKQUOTE => {
                    if node.props.get_str("fb2:type") == Some("annotation") {
                        if section_annotation.is_none() {
                            section_annotation = Some(blockquote_to_annotation(node));
                        }
                    } else {
                        content.push(SectionContent::Cite(blockquote_to_cite(node)));
                    }
                }
                node::IMAGE => {
                    let href = node.props.get_str(prop::URL).unwrap_or("").to_string();
                    let href = if href.starts_with('#') || href.contains("://") {
                        href
                    } else {
                        format!("#{href}")
                    };
                    content.push(SectionContent::Image(Image {
                        href,
                        ..Default::default()
                    }));
                }
                node::TABLE => {
                    content.push(SectionContent::Table(node_to_table(node)));
                }
                node::LIST => {
                    // FB2 has no native lists — render as paragraphs with markers
                    let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                    let mut num = 1u32;
                    for child in &node.children {
                        if child.kind.as_str() == node::LIST_ITEM {
                            let mut inlines = Vec::new();
                            let marker = if ordered {
                                let m = format!("{num}. ");
                                num += 1;
                                m
                            } else {
                                "• ".to_string()
                            };
                            inlines.push(InlineElement::Text(marker));
                            for item_child in &child.children {
                                if item_child.kind.as_str() == node::PARAGRAPH {
                                    inlines.extend(nodes_to_inlines(&item_child.children));
                                } else {
                                    inlines.push(node_to_inline(item_child));
                                }
                            }
                            content.push(SectionContent::Para(inlines));
                        }
                    }
                }
                node::CODE_BLOCK => {
                    // No code blocks in FB2 — wrap in p > code
                    let code_text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                    content.push(SectionContent::Para(vec![InlineElement::Code(code_text)]));
                }
                node::HORIZONTAL_RULE => {
                    content.push(SectionContent::EmptyLine);
                }
                node::FIGURE => {
                    // Recurse
                    let inner = nodes_to_section(&node.children);
                    content.extend(inner.content);
                }
                // Inline nodes at block level — wrap in p
                node::TEXT | node::EMPHASIS | node::STRONG | node::CODE | node::LINK => {
                    content.push(SectionContent::Para(vec![node_to_inline(node)]));
                }
                _ => {}
            }
        }
        Section {
            title: section_title,
            annotation: section_annotation,
            content,
            ..Default::default()
        }
    }

    /// Convert a `blockquote` tagged `fb2:type = "annotation"` (produced by
    /// [`super::read::convert_annotation`]) back into a `Section.annotation`. Mirrors
    /// `blockquote_to_cite`'s content-model walk but targets `AnnotationContent` instead of
    /// `CiteContent`, and additionally recognizes `HEADING` (used for `AnnotationContent::Subtitle`
    /// on the read side) since annotation content, unlike cite content, can contain a subtitle.
    fn blockquote_to_annotation(node: &Node) -> Annotation {
        let mut content = Vec::new();
        for child in &node.children {
            match child.kind.as_str() {
                node::PARAGRAPH => {
                    if child.children.is_empty() {
                        content.push(AnnotationContent::EmptyLine);
                    } else {
                        content.push(AnnotationContent::Para(nodes_to_inlines(&child.children)));
                    }
                }
                node::HEADING => {
                    content.push(AnnotationContent::Subtitle(nodes_to_inlines(
                        &child.children,
                    )));
                }
                node::TABLE => {
                    content.push(AnnotationContent::Table(node_to_table(child)));
                }
                node::DIV if child.props.get_str("html:class") == Some("poem") => {
                    content.push(AnnotationContent::Poem(div_to_poem(child)));
                }
                node::BLOCKQUOTE => {
                    content.push(AnnotationContent::Cite(blockquote_to_cite(child)));
                }
                _ => {}
            }
        }
        Annotation {
            id: node.props.get_str(prop::ID).map(|s| s.to_string()),
            lang: None,
            content,
        }
    }

    fn blockquote_to_cite(node: &Node) -> Cite {
        let mut content = Vec::new();
        let mut text_author = Vec::new();
        for child in &node.children {
            if child.props.get_str("html:class") == Some("text-author") {
                text_author.push(nodes_to_inlines(&child.children));
                continue;
            }
            match child.kind.as_str() {
                node::PARAGRAPH => {
                    if child.children.is_empty() {
                        content.push(CiteContent::EmptyLine);
                    } else {
                        content.push(CiteContent::Para(nodes_to_inlines(&child.children)));
                    }
                }
                node::TABLE => {
                    content.push(CiteContent::Table(node_to_table(child)));
                }
                node::DIV if child.props.get_str("html:class") == Some("poem") => {
                    content.push(CiteContent::Poem(div_to_poem(child)));
                }
                _ => {}
            }
        }
        Cite {
            content,
            text_author,
            ..Default::default()
        }
    }

    fn div_to_poem(node: &Node) -> Poem {
        let stanzas = node
            .children
            .iter()
            .filter_map(|child| {
                if child.kind.as_str() == node::DIV
                    && child.props.get_str("html:class") == Some("stanza")
                {
                    let v: Vec<Vec<InlineElement>> = child
                        .children
                        .iter()
                        .filter_map(|span| {
                            if span.kind.as_str() == node::SPAN {
                                // Verse line: SPAN with LINE_BREAK at end
                                let inlines: Vec<InlineElement> = span
                                    .children
                                    .iter()
                                    .filter(|n| n.kind.as_str() != node::LINE_BREAK)
                                    .map(node_to_inline)
                                    .collect();
                                Some(inlines)
                            } else {
                                None
                            }
                        })
                        .collect();
                    Some(Stanza {
                        v,
                        ..Default::default()
                    })
                } else {
                    None
                }
            })
            .collect();
        Poem {
            stanza: stanzas,
            ..Default::default()
        }
    }

    fn node_to_table(node: &Node) -> Table {
        let rows = node
            .children
            .iter()
            .filter_map(|row| {
                if row.kind.as_str() == node::TABLE_ROW {
                    let cells = row
                        .children
                        .iter()
                        .map(|cell| TableCell {
                            is_header: cell.kind.as_str() == node::TABLE_HEADER,
                            content: nodes_to_inlines(&cell.children),
                            ..Default::default()
                        })
                        .collect();
                    Some(TableRow {
                        cell: cells,
                        ..Default::default()
                    })
                } else {
                    None
                }
            })
            .collect();
        Table {
            row: rows,
            ..Default::default()
        }
    }

    fn nodes_to_inlines(nodes: &[Node]) -> Vec<InlineElement> {
        nodes.iter().map(node_to_inline).collect()
    }

    fn node_to_inline(node: &Node) -> InlineElement {
        match node.kind.as_str() {
            node::TEXT => {
                InlineElement::Text(node.props.get_str(prop::CONTENT).unwrap_or("").to_string())
            }
            node::STRONG => InlineElement::Strong(nodes_to_inlines(&node.children)),
            node::EMPHASIS => InlineElement::Emphasis(nodes_to_inlines(&node.children)),
            node::STRIKEOUT => InlineElement::Strikethrough(nodes_to_inlines(&node.children)),
            node::SUBSCRIPT => InlineElement::Sub(nodes_to_inlines(&node.children)),
            node::SUPERSCRIPT => InlineElement::Sup(nodes_to_inlines(&node.children)),
            node::CODE => {
                InlineElement::Code(node.props.get_str(prop::CONTENT).unwrap_or("").to_string())
            }
            node::IMAGE => {
                let href = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let href = if href.starts_with('#') || href.contains("://") {
                    href
                } else {
                    format!("#{href}")
                };
                InlineElement::Image(Image {
                    href,
                    ..Default::default()
                })
            }
            node::LINK => {
                let href = node.props.get_str(prop::URL).unwrap_or("").to_string();
                InlineElement::Link {
                    href,
                    kind: None,
                    children: nodes_to_inlines(&node.children),
                }
            }
            node::FOOTNOTE_REF => {
                let href = node.props.get_str(prop::URL).unwrap_or("").to_string();
                InlineElement::FootnoteRef {
                    href,
                    children: nodes_to_inlines(&node.children),
                }
            }
            node::LINE_BREAK => InlineElement::Text("\n".to_string()),
            node::SOFT_BREAK => InlineElement::Text(" ".to_string()),
            _ => InlineElement::Text(String::new()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_core::Properties;
        use rescribe_std::builder::*;

        fn emit_str(doc: &Document) -> String {
            let result = emit(doc).unwrap();
            String::from_utf8(result.value).unwrap()
        }

        #[test]
        fn test_emit_empty() {
            let doc = Document {
                content: Node::new(node::DOCUMENT),
                resources: Default::default(),
                metadata: Properties::new(),
                source: None,
            };

            let xml = emit_str(&doc);
            assert!(xml.contains("<FictionBook"));
            assert!(xml.contains("</FictionBook>"));
            assert!(xml.contains("<book-title>Untitled</book-title>"));
        }

        #[test]
        fn test_emit_with_title() {
            let mut metadata = Properties::new();
            metadata.set("title", "Test Book".to_string());

            let doc = Document {
                content: Node::new(node::DOCUMENT),
                resources: Default::default(),
                metadata,
                source: None,
            };

            let xml = emit_str(&doc);
            assert!(xml.contains("<book-title>Test Book</book-title>"));
        }

        #[test]
        fn test_emit_paragraph() {
            let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
            let xml = emit_str(&doc);
            assert!(xml.contains("<p>Hello, world!</p>"));
        }

        #[test]
        fn test_emit_heading() {
            let doc = doc(|d| d.heading(1, |h| h.text("Chapter Title")));
            let xml = emit_str(&doc);
            assert!(xml.contains("<title><p>Chapter Title</p></title>"));
        }

        #[test]
        fn test_emit_emphasis() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            let xml = emit_str(&doc);
            assert!(xml.contains("<emphasis>italic</emphasis>"));
        }

        #[test]
        fn test_emit_strong() {
            let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
            let xml = emit_str(&doc);
            assert!(xml.contains("<strong>bold</strong>"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("click"))));
            let xml = emit_str(&doc);
            assert!(xml.contains("<a l:href=\"http://example.com\">click</a>"));
        }

        #[test]
        fn test_emit_blockquote() {
            let doc = doc(|d| d.blockquote(|b| b.para(|p| p.text("Quoted text"))));
            let xml = emit_str(&doc);
            assert!(xml.contains("<cite>"));
            assert!(xml.contains("Quoted text"));
            assert!(xml.contains("</cite>"));
        }

        #[test]
        fn test_emit_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let xml = emit_str(&doc);
            assert!(xml.contains("• one"));
            assert!(xml.contains("• two"));
        }

        #[test]
        fn test_emit_section_annotation() {
            // A `blockquote` tagged `fb2:type = "annotation"` (as produced by
            // [`super::super::read::convert_annotation`]) must round-trip back to a
            // `<section><annotation>`, not a `<cite>`, and must be written before the
            // section's own content — mirroring how `Section.epigraph` is ordered.
            let annotation = Node::new(node::BLOCKQUOTE)
                .prop("fb2:type", "annotation")
                .children(vec![Node::new(node::PARAGRAPH).children(vec![
                    Node::new(node::TEXT).prop(rescribe_std::prop::CONTENT, "A summary."),
                ])]);
            let main = Node::new(node::PARAGRAPH).children(vec![
                Node::new(node::TEXT).prop(rescribe_std::prop::CONTENT, "Main content."),
            ]);
            let doc = Document {
                content: Node::new(node::DOCUMENT).children(vec![annotation, main]),
                resources: Default::default(),
                metadata: Properties::new(),
                source: None,
            };

            let xml = emit_str(&doc);
            assert!(xml.contains("<annotation>"), "xml: {xml}");
            assert!(xml.contains("A summary."));
            assert!(xml.contains("</annotation>"));
            assert!(
                !xml.contains("<cite>"),
                "annotation must not emit as <cite>: {xml}"
            );
            // Annotation content precedes the section's ordinary paragraph content.
            let ann_pos = xml.find("<annotation>").unwrap();
            let main_pos = xml.find("Main content.").unwrap();
            assert!(ann_pos < main_pos);
        }

        #[test]
        fn test_emit_ordered_list() {
            let doc =
                doc(|d| d.ordered_list(|l| l.item(|i| i.text("first")).item(|i| i.text("second"))));
            let xml = emit_str(&doc);
            assert!(xml.contains("1. first"));
            assert!(xml.contains("2. second"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
