//! RIS format AST types.

use std::collections::HashMap;

// ── Span / Diagnostic ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const NONE: Self = Self { start: 0, end: 0 };
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub message: String,
    pub severity: Severity,
    pub span: Span,
}

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
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RisDoc {
    pub entries: Vec<RisEntry>,
    pub span: Span,
}

impl RisDoc {
    /// Recursively reset all spans to [`Span::NONE`].
    pub fn strip_spans(&mut self) {
        self.span = Span::NONE;
        for entry in &mut self.entries {
            entry.strip_spans();
        }
    }
}

/// A single RIS bibliographic entry.
#[derive(Debug, Clone, PartialEq)]
pub struct RisEntry {
    pub entry_type: String,
    pub fields: HashMap<String, Vec<String>>,
    pub span: Span,
}

impl RisEntry {
    pub fn new(entry_type: &str) -> Self {
        Self {
            entry_type: entry_type.to_string(),
            fields: HashMap::new(),
            span: Span::NONE,
        }
    }

    /// Recursively reset all spans to [`Span::NONE`].
    pub fn strip_spans(&mut self) {
        self.span = Span::NONE;
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
