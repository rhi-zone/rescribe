//! Error types for odf-fmt.

use std::fmt;

/// A parse or I/O error when reading an ODF document.
#[derive(Debug)]
pub enum Error {
    /// The ZIP archive is invalid or corrupt.
    Zip(zip::result::ZipError),
    /// An XML parse error.
    Xml(quick_xml::Error),
    /// The file is not a valid ODF document.
    Invalid(String),
    /// An I/O error.
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Zip(e) => write!(f, "ZIP error: {e}"),
            Error::Xml(e) => write!(f, "XML error: {e}"),
            Error::Invalid(s) => write!(f, "invalid ODF: {s}"),
            Error::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Zip(e) => Some(e),
            Error::Xml(e) => Some(e),
            Error::Io(e) => Some(e),
            Error::Invalid(_) => None,
        }
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Error::Zip(e)
    }
}

impl From<quick_xml::Error> for Error {
    fn from(e: quick_xml::Error) -> Self {
        Error::Xml(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// A non-fatal warning produced during parsing.
///
/// Diagnostics describe constructs that were encountered but could not be
/// fully represented (fidelity warnings) or other non-fatal issues.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Human-readable message.
    pub message: String,
    /// Severity level.
    pub level: DiagLevel,
}

/// Severity of a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagLevel {
    /// Non-fatal: construct was preserved but may have lost some fidelity.
    Warning,
    /// Informational: construct was skipped (e.g. unsupported feature).
    Info,
}

impl Diagnostic {
    /// Create a warning-level diagnostic.
    pub fn warn(message: impl Into<String>) -> Self {
        Self { message: message.into(), level: DiagLevel::Warning }
    }

    /// Create an info-level diagnostic.
    pub fn info(message: impl Into<String>) -> Self {
        Self { message: message.into(), level: DiagLevel::Info }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.level {
            DiagLevel::Warning => "warning",
            DiagLevel::Info => "info",
        };
        write!(f, "[{prefix}] {}", self.message)
    }
}

/// Parse result: the parsed value plus any non-fatal diagnostics.
pub struct ParseResult<T> {
    pub value: T,
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> ParseResult<T> {
    pub fn ok(value: T) -> Self {
        Self { value, diagnostics: Vec::new() }
    }

    pub fn with_diagnostics(value: T, diagnostics: Vec<Diagnostic>) -> Self {
        Self { value, diagnostics }
    }
}
