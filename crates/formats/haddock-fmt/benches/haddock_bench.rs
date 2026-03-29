use criterion::{Criterion, criterion_group, criterion_main};
use haddock_fmt::{build, parse};

const SMALL: &str = r#"
= Hello World

This is a /short/ paragraph with @inline code@ and a "link"<https://example.com>.

* Item one
* Item two
* Item three

> fn main() {
>     println!("hello");
> }
"#;

const MEDIUM: &str = r#"
= Introduction

This document tests the Haddock parser with a medium-sized input containing many constructs.
It includes __bold__, /italic/, @code@, and "hyperlinks"<https://example.com>.

== Lists

Unordered list:

* First item with some text
* Second item with /emphasis/
* Third item with @inline code@

Ordered list:

(1) Step one
(2) Step two
(3) Step three

== Definition List

[Term] A short definition of the term.
[Another] A longer definition with more detail.

== Code Blocks

> def hello():
>     print("Hello, world!")
>     return 42

> pub fn add(a: i32, b: i32) -> i32 {
>     a + b
> }

=== Doc Tests

>>> 1 + 1
2

>>> map (+1) [1,2,3]
[2,3,4]

== Properties

@since 4.2.0
@deprecated Use newFunc instead
@param x The input value
@returns The computed result

== Emphasis Examples

This paragraph has __bold__, /italic/, @code@,
and 'identifier' reference and "Data.Map" module reference.

== Links

"Click here"<https://example.com> for more.

<https://bare-url.example.com>

== More Content

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.
"#;

fn bench_haddock_parse_small(c: &mut Criterion) {
    c.bench_function("haddock_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_haddock_parse_medium(c: &mut Criterion) {
    c.bench_function("haddock_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_haddock_roundtrip_small(c: &mut Criterion) {
    c.bench_function("haddock_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_haddock_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("haddock_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_haddock_emit_medium(c: &mut Criterion) {
    c.bench_function("haddock_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_haddock_parse_small,
    bench_haddock_parse_medium,
    bench_haddock_roundtrip_small,
    bench_haddock_roundtrip_medium,
    bench_haddock_emit_medium,
);
criterion_main!(benches);
