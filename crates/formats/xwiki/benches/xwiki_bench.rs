use criterion::{Criterion, criterion_group, criterion_main};
use xwiki::{parse, build};

const SMALL: &str = r#"
= Hello World =

This is a //short// paragraph with ##inline code## and a [[click here>>https://example.com]].

* Item one
* Item two
* Item three

{{code language="rust"}}
fn main() {
    println!("hello");
}
{{/code}}
"#;

const MEDIUM: &str = r#"
= Introduction =

This document tests the XWiki parser with a medium-sized input containing many constructs.
It includes **bold**, //italic//, ##code##, __underline__, and [[hyperlinks>>https://example.com]].

== Lists ==

Unordered list:

* First item with some text
* Second item with //emphasis//
* Third item with ##inline code##

Ordered list:

1. Step one
1. Step two
1. Step three

== Tables ==

|=Name|=Age|=City|
|Alice|30|New York|
|Bob|25|London|
|Carol|35|Tokyo|

== Code Blocks ==

{{code language="python"}}
def hello():
    print("Hello, World!")

class Example:
    def __init__(self):
        self.value = 42
{{/code}}

== Formatting ==

This paragraph has **bold text** and //italic text// and __underlined text__
and --strikethrough text-- and ^^superscript^^ and ~~subscript~~ formatting.

== Links and Images ==

Visit [[XWiki>>https://www.xwiki.org]] for more information.

[[image:logo.png||alt="XWiki Logo"]]

----

== Blockquote ==

{{quote}}
This is a quoted block of text.
It can span multiple lines.
{{/quote}}

== Macros ==

{{info}}
This is an information message.
{{/info}}

{{warning}}
This is a warning message.
{{/warning}}

{{toc/}}
"#;

fn bench_parse_small(c: &mut Criterion) {
    c.bench_function("xwiki_parse_small", |b| {
        b.iter(|| parse(SMALL));
    });
}

fn bench_parse_medium(c: &mut Criterion) {
    c.bench_function("xwiki_parse_medium", |b| {
        b.iter(|| parse(MEDIUM));
    });
}

fn bench_build_small(c: &mut Criterion) {
    let (doc, _) = parse(SMALL);
    c.bench_function("xwiki_build_small", |b| {
        b.iter(|| build(&doc));
    });
}

fn bench_build_medium(c: &mut Criterion) {
    let (doc, _) = parse(MEDIUM);
    c.bench_function("xwiki_build_medium", |b| {
        b.iter(|| build(&doc));
    });
}

fn bench_events_medium(c: &mut Criterion) {
    let (doc, _) = parse(MEDIUM);
    c.bench_function("xwiki_events_medium", |b| {
        b.iter(|| {
            let _: Vec<_> = xwiki::events::events(&doc).collect();
        });
    });
}

fn bench_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("xwiki_roundtrip_medium", |b| {
        b.iter(|| {
            let (doc, _) = parse(MEDIUM);
            let text = build(&doc);
            let (doc2, _) = parse(&text);
            let _ = build(&doc2);
        });
    });
}

criterion_group!(
    benches,
    bench_parse_small,
    bench_parse_medium,
    bench_build_small,
    bench_build_medium,
    bench_events_medium,
    bench_roundtrip_medium,
);
criterion_main!(benches);
