//! `Writer<W: Write>` — streaming EPUB writer built directly on
//! `zip-fmt::Writer`. The mandatory `mimetype` entry (stored, uncompressed,
//! first) is written automatically by [`Writer::new`]; every other part is
//! written as its own event, letting a caller emit a book without ever
//! holding the whole archive in memory (each event's content is handed
//! straight to `zip-fmt::Writer::write_entry`, which streams it through
//! `flate2`'s incremental Deflate encoder to the sink — see that module's
//! doc comment).

use std::io::Write;

use crate::ast::{
    CONTAINER_PATH, Container, ENCRYPTION_PATH, MIMETYPE_CONTENT, MIMETYPE_ENTRY, Ncx, Package,
};
use rescribe_format_api::Emit as _;
use zip_fmt::{CompressionMethod, Entry};

/// One EPUB part to write. Every variant carries its own archive path
/// (unlike the AST, where e.g. `Navigation`/`ContentDocument` own their
/// path field already) since streaming events have no surrounding tree to
/// read a path from.
pub enum WriteEvent {
    Container(Container),
    Package {
        path: String,
        package: Box<Package>,
    },
    Nav {
        path: String,
        doc: html_fmt::HtmlDoc,
    },
    Ncx {
        path: String,
        ncx: Ncx,
    },
    ContentDocument {
        path: String,
        doc: html_fmt::HtmlDoc,
    },
    Resource {
        path: String,
        content: Vec<u8>,
    },
    EncryptionXml(Vec<u8>),
    /// Any entry not covered above (e.g. re-emitting an
    /// [`crate::ast::EpubDoc::unclassified`] entry verbatim).
    Raw {
        path: String,
        content: Vec<u8>,
    },
}

pub struct Writer<W: Write> {
    inner: zip_fmt::Writer<W>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        let mut inner = zip_fmt::Writer::new(sink);
        inner.write_entry(&Entry {
            name: MIMETYPE_ENTRY.to_string(),
            is_utf8_name: true,
            compression: CompressionMethod::Store,
            content: MIMETYPE_CONTENT.as_bytes().to_vec(),
            ..Entry::default()
        });
        Writer { inner }
    }

    pub fn write_event(&mut self, event: WriteEvent) {
        match event {
            WriteEvent::Container(c) => {
                self.write(CONTAINER_PATH, crate::container::emit_container(&c))
            }
            WriteEvent::Package { path, package } => {
                self.write(&path, crate::opf::emit_package(&package))
            }
            WriteEvent::Nav { path, doc } => self.write(&path, doc.emit()),
            WriteEvent::Ncx { path, ncx } => self.write(&path, crate::ncx::emit_ncx(&ncx)),
            WriteEvent::ContentDocument { path, doc } => self.write(&path, doc.emit()),
            WriteEvent::Resource { path, content } => self.write(&path, content),
            WriteEvent::EncryptionXml(content) => self.write(ENCRYPTION_PATH, content),
            WriteEvent::Raw { path, content } => self.write(&path, content),
        }
    }

    fn write(&mut self, path: &str, content: Vec<u8>) {
        self.inner.write_entry(&Entry {
            name: path.to_string(),
            is_utf8_name: true,
            compression: CompressionMethod::Deflate,
            content,
            ..Entry::default()
        });
    }

    /// Finish the archive and return the sink. See
    /// `zip_fmt::Writer::finish`'s docs for why this is fallible (unlike
    /// most other `-fmt` crates' `Writer::finish`).
    pub fn finish(self) -> std::io::Result<W> {
        self.inner.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EpubDoc;
    use rescribe_format_api::Parse as _;

    #[test]
    fn streaming_writer_produces_parseable_epub() {
        let bytes = crate::testutil::sample_epub();
        let (doc, _) = EpubDoc::parse(&bytes);

        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(WriteEvent::Container(doc.container.clone()));
        w.write_event(WriteEvent::Package {
            path: doc.container.rootfiles[0].full_path.clone(),
            package: Box::new(doc.package.clone()),
        });
        if let Some(nav) = &doc.nav {
            w.write_event(WriteEvent::Nav {
                path: nav.path.clone(),
                doc: nav.doc.clone(),
            });
        }
        for cd in &doc.content_documents {
            w.write_event(WriteEvent::ContentDocument {
                path: cd.path.clone(),
                doc: cd.doc.clone(),
            });
        }
        for res in &doc.resources {
            w.write_event(WriteEvent::Resource {
                path: res.path.clone(),
                content: res.content.clone(),
            });
        }
        let bytes2 = w.finish().unwrap();

        let (doc2, diags) = EpubDoc::parse(&bytes2);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.strip_spans(), doc2.strip_spans());
    }
}
