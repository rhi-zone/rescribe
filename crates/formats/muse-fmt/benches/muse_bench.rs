use criterion::{Criterion, criterion_group, criterion_main};
use muse_fmt::{build, parse};

const SMALL: &str = r#"* Hello World

This is a *short* paragraph with **bold** and =inline code= and a [[https://example.com][link]].

 - Item one
 - Item two
 - Item three

<example>
fn main() {
    println!("hello");
}
</example>
"#;

const MEDIUM: &str = r#"#title Document Title
#author Test Author

* Introduction

This document tests the Muse parser with a medium-sized input containing many constructs.
It includes **bold**, *italic*, =code=, and [[https://example.com][hyperlinks]].

** Lists

Unordered list:

 - First item with some text
 - Second item with *emphasis*
 - Third item with =inline code=

Ordered list:

 1. Step one
 2. Step two
 3. Step three

** Tables

|| Header 1 || Header 2 || Header 3 ||
| Cell A    | Cell B    | Cell C    |
| Cell D    | Cell E    | Cell F    |

** Code Blocks

<example>
def hello():
    print("Hello, world!")
    return 42
</example>

<src lang="rust">
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
</src>

** Blockquotes

<blockquote>
This is a blockquote that spans multiple lines of text.
</blockquote>

** Text Formatting

This paragraph has **bold**, *italic*, _underline_, ~~strikethrough~~, =code=,
H<sub>2</sub>O subscript and x^2^ superscript.

** Links and Images

Plain link: [[https://example.com][Example Site]]

Image: [[photo.png]]

** Definition Lists

 term :: definition of the term
 another term :: a longer definition with more detail

** Footnotes

This text has a footnote[1].

[1] This is the footnote text.

** Centered and Right Blocks

<center>
Centered content here.
</center>

<right>
Right-aligned content.
</right>

** Verse

<verse>
Line one of the verse
Line two of the verse
</verse>

** Comments

;; This is a comment line

** More Content

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.

*** Nested Headings

Content under a level-3 heading.

**** Even Deeper

Content under a level-4 heading.

** Anchors and Line Breaks

<anchor intro>This paragraph has an anchor.

A paragraph with a hard<br>line break in it.

----
"#;

fn bench_muse_parse_small(c: &mut Criterion) {
    c.bench_function("muse_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_muse_parse_medium(c: &mut Criterion) {
    c.bench_function("muse_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_muse_emit_medium(c: &mut Criterion) {
    c.bench_function("muse_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_muse_parse_small,
    bench_muse_parse_medium,
    bench_muse_emit_medium,
);
criterion_main!(benches);
