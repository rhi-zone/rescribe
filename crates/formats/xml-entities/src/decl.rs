//! Parsing `<!ENTITY ...>` declarations out of a DTD internal subset.
//!
//! This is deliberately **not** a DTD validator: `<!ELEMENT>`, `<!ATTLIST>`
//! and `<!NOTATION>` declarations are recognized only well enough to be
//! skipped correctly (respecting quoted literals so an embedded `>` doesn't
//! end the declaration early); their content is discarded. Only entity
//! name -> replacement-text mappings are retained.

use std::collections::HashMap;
use std::fmt;

/// A single `<!ENTITY ...>` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityDecl {
    /// The entity name, without the leading `&`/`%` or trailing `;`.
    pub name: String,
    pub kind: EntityDeclKind,
}

/// What an entity declaration resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityDeclKind {
    /// `<!ENTITY name "literal text">` — the replacement text is fully
    /// known. Numeric character references (`&#60;`, `&#x3C;`) inside the
    /// literal have already been expanded to the characters they denote;
    /// nested general-entity references (`&other;`) are left as-is (per the
    /// XML spec, general entity references inside an entity's literal value
    /// are *not* expanded at declaration time — only when the entity itself
    /// is later referenced in document content). [`crate::EntityResolver`]
    /// performs that recursive expansion at resolve time.
    Internal(String),
    /// `<!ENTITY name SYSTEM "uri">` or
    /// `<!ENTITY name PUBLIC "pubid" "uri">` — an external entity. Its
    /// replacement text lives in a file this crate does not fetch (no
    /// network or filesystem access), so it cannot be resolved to text;
    /// [`EntityDeclKind::External`] still records the identifiers so a
    /// caller can decide to fetch it themselves if they want to.
    External {
        public_id: Option<String>,
        system_id: String,
        /// Set for unparsed external entities: `<!ENTITY name SYSTEM "uri" NDATA notation>`.
        ndata: Option<String>,
    },
}

/// A parse diagnostic: something in the internal subset that didn't fit the
/// narrow entity-declaration grammar this parser understands. Never fatal —
/// [`DtdEntities::parse_subset`]/[`DtdEntities::parse_doctype`] always
/// return a best-effort result alongside the diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    /// Byte offset into the subset text the diagnostic pertains to.
    pub offset: usize,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at byte {}: {}", self.offset, self.message)
    }
}

/// The entity declarations parsed out of one document's DTD internal
/// subset: a name -> declaration map for general entities (the `&name;`
/// kind used in document content) and a separate one for parameter entities
/// (the `%name;` kind used only within the DTD itself).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DtdEntities {
    general: HashMap<String, EntityDecl>,
    parameter: HashMap<String, EntityDecl>,
}

impl DtdEntities {
    /// An empty set — no document-declared entities. Useful as a base case
    /// for documents with no DOCTYPE or no internal subset.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Look up a general entity (`&name;`) declared in the document's
    /// internal subset.
    pub fn get(&self, name: &str) -> Option<&EntityDecl> {
        self.general.get(name)
    }

    /// Look up a parameter entity (`%name;`) declared in the document's
    /// internal subset. Parameter entities are DTD-internal macros, not
    /// something document content ever references directly, but they're
    /// exposed for callers building their own DTD tooling on top of this
    /// crate.
    pub fn get_parameter(&self, name: &str) -> Option<&EntityDecl> {
        self.parameter.get(name)
    }

    /// Iterate all declared general entities.
    pub fn iter(&self) -> impl Iterator<Item = &EntityDecl> {
        self.general.values()
    }

    /// Iterate all declared parameter entities.
    pub fn iter_parameters(&self) -> impl Iterator<Item = &EntityDecl> {
        self.parameter.values()
    }

    pub fn len(&self) -> usize {
        self.general.len()
    }

    pub fn is_empty(&self) -> bool {
        self.general.is_empty()
    }

    /// Parse entity declarations out of a full `DOCTYPE` declaration's text
    /// — everything between `<!DOCTYPE` and the closing `>`, *not* including
    /// those delimiters. This is the substring `quick-xml`'s
    /// `Event::DocType` (and equivalents in other XML libraries) hands you.
    ///
    /// If the DOCTYPE has no internal subset (no `[...]`, e.g.
    /// `<!DOCTYPE book SYSTEM "docbookx.dtd">`), returns an empty
    /// [`DtdEntities`] with no diagnostics — that is a completely normal
    /// document shape, not an error.
    pub fn parse_doctype(doctype: &str) -> (Self, Vec<Diagnostic>) {
        match extract_internal_subset(doctype) {
            Some(subset) => Self::parse_subset(subset),
            None => (Self::empty(), Vec::new()),
        }
    }

    /// Parse entity declarations out of an already-isolated internal
    /// subset: the content between the `[` and `]` of a DOCTYPE
    /// declaration, brackets not included.
    pub fn parse_subset(subset: &str) -> (Self, Vec<Diagnostic>) {
        let mut out = Self::empty();
        let mut diagnostics = Vec::new();
        parse_into(subset, &mut out, &mut diagnostics, 0);
        (out, diagnostics)
    }
}

const MAX_PARAMETER_EXPANSION_DEPTH: usize = 64;

/// Find the bracketed internal subset within a DOCTYPE declaration's text,
/// respecting quoted literals (a `[` or `]` inside a quoted PUBLIC/SYSTEM
/// literal doesn't count).
fn extract_internal_subset(doctype: &str) -> Option<&str> {
    let bytes = doctype.as_bytes();
    let mut i = 0;
    let mut in_quote: Option<u8> = None;
    let mut start = None;
    while i < bytes.len() {
        let b = bytes[i];
        match in_quote {
            Some(q) => {
                if b == q {
                    in_quote = None;
                }
            }
            None => match b {
                b'"' | b'\'' => in_quote = Some(b),
                b'[' if start.is_none() => start = Some(i + 1),
                b']' => {
                    if let Some(s) = start {
                        return Some(&doctype[s..i]);
                    }
                }
                _ => {}
            },
        }
        i += 1;
    }
    None
}

/// Parse declarations from `subset` into `out`, recursing into internally
/// declared parameter entities (`%name;` where `name` maps to an
/// [`EntityDeclKind::Internal`] value) up to `depth`. External parameter
/// entity references are recorded as unexpandable via a diagnostic, never
/// fetched.
fn parse_into(
    subset: &str,
    out: &mut DtdEntities,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
) {
    let mut cursor = Cursor::new(subset);
    while let Some(()) = cursor.skip_whitespace_and_check_more() {
        if cursor.starts_with("<!--") {
            if cursor.skip_through("-->").is_none() {
                diagnostics.push(Diagnostic {
                    message: "unterminated comment in internal subset".into(),
                    offset: cursor.pos,
                });
                return;
            }
        } else if cursor.starts_with("<?") {
            if cursor.skip_through("?>").is_none() {
                diagnostics.push(Diagnostic {
                    message: "unterminated processing instruction in internal subset".into(),
                    offset: cursor.pos,
                });
                return;
            }
        } else if cursor.starts_with("<!ENTITY") {
            let start = cursor.pos;
            cursor.advance("<!ENTITY".len());
            match parse_entity_decl(&mut cursor) {
                Ok(decl) => {
                    let is_parameter = decl.is_parameter;
                    let entry = EntityDecl {
                        name: decl.name,
                        kind: decl.kind,
                    };
                    if is_parameter {
                        out.parameter.entry(entry.name.clone()).or_insert(entry);
                    } else {
                        out.general.entry(entry.name.clone()).or_insert(entry);
                    }
                }
                Err(msg) => {
                    diagnostics.push(Diagnostic {
                        message: msg,
                        offset: start,
                    });
                    // Best-effort recovery: skip to the next unquoted '>'.
                    if cursor.skip_to_unquoted_close().is_none() {
                        return;
                    }
                }
            }
        } else if cursor.starts_with("<!") {
            // <!ELEMENT>, <!ATTLIST>, <!NOTATION>: not our concern, but must
            // be skipped correctly (respecting quotes) so we don't misparse
            // an embedded '>' as ending the declaration early.
            if cursor.skip_to_unquoted_close().is_none() {
                diagnostics.push(Diagnostic {
                    message: "unterminated markup declaration in internal subset".into(),
                    offset: cursor.pos,
                });
                return;
            }
        } else if cursor.peek() == Some(b'%') {
            // Top-level parameter-entity reference, e.g. `%isolat1;`.
            let start = cursor.pos;
            cursor.advance(1);
            let name = cursor.take_while(|b| b != b';');
            if cursor.peek() == Some(b';') {
                cursor.advance(1);
            }
            expand_parameter_reference(name, out, diagnostics, depth, start);
        } else {
            // Something we don't recognize (stray text, %-reference typo,
            // etc). Advance one byte and keep going rather than looping
            // forever or aborting the whole parse.
            diagnostics.push(Diagnostic {
                message: format!(
                    "unrecognized content in internal subset: {:?}",
                    cursor.peek_str(16)
                ),
                offset: cursor.pos,
            });
            cursor.advance(1);
        }
    }
}

fn expand_parameter_reference(
    name: &str,
    out: &mut DtdEntities,
    diagnostics: &mut Vec<Diagnostic>,
    depth: usize,
    offset: usize,
) {
    if name.is_empty() {
        diagnostics.push(Diagnostic {
            message: "empty parameter-entity reference '%;'".into(),
            offset,
        });
        return;
    }
    if depth >= MAX_PARAMETER_EXPANSION_DEPTH {
        diagnostics.push(Diagnostic {
            message: format!(
                "parameter entity '%{name};' not expanded: maximum expansion depth ({MAX_PARAMETER_EXPANSION_DEPTH}) reached"
            ),
            offset,
        });
        return;
    }
    match out.parameter.get(name).cloned() {
        Some(EntityDecl {
            kind: EntityDeclKind::Internal(value),
            ..
        }) => {
            // Parameter entities combining several declarations are a
            // real-world pattern (e.g. a document assembling entity sets
            // from several internally-declared parameter entities); expand
            // it in place.
            parse_into(&value, out, diagnostics, depth + 1);
        }
        Some(EntityDecl {
            kind: EntityDeclKind::External { system_id, .. },
            ..
        }) => {
            diagnostics.push(Diagnostic {
                message: format!(
                    "parameter entity '%{name};' references external subset '{system_id}', which was not fetched; entities it may declare are not included"
                ),
                offset,
            });
        }
        None => {
            diagnostics.push(Diagnostic {
                message: format!("parameter entity '%{name};' referenced but not declared"),
                offset,
            });
        }
    }
}

struct ParsedEntityDecl {
    name: String,
    is_parameter: bool,
    kind: EntityDeclKind,
}

fn parse_entity_decl(cursor: &mut Cursor<'_>) -> Result<ParsedEntityDecl, String> {
    cursor.skip_ws();
    let is_parameter = if cursor.peek() == Some(b'%') {
        cursor.advance(1);
        cursor.skip_ws();
        true
    } else {
        false
    };
    let name = cursor.take_name();
    if name.is_empty() {
        return Err("<!ENTITY declaration missing a name".into());
    }
    let name = name.to_string();
    cursor.skip_ws();

    let kind = if cursor.starts_with("SYSTEM") || cursor.starts_with("PUBLIC") {
        let is_public = cursor.starts_with("PUBLIC");
        cursor.advance(if is_public {
            "PUBLIC".len()
        } else {
            "SYSTEM".len()
        });
        cursor.skip_ws();
        let public_id = if is_public {
            Some(cursor.take_quoted_literal("PUBLIC identifier")?)
        } else {
            None
        };
        if is_public {
            cursor.skip_ws();
        }
        let system_id = cursor.take_quoted_literal("SYSTEM literal")?;
        cursor.skip_ws();
        let ndata = if !is_parameter && cursor.starts_with("NDATA") {
            cursor.advance("NDATA".len());
            cursor.skip_ws();
            let n = cursor.take_name();
            if n.is_empty() {
                return Err("NDATA missing a notation name".into());
            }
            Some(n.to_string())
        } else {
            None
        };
        EntityDeclKind::External {
            public_id,
            system_id,
            ndata,
        }
    } else if cursor.peek() == Some(b'"') || cursor.peek() == Some(b'\'') {
        let raw = cursor.take_quoted_literal("entity value")?;
        EntityDeclKind::Internal(expand_char_refs(&raw))
    } else {
        return Err(format!(
            "expected SYSTEM, PUBLIC, or a quoted literal after entity name '{name}'"
        ));
    };

    cursor.skip_ws();
    if cursor.peek() != Some(b'>') {
        return Err(format!(
            "expected '>' to close <!ENTITY {name} ...> declaration"
        ));
    }
    cursor.advance(1);

    Ok(ParsedEntityDecl {
        name,
        is_parameter,
        kind,
    })
}

/// Expand numeric character references (`&#60;`, `&#x3C;`) in an entity's
/// literal value. Per the XML spec these are resolved when the *containing*
/// entity is declared ("included as literal" processing for character
/// references), unlike general-entity references (`&name;`) which stay
/// literal until the entity itself is expanded.
fn expand_char_refs(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' && bytes.get(i + 1) == Some(&b'#') {
            let mut j = i + 2;
            let hex = bytes.get(j) == Some(&b'x') || bytes.get(j) == Some(&b'X');
            if hex {
                j += 1;
            }
            let digits_start = j;
            while j < bytes.len() && bytes[j] != b';' {
                j += 1;
            }
            let digits = &input[digits_start..j.min(input.len())];
            let value = if hex {
                u32::from_str_radix(digits, 16).ok()
            } else {
                digits.parse::<u32>().ok()
            };
            if j < bytes.len()
                && bytes[j] == b';'
                && !digits.is_empty()
                && let Some(ch) = value.and_then(char::from_u32)
            {
                out.push(ch);
                i = j + 1;
                continue;
            }
            // Not a well-formed numeric char ref after all; keep verbatim.
            out.push('&');
            i += 1;
        } else {
            // Copy one UTF-8 char at a time to stay on char boundaries.
            let ch_len = utf8_char_len(bytes[i]);
            out.push_str(&input[i..(i + ch_len).min(input.len())]);
            i += ch_len;
        }
    }
    out
}

fn utf8_char_len(first_byte: u8) -> usize {
    if first_byte & 0b1000_0000 == 0 {
        1
    } else if first_byte & 0b1110_0000 == 0b1100_0000 {
        2
    } else if first_byte & 0b1111_0000 == 0b1110_0000 {
        3
    } else if first_byte & 0b1111_1000 == 0b1111_0000 {
        4
    } else {
        1
    }
}

/// A minimal byte-oriented cursor over `&str` input. Kept private: this is
/// an implementation detail of the internal-subset scanner, not part of the
/// public API.
struct Cursor<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn rest(&self) -> &'a str {
        &self.input[self.pos.min(self.input.len())..]
    }

    fn peek(&self) -> Option<u8> {
        self.rest().as_bytes().first().copied()
    }

    fn peek_str(&self, max: usize) -> &'a str {
        let rest = self.rest();
        let end = rest
            .char_indices()
            .map(|(i, c)| i + c.len_utf8())
            .take(max)
            .last()
            .unwrap_or(0);
        &rest[..end]
    }

    fn starts_with(&self, pat: &str) -> bool {
        self.rest().starts_with(pat)
    }

    fn advance(&mut self, n: usize) {
        // Advance by `n` bytes, but never split a UTF-8 char boundary.
        let rest = self.rest();
        let mut end = n.min(rest.len());
        while end < rest.len() && !rest.is_char_boundary(end) {
            end += 1;
        }
        self.pos += end;
    }

    fn skip_ws(&mut self) {
        let rest = self.rest();
        let trimmed = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
        self.pos += rest.len() - trimmed.len();
    }

    /// Skips whitespace, then returns `Some(())` if there's still input left
    /// to parse, `None` at end of input. Drives the top-level parse loop.
    fn skip_whitespace_and_check_more(&mut self) -> Option<()> {
        self.skip_ws();
        if self.rest().is_empty() {
            None
        } else {
            Some(())
        }
    }

    fn skip_through(&mut self, end_marker: &str) -> Option<()> {
        let rest = self.rest();
        let idx = rest.find(end_marker)?;
        self.pos += idx + end_marker.len();
        Some(())
    }

    /// Skip a `<!...>` style declaration to its closing unquoted `>`.
    fn skip_to_unquoted_close(&mut self) -> Option<()> {
        let bytes = self.rest().as_bytes();
        let mut in_quote: Option<u8> = None;
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            match in_quote {
                Some(q) => {
                    if b == q {
                        in_quote = None;
                    }
                }
                None => match b {
                    b'"' | b'\'' => in_quote = Some(b),
                    b'>' => {
                        self.pos += i + 1;
                        return Some(());
                    }
                    _ => {}
                },
            }
            i += 1;
        }
        None
    }

    fn take_while(&mut self, mut pred: impl FnMut(u8) -> bool) -> &'a str {
        let rest = self.rest();
        let bytes = rest.as_bytes();
        let mut i = 0;
        while i < bytes.len() && pred(bytes[i]) {
            i += 1;
        }
        self.pos += i;
        &rest[..i]
    }

    /// XML `Name` production, approximated: anything up to the next
    /// whitespace or delimiter. Good enough for entity/notation names, which
    /// in practice are always ASCII-ish identifiers.
    fn take_name(&mut self) -> &'a str {
        self.take_while(|b| {
            !b.is_ascii_whitespace() && b != b'>' && b != b'"' && b != b'\'' && b != b';'
        })
    }

    fn take_quoted_literal(&mut self, what: &str) -> Result<String, String> {
        let quote = match self.peek() {
            Some(q @ (b'"' | b'\'')) => q,
            _ => return Err(format!("expected quoted {what}")),
        };
        self.advance(1);
        let rest = self.rest();
        let idx = rest
            .as_bytes()
            .iter()
            .position(|&b| b == quote)
            .ok_or_else(|| format!("unterminated {what}"))?;
        let content = rest[..idx].to_string();
        self.pos += idx + 1;
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_internal_entity() {
        let (entities, diags) = DtdEntities::parse_subset(r#"<!ENTITY foo "bar">"#);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            entities.get("foo"),
            Some(&EntityDecl {
                name: "foo".into(),
                kind: EntityDeclKind::Internal("bar".into()),
            })
        );
    }

    #[test]
    fn expands_numeric_char_refs_in_literal() {
        let (entities, diags) = DtdEntities::parse_subset(r#"<!ENTITY copy "&#169; company">"#);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            entities.get("copy").unwrap().kind,
            EntityDeclKind::Internal("\u{a9} company".into())
        );
    }

    #[test]
    fn expands_hex_numeric_char_refs() {
        let (entities, _) = DtdEntities::parse_subset(r#"<!ENTITY x "&#x41;">"#);
        assert_eq!(
            entities.get("x").unwrap().kind,
            EntityDeclKind::Internal("A".into())
        );
    }

    #[test]
    fn leaves_nested_general_entity_refs_unexpanded() {
        let (entities, _) = DtdEntities::parse_subset(r#"<!ENTITY a "x &b; y">"#);
        assert_eq!(
            entities.get("a").unwrap().kind,
            EntityDeclKind::Internal("x &b; y".into())
        );
    }

    #[test]
    fn parses_system_external_entity() {
        let (entities, diags) = DtdEntities::parse_subset(r#"<!ENTITY foo SYSTEM "foo.xml">"#);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            entities.get("foo").unwrap().kind,
            EntityDeclKind::External {
                public_id: None,
                system_id: "foo.xml".into(),
                ndata: None,
            }
        );
    }

    #[test]
    fn parses_public_external_entity() {
        let (entities, diags) = DtdEntities::parse_subset(
            r#"<!ENTITY foo PUBLIC "-//Example//TEXT foo//EN" "foo.xml">"#,
        );
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            entities.get("foo").unwrap().kind,
            EntityDeclKind::External {
                public_id: Some("-//Example//TEXT foo//EN".into()),
                system_id: "foo.xml".into(),
                ndata: None,
            }
        );
    }

    #[test]
    fn parses_unparsed_ndata_entity() {
        let (entities, diags) =
            DtdEntities::parse_subset(r#"<!ENTITY logo SYSTEM "logo.gif" NDATA gif>"#);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            entities.get("logo").unwrap().kind,
            EntityDeclKind::External {
                public_id: None,
                system_id: "logo.gif".into(),
                ndata: Some("gif".into()),
            }
        );
    }

    #[test]
    fn skips_comments_and_other_declarations() {
        let (entities, diags) = DtdEntities::parse_subset(
            r#"
            <!-- a comment with a > inside -->
            <!ELEMENT foo (#PCDATA)>
            <!ATTLIST foo bar CDATA #IMPLIED>
            <!ENTITY real "value">
            "#,
        );
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(entities.len(), 1);
        assert_eq!(
            entities.get("real").unwrap().kind,
            EntityDeclKind::Internal("value".into())
        );
    }

    #[test]
    fn skips_attlist_with_gt_inside_quoted_default() {
        let (entities, diags) =
            DtdEntities::parse_subset(r#"<!ATTLIST foo bar CDATA "a > b"><!ENTITY real "value">"#);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            entities.get("real").unwrap().kind,
            EntityDeclKind::Internal("value".into())
        );
    }

    #[test]
    fn parameter_entities_tracked_separately_from_general() {
        let (entities, diags) =
            DtdEntities::parse_subset(r#"<!ENTITY % pe "value"><!ENTITY ge "other">"#);
        assert!(diags.is_empty(), "{diags:?}");
        assert!(entities.get("pe").is_none());
        assert!(entities.get_parameter("pe").is_some());
        assert!(entities.get("ge").is_some());
    }

    #[test]
    fn expands_internal_parameter_entity_reference() {
        let (entities, diags) = DtdEntities::parse_subset(
            r#"<!ENTITY % common "<!ENTITY foo 'bar'><!ENTITY baz 'qux'>"> %common;"#,
        );
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            entities.get("foo").unwrap().kind,
            EntityDeclKind::Internal("bar".into())
        );
        assert_eq!(
            entities.get("baz").unwrap().kind,
            EntityDeclKind::Internal("qux".into())
        );
    }

    #[test]
    fn external_parameter_entity_reference_is_diagnosed_not_fetched() {
        let (entities, diags) = DtdEntities::parse_subset(
            r#"<!ENTITY % isolat1 PUBLIC "ISO 8879-1986//ENTITIES Added Latin 1//EN" "isolat1.ent"> %isolat1;"#,
        );
        assert!(entities.is_empty());
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("isolat1.ent"));
    }

    #[test]
    fn parse_doctype_extracts_bracketed_subset() {
        let (entities, diags) = DtdEntities::parse_doctype(
            r#"book PUBLIC "-//OASIS//DTD DocBook XML V4.5//EN" "docbookx.dtd" [ <!ENTITY foo "bar"> ]"#,
        );
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            entities.get("foo").unwrap().kind,
            EntityDeclKind::Internal("bar".into())
        );
    }

    #[test]
    fn parse_doctype_without_internal_subset_is_empty_no_diagnostics() {
        let (entities, diags) = DtdEntities::parse_doctype(r#"book SYSTEM "docbookx.dtd""#);
        assert!(entities.is_empty());
        assert!(diags.is_empty());
    }

    #[test]
    fn duplicate_declaration_keeps_first_per_xml_semantics() {
        // Per the XML spec, the first declaration of an entity is binding;
        // later redeclarations are ignored (not an error).
        let (entities, _) =
            DtdEntities::parse_subset(r#"<!ENTITY foo "first"><!ENTITY foo "second">"#);
        assert_eq!(
            entities.get("foo").unwrap().kind,
            EntityDeclKind::Internal("first".into())
        );
    }

    #[test]
    fn malformed_entity_declaration_produces_diagnostic_not_panic() {
        let (entities, diags) = DtdEntities::parse_subset(r#"<!ENTITY ><!ENTITY ok "value">"#);
        assert!(!diags.is_empty());
        assert_eq!(
            entities.get("ok").unwrap().kind,
            EntityDeclKind::Internal("value".into())
        );
    }
}
