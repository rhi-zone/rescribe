//! jq-style querying of the document IR, powered by the [`jaq`](https://github.com/01mf02/jaq)
//! engine.
//!
//! `Document` is serialized to `serde_json::Value` (via the `Serialize` impls added in
//! `rescribe-core`'s `serde` feature — see ADR 0009 and ADR 0010 for the two acknowledged
//! compromises in that mapping: non-finite floats become a string sentinel, and embedded
//! resource bytes become base64), converted into jaq's own `Val` type, and run through a
//! compiled jq filter. Results are converted back to `serde_json::Value` for the caller.
//!
//! This mirrors the embedding approach used by `normalize`'s `normalize-knowledge-graph`
//! crate (`jq_compile`/`jq_run_all` in `crates/normalize-knowledge-graph/src/store.rs`):
//! compile once, convert the input via `serde_json::from_value::<Val>` (jaq-json's `serde`
//! feature gives `Val: Deserialize`), run the filter, and convert each output back via
//! `serde_json::from_str(&format!("{val}"))` (jaq-json's `Val: Display` prints valid JSON
//! text). That round trip is cheaper than it looks — no double-parse of file bytes is
//! needed since we already hold a `serde_json::Value` from serializing the `Document`.
//!
//! # Example
//!
//! ```
//! # #[cfg(feature = "read-markdown")]
//! # {
//! let doc = rescribe::markdown::parse("# Hello").unwrap().value;
//! let results = rescribe::query::query(&doc, ".metadata").unwrap();
//! assert_eq!(results.len(), 1);
//! # }
//! ```

use crate::Document;
use jaq_core::{
    Ctx, Vars,
    compile::Compiler,
    load::{Arena, File, Loader},
};
use jaq_json::Val;

type Data = jaq_core::data::JustLut<Val>;
type Filter = jaq_core::Filter<Data>;

/// Error compiling or running a jq expression against a [`Document`].
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    /// The document could not be serialized to JSON in the first place. This should not
    /// happen in practice — `Document`'s `Serialize` impl is total over the IR — but the
    /// conversion goes through `serde_json::Value` and that step is fallible in principle.
    #[error("failed to serialize document: {0}")]
    Serialize(#[from] serde_json::Error),
    /// The filter expression failed to parse or compile (syntax error, undefined
    /// name/function, etc).
    #[error("jq expression error: {0}")]
    Compile(String),
    /// The filter compiled but raised an error while running against this document's data.
    #[error("jq runtime error: {0}")]
    Runtime(String),
}

/// A jq filter compiled once and ready to run against any number of documents.
///
/// Compiling is the expensive, fallible part (parsing + name resolution); reuse a
/// `CompiledQuery` across documents when running the same expression repeatedly (e.g. a
/// batch/corpus tool) instead of recompiling per document.
pub struct CompiledQuery {
    filter: Filter,
}

impl CompiledQuery {
    /// Parse and compile a jq filter expression.
    pub fn compile(expr: &str) -> Result<Self, QueryError> {
        let arena = Arena::default();
        let defs = jaq_core::defs()
            .chain(jaq_std::defs())
            .chain(jaq_json::defs());
        let loader = Loader::new(defs);
        let modules = loader
            .load(
                &arena,
                File {
                    code: expr,
                    path: (),
                },
            )
            .map_err(|errs| QueryError::Compile(format_load_errors(&errs)))?;

        let funs = jaq_core::funs::<Data>()
            .chain(jaq_std::funs::<Data>())
            .chain(jaq_json::funs::<Data>());
        let filter = Compiler::default()
            .with_funs(funs)
            .compile(modules)
            .map_err(|errs| QueryError::Compile(format_compile_errors(&errs)))?;

        Ok(Self { filter })
    }

    /// Run this filter against a document, collecting every output value.
    ///
    /// jq filters are multi-output by design (`.[]` yields one output per element), so the
    /// result is a `Vec` even for filters that always produce exactly one value. Collecting
    /// eagerly (rather than returning a borrowing iterator) keeps the lifetime simple for
    /// callers — the alternative would tie the returned iterator's lifetime to internal
    /// `Ctx`/`Val` locals via a self-referential type, which isn't worth the complexity for
    /// a single-document filter run. Runs stop at the first runtime error, matching the
    /// `normalize-knowledge-graph::jq_run_all` precedent this was adapted from.
    pub fn run(&self, doc: &Document) -> Result<Vec<serde_json::Value>, QueryError> {
        let json = serde_json::to_value(doc)?;
        let val: Val = serde_json::from_value(json).map_err(|e| {
            QueryError::Runtime(format!("failed to convert document to jq input: {e}"))
        })?;

        let ctx = Ctx::<Data>::new(&self.filter.lut, Vars::new([]));
        let mut results = Vec::new();
        for output in self.filter.id.run((ctx, val)) {
            let val = output.map_err(|e| QueryError::Runtime(format!("{e:?}")))?;
            results.push(val_to_json(&val)?);
        }
        Ok(results)
    }
}

/// Compile `expr` and run it against `doc` once, returning every output value.
///
/// Convenience wrapper around [`CompiledQuery::compile`] + [`CompiledQuery::run`] for
/// one-shot queries. Prefer [`CompiledQuery`] directly when running the same expression
/// against many documents, to avoid recompiling the filter each time.
pub fn query(doc: &Document, expr: &str) -> Result<Vec<serde_json::Value>, QueryError> {
    CompiledQuery::compile(expr)?.run(doc)
}

fn val_to_json(val: &Val) -> Result<serde_json::Value, QueryError> {
    serde_json::from_str(&format!("{val}"))
        .map_err(|e| QueryError::Runtime(format!("failed to convert jq output to JSON: {e}")))
}

fn format_load_errors<S: core::fmt::Debug, P: core::fmt::Debug>(
    errs: &jaq_core::load::Errors<S, P>,
) -> String {
    errs.iter()
        .flat_map(|(_, err)| match err {
            jaq_core::load::Error::Io(es) => {
                es.iter().map(|(p, e)| format!("{p:?}: {e}")).collect()
            }
            jaq_core::load::Error::Lex(es) => es
                .iter()
                .map(|(_, found)| format!("lex error near {found:?}"))
                .collect(),
            jaq_core::load::Error::Parse(es) => es
                .iter()
                .map(|(exp, found)| format!("parse error: expected {exp:?}, found {found:?}"))
                .collect::<Vec<_>>(),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn format_compile_errors<S: core::fmt::Debug, P: core::fmt::Debug>(
    errs: &jaq_core::compile::Errors<S, P>,
) -> String {
    errs.iter()
        .flat_map(|(_, es)| {
            es.iter()
                .map(|(_, undef)| format!("undefined: {undef:?}"))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(all(test, feature = "read-markdown"))]
mod tests {
    use super::*;

    fn doc() -> Document {
        rescribe_read_markdown::parse(
            "# Title\n\nSome *emphasized* text.\n\n## Subheading\n\nMore text.\n",
        )
        .unwrap()
        .value
    }

    #[test]
    fn identity_filter_returns_whole_document() {
        let results = query(&doc(), ".").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].get("content").is_some());
    }

    #[test]
    fn metadata_query_subsumes_a_dedicated_metadata_command() {
        let mut d = doc();
        d.metadata.set("title", "My Doc");
        let results = query(&d, ".metadata").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["title"], serde_json::json!("My Doc"));
    }

    #[test]
    fn node_kind_census_subsumes_a_dedicated_stats_command() {
        // `[.. | .kind?] | group_by(.) | map({kind: .[0], count: length})`
        let results = query(
            &doc(),
            "[.. | .kind?] | map(select(. != null)) | group_by(.) | map({kind: .[0], count: length})",
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        let census = results[0].as_array().unwrap();
        let heading_count = census
            .iter()
            .find(|entry| entry["kind"] == serde_json::json!("heading"))
            .and_then(|entry| entry["count"].as_u64())
            .unwrap();
        assert_eq!(heading_count, 2);
    }

    #[test]
    fn multi_output_filter_collects_every_result() {
        let results = query(&doc(), ".. | .kind? // empty").unwrap();
        assert!(results.len() > 1);
    }

    #[test]
    fn compiled_query_can_be_reused_across_documents() {
        let compiled = CompiledQuery::compile(".content.kind").unwrap();
        let a = compiled.run(&doc()).unwrap();
        let b = compiled.run(&doc()).unwrap();
        assert_eq!(a, b);
        assert_eq!(a[0], serde_json::json!("document"));
    }

    #[test]
    fn syntax_error_is_a_compile_error() {
        match CompiledQuery::compile(".foo[") {
            Err(QueryError::Compile(_)) => {}
            Err(other) => panic!("expected QueryError::Compile, got {other:?}"),
            Ok(_) => panic!("expected a compile error, but `.foo[` compiled successfully"),
        }
    }

    #[test]
    fn runtime_error_surfaces_as_runtime_error() {
        // Adding a number to a string is a type error at runtime, not compile time.
        let err = query(&doc(), ".content.kind + 1").unwrap_err();
        assert!(matches!(err, QueryError::Runtime(_)));
    }
}
