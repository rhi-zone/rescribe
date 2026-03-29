use criterion::{Criterion, criterion_group, criterion_main};
use texinfo::{emit, parse};

const SMALL: &str = r#"
@chapter Hello World

This is a /short/ paragraph with @code{inline code} and a @uref{https://example.com, link}.

@itemize
@item Item one
@item Item two
@item Item three
@end itemize

@example
fn main() {
    println!("hello");
}
@end example
"#;

const MEDIUM: &str = r#"
@settitle Medium Document

@chapter Introduction

This document tests the Texinfo parser with a medium-sized input containing many constructs.
It includes @strong{bold}, @emph{italic}, @code{code}, and @uref{https://example.com, hyperlinks}.

@chapter Lists

Unordered list:

@itemize
@item First item with some text
@item Second item with @emph{emphasis}
@item Third item with @code{inline code}
@end itemize

Ordered list:

@enumerate
@item Step one
@item Step two
@item Step three
@end enumerate

Definition list:

@table @asis
@item Term
A short definition of the term.
@item Another term
A longer definition with more detail.
@end table

@chapter Tables

@multitable @columnfractions .33 .33 .33
@headitem Name @tab Age @tab City
@item Alice @tab 30 @tab New York
@item Bob @tab 25 @tab London
@item Carol @tab 35 @tab Tokyo
@end multitable

@chapter Code Blocks

@example
def hello():
    print("Hello, world!")
    return 42
@end example

@verbatim
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
@end verbatim

@chapter Blockquotes

@quotation
This is a blockquote that spans
multiple lines of text.
@end quotation

@chapter Footnotes

This text has a footnote@footnote{This is the footnote text.}.

@chapter Inline Markup

This paragraph has @strong{bold}, @emph{italic}, @code{code},
@file{filename}, @command{cmd}, @option{-v}, @env{PATH},
@samp{sample}, @kbd{C-c}, @key{RET}, @var{variable},
@dfn{defined term}, @acronym{GNU}, and @dots{} symbols.

@chapter Cross References

See @xref{Introduction} for details.
Also @ref{Lists} and @pxref{Tables}.

@chapter More Content

Additional paragraphs to make the document longer. The parser should handle
this efficiently without any performance degradation as the document grows.

Another paragraph with more text to exercise the parser across multiple
blocks of content.
"#;

fn bench_texinfo_parse_small(c: &mut Criterion) {
    c.bench_function("texinfo_parse_small", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(SMALL));
        });
    });
}

fn bench_texinfo_parse_medium(c: &mut Criterion) {
    c.bench_function("texinfo_parse_medium", |b| {
        b.iter(|| {
            let _ = parse(std::hint::black_box(MEDIUM));
        });
    });
}

fn bench_texinfo_roundtrip_small(c: &mut Criterion) {
    c.bench_function("texinfo_roundtrip_small", |b| {
        let (doc, _) = parse(SMALL);
        b.iter(|| {
            let out = emit(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_texinfo_roundtrip_medium(c: &mut Criterion) {
    c.bench_function("texinfo_roundtrip_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let out = emit(std::hint::black_box(&doc));
            let _ = parse(std::hint::black_box(&out));
        });
    });
}

fn bench_texinfo_emit_medium(c: &mut Criterion) {
    c.bench_function("texinfo_emit_medium", |b| {
        let (doc, _) = parse(MEDIUM);
        b.iter(|| {
            let _ = emit(std::hint::black_box(&doc));
        });
    });
}

criterion_group!(
    benches,
    bench_texinfo_parse_small,
    bench_texinfo_parse_medium,
    bench_texinfo_roundtrip_small,
    bench_texinfo_roundtrip_medium,
    bench_texinfo_emit_medium,
);
criterion_main!(benches);
