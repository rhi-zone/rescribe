//! TSV emitter.

use crate::ast::TsvDoc;

/// Convert a [`TsvDoc`] into a TSV string.
pub fn emit(doc: &TsvDoc) -> String {
    let mut output = String::new();

    for row in &doc.rows {
        let fields: Vec<String> =
            row.cells.iter().map(|cell| escape_tsv_field(&cell.value)).collect();
        output.push_str(&fields.join("\t"));
        output.push('\n');
    }

    output
}

fn escape_tsv_field(field: &str) -> String {
    // TSV escaping: if field contains tab, newline, or quote, wrap in quotes
    if field.contains('\t') || field.contains('"') || field.contains('\n') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        field.to_string()
    }
}
