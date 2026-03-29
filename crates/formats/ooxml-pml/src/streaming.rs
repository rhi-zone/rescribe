//! Event-driven PPTX writer.
//!
//! [`PmlWriter`] accepts [`PmlEvent`] items via [`PmlWriter::write_event`]
//! and produces a complete `.pptx` file on [`PmlWriter::finish`].
//!
//! # Slide layout
//!
//! All shapes from the event stream are placed on a single slide.  Each
//! `StartShape` / `EndShape` pair becomes one text box stacked vertically at
//! the default margin with a standard width.  Shape geometry is not carried by
//! [`PmlEvent`], so positions are assigned automatically.
//!
//! # Memory model
//!
//! Events are buffered until `finish()` is called.
//!
//! # Example
//!
//! ```ignore
//! use std::io::BufWriter;
//! use std::fs::File;
//! use ooxml_pml::{PmlWriter, PmlEvent};
//!
//! let sink = BufWriter::new(File::create("output.pptx")?);
//! let mut writer = PmlWriter::new(sink);
//! writer.write_event(PmlEvent::StartPresentation);
//! writer.write_event(PmlEvent::StartShape);
//! writer.write_event(PmlEvent::StartParagraph { props: Box::default() });
//! writer.write_event(PmlEvent::StartRun { props: Box::default() });
//! writer.write_event(PmlEvent::Text("Hello, world!".into()));
//! writer.write_event(PmlEvent::EndRun);
//! writer.write_event(PmlEvent::EndParagraph);
//! writer.write_event(PmlEvent::EndShape);
//! writer.write_event(PmlEvent::EndPresentation);
//! writer.finish()?;
//! # Ok::<(), ooxml_pml::Error>(())
//! ```

use std::io::{Seek, Write};

use crate::writer::PresentationBuilder;
use crate::Result;
use crate::generated_events::{OwnedPmlEvent, PmlEvent};

// Default slide dimensions: 10 in × 7.5 in in EMU (914400 EMU/in).
const SLIDE_W: i64 = 9_144_000;
const MARGIN: i64 = 457_200; // 0.5 in
const SHAPE_W: i64 = SLIDE_W - 2 * MARGIN;
const SHAPE_H: i64 = 914_400; // 1 in per shape
const SHAPE_GAP: i64 = 228_600; // 0.25 in between shapes

/// Event-driven PPTX writer.
///
/// Feed [`PmlEvent`] items one at a time, then call [`finish`](PmlWriter::finish)
/// to produce a complete PPTX presentation.
pub struct PmlWriter<W: Write + Seek> {
    sink: W,
    events: Vec<OwnedPmlEvent>,
}

impl<W: Write + Seek> PmlWriter<W> {
    /// Create a new writer targeting `sink`.
    pub fn new(sink: W) -> Self {
        PmlWriter { sink, events: Vec::new() }
    }

    /// Buffer one event.
    pub fn write_event(&mut self, event: PmlEvent<'_>) {
        self.events.push(event.into_owned());
    }

    /// Convert buffered events to a PPTX and write to the underlying sink.
    pub fn finish(self) -> Result<()> {
        let mut builder = PresentationBuilder::new();
        process_pml_events(&self.events, &mut builder);
        builder.write(self.sink)
    }
}

fn process_pml_events(events: &[OwnedPmlEvent], builder: &mut PresentationBuilder) {
    // Collect all text blocks from shapes, then add one slide.
    let mut shapes: Vec<String> = Vec::new();
    let mut current_paragraphs: Vec<String> = Vec::new();
    let mut current_para = String::new();
    let mut in_shape = false;
    let mut in_para = false;

    for event in events {
        match event {
            PmlEvent::StartPresentation | PmlEvent::EndPresentation => {}

            PmlEvent::StartShape | PmlEvent::StartGraphicFrame => {
                in_shape = true;
                current_paragraphs.clear();
            }

            PmlEvent::EndShape | PmlEvent::EndGraphicFrame => {
                // Finalize any open paragraph
                if in_para && !current_para.is_empty() {
                    current_paragraphs.push(current_para.clone());
                }
                in_para = false;
                current_para.clear();

                if !current_paragraphs.is_empty() {
                    shapes.push(current_paragraphs.join("\n"));
                    current_paragraphs.clear();
                }
                in_shape = false;
            }

            PmlEvent::StartParagraph { .. } => {
                current_para.clear();
                in_para = true;
            }

            PmlEvent::EndParagraph => {
                if in_shape && !current_para.is_empty() {
                    current_paragraphs.push(current_para.clone());
                    current_para.clear();
                }
                in_para = false;
            }

            PmlEvent::StartRun { .. } | PmlEvent::EndRun => {}

            PmlEvent::Text(text) => {
                if in_para {
                    current_para.push_str(text);
                }
            }

            PmlEvent::LineBreak => {
                if in_para {
                    current_para.push('\n');
                }
            }

            // Hyperlinks: pass through, text is collected by the Text handler
            PmlEvent::StartHyperlink { .. } | PmlEvent::EndHyperlink => {}

            // Table content: collect text from table cells
            PmlEvent::StartTable { .. }
            | PmlEvent::EndTable
            | PmlEvent::StartTableRow
            | PmlEvent::EndTableRow
            | PmlEvent::StartTableCell { .. }
            | PmlEvent::EndTableCell
            | PmlEvent::FieldId { .. } => {}
        }
    }

    if !shapes.is_empty() {
        let slide = builder.add_slide();
        for (i, text) in shapes.iter().enumerate() {
            let y = MARGIN + i as i64 * (SHAPE_H + SHAPE_GAP);
            slide.add_text_at(text.as_str(), MARGIN, y, SHAPE_W, SHAPE_H);
        }
    }
}
