use criterion::{Criterion, criterion_group, criterion_main};
use twiki::{parse, build};

const SMALL: &str = r#"
---+ Hello World

This is a _short_ paragraph with =inline code= and a [[https://example.com][link]].

   * Item one
   * Item two
   * Item three

<verbatim>
fn main() {
    println!("hello");
}
</verbatim>
"#;

const MEDIUM: &str = r#"
---+ Introduction

This document tests the TWiki parser with a medium-sized input containing many constructs.
It includes *bold*, _italic_, =code=, ==bold code==, and [[https://example.com][hyperlinks]].

---++ Lists

Unordered list:

   * First item with some text
   * Second item with _emphasis_
   * Third item with =inline code=

Ordered list:

   1. Step one
   1. Step two
   1. Step three

---++ Tables

| *Name* | *Age* | *City* |
| Alice | 30 | New York |
| Bob | 25 | London |
| Carol | 35 | Tokyo |

---++ Code Blocks

<verbatim>
def hello():
    print("Hello, world!")
    return 42
</verbatim>

<verbatim>
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
</verbatim>

---+++ Inline Formatting

This paragraph has *bold*, _italic_, __bold italic__, =code=, ==bold code==,
<del>strikethrough</del>, <sup>superscript</sup>, and <sub>subscript</sub>.

---++ More Content

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.
"#;

fn bench_twiki_parse_small(c: &mut Criterion) {
    c.bench_function("twiki_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_twiki_parse_medium(c: &mut Criterion) {
    c.bench_function("twiki_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_twiki_roundtrip_small(c: &mut Criterion) {
    c.bench_function("twiki_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_twiki_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("twiki_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_twiki_emit_medium(c: &mut Criterion) {
    c.bench_function("twiki_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_twiki_parse_small,
    bench_twiki_parse_medium,
    bench_twiki_roundtrip_small,
    bench_twiki_roundtrip_medium,
    bench_twiki_emit_medium,
);
criterion_main!(benches);
