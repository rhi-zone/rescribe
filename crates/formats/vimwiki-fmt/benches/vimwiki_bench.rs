use criterion::{Criterion, criterion_group, criterion_main};
use vimwiki_fmt::{build, parse};

const SMALL: &str = r#"
= Hello World =

This is a short paragraph with `inline code` and a [[https://example.com|link]].

* Item one
* Item two
* Item three

{{{rust
fn main() {
    println!("hello");
}
}}}
"#;

const MEDIUM: &str = r#"
= Introduction =

This document tests the VimWiki parser with a medium-sized input containing many constructs.
It includes *bold*, _italic_, `code`, ~~strikethrough~~, and [[https://example.com|hyperlinks]].

== Lists ==

Unordered list:

* First item with some text
* Second item with _emphasis_
* Third item with `inline code`

Ordered list:

1. Step one
2. Step two
3. Step three

== Tables ==

| Name  | Age | City     |
| Alice |  30 | New York |
| Bob   |  25 | London   |
| Carol |  35 | Tokyo    |

== Code Blocks ==

{{{python
def hello():
    print("Hello, world!")
    return 42
}}}

{{{rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
}}}

== Blockquotes ==

> This is a blockquote that spans
> multiple lines of text.

== Emphasis Examples ==

This paragraph has *bold*, _italic_, ~~strikethrough~~,
^superscript^, and ,,subscript,, text.

== Links and Images ==

[[https://example.com|Example]]

{{image.png|Alt text}}

== Definition List ==

; Term
: Definition of term.
; Another term
: Another definition with more detail.

== More Content ==

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.
"#;

fn bench_vimwiki_parse_small(c: &mut Criterion) {
    c.bench_function("vimwiki_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_vimwiki_parse_medium(c: &mut Criterion) {
    c.bench_function("vimwiki_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_vimwiki_roundtrip_small(c: &mut Criterion) {
    c.bench_function("vimwiki_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_vimwiki_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("vimwiki_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_vimwiki_emit_medium(c: &mut Criterion) {
    c.bench_function("vimwiki_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_vimwiki_parse_small,
    bench_vimwiki_parse_medium,
    bench_vimwiki_roundtrip_small,
    bench_vimwiki_roundtrip_medium,
    bench_vimwiki_emit_medium,
);
criterion_main!(benches);
