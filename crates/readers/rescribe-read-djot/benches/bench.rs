use criterion::{Criterion, criterion_group, criterion_main};
use rescribe_read_djot::parse;
use rescribe_write_djot::emit;

const SMALL: &str = r#"# Hello World

This is a _short_ paragraph with `inline code` and a [link](https://example.com).

- Item one
- Item two
- Item three

``` rust
fn main() {
    println!("hello");
}
```
"#;

const MEDIUM: &str = r#"# Document Title

## Introduction

This document tests the Djot parser with a medium-sized input containing many constructs.
It includes *strong*, _emphasis_, `code`, and [hyperlinks](https://example.com).

## Lists

Unordered list:

- First item with some text
- Second item with _emphasis_
- Third item with `inline code`

Ordered list:

1. Step one
2. Step two
3. Step three

Definition list:

: term
  A short definition of the term.

: another term
  A longer definition with more detail.

## Tables

| Header 1 | Header 2 | Header 3 |
|----------|----------|----------|
| Cell A   | Cell B   | Cell C   |
| Cell D   | Cell E   | Cell F   |

## Code Blocks

``` python
def hello():
    print("Hello, world!")
    return 42
```

``` rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

## Blockquotes

> This is a blockquote that spans
> multiple lines of text.

## Footnotes

This text has a footnote.[^note]

[^note]: This is the footnote text.

## Math

Inline math: $x^2 + y^2 = r^2$

Display math:
$$x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$$

## More Content

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Smart punctuation: "Hello, World!" and 'it\'s fine' --- em dash -- en dash.
"#;

fn bench_djot_parse_small(c: &mut Criterion) {
    c.bench_function("djot_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_djot_parse_medium(c: &mut Criterion) {
    c.bench_function("djot_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_djot_roundtrip_small(c: &mut Criterion) {
    c.bench_function("djot_roundtrip_small", |b| {
        let doc = parse(SMALL).expect("parse ok").value;
        b.iter(|| {
            let out = emit(std::hint::black_box(&doc)).expect("emit ok").value;
            let _ = parse(std::hint::black_box(std::str::from_utf8(&out).unwrap()));
        });
    });
}

fn bench_djot_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("djot_roundtrip_medium", |b| {
        let doc = parse(MEDIUM).expect("parse ok").value;
        b.iter(|| {
            let out = emit(std::hint::black_box(&doc)).expect("emit ok").value;
            let _ = parse(std::hint::black_box(std::str::from_utf8(&out).unwrap()));
        });
    });
}

fn bench_djot_emit_medium(c: &mut Criterion) {
    c.bench_function("djot_emit_medium", |b| {
        let doc = parse(MEDIUM).expect("parse ok").value;
        b.iter(|| {
            let _ = emit(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_djot_parse_small,
    bench_djot_parse_medium,
    bench_djot_roundtrip_small,
    bench_djot_roundtrip_medium,
    bench_djot_emit_medium,
);
criterion_main!(benches);
