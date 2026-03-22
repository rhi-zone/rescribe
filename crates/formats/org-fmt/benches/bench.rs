use criterion::{Criterion, criterion_group, criterion_main};
use org_fmt::{parse, build};

const SMALL: &str = r#"
* Hello World

This is a /short/ paragraph with =inline code= and a [[https://example.com][link]].

- Item one
- Item two
- Item three

#+BEGIN_SRC rust
fn main() {
    println!("hello");
}
#+END_SRC
"#;

const MEDIUM: &str = r#"
#+TITLE: Medium Document
#+AUTHOR: Test Author
#+DATE: 2024-01-01

* Introduction

This document tests the Org parser with a medium-sized input containing many constructs.
It includes *bold*, /italic/, =code=, ~verbatim~, and [[https://example.com][hyperlinks]].

* Lists

Unordered list:

- First item with some text
- Second item with /emphasis/
- Third item with =inline code=
  - Nested item one
  - Nested item two

Ordered list:

1. Step one
2. Step two
3. Step three

Definition list:

- Term :: A short definition of the term.
- Another term :: A longer definition with more detail.

* Tables

| Name  | Age | City     |
|-------+-----+----------|
| Alice | 30  | New York |
| Bob   | 25  | London   |
| Carol | 35  | Tokyo    |

* Code Blocks

#+BEGIN_SRC python
def hello():
    print("Hello, world!")
    return 42
#+END_SRC

#+BEGIN_SRC rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
#+END_SRC

* Blockquotes

#+BEGIN_QUOTE
This is a blockquote that spans
multiple lines of text.
#+END_QUOTE

* Headings with Metadata

** TODO [#A] Task with priority :work:important:
   SCHEDULED: <2024-01-15 Mon>
   DEADLINE: <2024-01-20 Sat>

** DONE Completed task :archived:

* Footnotes

This text has a footnote [fn:1].

[fn:1] This is the footnote text.

* Math

Inline math: $x^2 + y^2 = r^2$.

Block math:
\begin{equation}
E = mc^2
\end{equation}

* Emphasis Examples

This paragraph has *bold*, /italic/, _underline_, +strikethrough+, =code=, ~verbatim~,
and H_2O subscript and E=mc^2 superscript.

* Links and Images

Plain link: https://example.com

Image: [[file:image.png][Alt text]]

Internal link: [[#introduction]]

* More Content

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.
"#;

fn bench_org_parse_small(c: &mut Criterion) {
    c.bench_function("org_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_org_parse_medium(c: &mut Criterion) {
    c.bench_function("org_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_org_roundtrip_small(c: &mut Criterion) {
    c.bench_function("org_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_org_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("org_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_org_emit_medium(c: &mut Criterion) {
    c.bench_function("org_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_org_parse_small,
    bench_org_parse_medium,
    bench_org_roundtrip_small,
    bench_org_roundtrip_medium,
    bench_org_emit_medium,
);
criterion_main!(benches);
