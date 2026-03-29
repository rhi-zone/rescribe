//! Event-driven DOCX writer.
//!
//! [`WmlWriter`] accepts [`WmlEvent`] items via [`WmlWriter::write_event`]
//! and produces a complete `.docx` file on [`WmlWriter::finish`].
//!
//! # Memory model
//!
//! Events are buffered until `finish()` is called, then converted to a DOCX
//! in one pass using the existing [`DocumentBuilder`] for OPC packaging.
//!
//! # Example
//!
//! ```ignore
//! use std::io::BufWriter;
//! use std::fs::File;
//! use ooxml_wml::{WmlWriter, WmlEvent};
//!
//! let sink = BufWriter::new(File::create("output.docx")?);
//! let mut writer = WmlWriter::new(sink);
//! writer.write_event(WmlEvent::StartDocument);
//! writer.write_event(WmlEvent::StartParagraph { props: Box::default() });
//! writer.write_event(WmlEvent::StartRun { props: Box::default() });
//! writer.write_event(WmlEvent::Text("Hello, world!".into()));
//! writer.write_event(WmlEvent::EndRun);
//! writer.write_event(WmlEvent::EndParagraph);
//! writer.write_event(WmlEvent::EndDocument);
//! writer.finish()?;
//! # Ok::<(), ooxml_wml::Error>(())
//! ```

use std::collections::HashMap;
use std::io::{Seek, Write};

use crate::types;
use crate::writer::{DocumentBuilder, Drawing};
use crate::Result;
use crate::generated_events::{OwnedWmlEvent, WmlEvent};

/// Internal stack frame for tracking nested content during event processing.
enum WmlFrame {
    Paragraph(Box<types::Paragraph>),
    Run(Box<types::Run>),
    #[cfg(feature = "wml-hyperlinks")]
    Hyperlink(Box<types::Hyperlink>),
    Table(Box<types::Table>),
    TableRow(Box<types::CTRow>),
    TableCell(Box<types::TableCell>),
}

/// Event-driven DOCX writer.
///
/// Feed [`WmlEvent`] items one at a time, then call [`finish`](WmlWriter::finish)
/// to produce a complete DOCX document.
pub struct WmlWriter<W: Write + Seek> {
    sink: W,
    events: Vec<OwnedWmlEvent>,
    /// Pending images keyed by caller-supplied rel_id.
    /// Value is (image bytes, content-type string).
    registered_images: HashMap<String, (Vec<u8>, String)>,
}

impl<W: Write + Seek> WmlWriter<W> {
    /// Create a new writer targeting `sink`.
    pub fn new(sink: W) -> Self {
        WmlWriter { sink, events: Vec::new(), registered_images: HashMap::new() }
    }

    /// Register image bytes under a caller-supplied rel_id.
    ///
    /// The `rel_id` must match the one used in [`WmlEvent::Image`] events.
    /// The builder assigns its own internal rel_id when writing the package;
    /// the mapping is handled transparently inside [`finish`](WmlWriter::finish).
    pub fn register_image(
        &mut self,
        rel_id: impl Into<String>,
        data: Vec<u8>,
        content_type: impl Into<String>,
    ) {
        self.registered_images.insert(rel_id.into(), (data, content_type.into()));
    }

    /// Buffer one event.
    pub fn write_event(&mut self, event: WmlEvent<'_>) {
        self.events.push(event.into_owned());
    }

    /// Convert buffered events to a DOCX and write to the underlying sink.
    pub fn finish(self) -> Result<()> {
        let mut builder = DocumentBuilder::new();

        // Register all images with the builder and build the original→builder rel_id map.
        let mut rel_id_map: HashMap<String, String> = HashMap::new();
        for (orig_rel_id, (data, content_type)) in self.registered_images {
            let assigned_rel_id = builder.add_image(data, &content_type);
            rel_id_map.insert(orig_rel_id, assigned_rel_id);
        }

        process_events(self.events, &mut builder, &rel_id_map)?;
        builder.write(self.sink)
    }
}

fn process_events(
    events: Vec<OwnedWmlEvent>,
    builder: &mut DocumentBuilder,
    rel_id_map: &HashMap<String, String>,
) -> Result<()> {
    let mut stack: Vec<WmlFrame> = Vec::new();

    for event in events {
        match event {
            WmlEvent::StartDocument | WmlEvent::EndDocument => {}

            WmlEvent::StartParagraph { props } => {
                let mut para = Box::new(types::Paragraph::default());
                #[cfg(feature = "wml-styling")]
                {
                    para.p_pr = Some(props);
                }
                #[cfg(not(feature = "wml-styling"))]
                {
                    let _ = props;
                }
                stack.push(WmlFrame::Paragraph(para));
            }

            WmlEvent::EndParagraph => {
                if let Some(WmlFrame::Paragraph(para)) = stack.pop() {
                    push_block(types::BlockContent::P(para), &mut stack, builder);
                }
            }

            WmlEvent::StartRun { props } => {
                let mut run = Box::new(types::Run::default());
                #[cfg(feature = "wml-styling")]
                {
                    run.r_pr = Some(props);
                }
                #[cfg(not(feature = "wml-styling"))]
                {
                    let _ = props;
                }
                stack.push(WmlFrame::Run(run));
            }

            WmlEvent::EndRun => {
                if let Some(WmlFrame::Run(run)) = stack.pop() {
                    push_para_content(types::ParagraphContent::R(run), &mut stack);
                }
            }

            WmlEvent::Text(text) => {
                if let Some(WmlFrame::Run(run)) = stack.last_mut() {
                    run.run_content.push(types::RunContent::T(Box::new(types::Text {
                        text: Some(text.into_owned()),
                        #[cfg(feature = "extra-children")]
                        extra_children: Vec::new(),
                    })));
                }
            }

            WmlEvent::LineBreak => {
                if let Some(WmlFrame::Run(run)) = stack.last_mut() {
                    run.run_content.push(types::RunContent::Br(Box::new(types::CTBr {
                        r#type: None,
                        clear: None,
                        #[cfg(feature = "extra-attrs")]
                        extra_attrs: Default::default(),
                    })));
                }
            }

            WmlEvent::StartHyperlink { rel_id, anchor } => {
                #[cfg(feature = "wml-hyperlinks")]
                {
                    let mut hl = Box::new(types::Hyperlink::default());
                    hl.id = rel_id.map(|s| s.into_owned());
                    hl.anchor = anchor.map(|s| s.into_owned());
                    stack.push(WmlFrame::Hyperlink(hl));
                }
                #[cfg(not(feature = "wml-hyperlinks"))]
                {
                    let _ = (rel_id, anchor);
                }
            }

            WmlEvent::EndHyperlink => {
                #[cfg(feature = "wml-hyperlinks")]
                {
                    if let Some(WmlFrame::Hyperlink(hl)) = stack.pop() {
                        push_para_content(types::ParagraphContent::Hyperlink(hl), &mut stack);
                    }
                }
            }

            WmlEvent::StartTable { props } => {
                let tbl = Box::new(types::Table {
                    range_markup: Vec::new(),
                    table_properties: props,
                    tbl_grid: Box::new(types::TableGrid::default()),
                    rows: Vec::new(),
                    #[cfg(feature = "extra-children")]
                    extra_children: Vec::new(),
                });
                stack.push(WmlFrame::Table(tbl));
            }

            WmlEvent::EndTable => {
                if let Some(WmlFrame::Table(tbl)) = stack.pop() {
                    push_block(types::BlockContent::Tbl(tbl), &mut stack, builder);
                }
            }

            WmlEvent::StartTableRow { props } => {
                let mut row = Box::new(types::CTRow::default());
                #[cfg(feature = "wml-tables")]
                {
                    row.row_properties = Some(props);
                }
                #[cfg(not(feature = "wml-tables"))]
                {
                    let _ = props;
                }
                stack.push(WmlFrame::TableRow(row));
            }

            WmlEvent::EndTableRow => {
                if let Some(WmlFrame::TableRow(row)) = stack.pop()
                    && let Some(WmlFrame::Table(tbl)) = stack.last_mut()
                {
                    tbl.rows.push(types::RowContent::Tr(row));
                }
            }

            WmlEvent::StartTableCell { props } => {
                let mut cell = Box::new(types::TableCell::default());
                #[cfg(feature = "wml-tables")]
                {
                    cell.cell_properties = Some(props);
                }
                #[cfg(not(feature = "wml-tables"))]
                {
                    let _ = props;
                }
                stack.push(WmlFrame::TableCell(cell));
            }

            WmlEvent::EndTableCell => {
                if let Some(WmlFrame::TableCell(cell)) = stack.pop()
                    && let Some(WmlFrame::TableRow(row)) = stack.last_mut()
                {
                    row.cells.push(types::CellContent::Tc(cell));
                }
            }

            WmlEvent::Image { rel_id } => {
                if let Some(WmlFrame::Run(run)) = stack.last_mut()
                    && let Some(builder_rel_id) = rel_id_map.get(rel_id.as_ref() as &str)
                {
                    let mut drawing = Drawing::new();
                    drawing.add_image(builder_rel_id.clone());
                    let ct_drawing = builder.build_drawing(drawing);
                    run.run_content.push(types::RunContent::Drawing(Box::new(ct_drawing)));
                }
            }

            // TODO: footnote and endnote support in streaming writer
            WmlEvent::FootnoteRef { .. } | WmlEvent::EndnoteRef { .. } => {}
        }
    }

    Ok(())
}

/// Push block content to the nearest block parent (innermost table cell or document body).
fn push_block(
    content: types::BlockContent,
    stack: &mut [WmlFrame],
    builder: &mut DocumentBuilder,
) {
    if let Some(WmlFrame::TableCell(cell)) = stack.last_mut() {
        cell.block_content.push(content);
        return;
    }
    builder.body_mut().block_content.push(content);
}

/// Push paragraph content to the nearest paragraph or hyperlink on the stack.
fn push_para_content(content: types::ParagraphContent, stack: &mut [WmlFrame]) {
    for frame in stack.iter_mut().rev() {
        match frame {
            WmlFrame::Paragraph(para) => {
                para.paragraph_content.push(content);
                return;
            }
            #[cfg(feature = "wml-hyperlinks")]
            WmlFrame::Hyperlink(hl) => {
                hl.paragraph_content.push(content);
                return;
            }
            _ => {}
        }
    }
}
