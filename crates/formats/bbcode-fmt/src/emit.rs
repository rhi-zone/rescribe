//! BBCode emitter — build a BBCode string from a [`BbcodeDoc`].

use crate::ast::{AlignKind, BbcodeDoc, Block, Inline, TableRow};

/// Build a BBCode string from a [`BbcodeDoc`].
pub fn emit(doc: &BbcodeDoc) -> String {
    let mut output = String::new();
    for block in &doc.blocks {
        emit_block(block, &mut output);
    }
    output
}

fn emit_block(block: &Block, output: &mut String) {
    match block {
        Block::Paragraph { inlines, .. } => {
            emit_inlines(inlines, output);
            output.push_str("\n\n");
        }

        Block::CodeBlock {
            language, content, ..
        } => {
            if let Some(lang) = language {
                output.push_str(&format!("[code={}]\n", lang));
            } else {
                output.push_str("[code]\n");
            }
            output.push_str(content);
            if !content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("[/code]\n\n");
        }

        Block::Blockquote {
            author, children, ..
        } => {
            if let Some(author) = author {
                output.push_str(&format!("[quote={}]\n", author));
            } else {
                output.push_str("[quote]\n");
            }
            for child in children {
                if let Block::Paragraph { inlines, .. } = child {
                    emit_inlines(inlines, output);
                    output.push('\n');
                } else {
                    emit_block(child, output);
                }
            }
            output.push_str("[/quote]\n\n");
        }

        Block::List { ordered, items, .. } => {
            if *ordered {
                output.push_str("[list=1]\n");
            } else {
                output.push_str("[list]\n");
            }

            for item_inlines in items {
                output.push_str("[*]");
                emit_inlines(item_inlines, output);
                output.push('\n');
            }

            output.push_str("[/list]\n\n");
        }

        Block::Table { rows, .. } => {
            emit_table(rows, output);
        }

        Block::HorizontalRule { .. } => {
            output.push_str("[hr]\n\n");
        }

        Block::Heading {
            level, children, ..
        } => {
            output.push_str(&format!("[h{}]", level));
            emit_inlines(children, output);
            output.push_str(&format!("[/h{}]\n\n", level));
        }

        Block::Alignment {
            kind, children, ..
        } => {
            let tag = match kind {
                AlignKind::Center => "center",
                AlignKind::Left => "left",
                AlignKind::Right => "right",
            };
            output.push_str(&format!("[{}]\n", tag));
            for child in children {
                emit_block(child, output);
            }
            output.push_str(&format!("[/{}]\n\n", tag));
        }

        Block::Spoiler { children, .. } => {
            output.push_str("[spoiler]\n");
            for child in children {
                emit_block(child, output);
            }
            output.push_str("[/spoiler]\n\n");
        }

        Block::Preformatted { content, .. } => {
            output.push_str("[pre]");
            output.push_str(content);
            output.push_str("[/pre]\n\n");
        }

        Block::Indent { children, .. } => {
            output.push_str("[indent]\n");
            for child in children {
                emit_block(child, output);
            }
            output.push_str("[/indent]\n\n");
        }
    }
}

pub(crate) fn emit_table(rows: &[TableRow], output: &mut String) {
    output.push_str("[table]\n");
    for row in rows {
        output.push_str("[tr]");
        for (is_header, inlines) in &row.cells {
            let tag = if *is_header { "th" } else { "td" };
            output.push_str(&format!("[{}]", tag));
            emit_inlines(inlines, output);
            output.push_str(&format!("[/{}]", tag));
        }
        output.push_str("[/tr]\n");
    }
    output.push_str("[/table]\n\n");
}

fn emit_inlines(inlines: &[Inline], output: &mut String) {
    for inline in inlines {
        emit_inline(inline, output);
    }
}

pub(crate) fn emit_inline(inline: &Inline, output: &mut String) {
    match inline {
        Inline::Text(s, _) => output.push_str(s),

        Inline::Bold(children, _) => {
            output.push_str("[b]");
            emit_inlines(children, output);
            output.push_str("[/b]");
        }

        Inline::Italic(children, _) => {
            output.push_str("[i]");
            emit_inlines(children, output);
            output.push_str("[/i]");
        }

        Inline::Underline(children, _) => {
            output.push_str("[u]");
            emit_inlines(children, output);
            output.push_str("[/u]");
        }

        Inline::Strikethrough(children, _) => {
            output.push_str("[s]");
            emit_inlines(children, output);
            output.push_str("[/s]");
        }

        Inline::Code(s, _) => {
            output.push_str("[code]");
            output.push_str(s);
            output.push_str("[/code]");
        }

        Inline::Link { url, children, .. } => {
            output.push_str(&format!("[url={}]", url));
            emit_inlines(children, output);
            output.push_str("[/url]");
        }

        Inline::Image {
            url,
            width,
            height,
            ..
        } => {
            if let (Some(w), Some(h)) = (width, height) {
                output.push_str(&format!("[img={}x{}]", w, h));
            } else {
                output.push_str("[img]");
            }
            output.push_str(url);
            output.push_str("[/img]");
        }

        Inline::Subscript(children, _) => {
            output.push_str("[sub]");
            emit_inlines(children, output);
            output.push_str("[/sub]");
        }

        Inline::Superscript(children, _) => {
            output.push_str("[sup]");
            emit_inlines(children, output);
            output.push_str("[/sup]");
        }

        Inline::Color {
            value, children, ..
        } => {
            output.push_str(&format!("[color={}]", value));
            emit_inlines(children, output);
            output.push_str("[/color]");
        }

        Inline::Size {
            value, children, ..
        } => {
            output.push_str(&format!("[size={}]", value));
            emit_inlines(children, output);
            output.push_str("[/size]");
        }

        Inline::Font {
            name, children, ..
        } => {
            output.push_str(&format!("[font={}]", name));
            emit_inlines(children, output);
            output.push_str("[/font]");
        }

        Inline::Email {
            addr, children, ..
        } => {
            output.push_str(&format!("[email={}]", addr));
            emit_inlines(children, output);
            output.push_str("[/email]");
        }

        Inline::Noparse(s, _) => {
            output.push_str("[noparse]");
            output.push_str(s);
            output.push_str("[/noparse]");
        }

        Inline::Span {
            attr,
            value,
            children,
            ..
        } => {
            output.push_str(&format!("[{}={}]", attr, value));
            emit_inlines(children, output);
            output.push_str(&format!("[/{}]", attr));
        }
    }
}
