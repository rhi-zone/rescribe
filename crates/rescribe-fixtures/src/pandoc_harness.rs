//! Pandoc corpus harness.
//!
//! Uses the Pandoc test corpus at `~/git/pandoc/test/` as a local oracle.
//! Fixtures are GPL — they never enter the repo.  Tests skip gracefully when
//! either the corpus directory or the `pandoc` binary is absent.
//!
//! # What is checked
//!
//! For each format:
//! 1. **No panic** — the rescribe reader must return `Ok(_)` on the corpus input.
//! 2. **Text coverage** (when `pandoc` is available) — extract every word from
//!    both the pandoc-JSON reference and our output; compute
//!    `coverage = |ref ∩ ours| / |ref|`.  We report but do not fail on low
//!    coverage; the point is to catalogue deficiencies, not gate CI.
//!
//! # Running locally
//!
//! ```bash
//! cargo test -p rescribe-fixtures -- --ignored --nocapture 2>&1 | grep -v "^$"
//! ```

use rescribe_core::{Document, Node};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Returns the Pandoc corpus directory (`~/git/pandoc/test/`) if it exists.
pub fn corpus_dir() -> Option<PathBuf> {
    let path = home_dir().join("git/pandoc/test");
    path.is_dir().then_some(path)
}

/// Returns the path to the `pandoc` binary if it can be found on PATH.
pub fn find_pandoc() -> Option<PathBuf> {
    // Try PATH first.
    if let Ok(out) = Command::new("sh")
        .args(["-c", "command -v pandoc"])
        .output()
        && out.status.success()
    {
        let s = String::from_utf8_lossy(&out.stdout);
        let p = PathBuf::from(s.trim());
        if p.is_file() {
            return Some(p);
        }
    }
    // Common nix profile locations.
    for prefix in &[
        "/home/me/.nix-profile/bin/pandoc",
        "/run/current-system/sw/bin/pandoc",
        "/nix/var/nix/profiles/default/bin/pandoc",
    ] {
        let p = Path::new(prefix);
        if p.is_file() {
            return Some(p.to_path_buf());
        }
    }
    None
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/home/me".into()))
}

// ---------------------------------------------------------------------------
// Oracle — run pandoc, parse JSON, extract text
// ---------------------------------------------------------------------------

/// Run `pandoc --from={from} --to=json {file}` and return the JSON string.
///
/// Returns `None` if pandoc is unavailable or fails (e.g. unsupported format).
pub fn pandoc_to_json(pandoc: &Path, from: &str, file: &Path) -> Option<String> {
    let out = Command::new(pandoc)
        .args(["--from", from, "--to", "json", "--quiet"])
        .arg(file)
        .output()
        .ok()?;
    if out.status.success() {
        String::from_utf8(out.stdout).ok()
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Text extraction
// ---------------------------------------------------------------------------

/// Extract all text words from a rescribe document (case-folded, alpha-only).
pub fn extract_words(doc: &Document) -> Vec<String> {
    let mut words = Vec::new();
    collect_words(&doc.content, &mut words);
    words
}

fn collect_words(node: &Node, out: &mut Vec<String>) {
    if node.kind.as_str() == "text"
        && let Some(content) = node.props.get_str("content")
    {
        for w in content.split_whitespace() {
            let word: String = w.chars().filter(|c| c.is_alphabetic()).collect();
            if word.len() >= 2 {
                out.push(word.to_lowercase());
            }
        }
    }
    for child in &node.children {
        collect_words(child, out);
    }
}

/// Word-level coverage: fraction of reference words present in `ours`.
///
/// Uses multisets so repeated words are counted correctly.
pub fn word_coverage(reference: &[String], ours: &[String]) -> f64 {
    if reference.is_empty() {
        return 1.0;
    }
    let mut ref_counts: HashMap<&str, usize> = HashMap::new();
    for w in reference {
        *ref_counts.entry(w.as_str()).or_default() += 1;
    }
    let mut our_counts: HashMap<&str, usize> = HashMap::new();
    for w in ours {
        *our_counts.entry(w.as_str()).or_default() += 1;
    }
    let matched: usize = ref_counts
        .iter()
        .map(|(w, &rc)| rc.min(*our_counts.get(*w).unwrap_or(&0)))
        .sum();
    matched as f64 / reference.len() as f64
}

// ---------------------------------------------------------------------------
// Per-format test descriptor
// ---------------------------------------------------------------------------

/// Describes one format's entry in the Pandoc corpus.
pub struct CorpusEntry {
    /// rescribe format name (also used for finding the reader).
    pub format: &'static str,
    /// Pandoc `--from` argument.
    pub pandoc_from: &'static str,
    /// Filename under `~/git/pandoc/test/`.
    pub filename: &'static str,
}

/// All Pandoc corpus entries that rescribe has a reader for.
pub const CORPUS: &[CorpusEntry] = &[
    CorpusEntry {
        format: "markdown",
        pandoc_from: "markdown",
        filename: "testsuite.txt",
    },
    CorpusEntry {
        format: "markdown",
        pandoc_from: "markdown",
        filename: "markdown-reader-more.txt",
    },
    CorpusEntry {
        format: "rst",
        pandoc_from: "rst",
        filename: "rst-reader.rst",
    },
    CorpusEntry {
        format: "html",
        pandoc_from: "html",
        filename: "html-reader.html",
    },
    CorpusEntry {
        format: "latex",
        pandoc_from: "latex",
        filename: "latex-reader.latex",
    },
    CorpusEntry {
        format: "org",
        pandoc_from: "org",
        filename: "org-select-tags.org",
    },
    CorpusEntry {
        format: "djot",
        pandoc_from: "djot",
        filename: "djot-reader.djot",
    },
    CorpusEntry {
        format: "mediawiki",
        pandoc_from: "mediawiki",
        filename: "mediawiki-reader.wiki",
    },
    CorpusEntry {
        format: "creole",
        pandoc_from: "creole",
        filename: "creole-reader.txt",
    },
    CorpusEntry {
        format: "textile",
        pandoc_from: "textile",
        filename: "textile-reader.textile",
    },
    CorpusEntry {
        format: "haddock",
        pandoc_from: "haddock",
        filename: "haddock-reader.haddock",
    },
    CorpusEntry {
        format: "jira",
        pandoc_from: "jira",
        filename: "jira-reader.jira",
    },
    CorpusEntry {
        format: "tikiwiki",
        pandoc_from: "tikiwiki",
        filename: "tikiwiki-reader.tikiwiki",
    },
    CorpusEntry {
        format: "twiki",
        pandoc_from: "twiki",
        filename: "twiki-reader.twiki",
    },
    CorpusEntry {
        format: "vimwiki",
        pandoc_from: "vimwiki",
        filename: "vimwiki-reader.wiki",
    },
    CorpusEntry {
        format: "t2t",
        pandoc_from: "t2t",
        filename: "txt2tags.t2t",
    },
    CorpusEntry {
        format: "pod",
        pandoc_from: "pod",
        filename: "pod-reader.pod",
    },
    CorpusEntry {
        format: "man",
        pandoc_from: "man",
        filename: "man-reader.man",
    },
    CorpusEntry {
        format: "asciidoc",
        pandoc_from: "asciidoc",
        filename: "asciidoc-reader.adoc",
    },
    CorpusEntry {
        format: "typst",
        pandoc_from: "typst",
        filename: "typst-reader.typ",
    },
    CorpusEntry {
        format: "docbook",
        pandoc_from: "docbook",
        filename: "docbook-reader.docbook",
    },
    CorpusEntry {
        format: "jats",
        pandoc_from: "jats",
        filename: "jats-reader.xml",
    },
    CorpusEntry {
        format: "dokuwiki",
        pandoc_from: "dokuwiki",
        filename: "dokuwiki_inline_formatting.dokuwiki",
    },
    CorpusEntry {
        format: "muse",
        pandoc_from: "muse",
        filename: "tables.muse",
    },
    CorpusEntry {
        format: "fb2",
        pandoc_from: "fb2",
        filename: "fb2/pb_brief.fb2",
    },
];

// ---------------------------------------------------------------------------
// Run result
// ---------------------------------------------------------------------------

/// Result of running one corpus entry through rescribe + the pandoc oracle.
#[derive(Debug)]
pub struct RunResult {
    pub format: &'static str,
    pub filename: &'static str,
    /// Whether rescribe parsed without error.
    pub parse_ok: bool,
    pub parse_error: Option<String>,
    /// Word coverage against the pandoc oracle (None if pandoc unavailable).
    pub coverage: Option<f64>,
    /// Reference word count (from pandoc oracle).
    pub ref_words: usize,
    /// Our word count.
    pub our_words: usize,
}

impl RunResult {
    pub fn coverage_pct(&self) -> Option<u32> {
        self.coverage.map(|c| (c * 100.0).round() as u32)
    }
}

// ---------------------------------------------------------------------------
// Core runner
// ---------------------------------------------------------------------------

/// Run a single corpus entry.
///
/// `parse_fn` receives the raw file bytes and must return a `Document` or
/// an error string.  Panics from the parser propagate normally (they are bugs).
pub fn run_entry(
    entry: &CorpusEntry,
    corpus: &Path,
    pandoc: Option<&Path>,
    parse_fn: impl Fn(&[u8]) -> Result<Document, String>,
) -> RunResult {
    let file = corpus.join(entry.filename);

    let input = match std::fs::read(&file) {
        Ok(b) => b,
        Err(e) => {
            return RunResult {
                format: entry.format,
                filename: entry.filename,
                parse_ok: false,
                parse_error: Some(format!("cannot read {}: {e}", file.display())),
                coverage: None,
                ref_words: 0,
                our_words: 0,
            };
        }
    };

    // Parse with rescribe.
    let (parse_ok, parse_error, our_doc) = match parse_fn(&input) {
        Ok(doc) => (true, None, Some(doc)),
        Err(e) => (false, Some(e), None),
    };

    // Oracle: run pandoc and compare text.
    let (coverage, ref_words, our_words) = if let (Some(pbin), Some(our)) = (pandoc, &our_doc) {
        match pandoc_to_json(pbin, entry.pandoc_from, &file) {
            None => (None, 0, 0),
            Some(json) => {
                let ref_doc = rescribe_read_pandoc_json::parse(&json)
                    .map(|r| r.value)
                    .ok();
                match ref_doc {
                    None => (None, 0, 0),
                    Some(ref_doc) => {
                        let ref_words = extract_words(&ref_doc);
                        let our_words_v = extract_words(our);
                        let cov = word_coverage(&ref_words, &our_words_v);
                        (Some(cov), ref_words.len(), our_words_v.len())
                    }
                }
            }
        }
    } else {
        let our_words = our_doc.as_ref().map_or(0, |d| extract_words(d).len());
        (None, 0, our_words)
    };

    RunResult {
        format: entry.format,
        filename: entry.filename,
        parse_ok,
        parse_error,
        coverage,
        ref_words,
        our_words,
    }
}

// ---------------------------------------------------------------------------
// Report formatting
// ---------------------------------------------------------------------------

/// Print a summary table of results to stderr.
pub fn print_report(results: &[RunResult], pandoc_available: bool) {
    eprintln!();
    eprintln!("{:<12} {:<38} {:<8} Coverage", "Format", "File", "Parse");
    eprintln!("{}", "-".repeat(75));
    for r in results {
        let parse_col = if r.parse_ok { "OK" } else { "FAIL" };
        let cov_col = match r.coverage_pct() {
            Some(pct) => format!("{pct:3}%  (ref={} ours={})", r.ref_words, r.our_words),
            None if !pandoc_available => "—  (pandoc not found)".into(),
            None => "—  (pandoc failed)".into(),
        };
        eprintln!(
            "{:<12} {:<38} {:<8} {}",
            r.format, r.filename, parse_col, cov_col
        );
        if let Some(ref e) = r.parse_error {
            eprintln!("             error: {e}");
        }
    }
    eprintln!("{}", "-".repeat(75));

    let total = results.len();
    let parsed = results.iter().filter(|r| r.parse_ok).count();
    let high_cov = results
        .iter()
        .filter(|r| r.coverage.is_some_and(|c| c >= 0.9))
        .count();
    eprintln!("Parse: {parsed}/{total} OK");
    if pandoc_available {
        eprintln!("Coverage ≥90%: {high_cov}/{total}");
    }
    eprintln!();
}
