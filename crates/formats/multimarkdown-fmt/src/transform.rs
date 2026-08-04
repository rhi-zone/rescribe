//! Bidirectional conversion between `commonmark_fmt::CmDoc`'s `Block`/
//! `Inline` tree and this crate's `MmdBlock`/`MmdInline` tree.
//!
//! `cm_to_mmd` is applied to the output of `commonmark_fmt::parse::parse_str`
//! (all actual CommonMark tokenizing already done); it upgrades citation-
//! definition paragraphs to `MmdBlock::CitationDefinition`, upgrades a
//! trailing `[Anchor]` on a heading to `MmdBlock::Heading::anchor`, and runs
//! `crate::citation::scan` over every text run to recognize citations and
//! cross-references. `mmd_to_cm` is the inverse, spelling MMD-unique nodes
//! back out as the literal bracket text CommonMark itself would leave
//! unresolved, then handing the whole tree to `commonmark_fmt::emit::emit`
//! for actual CommonMark writing — this crate never hand-writes Markdown
//! syntax itself.

use commonmark_fmt::{Block as CmBlock, Inline as CmInline};

use crate::ast::*;
use crate::citation;

use commonmark_fmt::{TableCell as CmTableCell, TableRow as CmTableRow};

use commonmark_fmt::DefinitionListItem as CmDefinitionListItem;

// ── commonmark -> mmd ────────────────────────────────────────────────────────

pub fn cm_to_mmd_blocks(blocks: &[CmBlock]) -> Vec<MmdBlock> {
    blocks.iter().map(cm_to_mmd_block).collect()
}

fn cm_to_mmd_block(block: &CmBlock) -> MmdBlock {
    match block {
        CmBlock::Paragraph { inlines, span } => {
            if let Some(CmInline::Text {
                content,
                span: text_span,
            }) = inlines.first()
                && let Some((label, remainder)) = citation::match_definition_prefix(content)
            {
                let mut src: Vec<CmInline> = Vec::new();
                if !remainder.is_empty() {
                    src.push(CmInline::Text {
                        content: remainder.to_string(),
                        span: text_span.clone(),
                    });
                }
                src.extend(inlines[1..].iter().cloned());
                return MmdBlock::CitationDefinition {
                    label: label.to_string(),
                    content: cm_to_mmd_inlines(&src),
                    span: span.clone(),
                };
            }
            MmdBlock::Paragraph {
                inlines: cm_to_mmd_inlines(inlines),
                span: span.clone(),
            }
        }
        CmBlock::Heading {
            level,
            inlines,
            span,
        } => {
            let (inlines, anchor) = extract_heading_anchor(cm_to_mmd_inlines(inlines));
            MmdBlock::Heading {
                level: *level,
                inlines,
                anchor,
                span: span.clone(),
            }
        }
        CmBlock::CodeBlock {
            language,
            content,
            span,
        } => MmdBlock::CodeBlock {
            language: language.clone(),
            content: content.clone(),
            span: span.clone(),
        },
        CmBlock::HtmlBlock { content, span } => MmdBlock::HtmlBlock {
            content: content.clone(),
            span: span.clone(),
        },
        CmBlock::Blockquote { blocks, span } => MmdBlock::Blockquote {
            blocks: cm_to_mmd_blocks(blocks),
            span: span.clone(),
        },
        CmBlock::List {
            kind,
            items,
            tight,
            span,
        } => MmdBlock::List {
            kind: kind.clone(),
            items: items
                .iter()
                .map(|item| MmdListItem {
                    blocks: cm_to_mmd_blocks(&item.blocks),
                    span: item.span.clone(),
                    checked: item.checked,
                })
                .collect(),
            tight: *tight,
            span: span.clone(),
        },
        CmBlock::ThematicBreak { span } => MmdBlock::ThematicBreak { span: span.clone() },
        CmBlock::Table {
            alignments,
            head,
            rows,
            span,
        } => MmdBlock::Table {
            alignments: alignments.clone(),
            head: cm_to_mmd_row(head),
            rows: rows.iter().map(cm_to_mmd_row).collect(),
            span: span.clone(),
        },
        CmBlock::FootnoteDefinition {
            label,
            blocks,
            span,
        } => MmdBlock::FootnoteDefinition {
            label: label.clone(),
            blocks: cm_to_mmd_blocks(blocks),
            span: span.clone(),
        },
        CmBlock::DefinitionList { items, tight, span } => MmdBlock::DefinitionList {
            items: items
                .iter()
                .map(|item| MmdDefinitionListItem {
                    term: cm_to_mmd_inlines(&item.term),
                    definitions: item
                        .definitions
                        .iter()
                        .map(|d| cm_to_mmd_blocks(d))
                        .collect(),
                    span: item.span.clone(),
                })
                .collect(),
            tight: *tight,
            span: span.clone(),
        },
    }
}

fn cm_to_mmd_row(row: &CmTableRow) -> MmdTableRow {
    MmdTableRow {
        cells: row
            .cells
            .iter()
            .map(|c| MmdTableCell {
                inlines: cm_to_mmd_inlines(&c.inlines),
                span: c.span.clone(),
            })
            .collect(),
        span: row.span.clone(),
    }
}

fn cm_to_mmd_inlines(inlines: &[CmInline]) -> Vec<MmdInline> {
    let mut out = Vec::with_capacity(inlines.len());
    for inline in inlines {
        match inline {
            CmInline::Text { content, span } => {
                let pieces = citation::scan(content);
                if pieces.len() == 1
                    && let MmdInline::Text { content: c, .. } = &pieces[0]
                    && c == content
                {
                    out.push(MmdInline::Text {
                        content: content.clone(),
                        span: span.clone(),
                    });
                } else {
                    out.extend(pieces);
                }
            }
            CmInline::SoftBreak { span } => out.push(MmdInline::SoftBreak { span: span.clone() }),
            CmInline::HardBreak { span } => out.push(MmdInline::HardBreak { span: span.clone() }),
            CmInline::Emphasis { inlines, span } => out.push(MmdInline::Emphasis {
                inlines: cm_to_mmd_inlines(inlines),
                span: span.clone(),
            }),
            CmInline::Strong { inlines, span } => out.push(MmdInline::Strong {
                inlines: cm_to_mmd_inlines(inlines),
                span: span.clone(),
            }),
            CmInline::Strikethrough { inlines, span } => out.push(MmdInline::Strikethrough {
                inlines: cm_to_mmd_inlines(inlines),
                span: span.clone(),
            }),
            CmInline::Code { content, span } => out.push(MmdInline::Code {
                content: content.clone(),
                span: span.clone(),
            }),
            CmInline::HtmlInline { content, span } => out.push(MmdInline::HtmlInline {
                content: content.clone(),
                span: span.clone(),
            }),
            CmInline::Link {
                inlines,
                url,
                title,
                span,
            } => out.push(MmdInline::Link {
                inlines: cm_to_mmd_inlines(inlines),
                url: url.clone(),
                title: title.clone(),
                span: span.clone(),
            }),
            CmInline::Image {
                alt,
                url,
                title,
                span,
            } => out.push(MmdInline::Image {
                alt: alt.clone(),
                url: url.clone(),
                title: title.clone(),
                span: span.clone(),
            }),
            CmInline::FootnoteReference { label, span } => {
                out.push(MmdInline::FootnoteReference {
                    label: label.clone(),
                    span: span.clone(),
                });
            }
            CmInline::InlineMath { source, span } => out.push(MmdInline::InlineMath {
                source: source.clone(),
                span: span.clone(),
            }),
            CmInline::DisplayMath { source, span } => out.push(MmdInline::DisplayMath {
                source: source.clone(),
                span: span.clone(),
            }),
        }
    }
    out
}

/// Pop a trailing shortcut `CrossReference` off `inlines` (with at most one
/// immediately-preceding whitespace-only `Text` trimmed of its trailing
/// space) and report it as a heading anchor instead. Only shortcut-form
/// (`[Anchor]`, not `[Anchor][]`) references are recognized as anchors —
/// the collapsed form is reserved for genuine cross-references.
fn extract_heading_anchor(mut inlines: Vec<MmdInline>) -> (Vec<MmdInline>, Option<String>) {
    let Some(MmdInline::CrossReference {
        target,
        collapsed: false,
        ..
    }) = inlines.last()
    else {
        return (inlines, None);
    };
    let anchor = target.clone();
    inlines.pop();
    if let Some(MmdInline::Text { content, .. }) = inlines.last_mut()
        && content.ends_with(' ')
    {
        content.pop();
        if content.is_empty() {
            inlines.pop();
        }
    }
    (inlines, Some(anchor))
}

// ── mmd -> commonmark ────────────────────────────────────────────────────────

pub fn mmd_to_cm_blocks(blocks: &[MmdBlock]) -> Vec<CmBlock> {
    blocks.iter().map(mmd_to_cm_block).collect()
}

fn mmd_to_cm_block(block: &MmdBlock) -> CmBlock {
    match block {
        MmdBlock::Paragraph { inlines, span } => CmBlock::Paragraph {
            inlines: mmd_to_cm_inlines(inlines),
            span: span.clone(),
        },
        MmdBlock::Heading {
            level,
            inlines,
            anchor,
            span,
        } => {
            let mut cm_inlines = mmd_to_cm_inlines(inlines);
            if let Some(anchor) = anchor {
                cm_inlines.push(CmInline::Text {
                    content: format!(" [{anchor}]"),
                    span: span.clone(),
                });
            }
            CmBlock::Heading {
                level: *level,
                inlines: cm_inlines,
                span: span.clone(),
            }
        }
        MmdBlock::CodeBlock {
            language,
            content,
            span,
        } => CmBlock::CodeBlock {
            language: language.clone(),
            content: content.clone(),
            span: span.clone(),
        },
        MmdBlock::HtmlBlock { content, span } => CmBlock::HtmlBlock {
            content: content.clone(),
            span: span.clone(),
        },
        MmdBlock::Blockquote { blocks, span } => CmBlock::Blockquote {
            blocks: mmd_to_cm_blocks(blocks),
            span: span.clone(),
        },
        MmdBlock::List {
            kind,
            items,
            tight,
            span,
        } => CmBlock::List {
            kind: kind.clone(),
            items: items
                .iter()
                .map(|item| commonmark_fmt::ListItem {
                    blocks: mmd_to_cm_blocks(&item.blocks),
                    span: item.span.clone(),
                    checked: item.checked,
                })
                .collect(),
            tight: *tight,
            span: span.clone(),
        },
        MmdBlock::ThematicBreak { span } => CmBlock::ThematicBreak { span: span.clone() },
        MmdBlock::Table {
            alignments,
            head,
            rows,
            span,
        } => CmBlock::Table {
            alignments: alignments.clone(),
            head: mmd_to_cm_row(head),
            rows: rows.iter().map(mmd_to_cm_row).collect(),
            span: span.clone(),
        },
        MmdBlock::FootnoteDefinition {
            label,
            blocks,
            span,
        } => CmBlock::FootnoteDefinition {
            label: label.clone(),
            blocks: mmd_to_cm_blocks(blocks),
            span: span.clone(),
        },
        MmdBlock::DefinitionList { items, tight, span } => CmBlock::DefinitionList {
            items: items
                .iter()
                .map(|item| CmDefinitionListItem {
                    term: mmd_to_cm_inlines(&item.term),
                    definitions: item
                        .definitions
                        .iter()
                        .map(|d| mmd_to_cm_blocks(d))
                        .collect(),
                    span: item.span.clone(),
                })
                .collect(),
            tight: *tight,
            span: span.clone(),
        },
        MmdBlock::CitationDefinition {
            label,
            content,
            span,
        } => {
            let mut inlines = vec![CmInline::Text {
                content: format!("[#{label}]: "),
                span: span.clone(),
            }];
            inlines.extend(mmd_to_cm_inlines(content));
            CmBlock::Paragraph {
                inlines,
                span: span.clone(),
            }
        }
    }
}

fn mmd_to_cm_row(row: &MmdTableRow) -> CmTableRow {
    CmTableRow {
        cells: row
            .cells
            .iter()
            .map(|c| CmTableCell {
                inlines: mmd_to_cm_inlines(&c.inlines),
                span: c.span.clone(),
            })
            .collect(),
        span: row.span.clone(),
    }
}

fn mmd_to_cm_inlines(inlines: &[MmdInline]) -> Vec<CmInline> {
    inlines.iter().map(mmd_to_cm_inline).collect()
}

fn mmd_to_cm_inline(inline: &MmdInline) -> CmInline {
    match inline {
        MmdInline::Text { content, span } => CmInline::Text {
            content: content.clone(),
            span: span.clone(),
        },
        MmdInline::SoftBreak { span } => CmInline::SoftBreak { span: span.clone() },
        MmdInline::HardBreak { span } => CmInline::HardBreak { span: span.clone() },
        MmdInline::Emphasis { inlines, span } => CmInline::Emphasis {
            inlines: mmd_to_cm_inlines(inlines),
            span: span.clone(),
        },
        MmdInline::Strong { inlines, span } => CmInline::Strong {
            inlines: mmd_to_cm_inlines(inlines),
            span: span.clone(),
        },
        MmdInline::Strikethrough { inlines, span } => CmInline::Strikethrough {
            inlines: mmd_to_cm_inlines(inlines),
            span: span.clone(),
        },
        MmdInline::Code { content, span } => CmInline::Code {
            content: content.clone(),
            span: span.clone(),
        },
        MmdInline::HtmlInline { content, span } => CmInline::HtmlInline {
            content: content.clone(),
            span: span.clone(),
        },
        MmdInline::Link {
            inlines,
            url,
            title,
            span,
        } => CmInline::Link {
            inlines: mmd_to_cm_inlines(inlines),
            url: url.clone(),
            title: title.clone(),
            span: span.clone(),
        },
        MmdInline::Image {
            alt,
            url,
            title,
            span,
        } => CmInline::Image {
            alt: alt.clone(),
            url: url.clone(),
            title: title.clone(),
            span: span.clone(),
        },
        MmdInline::FootnoteReference { label, span } => CmInline::FootnoteReference {
            label: label.clone(),
            span: span.clone(),
        },
        MmdInline::InlineMath { source, span } => CmInline::InlineMath {
            source: source.clone(),
            span: span.clone(),
        },
        MmdInline::DisplayMath { source, span } => CmInline::DisplayMath {
            source: source.clone(),
            span: span.clone(),
        },
        MmdInline::Citation {
            locator,
            label,
            span,
        } => CmInline::Text {
            content: format!("[{}][#{}]", locator.as_deref().unwrap_or(""), label),
            span: span.clone(),
        },
        MmdInline::CrossReference {
            target,
            collapsed,
            span,
        } => CmInline::Text {
            content: if *collapsed {
                format!("[{target}][]")
            } else {
                format!("[{target}]")
            },
            span: span.clone(),
        },
    }
}
