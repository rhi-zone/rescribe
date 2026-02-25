//! Fixture test runner for rescribe.
//!
//! Fixtures live in `fixtures/{format}/{name}/` at the workspace root:
//! - `input.{ext}` — the input document in the format under test
//! - `expected.json` — assertions about the parsed result
//!
//! See `fixtures/spec.md` for the full cross-language specification.

pub mod pandoc_harness;

use rescribe_core::{Node, PropValue};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

// ---------------------------------------------------------------------------
// Manifest types
// ---------------------------------------------------------------------------

/// A parsed fixture manifest (`expected.json`).
#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub description: String,
    pub assertions: Vec<Assertion>,
}

/// A single assertion in a fixture manifest.
#[derive(Debug, Deserialize)]
pub struct Assertion {
    /// Path from document content root, e.g. `"/0/1/2"`.
    pub path: String,
    /// Expected node kind (optional).
    pub kind: Option<String>,
    /// Expected props. JSON `null` means the prop must be absent.
    #[serde(default)]
    pub props: HashMap<String, serde_json::Value>,
    /// Expected number of children (optional).
    pub children_count: Option<usize>,
}

// ---------------------------------------------------------------------------
// Failure reporting
// ---------------------------------------------------------------------------

/// A single assertion failure.
#[derive(Debug)]
pub struct Failure {
    pub path: String,
    pub message: String,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "at {}: {}", self.path, self.message)
        }
    }
}

// ---------------------------------------------------------------------------
// Path walking
// ---------------------------------------------------------------------------

/// Walk a `/`-delimited integer path from a content root node.
///
/// `/0/1/2` → `root.children[0].children[1].children[2]`
pub fn walk_path<'a>(root: &'a Node, path: &str) -> Option<&'a Node> {
    let components = path.trim_start_matches('/');
    if components.is_empty() {
        return Some(root);
    }
    let mut node = root;
    for part in components.split('/') {
        let idx: usize = part.parse().ok()?;
        node = node.children.get(idx)?;
    }
    Some(node)
}

// ---------------------------------------------------------------------------
// Assertion checking
// ---------------------------------------------------------------------------

/// Check all assertions against `content_root` (the `document` node).
///
/// Returns a list of failures; empty means all assertions passed.
pub fn check(content_root: &Node, fixture: &Fixture) -> Vec<Failure> {
    let mut failures = Vec::new();
    for assertion in &fixture.assertions {
        match walk_path(content_root, &assertion.path) {
            None => failures.push(Failure {
                path: assertion.path.clone(),
                message: "node not found (path out of bounds)".into(),
            }),
            Some(node) => {
                check_assertion(node, assertion, &mut failures);
            }
        }
    }
    failures
}

fn check_assertion(node: &Node, assertion: &Assertion, failures: &mut Vec<Failure>) {
    if let Some(expected_kind) = &assertion.kind
        && node.kind.as_str() != expected_kind
    {
        failures.push(Failure {
            path: assertion.path.clone(),
            message: format!(
                "kind: expected {:?}, got {:?}",
                expected_kind,
                node.kind.as_str()
            ),
        });
    }
    for (key, expected_json) in &assertion.props {
        check_prop(&assertion.path, node, key, expected_json, failures);
    }
    if let Some(expected_count) = assertion.children_count {
        let actual = node.children.len();
        if actual != expected_count {
            failures.push(Failure {
                path: assertion.path.clone(),
                message: format!("children_count: expected {expected_count}, got {actual}"),
            });
        }
    }
}

fn check_prop(
    path: &str,
    node: &Node,
    key: &str,
    expected_json: &serde_json::Value,
    failures: &mut Vec<Failure>,
) {
    if expected_json.is_null() {
        if node.props.get(key).is_some() {
            failures.push(Failure {
                path: path.to_string(),
                message: format!("prop {key:?}: expected absent, but is present"),
            });
        }
        return;
    }
    let actual = match node.props.get(key) {
        Some(v) => v,
        None => {
            failures.push(Failure {
                path: path.to_string(),
                message: format!("prop {key:?}: expected {expected_json}, but prop is absent"),
            });
            return;
        }
    };
    let matches = match (expected_json, actual) {
        (serde_json::Value::String(s), PropValue::String(a)) => s == a,
        (serde_json::Value::Number(n), PropValue::Int(a)) => n.as_i64().is_some_and(|i| i == *a),
        (serde_json::Value::Number(n), PropValue::Float(a)) => {
            n.as_f64().is_some_and(|f| (f - a).abs() < 1e-9)
        }
        (serde_json::Value::Bool(b), PropValue::Bool(a)) => b == a,
        _ => false,
    };
    if !matches {
        failures.push(Failure {
            path: path.to_string(),
            message: format!("prop {key:?}: expected {expected_json}, got {actual:?}"),
        });
    }
}

// ---------------------------------------------------------------------------
// Fixture discovery and runner
// ---------------------------------------------------------------------------

/// Load a fixture from `expected.json` in `dir`.
pub fn load_fixture(dir: &Path) -> Result<Fixture, String> {
    let path = dir.join("expected.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&content).map_err(|e| format!("cannot parse {}: {e}", path.display()))
}

/// Return sorted fixture sub-directories for `fixtures_root/{format}/`.
///
/// Returns an empty vec (without failing) if the directory doesn't exist,
/// so tests skip gracefully when fixtures haven't been written yet.
pub fn discover_fixtures(fixtures_root: &Path, format: &str) -> Vec<std::path::PathBuf> {
    let format_dir = fixtures_root.join(format);
    let Ok(entries) = std::fs::read_dir(&format_dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<_> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .map(|e| e.path())
        .collect();
    dirs.sort();
    dirs
}

/// Run all fixtures for `format` against `parse_fn`.
///
/// `parse_fn` receives the raw input bytes and must return the document
/// content root (`Document::content`, the `"document"` node) or an error
/// string.  Panics if any fixture fails, printing a full report.
pub fn run_format_fixtures(
    fixtures_root: &Path,
    format: &str,
    parse_fn: impl Fn(&[u8]) -> Result<Node, String>,
) {
    let dirs = discover_fixtures(fixtures_root, format);
    if dirs.is_empty() {
        return; // No fixtures yet — skip gracefully.
    }

    let mut all_failures: Vec<(String, Vec<Failure>)> = Vec::new();

    for dir in &dirs {
        let fixture = match load_fixture(dir) {
            Ok(f) => f,
            Err(e) => panic!("fixture load error in {}: {e}", dir.display()),
        };

        let input_path = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_stem().and_then(|s| s.to_str()) == Some("input") && p.extension().is_some()
            })
            .unwrap_or_else(|| panic!("no input.* file in {}", dir.display()));

        let input = std::fs::read(&input_path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", input_path.display()));

        let content_root = match parse_fn(&input) {
            Ok(node) => node,
            Err(e) => {
                all_failures.push((
                    format!("{} ({})", fixture.description, dir.display()),
                    vec![Failure {
                        path: String::new(),
                        message: format!("parse error: {e}"),
                    }],
                ));
                continue;
            }
        };

        let failures = check(&content_root, &fixture);
        if !failures.is_empty() {
            all_failures.push((
                format!("{} ({})", fixture.description, dir.display()),
                failures,
            ));
        }
    }

    if !all_failures.is_empty() {
        let mut msg = format!(
            "{} fixture(s) failed for format {:?}:\n",
            all_failures.len(),
            format
        );
        for (desc, failures) in &all_failures {
            msg.push_str(&format!("\n  FAIL: {desc}\n"));
            for f in failures {
                msg.push_str(&format!("    - {f}\n"));
            }
        }
        panic!("{msg}");
    }
}
