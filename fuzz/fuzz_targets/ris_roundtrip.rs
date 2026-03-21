#![no_main]
//! RIS roundtrip fuzz target.
//!
//! Generates an arbitrary RisDoc, emits it to RIS text, parses it back,
//! strips spans, and asserts the result is equal to the original.
//!
//! Direction: arbitrary RisDoc → emit → parse → strip_spans → assert equal
//!
//! Constraints:
//! - Tags must be exactly 2 uppercase ASCII chars (the RIS format requirement)
//! - Values must not contain newlines (each field is one line)
//! - Entry type must not contain newlines

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use ris::{RisDoc, RisEntry, Span};
use std::collections::HashMap;

/// A small set of valid 2-char RIS tags.
static VALID_TAGS: &[&str] = &[
    "TI", "AU", "AB", "PY", "JO", "VL", "IS", "SP", "EP", "DO", "UR", "KW", "PB", "CY",
];

#[derive(Arbitrary, Debug)]
struct FuzzField {
    tag_idx: u8,
    value: String,
}

#[derive(Arbitrary, Debug)]
struct FuzzEntry {
    ty_idx: u8,
    fields: Vec<FuzzField>,
}

fn sanitise_value(s: &str) -> String {
    // Strip newlines and null bytes; trim whitespace (parser trims values)
    s.chars()
        .filter(|c| !matches!(*c, '\x00' | '\n' | '\r'))
        .collect::<String>()
        .trim()
        .to_string()
}

/// Known RIS entry types.
static RIS_TYPES: &[&str] = &[
    "JOUR", "BOOK", "CHAP", "CONF", "THES", "RPRT", "ELEC", "NEWS", "PAMP",
];

fuzz_target!(|entries: Vec<FuzzEntry>| {
    let mut doc_entries = Vec::new();

    for fe in &entries {
        let entry_type = RIS_TYPES[(fe.ty_idx as usize) % RIS_TYPES.len()];
        let mut entry = RisEntry::new(entry_type);
        let mut fields: HashMap<String, Vec<String>> = HashMap::new();

        for ff in &fe.fields {
            let tag = VALID_TAGS[(ff.tag_idx as usize) % VALID_TAGS.len()];
            let value = sanitise_value(&ff.value);
            if value.is_empty() {
                continue;
            }
            fields.entry(tag.to_string()).or_default().push(value);
        }
        entry.fields = fields;
        doc_entries.push(entry);
    }

    let mut doc = RisDoc { entries: doc_entries, span: Span::NONE };
    let ris_text = ris::emit(&doc);

    let (mut parsed, _diags) = ris::parse(&ris_text);
    parsed.strip_spans();
    doc.strip_spans();

    assert_eq!(
        doc,
        parsed,
        "RIS roundtrip mismatch\n  emitted:\n{ris_text}"
    );
});
