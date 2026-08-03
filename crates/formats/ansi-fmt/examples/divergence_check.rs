//! Check build() (AST-based emit) against the streaming Writer (fed by
//! events()) across every fixture in a directory, printing every diverging
//! fixture (not just the first, unlike the fixture-suite test in
//! rescribe-fixtures, which only ever surfaces the first divergence per run).
//!
//! Usage:
//!   cargo run -p ansi-fmt --example divergence_check -- [fixtures/ansi]
//!
//! Defaults to `fixtures/ansi` relative to the current directory (i.e. run
//! from the workspace root) if no path is given.

use std::path::{Path, PathBuf};

fn find_input(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.file_stem().map(|s| s == "input").unwrap_or(false))
}

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("fixtures/ansi"));

    let mut names: Vec<_> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("reading fixtures dir {}: {e}", root.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    names.sort();

    let mut diverged = vec![];
    for path in &names {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(input_path) = find_input(path) else {
            continue;
        };
        let input = std::fs::read(&input_path).expect("read fixture input");
        let (doc, _diags) = ansi_fmt::parse(&input);
        let built = ansi_fmt::emit(&doc);

        let mut w = ansi_fmt::Writer::new(Vec::<u8>::new());
        for e in ansi_fmt::events(&input) {
            w.write_event(e.into_owned());
        }
        let streamed_bytes = w.finish();
        let streamed = String::from_utf8_lossy(&streamed_bytes).into_owned();

        if built != streamed {
            diverged.push((name.clone(), built.clone(), streamed.clone()));
        }
    }

    println!(
        "checked {} fixtures, {} diverged",
        names.len(),
        diverged.len()
    );
    for (name, built, streamed) in &diverged {
        println!("--- {} ---", name);
        println!("  build(): {:?}", built);
        println!("  Writer : {:?}", streamed);
    }
}
