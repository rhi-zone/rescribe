use criterion::{Criterion, criterion_group, criterion_main};
use t2t::{emit, parse};

const SMALL: &str = r#"= Hello World =

This is a *short* paragraph with ``inline code`` and a [link http://example.com].

- Item one
- Item two
- Item three

```
fn main() {
    println!("hello");
}
```
"#;

const MEDIUM: &str = r#"= Document Title =

== Introduction ==

This document tests the txt2tags parser with a medium-sized input containing many constructs.
It includes **bold**, //italic//, ``code``, and [hyperlinks http://example.com].

== Lists ==

Unordered list:

- First item with some text
- Second item with //emphasis//
- Third item with ``inline code``

Ordered list:

+ Step one
+ Step two
+ Step three

== Tables ==

|| Header 1 | Header 2 | Header 3 |
| Cell A | Cell B | Cell C |
| Cell D | Cell E | Cell F |

== Code Blocks ==

```
def hello():
    print("Hello, world!")
    return 42
```

```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

== Blockquotes ==

	This is a blockquote that spans some text.

== Text Formatting ==

This paragraph has **bold**, //italic//, __underline__, --strikethrough--, ``code``.

== Links and Images ==

Plain link: [Example Site http://example.com]

Image: [image.png]

== Definition List ==

: Term One
The definition of term one.

: Term Two
A longer definition with more detail.

== More Content ==

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.

=== Nested Headings ===

Content under a level-3 heading.

==== Even Deeper ====

Content under a level-4 heading.
"#;

fn bench_t2t_parse_small(c: &mut Criterion) {
    c.bench_function("t2t_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_t2t_parse_medium(c: &mut Criterion) {
    c.bench_function("t2t_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_t2t_emit_medium(c: &mut Criterion) {
    c.bench_function("t2t_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = emit(std::hint::black_box(&doc));
        });
    });
}

fn bench_t2t_roundtrip_small(c: &mut Criterion) {
    c.bench_function("t2t_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = emit(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_t2t_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("t2t_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = emit(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

criterion_group!(
    benches,
    bench_t2t_parse_small,
    bench_t2t_parse_medium,
    bench_t2t_emit_medium,
    bench_t2t_roundtrip_small,
    bench_t2t_roundtrip_medium,
);
criterion_main!(benches);
