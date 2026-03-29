use criterion::{criterion_group, criterion_main, Criterion};
use jira_fmt::{build, parse};

const SMALL: &str = "h1. Hello World\n\n\
This is a _short_ paragraph with {{inline code}} and a [link|https://example.com].\n\n\
* Item one\n\
* Item two\n\
* Item three\n\n\
{code:java}\n\
public class Main {\n\
    public static void main(String[] args) {\n\
        System.out.println(\"hello\");\n\
    }\n\
}\n\
{code}\n";

const MEDIUM: &str = "h1. Introduction\n\n\
This document tests the Jira parser with a medium-sized input.\n\
It includes *bold*, _italic_, {{code}}, +underline+, and [links|https://example.com].\n\n\
h2. Lists\n\n\
* First item with some text\n\
* Second item with _emphasis_\n\
* Third item with {{inline code}}\n\n\
# Step one\n\
# Step two\n\
# Step three\n\n\
h2. Tables\n\n\
||Name||Age||City||\n\
|Alice|30|New York|\n\
|Bob|25|London|\n\
|Carol|35|Tokyo|\n\n\
h2. Code Blocks\n\n\
{code:rust}\n\
pub fn add(a: i32, b: i32) -> i32 {\n\
    a + b\n\
}\n\
{code}\n\n\
h2. Blockquotes\n\n\
{quote}\n\
This is a blockquote that spans multiple lines.\n\
{quote}\n\n\
h2. Emphasis Examples\n\n\
This paragraph has *bold*, _italic_, +underline+, -strikethrough-,\n\
^superscript^, ~subscript~, and {{monospace}} text.\n\n\
----\n\n\
h2. More Content\n\n\
Additional paragraphs to make the document longer.\n";

fn bench_jira_parse_small(c: &mut Criterion) {
    c.bench_function("jira_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_jira_parse_medium(c: &mut Criterion) {
    c.bench_function("jira_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_jira_roundtrip_small(c: &mut Criterion) {
    c.bench_function("jira_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_jira_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("jira_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = build(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_jira_emit_medium(c: &mut Criterion) {
    c.bench_function("jira_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = build(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_jira_parse_small,
    bench_jira_parse_medium,
    bench_jira_roundtrip_small,
    bench_jira_roundtrip_medium,
    bench_jira_emit_medium,
);
criterion_main!(benches);
