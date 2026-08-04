//! MultiMarkdown metadata block: `Key: value` lines at the very top of a
//! document, ended by a blank line — the classic MMD syntax, distinct from
//! (and predating) YAML/TOML frontmatter.
//!
//! Grammar (per the MultiMarkdown 6 syntax guide):
//! - Keys must start with an ASCII letter or digit; subsequent characters
//!   may be letters, digits, spaces, `-`, or `_`. Keys are matched
//!   case-insensitively downstream (not here — this module preserves the
//!   exact spelling written).
//! - A value may continue onto following lines if each continuation line is
//!   indented (this is *required* when a value itself contains a `:`, to
//!   keep it from being mistaken for a new key).
//! - The block ends at the first blank line.
//! - The block may optionally be wrapped in `---` ... `---` (or `...` as the
//!   closer, for YAML-tooling compatibility) — purely cosmetic, same
//!   key/value grammar inside.
//!
//! This is genuinely distinct grammar from commonmark-fmt's `frontmatter`
//! feature (which requires `---`/`+++` delimiters and captures the interior
//! verbatim rather than parsing it) — MMD's bare, undelimited form has no
//! commonmark-fmt equivalent, so this crate implements its own detection
//! and parsing directly rather than routing through commonmark-fmt at all.

use crate::ast::{MetadataEntry, MetadataStyle};

/// Detect and parse a leading metadata block, if any. Returns the parsed
/// entries, which style was used, and the remaining input (with the
/// metadata block, its delimiters, and the following blank line all
/// stripped) to hand to `commonmark_fmt::parse::parse_str`.
pub fn extract(input: &str) -> (Vec<MetadataEntry>, MetadataStyle, &str) {
    if let Some(body) = strip_first_line_if(input, "---")
        && let Some((entries, rest)) = parse_delimited(body)
    {
        return (entries, MetadataStyle::Delimited, rest);
    }
    if let Some((entries, rest)) = parse_bare(input)
        && !entries.is_empty()
    {
        return (entries, MetadataStyle::Bare, rest);
    }
    (Vec::new(), MetadataStyle::None, input)
}

/// If `input`'s first line, trimmed of trailing whitespace, equals `marker`
/// exactly, return the input starting at the following line.
fn strip_first_line_if<'a>(input: &'a str, marker: &str) -> Option<&'a str> {
    let (first, rest) = split_first_line(input);
    if first.trim_end_matches(['\r', '\n']).trim_end() == marker {
        Some(rest)
    } else {
        None
    }
}

/// Split `input` into (first line including its trailing `\n`, remainder).
/// If there is no `\n`, the whole input is the "first line" and the
/// remainder is empty.
fn split_first_line(input: &str) -> (&str, &str) {
    match input.find('\n') {
        Some(idx) => input.split_at(idx + 1),
        None => (input, ""),
    }
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

/// A key:value leading line, e.g. `Title: My Document`.
fn match_kv_line(line: &str) -> Option<(&str, &str)> {
    let idx = line.find(':')?;
    let key = &line[..idx];
    if key.is_empty() {
        return None;
    }
    let mut chars = key.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphanumeric() {
        return None;
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_')
    {
        return None;
    }
    let value = line[idx + 1..].trim_end_matches(['\r', '\n']).trim();
    Some((key.trim_end(), value))
}

/// A continuation line: indented (starts with a space or tab).
fn is_continuation_line(line: &str) -> bool {
    matches!(line.chars().next(), Some(' ') | Some('\t'))
}

enum ScanOutcome<'a> {
    /// A `---`/`...` closer line was found; body ends there.
    Closer(&'a str),
    /// End of input reached without a closer.
    Eof,
    /// A blank line was found (bare-form terminator).
    Blank(&'a str),
    /// A line that is neither kv, continuation, blank, nor (in delimited
    /// mode) a closer — the leading run is not a valid metadata block.
    Invalid,
}

/// Scan leading `Key: value` (+ continuation) lines from `input`, stopping
/// per `stop`. Returns the parsed entries and, on success, the input
/// remaining after the stop point.
fn scan_kv_lines<'a>(mut input: &'a str, delimited: bool) -> (Vec<MetadataEntry>, ScanOutcome<'a>) {
    let mut entries: Vec<MetadataEntry> = Vec::new();
    loop {
        if input.is_empty() {
            return (entries, ScanOutcome::Eof);
        }
        let (line_raw, rest) = split_first_line(input);
        let line = line_raw.trim_end_matches(['\r', '\n']);

        if delimited && (line.trim_end() == "---" || line.trim_end() == "...") {
            return (entries, ScanOutcome::Closer(rest));
        }
        if is_blank(line) {
            return (entries, ScanOutcome::Blank(rest));
        }
        if let Some((key, value)) = match_kv_line(line) {
            entries.push(MetadataEntry {
                key: key.to_string(),
                value: value.to_string(),
            });
            input = rest;
            continue;
        }
        if is_continuation_line(line)
            && let Some(last) = entries.last_mut()
        {
            last.value.push('\n');
            last.value.push_str(line.trim());
            input = rest;
            continue;
        }
        return (entries, ScanOutcome::Invalid);
    }
}

fn parse_delimited(body: &str) -> Option<(Vec<MetadataEntry>, &str)> {
    let (entries, outcome) = scan_kv_lines(body, true);
    match outcome {
        ScanOutcome::Closer(rest) => {
            // Consume one following blank line, if present.
            let (line_raw, after_blank) = split_first_line(rest);
            let rest = if is_blank(line_raw) {
                after_blank
            } else {
                rest
            };
            Some((entries, rest))
        }
        _ => None,
    }
}

fn parse_bare(input: &str) -> Option<(Vec<MetadataEntry>, &str)> {
    let (entries, outcome) = scan_kv_lines(input, false);
    match outcome {
        ScanOutcome::Blank(rest) | ScanOutcome::Closer(rest) => Some((entries, rest)),
        ScanOutcome::Eof if !entries.is_empty() => Some((entries, "")),
        _ => None,
    }
}

/// Emit a metadata block in the given style. No-op if `entries` is empty.
pub fn emit(entries: &[MetadataEntry], style: MetadataStyle, out: &mut String) {
    if entries.is_empty() || style == MetadataStyle::None {
        return;
    }
    if style == MetadataStyle::Delimited {
        out.push_str("---\n");
    }
    for entry in entries {
        out.push_str(&entry.key);
        out.push_str(": ");
        let mut lines = entry.value.split('\n');
        if let Some(first) = lines.next() {
            out.push_str(first);
        }
        out.push('\n');
        for cont in lines {
            out.push_str("    ");
            out.push_str(cont);
            out.push('\n');
        }
    }
    if style == MetadataStyle::Delimited {
        out.push_str("---\n");
    }
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_metadata() {
        let input = "Title: My Document\nAuthor: Jane Doe\n\nBody text.\n";
        let (entries, style, rest) = extract(input);
        assert_eq!(style, MetadataStyle::Bare);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "Title");
        assert_eq!(entries[0].value, "My Document");
        assert_eq!(rest, "Body text.\n");
    }

    #[test]
    fn delimited_metadata() {
        let input = "---\nTitle: My Document\n---\n\nBody text.\n";
        let (entries, style, rest) = extract(input);
        assert_eq!(style, MetadataStyle::Delimited);
        assert_eq!(entries.len(), 1);
        assert_eq!(rest, "Body text.\n");
    }

    #[test]
    fn delimited_metadata_dots_closer() {
        let input = "---\nTitle: My Document\n...\n\nBody text.\n";
        let (entries, style, rest) = extract(input);
        assert_eq!(style, MetadataStyle::Delimited);
        assert_eq!(entries.len(), 1);
        assert_eq!(rest, "Body text.\n");
    }

    #[test]
    fn continuation_line() {
        let input = "Note: first line\n    second line\n\nBody.\n";
        let (entries, style, rest) = extract(input);
        assert_eq!(style, MetadataStyle::Bare);
        assert_eq!(entries[0].value, "first line\nsecond line");
        assert_eq!(rest, "Body.\n");
    }

    #[test]
    fn no_metadata_plain_document() {
        let input = "# Heading\n\nParagraph.\n";
        let (entries, style, rest) = extract(input);
        assert!(entries.is_empty());
        assert_eq!(style, MetadataStyle::None);
        assert_eq!(rest, input);
    }

    #[test]
    fn thematic_break_not_mistaken_for_delimiter() {
        let input = "---\n\nParagraph.\n";
        let (entries, style, rest) = extract(input);
        assert!(entries.is_empty());
        assert_eq!(style, MetadataStyle::None);
        assert_eq!(rest, input);
    }

    #[test]
    fn roundtrip_bare() {
        let entries = vec![MetadataEntry {
            key: "Title".to_string(),
            value: "My Doc".to_string(),
        }];
        let mut out = String::new();
        emit(&entries, MetadataStyle::Bare, &mut out);
        let (parsed, style, _rest) = extract(&out);
        assert_eq!(parsed, entries);
        assert_eq!(style, MetadataStyle::Bare);
    }
}
