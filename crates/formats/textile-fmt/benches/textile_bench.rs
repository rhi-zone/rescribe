use criterion::{Criterion, criterion_group, criterion_main};
use textile_fmt::{emit, parse};

const SMALL: &str = r#"h1. Hello World

This is a *short* paragraph with @inline code@ and a "link":https://example.com.

* Item one
* Item two
* Item three

bc. fn main() {
    println!("hello");
}
"#;

const MEDIUM: &str = r#"h1. Document Title

h2. Introduction

This document tests the Textile parser with a medium-sized input containing many constructs.
It includes *bold*, _italic_, @code@, and "hyperlinks":https://example.com.

h2. Lists

Unordered list:

* First item with some text
* Second item with _emphasis_
* Third item with @inline code@

Ordered list:

# Step one
# Step two
# Step three

h2. Tables

|_. Header 1|_. Header 2|_. Header 3|
| Cell A    | Cell B    | Cell C    |
| Cell D    | Cell E    | Cell F    |

h2. Code Blocks

bc. def hello():
    print("Hello, world!")
    return 42

bc. pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

h2. Blockquotes

bq. This is a blockquote that spans
multiple lines of text.

h2. Text Formatting

This paragraph has *bold*, _italic_, +underline+, -strikethrough-, @code@,
H~2~O subscript and E=mc^2^ superscript.

h2. Links and Images

Plain link: "Example Site":https://example.com

Image: !image.png(An example image)!

Image with link: !"click here":https://example.com!image.png!

h2. Definition List

- term := definition of the term
- another term := a longer definition with more detail

h2. Footnotes

This text has a footnote[1].

fn1. This is the footnote text.

h2. Block Attributes

p(myclass). A paragraph with a CSS class.

p{color:red}. A paragraph with inline style.

h2. More Content

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.

h3. Nested Headings

Content under a level-3 heading.

h4. Even Deeper

Content under a level-4 heading.

h2. Inline Span

A paragraph with *(myclass)classed bold* and _(highlight)italic span_ text.
"#;

fn bench_textile_parse_small(c: &mut Criterion) {
    c.bench_function("textile_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_textile_parse_medium(c: &mut Criterion) {
    c.bench_function("textile_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_textile_emit_medium(c: &mut Criterion) {
    c.bench_function("textile_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = emit(std::hint::black_box(&doc));
        });
    });
}

fn bench_textile_roundtrip_small(c: &mut Criterion) {
    c.bench_function("textile_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = emit(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_textile_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("textile_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = emit(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

criterion_group!(
    benches,
    bench_textile_parse_small,
    bench_textile_parse_medium,
    bench_textile_emit_medium,
    bench_textile_roundtrip_small,
    bench_textile_roundtrip_medium,
);
criterion_main!(benches);
