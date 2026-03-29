use criterion::{criterion_group, criterion_main, Criterion};
use tikiwiki::{build, parse};

const SMALL: &str = r#"
! Hello World

This is a ''short'' paragraph with -+inline code+- and a [https://example.com|link].

* Item one
* Item two
* Item three

{CODE(lang=rust)}
fn main() {
    println!("hello");
}
{CODE}
"#;

const MEDIUM: &str = r#"
! Introduction

This document tests the TikiWiki parser with a medium-sized input.
It includes __bold__, ''italic'', -+code+-, ===underline===, and [https://example.com|links].

!! Lists

* First item with some text
* Second item with ''emphasis''
** Nested item one
** Nested item two

# Step one
# Step two

!! Tables

||^Name^|^Age^||
||Alice|30||
||Bob|25||

!! Code Blocks

{CODE(lang=python)}
def hello():
    print("Hello, world!")
{CODE}

!! Blockquotes

{QUOTE()}
This is a blockquote.
{QUOTE}

!! Inline Features

__bold__ ''italic'' ===underline=== --strike-- -+mono+- ^sup^ ,,sub,,

!! Links

[https://example.com|Example Site]
((WikiWord))

---

Final section.
"#;

fn bench_tikiwiki_parse_small(c: &mut Criterion) {
    c.bench_function("tikiwiki_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_tikiwiki_parse_medium(c: &mut Criterion) {
    c.bench_function("tikiwiki_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_tikiwiki_roundtrip_small(c: &mut Criterion) {
    c.bench_function("tikiwiki_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_tikiwiki_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("tikiwiki_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_tikiwiki_emit_medium(c: &mut Criterion) {
    c.bench_function("tikiwiki_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_tikiwiki_parse_small,
    bench_tikiwiki_parse_medium,
    bench_tikiwiki_roundtrip_small,
    bench_tikiwiki_roundtrip_medium,
    bench_tikiwiki_emit_medium,
);
criterion_main!(benches);
