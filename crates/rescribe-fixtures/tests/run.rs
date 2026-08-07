//! Fixture integration tests.
//!
//! Discovers fixtures from `fixtures/{format}/` at the workspace root and
//! runs each against the appropriate reader.  Tests skip gracefully if a
//! format directory doesn't exist yet.
//!
//! Writer smoke tests discover fixtures from `fixtures/writers/{format}/` and
//! run them against the appropriate emitter.

use rescribe_fixtures::{
    run_format_fixtures, run_format_fixtures_excluding, run_format_writer_fixtures,
};
use std::path::PathBuf;

fn fixtures_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/rescribe-fixtures
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // workspace root
        .join("fixtures")
}

#[test]
fn markdown() {
    // Exercise the DEFAULT backend (commonmark-fmt-based) — this is the path
    // every real caller of `rescribe_read_markdown::parse` hits. Previously
    // this test used `backend_pulldown::parse` instead, which meant the
    // fixture suite was blind to bugs in the default backend (e.g. YAML/TOML
    // frontmatter silently misparsing as a bogus horizontal_rule + heading).
    // See docs/adr/0011-commonmark-extension-feature-gating.md and TODO.md.
    //
    // "footnote" is excluded: footnotes are a tracked, not-yet-implemented
    // gap in the default backend (see TODO.md) — the pulldown backend still
    // covers it, exercised separately in rescribe-read-markdown's own tests
    // and by `markdown_backends_agree` below (which explicitly skips it too).
    run_format_fixtures_excluding(&fixtures_root(), "markdown", &["footnote"], |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_markdown::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

/// Parity check: run every markdown fixture through both the default
/// (commonmark-fmt) and pulldown backends, and assert they agree on node-kind
/// shape and metadata wherever both backends model the construct. Backends
/// are independent implementations (see repository CLAUDE.md); this test is
/// what catches them silently drifting apart.
///
/// Constructs the pulldown backend still supports and the default backend
/// does not (footnotes, definition lists, math) are the accepted, tracked
/// exception — those fixtures are skipped here and tracked in TODO.md rather
/// than asserted to agree.
#[test]
fn markdown_backends_agree() {
    use std::path::Path;

    // Constructs not yet modeled by the default backend (see TODO.md).
    const SKIP: &[&str] = &["footnote"];

    let root = fixtures_root().join("markdown");
    for entry in std::fs::read_dir(&root).expect("markdown fixtures dir") {
        let entry = entry.unwrap();
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if SKIP.contains(&name.as_str()) {
            continue;
        }
        let input_path = find_input(&path);
        let Some(input_path) = input_path else {
            continue;
        };
        let input = std::fs::read_to_string(&input_path).expect("read fixture input");

        let default_doc = rescribe_read_markdown::parse(&input).map(|r| r.value);
        let pulldown_doc = rescribe_read_markdown::backend_pulldown::parse(&input).map(|r| r.value);
        // If either backend errors, that's a separate (pre-existing) concern
        // this parity check doesn't adjudicate.
        if let (Ok(d), Ok(p)) = (default_doc, pulldown_doc) {
            assert_eq!(
                node_kind_shape(&d.content),
                node_kind_shape(&p.content),
                "backends disagree on node-kind shape for fixture {name}"
            );
        }
    }

    fn find_input(dir: &Path) -> Option<std::path::PathBuf> {
        std::fs::read_dir(dir)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| p.file_stem().is_some_and(|s| s == "input"))
    }

    fn node_kind_shape(node: &rescribe_core::Node) -> Vec<String> {
        let mut out = vec![node.kind.as_str().to_string()];
        for child in &node.children {
            out.extend(node_kind_shape(child));
        }
        out
    }
}

#[test]
fn commonmark() {
    run_format_fixtures(&fixtures_root(), "commonmark", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_commonmark::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn gfm() {
    run_format_fixtures(&fixtures_root(), "gfm", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_gfm::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn html() {
    run_format_fixtures(&fixtures_root(), "html", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        html_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn asciidoc() {
    run_format_fixtures(&fixtures_root(), "asciidoc", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        asciidoc::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn mediawiki() {
    run_format_fixtures(&fixtures_root(), "mediawiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        mediawiki_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn latex() {
    // See fixtures/latex/COVERAGE.md for why the fixture suite's
    // expectations look different from a typical format (raw_block-heavy:
    // no built-in "standard commands" table, see crate::parse's
    // resolution model).
    run_format_fixtures(&fixtures_root(), "latex", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        latex_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn rst() {
    run_format_fixtures(&fixtures_root(), "rst", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rst_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn org() {
    run_format_fixtures(&fixtures_root(), "org", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        org_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn creole() {
    run_format_fixtures(&fixtures_root(), "creole", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        creole::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn djot() {
    run_format_fixtures(&fixtures_root(), "djot", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        djot_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn textile() {
    run_format_fixtures(&fixtures_root(), "textile", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        textile_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn muse() {
    run_format_fixtures(&fixtures_root(), "muse", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        muse_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn t2t() {
    run_format_fixtures(&fixtures_root(), "t2t", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        t2t::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn tikiwiki() {
    run_format_fixtures(&fixtures_root(), "tikiwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        tikiwiki::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn twiki() {
    run_format_fixtures(&fixtures_root(), "twiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        twiki::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn vimwiki() {
    run_format_fixtures(&fixtures_root(), "vimwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        vimwiki_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn dokuwiki() {
    run_format_fixtures(&fixtures_root(), "dokuwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        dokuwiki::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn jira() {
    run_format_fixtures(&fixtures_root(), "jira", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        jira_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn haddock() {
    run_format_fixtures(&fixtures_root(), "haddock", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        haddock_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn pod() {
    run_format_fixtures(&fixtures_root(), "pod", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        pod_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn man() {
    run_format_fixtures(&fixtures_root(), "man", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        man_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn xwiki() {
    run_format_fixtures(&fixtures_root(), "xwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        xwiki::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn zimwiki() {
    run_format_fixtures(&fixtures_root(), "zimwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        zimwiki::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn bbcode() {
    run_format_fixtures(&fixtures_root(), "bbcode", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        bbcode_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn texinfo() {
    run_format_fixtures(&fixtures_root(), "texinfo", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        texinfo::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn markua() {
    run_format_fixtures(&fixtures_root(), "markua", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        markua::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn fountain() {
    run_format_fixtures(&fixtures_root(), "fountain", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        fountain_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn ansi() {
    run_format_fixtures(&fixtures_root(), "ansi", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        ansi_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn csl_json() {
    run_format_fixtures(&fixtures_root(), "csl-json", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_fmt_csl_json::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn native() {
    run_format_fixtures(&fixtures_root(), "native", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        native::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn pandoc_json() {
    run_format_fixtures(&fixtures_root(), "pandoc-json", |input| {
        rescribe_fmt_pandoc_json::parse(std::str::from_utf8(input).map_err(|e| e.to_string())?)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn docbook() {
    run_format_fixtures(&fixtures_root(), "docbook", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        docbook_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn fb2() {
    run_format_fixtures(&fixtures_root(), "fb2", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        fb2_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn ipynb() {
    run_format_fixtures(&fixtures_root(), "ipynb", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_fmt_ipynb::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn csv() {
    run_format_fixtures(&fixtures_root(), "csv", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        csv_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn tsv() {
    run_format_fixtures(&fixtures_root(), "tsv", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        tsv_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn opml() {
    run_format_fixtures(&fixtures_root(), "opml", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        opml_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn ris() {
    run_format_fixtures(&fixtures_root(), "ris", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        ris::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn bibtex() {
    run_format_fixtures(&fixtures_root(), "bibtex", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_fmt_bibtex::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn biblatex() {
    run_format_fixtures(&fixtures_root(), "biblatex", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_fmt_biblatex::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn typst() {
    run_format_fixtures(&fixtures_root(), "typst", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        typst_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn jats() {
    run_format_fixtures(&fixtures_root(), "jats", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        jats_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn endnote_xml() {
    run_format_fixtures(&fixtures_root(), "endnotexml", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        endnotexml_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn tei() {
    run_format_fixtures(&fixtures_root(), "tei", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        tei_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn docx() {
    run_format_fixtures(&fixtures_root(), "docx", |input| {
        rescribe_fmt_ooxml::docx::parse_bytes(input)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn odt() {
    run_format_fixtures(&fixtures_root(), "odt", |input| {
        odf_fmt::rescribe::parse(input)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn epub() {
    run_format_fixtures(&fixtures_root(), "epub", |input| {
        epub_fmt::rescribe::parse(input)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn pptx() {
    run_format_fixtures(&fixtures_root(), "pptx", |input| {
        rescribe_fmt_ooxml::pptx::parse(input)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn xlsx() {
    run_format_fixtures(&fixtures_root(), "xlsx", |input| {
        rescribe_fmt_ooxml::xlsx::parse_bytes(input)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn pdf() {
    run_format_fixtures(&fixtures_root(), "pdf", |input| {
        rescribe_fmt_pdf::parse(input)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn rtf() {
    run_format_fixtures(&fixtures_root(), "rtf", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rtf_fmt::rescribe::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn multimarkdown() {
    // "subscript"/"superscript" are skipped: MultiMarkdown's `~sub~`/`^sup^`
    // syntax is pulldown-cmark's own ENABLE_SUBSCRIPT/ENABLE_SUPERSCRIPT
    // extensions, which commonmark-fmt does not expose as a feature (its
    // feature set is tables/task-lists/strikethrough/frontmatter/footnotes/
    // definition-lists/math — confirmed by reading its Cargo.toml). Single-
    // tilde `~sub~` additionally collides with commonmark-fmt's own
    // strikethrough feature (GFM strikethrough accepts single *or* double
    // tilde), which claims it as `Strikethrough` before multimarkdown-fmt's
    // citation/cross-reference-style post-processing ever sees literal
    // text to rescan — closing this gap requires adding subscript/
    // superscript support to commonmark-fmt itself (a separate vertical),
    // not something multimarkdown-fmt can add on top. Tracked in TODO.md.
    run_format_fixtures_excluding(
        &fixtures_root(),
        "multimarkdown",
        &["subscript", "superscript"],
        |input| {
            let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
            multimarkdown_fmt::rescribe::parse(s)
                .map(|r| r.value)
                .map_err(|e| e.to_string())
        },
    );
}

// ---------------------------------------------------------------------------
// Writer harness tests for bidirectional formats
// ---------------------------------------------------------------------------

#[test]
fn djot_writer() {
    run_format_writer_fixtures(&fixtures_root(), "djot", |doc| {
        djot_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn rtf_writer() {
    run_format_writer_fixtures(&fixtures_root(), "rtf", |doc| {
        rtf_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

// ---------------------------------------------------------------------------
// Tier D presentation writer smoke tests
// ---------------------------------------------------------------------------

#[test]
fn beamer_writer() {
    run_format_writer_fixtures(&fixtures_root(), "beamer", |doc| {
        rescribe_write_beamer::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn revealjs_writer() {
    run_format_writer_fixtures(&fixtures_root(), "revealjs", |doc| {
        rescribe_write_revealjs::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn slidy_writer() {
    run_format_writer_fixtures(&fixtures_root(), "slidy", |doc| {
        rescribe_write_slidy::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn s5_writer() {
    run_format_writer_fixtures(&fixtures_root(), "s5", |doc| {
        rescribe_write_s5::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn dzslides_writer() {
    run_format_writer_fixtures(&fixtures_root(), "dzslides", |doc| {
        rescribe_write_dzslides::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn slideous_writer() {
    run_format_writer_fixtures(&fixtures_root(), "slideous", |doc| {
        rescribe_write_slideous::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn context_writer() {
    run_format_writer_fixtures(&fixtures_root(), "context", |doc| {
        rescribe_write_context::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn ms_writer() {
    run_format_writer_fixtures(&fixtures_root(), "ms", |doc| {
        rescribe_write_ms::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn icml_writer() {
    run_format_writer_fixtures(&fixtures_root(), "icml", |doc| {
        rescribe_write_icml::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn chunkedhtml_writer() {
    run_format_writer_fixtures(&fixtures_root(), "chunkedhtml", |doc| {
        rescribe_write_chunkedhtml::emit(doc)
            .map(|r| {
                r.value
                    .iter()
                    .flat_map(|chunk| chunk.content.iter().copied())
                    .collect()
            })
            .map_err(|e| e.to_string())
    });
}

// ---------------------------------------------------------------------------
// Major text format writer smoke tests
// ---------------------------------------------------------------------------

#[test]
fn markdown_writer() {
    run_format_writer_fixtures(&fixtures_root(), "markdown", |doc| {
        rescribe_write_markdown::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn commonmark_writer() {
    run_format_writer_fixtures(&fixtures_root(), "commonmark", |doc| {
        rescribe_write_commonmark::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn gfm_writer() {
    run_format_writer_fixtures(&fixtures_root(), "gfm", |doc| {
        rescribe_write_gfm::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn html_writer() {
    run_format_writer_fixtures(&fixtures_root(), "html", |doc| {
        html_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn latex_writer() {
    run_format_writer_fixtures(&fixtures_root(), "latex", |doc| {
        latex_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn org_writer() {
    run_format_writer_fixtures(&fixtures_root(), "org", |doc| {
        org_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn rst_writer() {
    run_format_writer_fixtures(&fixtures_root(), "rst", |doc| {
        rst_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn asciidoc_writer() {
    run_format_writer_fixtures(&fixtures_root(), "asciidoc", |doc| {
        asciidoc::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn mediawiki_writer() {
    run_format_writer_fixtures(&fixtures_root(), "mediawiki", |doc| {
        mediawiki_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn dokuwiki_writer() {
    run_format_writer_fixtures(&fixtures_root(), "dokuwiki", |doc| {
        dokuwiki::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn fb2_writer() {
    run_format_writer_fixtures(&fixtures_root(), "fb2", |doc| {
        fb2_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn docbook_writer() {
    run_format_writer_fixtures(&fixtures_root(), "docbook", |doc| {
        docbook_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn typst_writer() {
    run_format_writer_fixtures(&fixtures_root(), "typst", |doc| {
        typst_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn tei_writer() {
    run_format_writer_fixtures(&fixtures_root(), "tei", |doc| {
        tei_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn texinfo_writer() {
    run_format_writer_fixtures(&fixtures_root(), "texinfo", |doc| {
        texinfo::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn textile_writer() {
    run_format_writer_fixtures(&fixtures_root(), "textile", |doc| {
        textile_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn jats_writer() {
    run_format_writer_fixtures(&fixtures_root(), "jats", |doc| {
        jats_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn jira_writer() {
    run_format_writer_fixtures(&fixtures_root(), "jira", |doc| {
        jira_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn man_writer() {
    run_format_writer_fixtures(&fixtures_root(), "man", |doc| {
        man_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn muse_writer() {
    run_format_writer_fixtures(&fixtures_root(), "muse", |doc| {
        muse_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn opml_writer() {
    run_format_writer_fixtures(&fixtures_root(), "opml", |doc| {
        opml_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn pod_writer() {
    run_format_writer_fixtures(&fixtures_root(), "pod", |doc| {
        pod_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn t2t_writer() {
    run_format_writer_fixtures(&fixtures_root(), "t2t", |doc| {
        t2t::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn tikiwiki_writer() {
    run_format_writer_fixtures(&fixtures_root(), "tikiwiki", |doc| {
        tikiwiki::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn tsv_writer() {
    run_format_writer_fixtures(&fixtures_root(), "tsv", |doc| {
        tsv_fmt::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn twiki_writer() {
    run_format_writer_fixtures(&fixtures_root(), "twiki", |doc| {
        twiki::rescribe::emit(doc)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}
