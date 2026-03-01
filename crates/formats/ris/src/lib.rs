//! RIS (Research Information Systems) citation format parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-ris` and `rescribe-write-ris` as thin adapter layers.

use std::collections::HashMap;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum RisError {
    Invalid(String),
    UnsupportedFormat(String),
    Io(std::io::Error),
}

impl std::fmt::Display for RisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RisError::Invalid(msg) => write!(f, "Invalid RIS: {}", msg),
            RisError::UnsupportedFormat(msg) => write!(f, "Unsupported RIS format: {}", msg),
            RisError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for RisError {}

impl From<std::io::Error> for RisError {
    fn from(err: std::io::Error) -> Self {
        RisError::Io(err)
    }
}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed RIS document containing bibliography records.
#[derive(Debug, Clone, Default)]
pub struct RisDoc {
    pub entries: Vec<RisEntry>,
}

/// A single RIS bibliographic entry.
#[derive(Debug, Clone)]
pub struct RisEntry {
    pub entry_type: String,
    pub fields: HashMap<String, Vec<String>>,
}

impl RisEntry {
    pub fn new(entry_type: &str) -> Self {
        Self {
            entry_type: entry_type.to_string(),
            fields: HashMap::new(),
        }
    }

    pub fn add_field(&mut self, tag: &str, value: &str) {
        self.fields
            .entry(tag.to_string())
            .or_default()
            .push(value.to_string());
    }

    pub fn get_first(&self, tag: &str) -> Option<&str> {
        self.fields
            .get(tag)
            .and_then(|v| v.first().map(|s| s.as_str()))
    }

    pub fn get_all(&self, tag: &str) -> Vec<&str> {
        self.fields
            .get(tag)
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Generate a citation key from author and year.
    pub fn generate_cite_key(&self) -> String {
        let author_part = self
            .get_first("AU")
            .map(|a| {
                // Get last name (before comma) and take first 8 chars
                a.split(',')
                    .next()
                    .unwrap_or(a)
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .take(8)
                    .collect::<String>()
                    .to_lowercase()
            })
            .unwrap_or_else(|| "unknown".to_string());

        let year_part = self
            .get_first("PY")
            .or_else(|| self.get_first("Y1"))
            .map(|y| y.split('/').next().unwrap_or(y).to_string())
            .unwrap_or_default();

        format!("{}{}", author_part, year_part)
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse RIS text into a [`RisDoc`].
pub fn parse(input: &str) -> Result<RisDoc, RisError> {
    let mut entries = Vec::new();
    let mut current_entry: Option<RisEntry> = None;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // RIS format: TAG  - VALUE (tag is 2-4 chars, followed by two spaces, dash, space, value)
        if line.len() >= 6 && &line[4..6] == "- " {
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
                    }
                }
            }
        }
    }

    // Handle entry without ER tag
    if let Some(entry) = current_entry {
        entries.push(entry);
    }

    Ok(RisDoc { entries })
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build an RIS string from a [`RisDoc`].
pub fn build(doc: &RisDoc) -> String {
    let mut output = String::new();

    for entry in &doc.entries {
        write_entry(&mut output, entry);
    }

    output
}

fn write_entry(output: &mut String, entry: &RisEntry) {
    write_tag(output, "TY", &entry.entry_type);

    // Emit all fields in the order they appear in the fields map
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

// ── Helper Functions ──────────────────────────────────────────────────────────

/// Map RIS reference types to BibTeX types.
pub fn ris_type_to_bibtex(ris_type: &str) -> &'static str {
    match ris_type {
        "JOUR" => "article",
        "BOOK" => "book",
        "CHAP" | "SECT" => "incollection",
        "CONF" | "CPAPER" => "inproceedings",
        "THES" => "phdthesis",
        "RPRT" => "techreport",
        "MGZN" | "NEWS" => "article",
        "ELEC" | "WEB" => "online",
        "COMP" => "software",
        "DATA" => "dataset",
        "ABST" | "INPR" | "JFULL" => "article",
        "EDBOOK" => "book",
        "GEN" | "CTLG" | "ENCYC" | "DICT" => "misc",
        "MANSCPT" | "UNPB" => "unpublished",
        "PAMP" => "booklet",
        "PAT" => "misc",
        "SER" => "book",
        "SLIDE" | "VIDEO" | "SOUND" | "MAP" | "ADVS" | "ART" => "misc",
        _ => "misc",
    }
}

/// Map BibTeX reference types to RIS types.
pub fn bibtex_type_to_ris(bibtex: &str) -> &'static str {
    match bibtex.to_lowercase().as_str() {
        "article" => "JOUR",
        "book" => "BOOK",
        "incollection" | "inbook" => "CHAP",
        "inproceedings" | "conference" => "CONF",
        "phdthesis" => "THES",
        "mastersthesis" => "THES",
        "techreport" => "RPRT",
        "online" => "ELEC",
        "software" => "COMP",
        "dataset" => "DATA",
        "unpublished" => "UNPB",
        "booklet" => "PAMP",
        "proceedings" => "CONF",
        "manual" => "BOOK",
        _ => "GEN",
    }
}

/// Map BibTeX field names to RIS tags.
pub fn bibtex_field_to_ris(field: &str) -> Option<&'static str> {
    match field {
        "author" => Some("AU"),
        "title" => Some("TI"),
        "journal" => Some("JO"),
        "booktitle" => Some("T2"),
        "year" => Some("PY"),
        "volume" => Some("VL"),
        "number" => Some("IS"),
        "publisher" => Some("PB"),
        "address" => Some("CY"),
        "doi" => Some("DO"),
        "url" => Some("UR"),
        "abstract" => Some("AB"),
        "keywords" => Some("KW"),
        "isbn" | "issn" => Some("SN"),
        "edition" => Some("ET"),
        "note" => Some("N1"),
        "type" | "key" => None,
        _ => None,
    }
}

/// Map CSL JSON types to RIS types.
pub fn csl_type_to_ris(csl: &str) -> &'static str {
    match csl {
        "article-journal" | "article-magazine" | "article-newspaper" => "JOUR",
        "book" => "BOOK",
        "chapter" => "CHAP",
        "paper-conference" => "CONF",
        "thesis" => "THES",
        "report" => "RPRT",
        "webpage" | "post-weblog" => "ELEC",
        "software" => "COMP",
        "dataset" => "DATA",
        _ => "GEN",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_article() {
        let ris = r#"TY  - JOUR
AU  - Smith, John
AU  - Doe, Jane
TI  - A Great Paper
JO  - Nature
PY  - 2020
VL  - 123
SP  - 45
EP  - 67
DO  - 10.1234/nature.2020
ER  -"#;

        let doc = parse(ris).unwrap();
        assert_eq!(doc.entries.len(), 1);
        let entry = &doc.entries[0];
        assert_eq!(entry.entry_type, "JOUR");
        assert_eq!(entry.get_first("TI"), Some("A Great Paper"));
        assert_eq!(entry.get_all("AU").len(), 2);
    }

    #[test]
    fn test_parse_book() {
        let ris = r#"TY  - BOOK
AU  - Knuth, Donald E.
TI  - The Art of Computer Programming
PY  - 1997
ER  -"#;

        let doc = parse(ris).unwrap();
        assert_eq!(doc.entries.len(), 1);
        let entry = &doc.entries[0];
        assert_eq!(entry.entry_type, "BOOK");
    }

    #[test]
    fn test_parse_multiple() {
        let ris = r#"TY  - JOUR
AU  - First, Author
TI  - First Paper
ER  -
TY  - JOUR
AU  - Second, Author
TI  - Second Paper
ER  -"#;

        let doc = parse(ris).unwrap();
        assert_eq!(doc.entries.len(), 2);
    }

    #[test]
    fn test_parse_empty() {
        let ris = "";
        let doc = parse(ris).unwrap();
        assert!(doc.entries.is_empty());
    }

    #[test]
    fn test_generate_cite_key() {
        let mut entry = RisEntry::new("JOUR");
        entry.add_field("AU", "Smith, John");
        entry.add_field("PY", "2020");

        let key = entry.generate_cite_key();
        assert_eq!(key, "smith2020");
    }

    #[test]
    fn test_build_simple() {
        let mut entry = RisEntry::new("JOUR");
        entry.add_field("AU", "Smith, John");
        entry.add_field("TI", "A Paper");

        let doc = RisDoc {
            entries: vec![entry],
        };

        let output = build(&doc);
        assert!(output.contains("TY  - JOUR"));
        assert!(output.contains("AU  - Smith, John"));
        assert!(output.contains("TI  - A Paper"));
        assert!(output.contains("ER  -"));
    }

    #[test]
    fn test_ris_type_to_bibtex() {
        assert_eq!(ris_type_to_bibtex("JOUR"), "article");
        assert_eq!(ris_type_to_bibtex("BOOK"), "book");
        assert_eq!(ris_type_to_bibtex("THES"), "phdthesis");
    }

    #[test]
    fn test_bibtex_type_to_ris() {
        assert_eq!(bibtex_type_to_ris("article"), "JOUR");
        assert_eq!(bibtex_type_to_ris("book"), "BOOK");
        assert_eq!(bibtex_type_to_ris("phdthesis"), "THES");
    }
}
