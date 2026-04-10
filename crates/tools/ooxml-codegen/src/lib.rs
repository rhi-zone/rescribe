//! Code generator for OOXML types from RELAX NG schemas.
//!
//! This crate parses RELAX NG Compact (.rnc) schema files from the ECMA-376
//! specification and generates Rust structs for Office Open XML types.

pub mod analysis;
pub mod ast;
pub mod codegen;
pub mod event_gen;
pub mod lexer;
pub mod parser;
pub mod parser_gen;
pub mod serializer_gen;

pub use analysis::{ModuleReport, analyze_schema};
pub use ast::{DatatypeParam, Definition, Namespace, Pattern, QName, Schema};
pub use codegen::{CodegenConfig, FeatureMappings, ModuleMappings, NameMappings, generate};
pub use lexer::{LexError, Lexer};
pub use parser::{ParseError, Parser};
pub use event_gen::{AttrFieldDef, ContainerDef, EventConfig, LeafDef, generate_events};
pub use parser_gen::generate_parsers;
pub use serializer_gen::generate_serializers;

/// Strip RNC annotations from input before parsing.
///
/// Handles two annotation forms used in ODF schemas (not present in OOXML):
/// - Abbreviated: `>> QName [ ... ]`
/// - Inline block: `[ QName [ ... ] ... ]` (standalone annotation block)
///
/// Annotations carry no schema semantics; stripping them is safe for codegen.
/// String literals (`"..."`) are passed through unchanged so `[` inside strings
/// (e.g. inside xsd:string patterns) is never mis-identified as an annotation.
fn strip_rnc_annotations(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    /// Skip a balanced `[ ... ]` block, consuming the opening `[`.
    fn skip_bracket_block(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
        let mut depth = 1usize;
        loop {
            match chars.next() {
                None => break,
                Some('"') => { while !matches!(chars.next(), None | Some('"')) {} }
                Some('[') => depth += 1,
                Some(']') => { depth -= 1; if depth == 0 { break; } }
                _ => {}
            }
        }
    }

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                // Pass string literals through verbatim
                out.push('"');
                for sc in chars.by_ref() {
                    out.push(sc);
                    if sc == '"' { break; }
                }
            }
            '>' if chars.peek() == Some(&'>') => {
                // >> QName [ ... ] — skip the whole annotation
                chars.next(); // second '>'
                while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) { chars.next(); }
                while chars.peek().map(|c| !c.is_whitespace() && *c != '[').unwrap_or(false) { chars.next(); }
                while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) { chars.next(); }
                if chars.peek() == Some(&'[') { chars.next(); skip_bracket_block(&mut chars); }
            }
            '[' => {
                // Standalone annotation block — skip entirely
                skip_bracket_block(&mut chars);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Parse an RNC schema from a string.
pub fn parse_rnc(input: &str) -> Result<Schema, Error> {
    let stripped = strip_rnc_annotations(input);
    let tokens = Lexer::new(&stripped).tokenize()?;
    let schema = Parser::new(tokens).parse()?;
    Ok(schema)
}

/// Lex an RNC schema into tokens (for debugging).
pub fn lex_rnc(input: &str) -> Result<Vec<lexer::Token>, LexError> {
    Lexer::new(input).tokenize()
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("lexer error: {0}")]
    Lex(#[from] LexError),
    #[error("parser error: {0}")]
    Parse(#[from] ParseError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_schema() {
        let input = r#"
namespace w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"

w_CT_Empty = empty
w_CT_OnOff = attribute w:val { s_ST_OnOff }?
w_ST_HighlightColor =
  string "black"
  | string "blue"
  | string "cyan"
"#;
        let schema = parse_rnc(input).unwrap();
        assert_eq!(schema.namespaces.len(), 1);
        assert_eq!(schema.definitions.len(), 3);
    }
}
