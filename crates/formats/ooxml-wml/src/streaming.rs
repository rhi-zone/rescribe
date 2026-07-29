//! Event-driven DOCX writer.
//!
//! [`WmlWriter`] accepts [`WmlEvent`] items via [`WmlWriter::write_event`]
//! and produces a complete `.docx` file on [`WmlWriter::finish`].
//!
//! # Memory model
//!
//! Each event is serialised to XML bytes and pushed straight into the open
//! `word/document.xml` ZIP entry as it arrives. Nothing about the document body
//! is retained: there is no event buffer and no partial AST. Peak memory is
//!
//! ```text
//! O(nesting depth) + O(largest single props element) + O(number of relationships)
//! ```
//!
//! and is independent of the number of paragraphs, runs, tables or characters
//! written.
//!
//! ## What is deferred, and why
//!
//! Three things genuinely cannot be emitted at the moment the event arrives:
//!
//! * **`word/_rels/document.xml.rels`** — a relationship is only known to exist
//!   once the event referencing it has been seen, so the rels part is written
//!   after `word/document.xml` is closed. ZIP entry order is not significant to
//!   OPC consumers, so this costs O(number of relationships) — a count, not a
//!   content size.
//! * **`[Content_Types].xml`** — same shape; written last by
//!   [`PackageWriter::finish`], costing O(number of parts).
//! * **The ZIP central directory** — written by `ZipWriter` at finish, as the
//!   format requires. This is the container's own structure, not buffered
//!   content.
//!
//! Everything else — paragraphs, runs, text, breaks, hyperlinks, tables, rows,
//! cells, images, footnote/endnote references — is written straight through.
//!
//! Image bytes handed to [`register_image`](WmlWriter::register_image) are
//! written into the package immediately if the document part has not been opened
//! yet (the normal usage: register up front, then feed events). Registering an
//! image after the first event forces it to be held until `finish()`, because a
//! ZIP archive can only have one entry open at a time.
//!
//! # Errors
//!
//! [`write_event`](WmlWriter::write_event) is infallible so that it can be used
//! as a `Handler`-style sink. The first I/O or serialisation error is latched and
//! returned from [`finish`](WmlWriter::finish); subsequent events are dropped.
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

use ooxml_opc::{PackageWriter, Relationship, Relationships, content_type, rel_type};
use quick_xml::Writer as XmlWriter;

use crate::generated_events::WmlEvent;
use crate::generated_serializers::ToXml;
use crate::types;
use crate::writer::{Drawing, NS_A, NS_PIC, NS_R, NS_W, NS_WP};
use crate::{Error, Result};

/// XML declaration written at the head of every part.
const XML_DECL: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n";

/// Which container a stack frame corresponds to.
///
/// One byte per level of nesting — this is the writer's entire structural state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Open {
    Paragraph,
    Run,
    Hyperlink,
    Table,
    TableRow,
    TableCell,
}

impl Open {
    fn open_tag(self) -> &'static [u8] {
        match self {
            Open::Paragraph => b"<w:p>",
            Open::Run => b"<w:r>",
            Open::Hyperlink => b"<w:hyperlink",
            Open::Table => b"<w:tbl>",
            Open::TableRow => b"<w:tr>",
            Open::TableCell => b"<w:tc>",
        }
    }

    fn close_tag(self) -> &'static [u8] {
        match self {
            Open::Paragraph => b"</w:p>",
            Open::Run => b"</w:r>",
            Open::Hyperlink => b"</w:hyperlink>",
            Open::Table => b"</w:tbl>",
            Open::TableRow => b"</w:tr>",
            Open::TableCell => b"</w:tc>",
        }
    }
}

/// How much emitted XML to accumulate before pushing it into the ZIP entry.
///
/// Handing single tags to the deflate encoder one at a time is what made the
/// first version of this writer ~8x slower than the builder path, which
/// serialises into one `Vec` and writes it once. A fixed window restores the
/// throughput without reintroducing document-sized buffering: this is O(1),
/// not O(document).
const OUT_BUF_CAPACITY: usize = 64 * 1024;

/// An image whose bytes could not be written immediately because the
/// `word/document.xml` entry was already open.
struct DeferredImage {
    path: String,
    content_type: String,
    data: Vec<u8>,
}

/// Event-driven DOCX writer.
///
/// Feed [`WmlEvent`] items one at a time, then call [`finish`](WmlWriter::finish)
/// to produce a complete DOCX document. See the [module docs](self) for the
/// memory model.
pub struct WmlWriter<W: Write + Seek> {
    pkg: PackageWriter<W>,
    /// Fixed-capacity window of emitted XML, flushed into the open ZIP entry
    /// whenever it fills. O(1) — see [`OUT_BUF_CAPACITY`].
    out: Vec<u8>,
    /// Relationships for `word/document.xml`. O(number of relationships).
    doc_rels: Relationships,
    /// Caller-supplied rel id -> package rel id, for images.
    image_rels: HashMap<String, String>,
    /// Images registered after the document part was opened.
    deferred_images: Vec<DeferredImage>,
    /// Open container stack. O(nesting depth) — the only per-content state.
    stack: Vec<Open>,
    /// Next `rIdN` to hand out.
    next_rel_id: u32,
    /// Next DrawingML object id.
    next_drawing_id: usize,
    /// True once `word/document.xml` has been opened.
    started: bool,
    /// True once `</w:document>` has been written.
    ended: bool,
    /// First error seen; surfaced by `finish()`.
    error: Option<Error>,
}

impl<W: Write + Seek> WmlWriter<W> {
    /// Create a new writer targeting `sink`.
    pub fn new(sink: W) -> Self {
        WmlWriter {
            pkg: PackageWriter::new(sink),
            out: Vec::with_capacity(OUT_BUF_CAPACITY + 4096),
            doc_rels: Relationships::new(),
            image_rels: HashMap::new(),
            deferred_images: Vec::new(),
            stack: Vec::new(),
            next_rel_id: 1,
            next_drawing_id: 1,
            started: false,
            ended: false,
            error: None,
        }
    }

    fn alloc_rel_id(&mut self) -> String {
        let id = format!("rId{}", self.next_rel_id);
        self.next_rel_id += 1;
        id
    }

    /// Register image bytes under a caller-supplied rel_id.
    ///
    /// The `rel_id` must match the one used in [`WmlEvent::Image`] events; the
    /// mapping to the package's own relationship id is handled internally.
    ///
    /// Call this before feeding events: the image is then written into the
    /// package immediately and its bytes are not retained. Registering after the
    /// first event holds the bytes until [`finish`](Self::finish), because a ZIP
    /// archive permits only one open entry at a time.
    pub fn register_image(
        &mut self,
        rel_id: impl Into<String>,
        data: Vec<u8>,
        content_type: impl Into<String>,
    ) {
        let content_type = content_type.into();
        let pkg_rel_id = self.alloc_rel_id();
        let ext = extension_for(&content_type);
        let filename = format!("image{}.{}", self.next_rel_id - 1, ext);
        let path = format!("word/media/{}", filename);

        self.doc_rels.add(Relationship::new(
            &pkg_rel_id,
            rel_type::IMAGE,
            format!("media/{}", filename),
        ));
        self.image_rels.insert(rel_id.into(), pkg_rel_id);

        if self.started {
            self.deferred_images.push(DeferredImage {
                path,
                content_type,
                data,
            });
        } else {
            let r = write_whole_part(&mut self.pkg, &path, &content_type, &data);
            self.latch(r);
        }
    }

    /// Declare the target of a hyperlink relationship.
    ///
    /// [`WmlEvent::StartHyperlink`] carries only the relationship id, so an
    /// external hyperlink's URL must be supplied here for the emitted `r:id` to
    /// resolve. Costs O(1) per hyperlink; no content is buffered.
    pub fn register_hyperlink(&mut self, rel_id: impl Into<String>, url: &str) {
        self.doc_rels.add(Relationship::external(
            rel_id.into(),
            rel_type::HYPERLINK,
            url,
        ));
    }

    /// Record the first error and ignore later ones.
    fn latch(&mut self, r: Result<()>) {
        if let Err(e) = r
            && self.error.is_none()
        {
            self.error = Some(e);
        }
    }

    /// Open the package and the `word/document.xml` part.
    fn start(&mut self) -> Result<()> {
        self.pkg
            .add_default_content_type("rels", content_type::RELATIONSHIPS);
        self.pkg.add_default_content_type("xml", content_type::XML);
        self.pkg.add_default_content_type("png", "image/png");
        self.pkg.add_default_content_type("jpg", "image/jpeg");
        self.pkg.add_default_content_type("jpeg", "image/jpeg");
        self.pkg.add_default_content_type("gif", "image/gif");

        // Package relationships are fully known up front.
        let mut pkg_rels = Relationships::new();
        pkg_rels.add(Relationship::new(
            "rId1",
            rel_type::OFFICE_DOCUMENT,
            "word/document.xml",
        ));
        write_whole_part(
            &mut self.pkg,
            "_rels/.rels",
            content_type::RELATIONSHIPS,
            pkg_rels.serialize().as_bytes(),
        )?;

        self.pkg
            .start_part("word/document.xml", content_type::WORDPROCESSING_DOCUMENT)?;
        self.started = true;

        self.out.extend_from_slice(XML_DECL);
        self.out.extend_from_slice(b"<w:document");
        for (prefix, uri) in [
            ("w", NS_W),
            ("r", NS_R),
            ("wp", NS_WP),
            ("a", NS_A),
            ("pic", NS_PIC),
        ] {
            self.out.extend_from_slice(b" xmlns:");
            self.out.extend_from_slice(prefix.as_bytes());
            self.out.extend_from_slice(b"=\"");
            self.out.extend_from_slice(uri.as_bytes());
            self.out.push(b'"');
        }
        self.out.extend_from_slice(b"><w:body>");
        Ok(())
    }

    /// Push the accumulated window into the open ZIP entry and reset it.
    fn flush_out(&mut self) -> Result<()> {
        if !self.out.is_empty() {
            self.pkg.write_part_data(&self.out)?;
            self.out.clear();
        }
        Ok(())
    }

    /// Flush if the window has grown past its target size.
    fn flush_if_full(&mut self) -> Result<()> {
        if self.out.len() >= OUT_BUF_CAPACITY {
            self.flush_out()?;
        }
        Ok(())
    }

    fn ensure_started(&mut self) {
        if !self.started && self.error.is_none() {
            let r = self.start();
            self.latch(r);
        }
    }

    /// Serialise one `ToXml` value into the output window.
    fn emit<T: ToXml>(&mut self, value: &T, tag: &str) -> Result<()> {
        let mut xw = XmlWriter::new(&mut self.out);
        value.write_element(tag, &mut xw)?;
        self.flush_if_full()
    }

    fn raw(&mut self, bytes: &[u8]) -> Result<()> {
        self.out.extend_from_slice(bytes);
        self.flush_if_full()
    }

    /// Close the innermost open container if it matches `expected`.
    fn close(&mut self, expected: Open) -> Result<()> {
        if self.stack.last() == Some(&expected) {
            self.stack.pop();
            self.raw(expected.close_tag())?;
        }
        Ok(())
    }

    /// Process one event, writing it into the package immediately.
    ///
    /// Infallible by design; the first error is latched and returned from
    /// [`finish`](Self::finish).
    pub fn write_event(&mut self, event: WmlEvent<'_>) {
        if self.error.is_some() || self.ended {
            return;
        }
        self.ensure_started();
        if self.error.is_some() {
            return;
        }
        let r = self.write_event_inner(event);
        self.latch(r);
    }

    fn write_event_inner(&mut self, event: WmlEvent<'_>) -> Result<()> {
        match event {
            // `start()` already wrote the prologue; `finish()` writes the epilogue.
            WmlEvent::StartDocument | WmlEvent::EndDocument => {}

            WmlEvent::StartParagraph { props } => {
                self.stack.push(Open::Paragraph);
                self.raw(Open::Paragraph.open_tag())?;
                #[cfg(feature = "wml-styling")]
                self.emit(&props, "w:pPr")?;
                #[cfg(not(feature = "wml-styling"))]
                let _ = props;
            }
            WmlEvent::EndParagraph => self.close(Open::Paragraph)?,

            WmlEvent::StartRun { props } => {
                self.stack.push(Open::Run);
                self.raw(Open::Run.open_tag())?;
                #[cfg(feature = "wml-styling")]
                self.emit(&props, "w:rPr")?;
                #[cfg(not(feature = "wml-styling"))]
                let _ = props;
            }
            WmlEvent::EndRun => self.close(Open::Run)?,

            // Written by hand rather than through `types::Text`'s serializer:
            // this is the hot path, and going through the typed struct would
            // force an owning `String` per text event. The emitted bytes are
            // identical, `xml:space` rule included.
            WmlEvent::Text(text) => {
                self.out.extend_from_slice(b"<w:t");
                if text.starts_with(' ') || text.ends_with(' ') {
                    self.out.extend_from_slice(b" xml:space=\"preserve\"");
                }
                self.out.push(b'>');
                self.out
                    .extend_from_slice(quick_xml::escape::escape(&*text).as_bytes());
                self.out.extend_from_slice(b"</w:t>");
                self.flush_if_full()?;
            }

            WmlEvent::LineBreak => self.raw(b"<w:br/>")?,

            WmlEvent::StartHyperlink { rel_id, anchor } => {
                #[cfg(feature = "wml-hyperlinks")]
                {
                    self.stack.push(Open::Hyperlink);
                    self.out.extend_from_slice(Open::Hyperlink.open_tag());
                    if let Some(id) = rel_id.as_deref() {
                        self.out.extend_from_slice(b" r:id=\"");
                        self.out
                            .extend_from_slice(quick_xml::escape::escape(id).as_bytes());
                        self.out.push(b'"');
                    }
                    if let Some(a) = anchor.as_deref() {
                        self.out.extend_from_slice(b" w:anchor=\"");
                        self.out
                            .extend_from_slice(quick_xml::escape::escape(a).as_bytes());
                        self.out.push(b'"');
                    }
                    self.raw(b">")?;
                }
                #[cfg(not(feature = "wml-hyperlinks"))]
                let _ = (rel_id, anchor);
            }
            WmlEvent::EndHyperlink => {
                #[cfg(feature = "wml-hyperlinks")]
                self.close(Open::Hyperlink)?;
            }

            WmlEvent::StartTable { props } => {
                self.stack.push(Open::Table);
                self.raw(Open::Table.open_tag())?;
                self.emit(&props, "w:tblPr")?;
                self.emit(&types::TableGrid::default(), "w:tblGrid")?;
            }
            WmlEvent::EndTable => self.close(Open::Table)?,

            WmlEvent::StartTableRow { props } => {
                self.stack.push(Open::TableRow);
                self.raw(Open::TableRow.open_tag())?;
                #[cfg(feature = "wml-tables")]
                self.emit(&props, "w:trPr")?;
                #[cfg(not(feature = "wml-tables"))]
                let _ = props;
            }
            WmlEvent::EndTableRow => self.close(Open::TableRow)?,

            WmlEvent::StartTableCell { props } => {
                self.stack.push(Open::TableCell);
                self.raw(Open::TableCell.open_tag())?;
                #[cfg(feature = "wml-tables")]
                self.emit(&props, "w:tcPr")?;
                #[cfg(not(feature = "wml-tables"))]
                let _ = props;
            }
            WmlEvent::EndTableCell => self.close(Open::TableCell)?,

            WmlEvent::Image { rel_id } => {
                if let Some(pkg_rel_id) = self.image_rels.get(&*rel_id).cloned() {
                    let mut drawing = Drawing::new();
                    drawing.add_image(pkg_rel_id);
                    let ct = drawing.build(&mut self.next_drawing_id);
                    self.emit(&ct, "w:drawing")?;
                }
            }

            WmlEvent::FootnoteRef { id } => {
                let r = footnote_endnote_ref(id);
                self.emit(&r, "w:footnoteReference")?;
            }
            WmlEvent::EndnoteRef { id } => {
                let r = footnote_endnote_ref(id);
                self.emit(&r, "w:endnoteReference")?;
            }
        }
        Ok(())
    }

    /// Close the document part and write the package.
    pub fn finish(mut self) -> Result<()> {
        if let Some(e) = self.error.take() {
            return Err(e);
        }
        self.ensure_started();
        if let Some(e) = self.error.take() {
            return Err(e);
        }

        // Close any containers the caller left open, innermost first.
        while let Some(open) = self.stack.pop() {
            self.out.extend_from_slice(open.close_tag());
        }
        self.out.extend_from_slice(b"</w:body></w:document>");
        // The window must be drained before the next ZIP entry is opened.
        self.flush_out()?;
        self.ended = true;

        // Images registered after the document part was opened.
        for img in std::mem::take(&mut self.deferred_images) {
            write_whole_part(&mut self.pkg, &img.path, &img.content_type, &img.data)?;
        }

        // Relationship table: known only once every event has been seen.
        write_whole_part(
            &mut self.pkg,
            "word/_rels/document.xml.rels",
            content_type::RELATIONSHIPS,
            self.doc_rels.serialize().as_bytes(),
        )?;

        self.pkg.finish()?;
        Ok(())
    }
}

fn footnote_endnote_ref(id: i32) -> types::FootnoteEndnoteRef {
    types::FootnoteEndnoteRef {
        #[cfg(feature = "wml-comments")]
        custom_mark_follows: None,
        id: id.into(),
        #[cfg(feature = "extra-attrs")]
        extra_attrs: Default::default(),
    }
}

fn write_whole_part<W: Write + Seek>(
    pkg: &mut PackageWriter<W>,
    path: &str,
    content_type: &str,
    data: &[u8],
) -> Result<()> {
    pkg.add_part(path, content_type, data)?;
    Ok(())
}

fn extension_for(content_type: &str) -> &'static str {
    match content_type {
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/tiff" => "tiff",
        _ => "png",
    }
}
