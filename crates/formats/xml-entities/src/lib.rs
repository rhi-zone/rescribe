//! DTD-aware XML named entity declaration parsing and resolution.
//!
//! XML defines exactly five predefined named entities (`&lt;` `&gt;` `&amp;`
//! `&apos;` `&quot;`). Every other named entity reference — `&eacute;`,
//! `&mdash;`, `&hellip;`, and the ISO 8879/9573 sets DocBook, JATS, and TEI
//! documents lean on heavily (`ISOlat1`, `ISOnum`, `ISOpub`, `ISOtech`,
//! `mmlalias`, and so on) — only means something in light of the document's
//! DTD, which declares what each name expands to. General-purpose XML
//! parsers (`quick-xml`, `xml-rs`, `roxmltree`) deliberately don't parse DTD
//! internal subsets, so they hand callers the bare `&name;` reference and
//! leave resolution up to them.
//!
//! This crate fills that gap, in two layers:
//!
//! 1. [`DtdEntities`] parses `<!ENTITY ...>` declarations out of a document's
//!    own DOCTYPE internal subset — the common case for documents that
//!    declare a handful of custom entities directly (e.g. a company name
//!    abbreviation). This is a narrow, special-purpose parser: it recognizes
//!    just enough of `<!ELEMENT>`/`<!ATTLIST>`/`<!NOTATION>` declarations to
//!    skip over them correctly, and does not fetch external
//!    (`SYSTEM`/`PUBLIC`) subsets or entities — no network or filesystem
//!    access happens anywhere in this crate.
//! 2. [`standard::resolve_standard`] looks up the WHATWG HTML5 named
//!    character reference table (via the [`entities`] crate), which is a
//!    close superset of the ISO 8879/9573 sets — in practice this is what
//!    resolves most of the entities those big DTDs pull in via an
//!    externally-fetched parameter entity (which this crate, again, does
//!    not fetch).
//!
//! [`EntityResolver`] combines both layers into one query surface: build it
//! once per document from that document's [`DtdEntities`], then call
//! [`EntityResolver::resolve`] for every `&name;` reference found while
//! walking the document. Names neither layer recognizes resolve to
//! [`Resolution::Unknown`] rather than an error, so callers building a
//! lossless converter (or any other tool that must not silently drop
//! content) can raw-preserve the original reference text.
//!
//! # What this crate does not do
//!
//! - **Not a DTD validator.** Element content models, attribute lists, and
//!   notations are recognized only well enough to be skipped; nothing about
//!   them is retained or validated.
//! - **Does not fetch external subsets or external entities.** A
//!   `<!ENTITY % isolat1 PUBLIC "..." "isolat1.ent">` parameter entity (the
//!   idiom DocBook and friends actually use to pull in the ISO sets) is
//!   recorded, but its content is never fetched — resolving those names
//!   depends on the standard-table fallback instead, since HTML5's table
//!   absorbed nearly all of them.
//! - **No SGML support.** These formats are typically consumed as
//!   well-formed XML (DocBook/JATS/TEI's XML flavors); genuine SGML parsing
//!   (implied tags, markup minimization, etc.) is out of scope.

mod decl;
mod resolve;
mod standard;

pub use decl::{Diagnostic, DtdEntities, EntityDecl, EntityDeclKind};
pub use resolve::{EntityResolver, Resolution, Source};
pub use standard::{resolve_standard, standard_table_len};
