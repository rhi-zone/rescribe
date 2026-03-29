use criterion::{Criterion, criterion_group, criterion_main};
use creole::{parse, build};

const SMALL: &str = r#"
= Hello World

This is a //short// paragraph with {{{inline code}}} and a [[https://example.com|link]].

* Item one
* Item two
* Item three

{{{
fn main() {
    println!("hello");
}
}}}
"#;

const MEDIUM: &str = r#"
= Introduction

This document tests the Creole parser with a medium-sized input containing many constructs.
It includes **bold**, //italic//, {{{code}}}, and [[https://example.com|hyperlinks]].

== Lists

Unordered list:

* First item with some text
* Second item with //emphasis//
* Third item with {{{inline code}}}
** Nested item one
** Nested item two

Ordered list:

# Step one
# Step two
# Step three

; Term
: A short definition of the term.
; Another term
: A longer definition with more detail.

== Tables

|= Name |= Age |= City |
| Alice | 30 | New York |
| Bob | 25 | London |
| Carol | 35 | Tokyo |

== Code Blocks

{{{
def hello():
    print("Hello, world!")
    return 42
}}}

{{{
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
}}}

== Blockquotes

> This is a blockquote that spans
> multiple lines of text.

== Emphasis Examples

This paragraph has **bold**, //italic//, {{{code}}},
and a forced line break here\\and here.

== Links and Images

See [[https://example.com|Example Site]] for details.

{{image.png|Alt text}}

== More Content

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.

----

Final paragraph after the horizontal rule.
"#;

fn bench_creole_parse_small(c: &mut Criterion) {
    c.bench_function("creole_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_creole_parse_medium(c: &mut Criterion) {
    c.bench_function("creole_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_creole_emit_medium(c: &mut Criterion) {
    c.bench_function("creole_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_creole_parse_small,
    bench_creole_parse_medium,
    bench_creole_emit_medium,
);
criterion_main!(benches);
