//! Print IR tree for DOCX fixture files (used to write expected.json).
//!
//! Run with: cargo run -p rescribe-read-docx --example probe_fixtures

use rescribe_core::Node;
use rescribe_read_docx::parse_bytes;
use std::path::Path;
use std::{env, fs};

fn print_node(node: &Node, path: &str) {
    let kind = node.kind.as_str();
    let props: Vec<String> = node
        .props
        .iter()
        .map(|(k, v)| format!("\"{}\":{}", k, prop_to_json(v)))
        .collect();
    let props_str = if props.is_empty() {
        String::new()
    } else {
        format!(", \"props\": {{ {} }}", props.join(", "))
    };
    println!(
        "  {{ \"path\": \"{}\", \"kind\": \"{}\"{}  }},",
        path, kind, props_str
    );
    for (i, child) in node.children.iter().enumerate() {
        let child_path = format!("{}/{}", path, i);
        print_node(child, &child_path);
    }
}

fn prop_to_json(v: &rescribe_core::PropValue) -> String {
    match v {
        rescribe_core::PropValue::String(s) => {
            format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
        }
        rescribe_core::PropValue::Int(i) => i.to_string(),
        rescribe_core::PropValue::Float(f) => f.to_string(),
        rescribe_core::PropValue::Bool(b) => b.to_string(),
        _ => "null".to_string(),
    }
}

fn probe(name: &str, path: &str) {
    let bytes = fs::read(path).expect(path);
    let result = parse_bytes(&bytes).expect("parse failed");
    println!("=== {} ===", name);
    println!("assertions: [");
    print_node(&result.value.content, "/");
    println!("]");
    if !result.warnings.is_empty() {
        println!("WARNINGS:");
        for w in &result.warnings {
            println!("  - {}", w.message);
        }
    }
    println!();
}

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .unwrap() // readers/
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap(); // workspace root

    let fixtures = [
        "inline_bold",
        "inline_italic",
        "inline_color",
        "inline_font_size",
        "inline_underline",
        "alignment",
        "list",
        "list_ordered",
        "table",
        "footnote",
    ];

    for name in &fixtures {
        let path = workspace_root.join(format!("fixtures/docx/{}/input.docx", name));
        probe(name, path.to_str().unwrap());
    }
}
