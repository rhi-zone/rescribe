//! Fixture integration tests.
//!
//! Discovers fixtures from `fixtures/{format}/` at the workspace root and
//! runs each against the appropriate reader.  Tests skip gracefully if a
//! format directory doesn't exist yet.

use rescribe_fixtures::run_format_fixtures;
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
    run_format_fixtures(&fixtures_root(), "markdown", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_markdown::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn html() {
    run_format_fixtures(&fixtures_root(), "html", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_html::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn asciidoc() {
    run_format_fixtures(&fixtures_root(), "asciidoc", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_asciidoc::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn mediawiki() {
    run_format_fixtures(&fixtures_root(), "mediawiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_mediawiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn latex() {
    run_format_fixtures(&fixtures_root(), "latex", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_latex::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn rst() {
    run_format_fixtures(&fixtures_root(), "rst", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_rst::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn org() {
    run_format_fixtures(&fixtures_root(), "org", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_org::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn creole() {
    run_format_fixtures(&fixtures_root(), "creole", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_creole::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn djot() {
    run_format_fixtures(&fixtures_root(), "djot", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_djot::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn textile() {
    run_format_fixtures(&fixtures_root(), "textile", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_textile::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn muse() {
    run_format_fixtures(&fixtures_root(), "muse", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_muse::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn t2t() {
    run_format_fixtures(&fixtures_root(), "t2t", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_t2t::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn tikiwiki() {
    run_format_fixtures(&fixtures_root(), "tikiwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_tikiwiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn twiki() {
    run_format_fixtures(&fixtures_root(), "twiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_twiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn vimwiki() {
    run_format_fixtures(&fixtures_root(), "vimwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_vimwiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn dokuwiki() {
    run_format_fixtures(&fixtures_root(), "dokuwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_dokuwiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn jira() {
    run_format_fixtures(&fixtures_root(), "jira", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_jira::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn haddock() {
    run_format_fixtures(&fixtures_root(), "haddock", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_haddock::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn pod() {
    run_format_fixtures(&fixtures_root(), "pod", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_pod::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn man() {
    run_format_fixtures(&fixtures_root(), "man", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_man::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn xwiki() {
    run_format_fixtures(&fixtures_root(), "xwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_xwiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn zimwiki() {
    run_format_fixtures(&fixtures_root(), "zimwiki", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_zimwiki::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn bbcode() {
    run_format_fixtures(&fixtures_root(), "bbcode", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_bbcode::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn texinfo() {
    run_format_fixtures(&fixtures_root(), "texinfo", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_texinfo::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn markua() {
    run_format_fixtures(&fixtures_root(), "markua", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_markua::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn fountain() {
    run_format_fixtures(&fixtures_root(), "fountain", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_fountain::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn ansi() {
    run_format_fixtures(&fixtures_root(), "ansi", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_ansi::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn csl_json() {
    run_format_fixtures(&fixtures_root(), "csl-json", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_csl_json::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn native() {
    run_format_fixtures(&fixtures_root(), "native", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_native::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn pandoc_json() {
    run_format_fixtures(&fixtures_root(), "pandoc-json", |input| {
        rescribe_read_pandoc_json::parse(std::str::from_utf8(input).map_err(|e| e.to_string())?)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn docbook() {
    run_format_fixtures(&fixtures_root(), "docbook", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_docbook::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn fb2() {
    run_format_fixtures(&fixtures_root(), "fb2", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_fb2::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn ipynb() {
    run_format_fixtures(&fixtures_root(), "ipynb", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_ipynb::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn csv() {
    run_format_fixtures(&fixtures_root(), "csv", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_csv::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn tsv() {
    run_format_fixtures(&fixtures_root(), "tsv", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_tsv::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn opml() {
    run_format_fixtures(&fixtures_root(), "opml", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_opml::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn ris() {
    run_format_fixtures(&fixtures_root(), "ris", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_ris::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn bibtex() {
    run_format_fixtures(&fixtures_root(), "bibtex", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_bibtex::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn biblatex() {
    run_format_fixtures(&fixtures_root(), "biblatex", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_biblatex::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn typst() {
    run_format_fixtures(&fixtures_root(), "typst", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_typst::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn jats() {
    run_format_fixtures(&fixtures_root(), "jats", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_jats::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn endnote_xml() {
    run_format_fixtures(&fixtures_root(), "endnotexml", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_endnotexml::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}

#[test]
fn tei() {
    run_format_fixtures(&fixtures_root(), "tei", |input| {
        let s = std::str::from_utf8(input).map_err(|e| e.to_string())?;
        rescribe_read_tei::parse(s)
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    });
}
