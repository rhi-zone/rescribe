use criterion::{Criterion, criterion_group, criterion_main};
use markua::{emit, parse};

const SMALL: &str = r#"# Hello World

This is a *short* paragraph with `inline code` and a [link](https://example.com).

- Item one
- Item two
- Item three

```
fn main() {
    println!("hello");
}
```
"#;

const MEDIUM: &str = r#"# Document Title

## Introduction

This document tests the Markua parser with a medium-sized input containing many constructs.
It includes **bold**, *italic*, `code`, and [hyperlinks](https://example.com).

## Lists

Unordered list:

- First item with some text
- Second item with *emphasis*
- Third item with `inline code`

Ordered list:

1. Step one
2. Step two
3. Step three

## Tables

| Header 1 | Header 2 | Header 3 |
| --- | --- | --- |
| Cell A | Cell B | Cell C |
| Cell D | Cell E | Cell F |

## Code Blocks

```python
def hello():
    print("Hello, world!")
    return 42
```

```rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

## Blockquotes

> This is a blockquote that spans
> multiple lines of text.

## Text Formatting

This paragraph has **bold**, *italic*, ~~strikethrough~~, `code`,
H~2~O subscript and x^2^ superscript.

## Special Blocks

W> This is a warning block with some content.

A> This is an aside with some content.

T> This is a tip with some content.

## Math and Footnotes

The equation $E = mc^2$ is famous^[Einstein's mass-energy equivalence].

## Page Break

{pagebreak}

## More Content

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.

### Nested Headings

Content under a level-3 heading.

#### Even Deeper

Content under a level-4 heading.

## Definition List

Term One
: The first definition.

Term Two
: The second definition.

## Index Terms

See i[Markua] and i[Leanpub] for more information.
"#;

fn bench_markua_parse_small(c: &mut Criterion) {
    c.bench_function("markua_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_markua_parse_medium(c: &mut Criterion) {
    c.bench_function("markua_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_markua_emit_medium(c: &mut Criterion) {
    c.bench_function("markua_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = emit(std::hint::black_box(&doc));
        });
    });
}

fn bench_markua_roundtrip_small(c: &mut Criterion) {
    c.bench_function("markua_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = emit(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_markua_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("markua_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = emit(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

criterion_group!(
    benches,
    bench_markua_parse_small,
    bench_markua_parse_medium,
    bench_markua_emit_medium,
    bench_markua_roundtrip_small,
    bench_markua_roundtrip_medium,
);
criterion_main!(benches);
