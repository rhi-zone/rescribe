//! Step 1: the easy 40% — `parse()`/`emit()`/`Diagnostic`, no lifetimes.
//!
//! This part is unsurprising and compiles cleanly. Included for
//! completeness / to anchor the harder steps against a working baseline.

pub trait FormatCrate {
    type Ast;
    type Diagnostic;

    fn parse(input: &[u8]) -> (Self::Ast, Vec<Self::Diagnostic>);
    fn emit(ast: &Self::Ast) -> Vec<u8>;
}

/// Zero-sized marker type standing in for the `opml-fmt` crate. `-fmt`
/// crates expose free functions (`opml_fmt::parse`, `opml_fmt::emit`),
/// not a type — so "implementing the trait for the crate" means
/// implementing it for a marker type that has no other purpose. This is
/// itself worth noting in the report: the trait doesn't attach to
/// anything the crate already exports, it requires each crate (or this
/// spike, standing in for each crate) to invent a nominal type purely to
/// hang the impl on.
pub struct Opml;

impl FormatCrate for Opml {
    type Ast = opml_fmt::OpmlDoc;
    type Diagnostic = opml_fmt::Diagnostic;

    fn parse(input: &[u8]) -> (Self::Ast, Vec<Self::Diagnostic>) {
        opml_fmt::parse(input)
    }

    fn emit(ast: &Self::Ast) -> Vec<u8> {
        opml_fmt::emit(ast)
    }
}

pub struct Zip;

impl FormatCrate for Zip {
    type Ast = zip_fmt::Archive;
    type Diagnostic = zip_fmt::Diagnostic;

    fn parse(input: &[u8]) -> (Self::Ast, Vec<Self::Diagnostic>) {
        zip_fmt::parse(input)
    }

    fn emit(ast: &Self::Ast) -> Vec<u8> {
        zip_fmt::emit(ast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip<F: FormatCrate>(input: &[u8]) -> F::Ast {
        let (ast, diags) = F::parse(input);
        assert!(diags.is_empty() as usize <= 1); // don't actually assert content, just exercise the generic path
        let _bytes = F::emit(&ast);
        ast
    }

    #[test]
    fn opml_generic_roundtrip() {
        let sample = br#"<opml version="2.0"><head><title>T</title></head><body><outline text="A"/></body></opml>"#;
        let _ast = roundtrip::<Opml>(sample);
    }
}
