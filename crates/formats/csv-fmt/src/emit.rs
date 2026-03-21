//! CSV emitter.

use crate::ast::CsvDoc;

/// Convert a [`CsvDoc`] into a CSV string.
pub fn emit(doc: &CsvDoc) -> String {
    let mut output = String::new();

    for row in &doc.rows {
        let fields: Vec<String> =
            row.cells.iter().map(|cell| escape_csv_field(&cell.value)).collect();
        output.push_str(&fields.join(","));
        output.push('\n');
    }

    output
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        field.to_string()
    }
}
