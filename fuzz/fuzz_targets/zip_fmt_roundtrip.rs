#![no_main]

//! zip-fmt native AST roundtrip fuzz target.
//!
//! Constructs an arbitrary `Archive` from fuzz data, emits it via
//! `zip_fmt::emit`, parses it back via `zip_fmt::parse`, and asserts
//! equality on the fields that are actually guaranteed to survive that
//! round trip.
//!
//! Direction: arbitrary_zip_ast → emit → parse → assert equality. Per
//! `CLAUDE.md`'s roundtrip-direction rule, this starts from the format
//! crate's own `Archive`/`Entry` type, not from `parse(bytes)`.
//!
//! # Why the comparison is scoped down, not full-`Entry`
//!
//! `zip::write::FileOptions` (the `zip` crate's write API, which
//! `emit()`/`crate::Writer` both build on — see `emit.rs`'s and
//! `lib.rs`'s doc comments) has **no setter** for: the raw
//! general-purpose flags word, `version_made_by`, `internal_attrs`, the
//! DOS-attribute half of `external_attrs`, or a per-entry comment (only
//! an *archive*-level comment setter exists — `ZipWriter::set_raw_comment`,
//! confirmed by grepping `zip-7.2.0/src/write.rs` for `comment`: the only
//! per-file comment field lives in `ZipFileData`, populated on read, with
//! no corresponding writer setter). Those `Entry` fields are therefore
//! **not** currently round-trippable through any of this crate's five
//! APIs — not a bug in this fuzz target, but a real, source-verified gap
//! in the underlying library's write surface (flagged as an open
//! question in the crate's top-level design report: whether zip-fmt
//! should eventually hand-roll a raw-header-preserving writer to close
//! it, trading away the "adapt zip::write::ZipWriter, don't hand-roll an
//! encoder" guidance this crate otherwise follows).
//!
//! What **is** verified here, because it is what `emit()`/`parse()`
//! actually carry through: entry names, entry content bytes, compression
//! method (restricted to `Store`/`Deflate` — the only two methods this
//! crate both re-encodes *and* decodes; see `lib.rs`'s "Compression
//! coverage" section), and the archive-level comment.

use libfuzzer_sys::fuzz_target;
use zip_fmt::{Archive, CompressionMethod, Entry};

struct Gen<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Gen<'a> {
    fn new(data: &'a [u8]) -> Self {
        Gen { data, pos: 0 }
    }

    fn byte(&mut self) -> u8 {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            b
        } else {
            0
        }
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        let start = self.pos;
        let end = (self.pos + n).min(self.data.len());
        self.pos = end;
        self.data[start..end].to_vec()
    }

    /// A safe, ASCII, non-empty entry name with no leading `/` (this
    /// crate does not validate/reject conventionally-invalid names — the
    /// fuzz target sticks to well-formed ones to isolate the property
    /// under test).
    fn name(&mut self) -> String {
        let n = (self.byte() % 12) as usize + 1;
        let raw: String = self
            .bytes(n)
            .iter()
            .map(|b| ((*b % 26) + b'a') as char)
            .collect();
        if raw.is_empty() { "x".to_string() } else { raw }
    }

    fn compression(&mut self) -> CompressionMethod {
        if self.byte() % 2 == 0 {
            CompressionMethod::Store
        } else {
            CompressionMethod::Deflate
        }
    }

    fn content(&mut self) -> Vec<u8> {
        let n = (self.byte() as usize) * 4;
        self.bytes(n)
    }

    fn entry(&mut self) -> Entry {
        Entry {
            name: self.name(),
            compression: self.compression(),
            content: self.content(),
            is_utf8_name: true,
            ..Entry::default()
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let mut g = Gen::new(data);
    let count = (g.byte() % 6) + 1;
    // Names must be unique — `zip`'s writer does not reject duplicate
    // names, but `parse()`'s entry ordering/identity comparison here
    // assumes one entry per name so the zip-order-vs-generation-order
    // pairing below stays meaningful.
    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::new();
    for _ in 0..count {
        let mut e = g.entry();
        while !seen.insert(e.name.clone()) {
            e.name.push('x');
        }
        entries.push(e);
    }
    let comment_len = (g.byte() % 8) as usize;
    let comment = g.bytes(comment_len);

    let archive = Archive {
        entries,
        comment,
        span: Default::default(),
    };

    let emitted = zip_fmt::emit(&archive);
    let (archive2, diags) = zip_fmt::parse(&emitted);
    assert!(
        diags.is_empty(),
        "unexpected diagnostics reparsing generated archive: {diags:?}"
    );

    assert_eq!(
        archive.comment, archive2.comment,
        "archive comment mismatch"
    );
    assert_eq!(
        archive.entries.len(),
        archive2.entries.len(),
        "entry count mismatch"
    );
    for (a, b) in archive.entries.iter().zip(archive2.entries.iter()) {
        assert_eq!(a.name, b.name, "entry name mismatch");
        assert_eq!(a.compression, b.compression, "entry compression mismatch");
        assert_eq!(a.content, b.content, "entry content mismatch");
    }
});
