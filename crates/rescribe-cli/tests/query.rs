//! End-to-end tests for `rescribe query`.

use std::io::Write;
use std::process::{Command, Stdio};

fn rescribe() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rescribe"))
}

fn write_temp_md(contents: &str) -> tempfile_lite::TempPath {
    tempfile_lite::write_temp("md", contents)
}

#[test]
fn query_metadata_on_a_file() {
    let path = write_temp_md("# Title\n\nSome *emphasized* text.\n");
    let out = rescribe()
        .args(["query", ".metadata", path.as_str()])
        .output()
        .expect("failed to run rescribe query");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert!(value.is_object());
}

#[test]
fn query_node_kind_census_via_stdin() {
    let mut child = rescribe()
        .args([
            "query",
            "--from",
            "markdown",
            "-c",
            "[.. | .kind?] | map(select(. != null)) | group_by(.) | map({kind: .[0], count: length})",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn rescribe query");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"# Title\n\n## Sub\n\nText.\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let census = value.as_array().expect("expected an array");
    let heading_count = census
        .iter()
        .find(|entry| entry["kind"] == serde_json::json!("heading"))
        .and_then(|entry| entry["count"].as_u64())
        .expect("expected a heading entry in the census");
    assert_eq!(heading_count, 2);
}

#[test]
fn query_raw_output_unquotes_strings() {
    let path = write_temp_md("# Title\n\nText.\n");
    let out = rescribe()
        .args(["query", "-r", ".content.kind", path.as_str()])
        .output()
        .expect("failed to run rescribe query");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "document");
}

#[test]
fn query_compile_error_exits_nonzero_with_message() {
    let path = write_temp_md("# Title\n");
    let out = rescribe()
        .args(["query", ".foo[", path.as_str()])
        .output()
        .expect("failed to run rescribe query");
    assert!(!out.status.success());
    assert!(!out.stderr.is_empty());
}

mod tempfile_lite {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    pub struct TempPath(PathBuf);

    impl TempPath {
        pub fn as_str(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }

    impl Drop for TempPath {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    pub fn write_temp(ext: &str, contents: &str) -> TempPath {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rescribe-cli-test-{}-{}.{ext}",
            std::process::id(),
            id
        ));
        fs::write(&path, contents).unwrap();
        TempPath(path)
    }
}
