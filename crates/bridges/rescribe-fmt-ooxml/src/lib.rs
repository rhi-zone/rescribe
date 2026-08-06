//! DOCX/PPTX/XLSX (OOXML) reader+writer IR adapters for rescribe.
//!
//! Consolidates what were six separate thin adapter crates
//! (`rescribe-read-docx`, `rescribe-write-docx`, `rescribe-read-pptx`,
//! `rescribe-write-pptx`, `rescribe-read-xlsx`, `rescribe-write-xlsx`) into
//! one crate with a Cargo feature per format (`docx`, `pptx`, `xlsx`, all
//! default-on). Each format's translation logic lives in its own module and
//! wraps the workspace's own OOXML schema crates — `ooxml-wml` (docx),
//! `ooxml-pml`/`ooxml-dml` (pptx), `ooxml-sml` (xlsx).
//!
//! There is no standalone `docx-fmt`/`pptx-fmt`/`xlsx-fmt` library crate:
//! unlike the hand-rolled `{format}-fmt` crates (rst-fmt, rtf-fmt, ...),
//! OOXML parsing/emitting itself lives in the `ooxml-*` schema crates, which
//! are format libraries in their own right. This crate is purely the IR
//! adapter layer on top of them — AST↔`Document` translation only, no
//! tokenizing/parsing/emitting of OOXML bytes (see CLAUDE.md's "adapter
//! layer must never contain parsing or writing logic").

#[cfg(feature = "docx")]
pub mod docx;
#[cfg(feature = "pptx")]
pub mod pptx;
#[cfg(feature = "xlsx")]
pub mod xlsx;
