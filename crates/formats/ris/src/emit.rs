//! RIS format emitter.

use crate::ast::{RisDoc, RisEntry};

/// Emit a [`RisDoc`] as an RIS-formatted string.
pub fn emit(doc: &RisDoc) -> String {
    let mut output = String::new();
    for entry in &doc.entries {
        write_entry(&mut output, entry);
    }
    output
}

fn write_entry(output: &mut String, entry: &RisEntry) {
    write_tag(output, "TY", &entry.entry_type);

    for (tag, values) in &entry.fields {
        for value in values {
            write_tag(output, tag, value);
        }
    }

    write_tag(output, "ER", "");
    output.push('\n');
}

fn write_tag(output: &mut String, tag: &str, value: &str) {
    output.push_str(tag);
    output.push_str("  - ");
    output.push_str(value);
    output.push('\n');
}
