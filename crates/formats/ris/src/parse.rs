//! RIS format parser.

use crate::ast::{Diagnostic, RisDoc, RisEntry, Span};

/// Parse RIS text into a [`RisDoc`].
///
/// This function is infallible: errors are returned as [`Diagnostic`]s rather
/// than causing a hard failure.
pub fn parse(input: &str) -> (RisDoc, Vec<Diagnostic>) {
    let mut entries = Vec::new();
    let mut diagnostics = Vec::new();
    let mut current_entry: Option<RisEntry> = None;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // RIS format: TAG  - VALUE (tag is 2-4 chars, followed by two spaces, dash, space, value)
        if line.len() >= 6
            && line.is_char_boundary(2)
            && line.is_char_boundary(4)
            && line.is_char_boundary(6)
            && &line[4..6] == "- "
        {
            let tag = line[0..2].trim();
            let value = line[6..].trim();

            match tag {
                "TY" => {
                    // Start of new entry
                    if let Some(entry) = current_entry.take() {
                        entries.push(entry);
                    }
                    current_entry = Some(RisEntry::new(value));
                }
                "ER" => {
                    // End of entry
                    if let Some(entry) = current_entry.take() {
                        entries.push(entry);
                    }
                }
                _ => {
                    // Add field to current entry
                    if let Some(ref mut entry) = current_entry {
                        entry.add_field(tag, value);
                    } else {
                        diagnostics.push(Diagnostic {
                            message: format!(
                                "field '{tag}' outside of any entry; ignoring"
                            ),
                            severity: crate::ast::Severity::Warning,
                            span: Span::NONE,
                        });
                    }
                }
            }
        }
    }

    // Handle entry without ER tag
    if let Some(entry) = current_entry {
        entries.push(entry);
    }

    (RisDoc { entries, span: Span::NONE }, diagnostics)
}
