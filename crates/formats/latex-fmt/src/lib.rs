//! LaTeX tokenizer, AST, and emitter.
//!
//! A standalone crate with **no rescribe dependency by default** — usable
//! as a general-purpose Rust LaTeX library. The optional `rescribe` feature
//! (default off) adds `crate::rescribe::{to_document, from_document}`, a
//! thin adapter translating [`LatexDoc`] to and from rescribe's `Document`
//! IR.
//!
//! # Architecture
//!
//! Tokenization ([`tokenize`]) is catcode-driven and command-name-agnostic
//! — see that module's docs for the full rationale, including the two
//! explicit scope boundaries: no runtime `\catcode` redefinition, and `~`
//! / `^` / `_` are tokenized as ordinary text rather than distinct catcode
//! classes (math content is captured as a raw source span instead).
//!
//! A separate semantic layer ([`parse`]) walks tokens and resolves a
//! control sequence's structural meaning **only** from an in-document
//! `\def`/`\newcommand`/`\newenvironment`-family definition (tracked with
//! real TeX grouping-scope semantics — see [`parse`]'s module docs for the
//! full resolution model and its rationale). There is no built-in table of
//! "standard LaTeX commands" consulted as a fallback: `\section`,
//! `\textbf`, `\item`, and everything else the LaTeX kernel/document
//! class/packages would normally define is raw-preserved
//! ([`ast::Node::ControlSymbol`] / [`ast::Node::RawEnvironment`], no
//! arguments consumed) with an `Info`-severity [`Diagnostic`] recording
//! that the construct's meaning was not verified — unless the document
//! itself locally defines it, in which case that definition is ground
//! truth. Still fully round-trippable either way, just not semantically
//! modeled in the unresolved case.
//!
//! # API layers
//!
//! `LatexDoc` implements the shared `rescribe-format-api` capability
//! traits:
//!
//! ```text
//! use rescribe_format_api::{Emit, Events, Parse, StreamingParse, StreamingWrite};
//!
//! let (doc, diags): (LatexDoc, Vec<Diagnostic>) = LatexDoc::parse(input);
//! let it = LatexDoc::events(input);
//! let mut p = LatexDoc::streaming_parser(|ev| { /* ... */ });
//! p.feed(chunk); p.finish();
//! let bytes: Vec<u8> = doc.emit();
//! let mut w = LatexDoc::writer(sink);
//! w.write_event(event); w.finish();
//! ```
//!
//! # Known limitations (tracked in TODO.md)
//!
//! - No runtime `\catcode` redefinition support (see [`tokenize`]).
//! - Optional arguments (`\cmd[...]`) are recognized via a bracket-depth
//!   scan over raw source bytes rather than full token-level parsing; an
//!   optional argument containing unbalanced `[`/`]` characters *inside* a
//!   nested `{...}` group is not specially handled.
//! - By default (and always, regardless of feature flags, for
//!   [`parse::parse`]/[`events::events`]/[`batch::StreamingParser`]), there
//!   is no resolution of unknown names against real `.cls`/`.sty` package
//!   content. As a consequence, essentially every command a typical real
//!   document uses (`\section`, `\textbf`, `\cite`, ...) raw-preserves with
//!   an `Info` diagnostic rather than being semantically modeled, since
//!   real documents almost never locally `\def`/`\newcommand` their own
//!   structural commands. The optional `package-resolve` feature (default
//!   off) adds [`resolve::parse_with_package_resolution`], which resolves
//!   `\documentclass`/`\usepackage`/`\LoadClass`/`\RequirePackage` targets
//!   against real package content via a local content-addressed disk
//!   cache, a best-effort local-TeX-install filesystem probe, and a
//!   best-effort CTAN/mirror network fetch — no package source is ever
//!   vendored into this crate; see [`resolve`]'s module docs for the full
//!   design, including why nothing here is a redistribution concern.
//! - `\begingroup`/`\endgroup` are not modeled as scope boundaries for
//!   in-document macro tracking (only `{...}` groups, command/environment
//!   argument groups, and known-environment bodies are).
//! - `\def`'s implied argument count is inferred by counting `#<digit>`
//!   tokens in its parameter text; TeX's more general delimited-parameter
//!   patterns are not specially handled (the raw parameter-text is still
//!   captured verbatim for round-tripping even when the inferred count is
//!   wrong).

pub mod ast;
pub mod batch;
pub mod emit;
pub mod events;
pub mod parse;
#[cfg(feature = "rescribe")]
pub mod rescribe;
#[cfg(feature = "package-resolve")]
pub mod resolve;
pub mod tokenize;
pub mod writer;

pub use ast::{Ast, Diagnostic, LatexDoc, Node, Severity, Span};

impl rescribe_format_api::Parse for LatexDoc {
    fn parse(input: &[u8]) -> (Self, Vec<Diagnostic>) {
        let text = String::from_utf8_lossy(input);
        parse::parse(&text)
    }
}

impl rescribe_format_api::Emit for LatexDoc {
    fn emit(&self) -> Vec<u8> {
        emit::emit(self).into_bytes()
    }
}

impl rescribe_format_api::Events for LatexDoc {
    type Event<'a> = events::Event<'a>;
    type EventIter<'a> = events::EventIter<'a>;

    fn events(input: &[u8]) -> events::EventIter<'_> {
        events::events(input)
    }
}

impl rescribe_format_api::StreamingParse for LatexDoc {
    type Event = events::OwnedEvent;
    type Parser<H: rescribe_format_api::Handler<events::OwnedEvent>> = batch::StreamingParser<H>;

    fn streaming_parser<H: rescribe_format_api::Handler<events::OwnedEvent>>(
        handler: H,
    ) -> batch::StreamingParser<H> {
        batch::StreamingParser::new(handler)
    }
}

impl rescribe_format_api::StreamingWrite for LatexDoc {
    type Writer<W: std::io::Write> = writer::Writer<W>;

    fn writer<W: std::io::Write>(sink: W) -> writer::Writer<W> {
        writer::Writer::new(sink)
    }
}
