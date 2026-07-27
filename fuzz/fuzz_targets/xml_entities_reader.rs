#![no_main]

//! xml-entities no-panic gate.
//!
//! Feeds arbitrary bytes (interpreted as UTF-8, lossily) to
//! `DtdEntities::parse_subset`/`parse_doctype` and then resolves every
//! declared name plus a couple of fixed standard-table probes through
//! `EntityResolver`. Must not panic regardless of input — malformed or
//! adversarial DTD internal subsets (unterminated literals, self-referential
//! or mutually-recursive entities, runaway parameter-entity expansion) are
//! reported as best-effort results/diagnostics, never a panic or a hang.

use libfuzzer_sys::fuzz_target;
use xml_entities::{DtdEntities, EntityResolver};

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);

    let (subset_entities, _diagnostics) = DtdEntities::parse_subset(&text);
    let resolver = EntityResolver::new(subset_entities);
    for decl in resolver.declared().iter() {
        let _ = resolver.resolve(&decl.name);
    }
    for decl in resolver.declared().iter_parameters() {
        let _ = resolver.declared().get_parameter(&decl.name);
    }

    let (doctype_entities, _diagnostics) = DtdEntities::parse_doctype(&text);
    let resolver = EntityResolver::new(doctype_entities);
    for decl in resolver.declared().iter() {
        let _ = resolver.resolve(&decl.name);
    }

    // Fixed probes into the standard table, exercised on every input so ASan
    // sees the OnceLock init path under fuzzing too.
    let _ = xml_entities::resolve_standard(&text);
});
