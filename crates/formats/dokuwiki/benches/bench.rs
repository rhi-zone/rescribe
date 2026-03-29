use criterion::{Criterion, criterion_group, criterion_main};
use dokuwiki::{build, parse};

const SMALL: &str = r#"
====== Hello World ======

This is a //short// paragraph with ''inline code'' and a [[https://example.com|link]].

  * Item one
  * Item two
  * Item three

<code rust>
fn main() {
    println!("hello");
}
</code>
"#;

const MEDIUM: &str = r#"
====== Introduction ======

This document tests the DokuWiki parser with a medium-sized input containing many constructs.
It includes **bold**, //italic//, ''code'', __underline__, and [[https://example.com|hyperlinks]].

===== Lists =====

Unordered list:

  * First item with some text
  * Second item with //emphasis//
  * Third item with ''inline code''
    * Nested item one
    * Nested item two

Ordered list:

  - Step one
  - Step two
  - Step three

===== Tables =====

^ Name  ^ Age ^ City     ^
| Alice | 30  | New York |
| Bob   | 25  | London   |
| Carol | 35  | Tokyo    |

===== Code Blocks =====

<code python>
def hello():
    print("Hello, world!")
    return 42
</code>

<code rust>
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
</code>

===== Blockquotes =====

> This is a blockquote that spans
> multiple lines of text.

===== Inline Formatting =====

This paragraph has **bold**, //italic//, __underline__, <del>strikethrough</del>,
''code'', and H<sub>2</sub>O subscript and E=mc<sup>2</sup> superscript.

===== Links and Images =====

External link: [[https://example.com|Example Site]]

Image: {{image.png|Alt text}}

===== Footnotes =====

This text has a footnote((This is the footnote text.)).

===== More Content =====

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.
"#;

fn bench_dokuwiki_parse_small(c: &mut Criterion) {
    c.bench_function("dokuwiki_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_dokuwiki_parse_medium(c: &mut Criterion) {
    c.bench_function("dokuwiki_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_dokuwiki_roundtrip_small(c: &mut Criterion) {
    c.bench_function("dokuwiki_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_dokuwiki_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("dokuwiki_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_dokuwiki_emit_medium(c: &mut Criterion) {
    c.bench_function("dokuwiki_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_dokuwiki_parse_small,
    bench_dokuwiki_parse_medium,
    bench_dokuwiki_roundtrip_small,
    bench_dokuwiki_roundtrip_medium,
    bench_dokuwiki_emit_medium,
);
criterion_main!(benches);
