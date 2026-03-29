use criterion::{Criterion, criterion_group, criterion_main};
use mediawiki_fmt::{parse, emit};

const SMALL: &str = r#"
== Hello World ==

This is a ''short'' paragraph with <code>inline code</code> and a [https://example.com link].

* Item one
* Item two
* Item three

 fn main() {
     println!("hello");
 }
"#;

const MEDIUM: &str = r#"
= Introduction =

This document tests the MediaWiki parser with a medium-sized input containing many constructs.
It includes '''bold''', ''italic'', <code>code</code>, <u>underline</u>, and [https://example.com hyperlinks].

== Lists ==

Unordered list:

* First item with some text
* Second item with ''emphasis''
* Third item with <code>inline code</code>

Ordered list:

# Step one
# Step two
# Step three

Definition list:

; Term
: A short definition of the term.
; Another term
: A longer definition with more detail.

== Tables ==

{|
! Name !! Age !! City
|-
| Alice || 30 || New York
|-
| Bob || 25 || London
|-
| Carol || 35 || Tokyo
|}

== Code Blocks ==

<syntaxhighlight lang="python">
def hello():
    print("Hello, world!")
    return 42
</syntaxhighlight>

== Blockquotes ==

<blockquote>
This is a blockquote that spans
multiple lines of text.
</blockquote>

== Footnotes ==

This text has a footnote<ref>This is the footnote text.</ref>.

== Math ==

Inline math: <math>x^2 + y^2 = r^2</math>.

== Emphasis Examples ==

This paragraph has '''bold''', ''italic'', <u>underline</u>, <s>strikethrough</s>,
<code>code</code>, and H<sub>2</sub>O subscript and E=mc<sup>2</sup> superscript.

== Links and Images ==

Internal link: [[Main Page]]

Image: [[File:image.png|Alt text]]

External: [https://example.com Example site]

== More Content ==

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.
"#;

fn bench_mediawiki_parse_small(c: &mut Criterion) {
    c.bench_function("mediawiki_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_mediawiki_parse_medium(c: &mut Criterion) {
    c.bench_function("mediawiki_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_mediawiki_roundtrip_small(c: &mut Criterion) {
    c.bench_function("mediawiki_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = emit(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_mediawiki_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("mediawiki_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = emit(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_mediawiki_emit_medium(c: &mut Criterion) {
    c.bench_function("mediawiki_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = emit(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_mediawiki_parse_small,
    bench_mediawiki_parse_medium,
    bench_mediawiki_roundtrip_small,
    bench_mediawiki_roundtrip_medium,
    bench_mediawiki_emit_medium,
);
criterion_main!(benches);
