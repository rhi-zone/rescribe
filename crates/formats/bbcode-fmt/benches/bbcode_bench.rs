use criterion::{Criterion, criterion_group, criterion_main};
use bbcode_fmt::{parse, emit, events};

const SMALL: &str = r#"
[h1]Hello World[/h1]

This is a [b]short[/b] paragraph with [code]inline code[/code] and a [url=https://example.com]link[/url].

[list]
[*]Item one
[*]Item two
[*]Item three
[/list]

[code]
fn main() {
    println!("hello");
}
[/code]
"#;

const MEDIUM: &str = r#"
[h1]Introduction[/h1]

This document tests the BBCode parser with a medium-sized input containing many constructs.
It includes [b]bold[/b], [i]italic[/i], [code]code[/code], and [url=https://example.com]hyperlinks[/url].

[h2]Lists[/h2]

Unordered list:

[list]
[*]First item with some text
[*]Second item with [i]emphasis[/i]
[*]Third item with [code]inline code[/code]
[/list]

Ordered list:

[list=1]
[*]First
[*]Second
[*]Third
[/list]

[h2]Quote[/h2]

[quote=Author]
This is a quoted paragraph with [b]bold[/b] text inside.
[/quote]

[h2]Table[/h2]

[table]
[tr][th]Header 1[/th][th]Header 2[/th][th]Header 3[/th][/tr]
[tr][td]Cell 1[/td][td]Cell 2[/td][td]Cell 3[/td][/tr]
[tr][td]Cell 4[/td][td]Cell 5[/td][td]Cell 6[/td][/tr]
[/table]

[h2]Formatting[/h2]

[color=red]Red text[/color] and [size=20]big text[/size] and [font=Courier]monospaced[/font].

[hr]

[center]
Centered paragraph.
[/center]

[spoiler]
This is hidden content.
[/spoiler]

[img]https://example.com/image.png[/img]
"#;

fn bench_parse(c: &mut Criterion) {
    c.bench_function("parse_small", |b| b.iter(|| parse(SMALL)));
    c.bench_function("parse_medium", |b| b.iter(|| parse(MEDIUM)));
}

fn bench_emit(c: &mut Criterion) {
    let (small_doc, _) = parse(SMALL);
    let (medium_doc, _) = parse(MEDIUM);
    c.bench_function("emit_small", |b| b.iter(|| emit(&small_doc)));
    c.bench_function("emit_medium", |b| b.iter(|| emit(&medium_doc)));
}

fn bench_events(c: &mut Criterion) {
    c.bench_function("events_small", |b| {
        b.iter(|| {
            let _: Vec<_> = events(SMALL).collect();
        })
    });
    c.bench_function("events_medium", |b| {
        b.iter(|| {
            let _: Vec<_> = events(MEDIUM).collect();
        })
    });
}

fn bench_roundtrip(c: &mut Criterion) {
    c.bench_function("roundtrip_medium", |b| {
        b.iter(|| {
            let (doc, _) = parse(MEDIUM);
            let text = emit(&doc);
            let (doc2, _) = parse(&text);
            let _ = emit(&doc2);
        })
    });
}

criterion_group!(benches, bench_parse, bench_emit, bench_events, bench_roundtrip);
criterion_main!(benches);
