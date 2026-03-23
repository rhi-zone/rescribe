//! AST types for Djot documents.

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const NONE: Span = Span { start: 0, end: 0 };
}

impl Default for Span {
    fn default() -> Self {
        Span::NONE
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Attr {
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub kv: Vec<(String, String)>,
}

impl Attr {
    pub fn is_empty(&self) -> bool {
        self.id.is_none() && self.classes.is_empty() && self.kv.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DjotDoc {
    pub blocks: Vec<Block>,
    pub footnotes: Vec<FootnoteDef>,
    pub link_defs: Vec<LinkDef>,
}

impl DjotDoc {
    pub fn strip_spans(&self) -> DjotDoc {
        DjotDoc {
            blocks: self.blocks.iter().map(|b| b.strip_spans()).collect(),
            footnotes: self.footnotes.iter().map(|f| f.strip_spans()).collect(),
            link_defs: self.link_defs.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
        attr: Attr,
        span: Span,
    },
    Heading {
        level: u8,
        inlines: Vec<Inline>,
        attr: Attr,
        span: Span,
    },
    Blockquote {
        blocks: Vec<Block>,
        attr: Attr,
        span: Span,
    },
    List {
        kind: ListKind,
        items: Vec<ListItem>,
        tight: bool,
        attr: Attr,
        span: Span,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
        attr: Attr,
        span: Span,
    },
    RawBlock {
        format: String,
        content: String,
        attr: Attr,
        span: Span,
    },
    Div {
        class: Option<String>,
        blocks: Vec<Block>,
        attr: Attr,
        span: Span,
    },
    Table {
        caption: Option<Vec<Inline>>,
        rows: Vec<TableRow>,
        span: Span,
    },
    ThematicBreak {
        attr: Attr,
        span: Span,
    },
    DefinitionList {
        items: Vec<DefItem>,
        attr: Attr,
        span: Span,
    },
}

impl Block {
    pub fn strip_spans(&self) -> Block {
        match self {
            Block::Paragraph { inlines, attr, .. } => Block::Paragraph {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Block::Heading { level, inlines, attr, .. } => Block::Heading {
                level: *level,
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Block::Blockquote { blocks, attr, .. } => Block::Blockquote {
                blocks: blocks.iter().map(|b| b.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Block::List { kind, items, tight, attr, .. } => Block::List {
                kind: kind.clone(),
                items: items.iter().map(|item| item.strip_spans()).collect(),
                tight: *tight,
                attr: attr.clone(),
                span: Span::NONE,
            },
            Block::CodeBlock { language, content, attr, .. } => Block::CodeBlock {
                language: language.clone(),
                content: content.clone(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Block::RawBlock { format, content, attr, .. } => Block::RawBlock {
                format: format.clone(),
                content: content.clone(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Block::Div { class, blocks, attr, .. } => Block::Div {
                class: class.clone(),
                blocks: blocks.iter().map(|b| b.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Block::Table { caption, rows, .. } => Block::Table {
                caption: caption
                    .as_ref()
                    .map(|c| c.iter().map(|i| i.strip_spans()).collect()),
                rows: rows.iter().map(|r| r.strip_spans()).collect(),
                span: Span::NONE,
            },
            Block::ThematicBreak { attr, .. } => Block::ThematicBreak {
                attr: attr.clone(),
                span: Span::NONE,
            },
            Block::DefinitionList { items, attr, .. } => Block::DefinitionList {
                items: items.iter().map(|i| i.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ListKind {
    Bullet(BulletStyle),
    Ordered {
        style: OrderedStyle,
        delimiter: OrderedDelimiter,
        start: u32,
    },
    Task,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BulletStyle {
    Dash,
    Star,
    Plus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OrderedStyle {
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OrderedDelimiter {
    Period,
    Paren,
    Enclosed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListItem {
    pub blocks: Vec<Block>,
    pub checked: Option<bool>,
    pub span: Span,
}

impl ListItem {
    pub fn strip_spans(&self) -> ListItem {
        ListItem {
            blocks: self.blocks.iter().map(|b| b.strip_spans()).collect(),
            checked: self.checked,
            span: Span::NONE,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefItem {
    pub term: Vec<Inline>,
    pub definitions: Vec<Block>,
    pub span: Span,
}

impl DefItem {
    pub fn strip_spans(&self) -> DefItem {
        DefItem {
            term: self.term.iter().map(|i| i.strip_spans()).collect(),
            definitions: self.definitions.iter().map(|b| b.strip_spans()).collect(),
            span: Span::NONE,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub is_header: bool,
    pub span: Span,
}

impl TableRow {
    pub fn strip_spans(&self) -> TableRow {
        TableRow {
            cells: self.cells.iter().map(|c| c.strip_spans()).collect(),
            is_header: self.is_header,
            span: Span::NONE,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableCell {
    pub inlines: Vec<Inline>,
    pub alignment: Alignment,
    pub span: Span,
}

impl TableCell {
    pub fn strip_spans(&self) -> TableCell {
        TableCell {
            inlines: self.inlines.iter().map(|i| i.strip_spans()).collect(),
            alignment: self.alignment.clone(),
            span: Span::NONE,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Alignment {
    Left,
    Right,
    Center,
    Default,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    Text {
        content: String,
        span: Span,
    },
    SoftBreak {
        span: Span,
    },
    HardBreak {
        span: Span,
    },
    Emphasis {
        inlines: Vec<Inline>,
        attr: Attr,
        span: Span,
    },
    Strong {
        inlines: Vec<Inline>,
        attr: Attr,
        span: Span,
    },
    Delete {
        inlines: Vec<Inline>,
        attr: Attr,
        span: Span,
    },
    Insert {
        inlines: Vec<Inline>,
        attr: Attr,
        span: Span,
    },
    Highlight {
        inlines: Vec<Inline>,
        attr: Attr,
        span: Span,
    },
    Subscript {
        inlines: Vec<Inline>,
        attr: Attr,
        span: Span,
    },
    Superscript {
        inlines: Vec<Inline>,
        attr: Attr,
        span: Span,
    },
    Verbatim {
        content: String,
        attr: Attr,
        span: Span,
    },
    MathInline {
        content: String,
        span: Span,
    },
    MathDisplay {
        content: String,
        span: Span,
    },
    RawInline {
        format: String,
        content: String,
        span: Span,
    },
    Link {
        inlines: Vec<Inline>,
        url: String,
        title: Option<String>,
        attr: Attr,
        span: Span,
    },
    Image {
        inlines: Vec<Inline>,
        url: String,
        title: Option<String>,
        attr: Attr,
        span: Span,
    },
    Span {
        inlines: Vec<Inline>,
        attr: Attr,
        span: Span,
    },
    FootnoteRef {
        label: String,
        span: Span,
    },
    Symbol {
        name: String,
        span: Span,
    },
    Autolink {
        url: String,
        is_email: bool,
        span: Span,
    },
}

impl Inline {
    pub fn strip_spans(&self) -> Inline {
        match self {
            Inline::Text { content, .. } => Inline::Text {
                content: content.clone(),
                span: Span::NONE,
            },
            Inline::SoftBreak { .. } => Inline::SoftBreak { span: Span::NONE },
            Inline::HardBreak { .. } => Inline::HardBreak { span: Span::NONE },
            Inline::Emphasis { inlines, attr, .. } => Inline::Emphasis {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Inline::Strong { inlines, attr, .. } => Inline::Strong {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Inline::Delete { inlines, attr, .. } => Inline::Delete {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Inline::Insert { inlines, attr, .. } => Inline::Insert {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Inline::Highlight { inlines, attr, .. } => Inline::Highlight {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Inline::Subscript { inlines, attr, .. } => Inline::Subscript {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Inline::Superscript { inlines, attr, .. } => Inline::Superscript {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Inline::Verbatim { content, attr, .. } => Inline::Verbatim {
                content: content.clone(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Inline::MathInline { content, .. } => Inline::MathInline {
                content: content.clone(),
                span: Span::NONE,
            },
            Inline::MathDisplay { content, .. } => Inline::MathDisplay {
                content: content.clone(),
                span: Span::NONE,
            },
            Inline::RawInline { format, content, .. } => Inline::RawInline {
                format: format.clone(),
                content: content.clone(),
                span: Span::NONE,
            },
            Inline::Link { inlines, url, title, attr, .. } => Inline::Link {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                url: url.clone(),
                title: title.clone(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Inline::Image { inlines, url, title, attr, .. } => Inline::Image {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                url: url.clone(),
                title: title.clone(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Inline::Span { inlines, attr, .. } => Inline::Span {
                inlines: inlines.iter().map(|i| i.strip_spans()).collect(),
                attr: attr.clone(),
                span: Span::NONE,
            },
            Inline::FootnoteRef { label, .. } => Inline::FootnoteRef {
                label: label.clone(),
                span: Span::NONE,
            },
            Inline::Symbol { name, .. } => Inline::Symbol {
                name: name.clone(),
                span: Span::NONE,
            },
            Inline::Autolink { url, is_email, .. } => Inline::Autolink {
                url: url.clone(),
                is_email: *is_email,
                span: Span::NONE,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FootnoteDef {
    pub label: String,
    pub blocks: Vec<Block>,
    pub span: Span,
}

impl FootnoteDef {
    pub fn strip_spans(&self) -> FootnoteDef {
        FootnoteDef {
            label: self.label.clone(),
            blocks: self.blocks.iter().map(|b| b.strip_spans()).collect(),
            span: Span::NONE,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinkDef {
    pub label: String,
    pub url: String,
    pub title: Option<String>,
    pub attr: Attr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}
