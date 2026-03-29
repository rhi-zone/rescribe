use criterion::{Criterion, criterion_group, criterion_main};
use zimwiki::{parse, build};

const SMALL: &str = r#"
====== Hello World ======

This is a //short// paragraph with ''inline code'' and a [[https://example.com|link]].

* Item one
* Item two
* Item three

'''
fn main() {
    println!("hello");
}
'''
"#;

const MEDIUM: &str = r#"
====== Introduction ======

This document tests the ZimWiki parser with a medium-sized input containing many constructs.
It includes **bold**, //italic//, ''code'', ~~strikethrough~~, and [[https://example.com|hyperlinks]].

===== Lists =====

Unordered list:

* First item with some text
* Second item with //emphasis//
* Third item with ''inline code''

Ordered list:

1. Step one
2. Step two
3. Step three

===== Tables =====

| Name  | Age | City     |
| Alice | 30  | New York |
| Bob   | 25  | London   |
| Carol | 35  | Tokyo    |

===== Code Blocks =====

'''
def hello():
    print("Hello, world!")
    return 42
'''

'''
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
'''

===== Blockquotes =====

> This is a blockquote that spans
> multiple lines of text.

===== More Content =====

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.

E = mc^{2} and H_{2}O are common formulas.

__Underlined text__ and ~~strikethrough text~~ are also supported.

[ ] Unchecked item
[*] Checked item
[x] Another checked item
"#;

fn bench_zimwiki_parse_small(c: &mut Criterion) {
    c.bench_function("zimwiki_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_zimwiki_parse_medium(c: &mut Criterion) {
    c.bench_function("zimwiki_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_zimwiki_roundtrip_small(c: &mut Criterion) {
    c.bench_function("zimwiki_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_zimwiki_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("zimwiki_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_zimwiki_emit_medium(c: &mut Criterion) {
    c.bench_function("zimwiki_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_zimwiki_parse_small,
    bench_zimwiki_parse_medium,
    bench_zimwiki_roundtrip_small,
    bench_zimwiki_roundtrip_medium,
    bench_zimwiki_emit_medium,
);
criterion_main!(benches);
