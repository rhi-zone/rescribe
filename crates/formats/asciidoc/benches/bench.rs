use criterion::{Criterion, criterion_group, criterion_main};
use asciidoc::{parse, build};

const SMALL: &str = r#"
== Hello World

This is a *short* paragraph with `inline code` and a https://example.com[link].

* Item one
* Item two
* Item three

[source,rust]
----
fn main() {
    println!("hello");
}
----
"#;

const MEDIUM: &str = r#"
= Medium Document
Author Name <author@example.com>
:toc:
:sectnums:

== Introduction

This document tests the AsciiDoc parser with a medium-sized input containing many constructs.
It includes *bold*, _italic_, `code`, and https://example.com[hyperlinks].

== Lists

Unordered list:

* First item with some text
* Second item with _emphasis_
* Third item with `inline code`
** Nested item one
** Nested item two

Ordered list:

. Step one
. Step two
. Step three

Checklist:

* [x] Completed item
* [ ] Pending item

== Tables

|===
| Name  | Age | City

| Alice | 30  | New York
| Bob   | 25  | London
| Carol | 35  | Tokyo
|===

== Code Blocks

[source,python]
----
def hello():
    print("Hello, world!")
    return 42
----

[source,rust]
----
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
----

== Blockquotes

[quote,Someone Famous]
____
This is a blockquote that spans
multiple lines of text.
____

== Images

image::path/to/image.png[Alt text,400,300]

== Admonitions

NOTE: This is a note admonition.

WARNING: This is a warning admonition.

TIP: This is a tip admonition.

== Sidebar

****
This is a sidebar block with additional content.
****

== Examples

====
This is an example block.
====

== Literal Blocks

....
This is literal text.
No markup is processed here.
....

== Footnotes

This text has a footnote.footnote:[This is the footnote text.]

== Inline Formatting

This paragraph has *bold*, _italic_, `code`, +passthrough+, ^super^, ~sub~,
and #highlighted# text.

== Links

External link: https://example.com[Visit Example]

Cross-reference: <<introduction>>

== More Content

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.
"#;

fn bench_asciidoc_parse_small(c: &mut Criterion) {
    c.bench_function("asciidoc_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_asciidoc_parse_medium(c: &mut Criterion) {
    c.bench_function("asciidoc_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_asciidoc_roundtrip_small(c: &mut Criterion) {
    c.bench_function("asciidoc_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_asciidoc_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("asciidoc_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_asciidoc_emit_medium(c: &mut Criterion) {
    c.bench_function("asciidoc_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_asciidoc_parse_small,
    bench_asciidoc_parse_medium,
    bench_asciidoc_roundtrip_small,
    bench_asciidoc_roundtrip_medium,
    bench_asciidoc_emit_medium,
);
criterion_main!(benches);
