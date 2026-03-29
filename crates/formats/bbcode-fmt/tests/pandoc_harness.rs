//! Pandoc oracle harness for BBCode.
//!
//! **Pandoc cannot read BBCode**, so there is no oracle comparison here.
//! This file provides a `parse_sample_no_panic` integration test only.

#[test]
fn parse_sample_no_panic() {
    let sample = r#"
[h1]Welcome[/h1]

[b]Bold[/b] and [i]italic[/i] text.

[quote=Author]
Quoted text here.
[/quote]

[list]
[*]Item one
[*]Item two
[/list]

[code=python]
print("hello")
[/code]

[table]
[tr][th]A[/th][th]B[/th][/tr]
[tr][td]1[/td][td]2[/td][/tr]
[/table]

[hr]

[center]
Centered
[/center]

[color=blue]Blue text[/color]
[size=20]Big text[/size]
[font=Arial]Arial text[/font]

[img=200x100]https://example.com/logo.png[/img]
[url=https://example.com]Link[/url]
[email]user@example.com[/email]

[spoiler]
Hidden
[/spoiler]

[noparse][b]literal[/b][/noparse]

[sub]sub[/sub] and [sup]sup[/sup]

[pre]preformatted[/pre]

[s]struck[/s]
"#;

    let (doc, _diags) = bbcode_fmt::parse(sample);
    assert!(
        doc.blocks.len() > 10,
        "Expected many blocks, got {}",
        doc.blocks.len()
    );

    // Emit shouldn't panic
    let output = bbcode_fmt::emit(&doc);
    assert!(!output.is_empty());

    // Events shouldn't panic
    let evts: Vec<_> = bbcode_fmt::events(sample).collect();
    assert!(!evts.is_empty());

    // Batch parser shouldn't panic
    let mut bp = bbcode_fmt::BatchParser::new();
    bp.feed(sample.as_bytes());
    let (doc2, _) = bp.finish();
    assert!(doc2.blocks.len() > 10);

    // Streaming parser shouldn't panic
    let mut sevts = Vec::new();
    let mut sp = bbcode_fmt::StreamingParser::new(|ev| sevts.push(ev));
    for chunk in sample.as_bytes().chunks(13) {
        sp.feed(chunk);
    }
    sp.finish();
    assert!(!sevts.is_empty());

    // Writer shouldn't panic
    let mut w = bbcode_fmt::Writer::new(Vec::<u8>::new());
    for ev in bbcode_fmt::events(sample) {
        w.write_event(ev.into_owned());
    }
    let bytes = w.finish();
    assert!(!bytes.is_empty());
}
