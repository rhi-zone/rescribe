//! Pandoc oracle harness for ANSI.
//!
//! **Pandoc cannot read ANSI**, so there is no oracle comparison here.
//! This file provides a `parse_sample_no_panic` integration test only.

#[test]
fn parse_sample_no_panic() {
    let sample = b"\x1b[1mbold\x1b[0m \x1b[3mitalic\x1b[0m \x1b[4munderline\x1b[0m \
        \x1b[2mdim\x1b[0m \x1b[5mblink\x1b[0m \x1b[7mreverse\x1b[0m \
        \x1b[8mhidden\x1b[0m \x1b[9mstrike\x1b[0m \x1b[21mdouble-ul\x1b[0m \
        \x1b[53moverline\x1b[0m \
        \x1b[31mred\x1b[0m \x1b[91mbright-red\x1b[0m \
        \x1b[38;5;196m256-color\x1b[0m \x1b[38;2;255;128;0mtruecolor\x1b[0m \
        \x1b[41mred-bg\x1b[0m \x1b[101mbright-red-bg\x1b[0m \
        \x1b[48;5;196m256-bg\x1b[0m \x1b[48;2;255;128;0mtrue-bg\x1b[0m \
        \x1b[5A\x1b[3B\x1b[2C\x1b[4D\x1b[10;20H \
        \x1b[2J\x1b[K \
        \x1b]8;;https://example.com\x07Click\x1b]8;;\x07 \
        Hello\nWorld";
    let (doc, _) = ansi_fmt::parse(sample);
    assert!(!doc.nodes.is_empty());
}

#[test]
fn parse_adversarial_no_panic() {
    let inputs: &[&[u8]] = &[
        b"",
        b"\x1b",
        b"\x1b[",
        b"\x1b[1",
        b"\x1b[999999999m",
        b"\x1b[38;5m",
        b"\x1b[38;2m",
        b"\x1b]",
        b"\x1b]8;;\x07",
        b"\x1bX",
        b"\x1b\x1b\x1b",
        b"\x1b[?99z",
        b"\xff\xfe\xfd",
        b"\x1b[38;5;999m",
        b"\x1b[48;2;999;999;999m",
    ];
    for input in inputs {
        let _ = ansi_fmt::parse(input);
    }
}

#[test]
fn events_no_panic() {
    let inputs: &[&[u8]] = &[
        b"",
        b"\x1b",
        b"\x1b[",
        b"\x1b[1",
        b"\x1b[999999999m",
        b"\x1bX",
        b"\x1b\x1b\x1b",
        b"\x1b[?99z",
    ];
    for input in inputs {
        let _: Vec<_> = ansi_fmt::events(input).collect();
    }
}

#[test]
fn roundtrip_sample() {
    let sample = b"\x1b[1mBold\x1b[0m \x1b[31mRed\x1b[0m \x1b[38;5;196mPalette\x1b[0m plain";
    let (doc, _) = ansi_fmt::parse(sample);
    let emitted = ansi_fmt::emit(&doc);
    let (doc2, _) = ansi_fmt::parse(emitted.as_bytes());
    assert_eq!(ansi_fmt::collect_text(&doc), ansi_fmt::collect_text(&doc2));
}

#[test]
fn streaming_parser_roundtrip() {
    use ansi_fmt::batch::StreamingParser;
    use ansi_fmt::OwnedEvent;

    let input = b"\x1b[1mHello\x1b[0m \x1b[31mWorld\x1b[0m";
    let mut evs: Vec<OwnedEvent> = Vec::new();
    let mut p = StreamingParser::new(|ev| evs.push(ev));
    // Feed in small chunks to test boundary handling.
    for chunk in input.chunks(3) {
        p.feed(chunk);
    }
    p.finish();
    // When fed in small chunks, text may arrive fragmented.
    // Concatenate all text events and check the combined result.
    let text: String = evs
        .iter()
        .filter_map(|e| match e {
            OwnedEvent::Text { text, .. } => Some(text.as_ref()),
            _ => None,
        })
        .collect();
    assert!(text.contains("Hello"));
    assert!(text.contains("World"));
}

#[test]
fn writer_roundtrip() {
    use ansi_fmt::ast::Style;
    use ansi_fmt::writer::Writer;
    use ansi_fmt::OwnedEvent;

    let mut w = Writer::new(Vec::<u8>::new());
    let s = Style {
        bold: true,
        ..Style::default()
    };
    w.write_event(OwnedEvent::SetStyle(s.clone()));
    w.write_event(OwnedEvent::Text {
        text: "Hello".to_string().into(),
        style: s,
    });
    w.write_event(OwnedEvent::ResetStyle);
    let bytes = w.finish();
    let output = String::from_utf8_lossy(&bytes);
    assert!(output.contains("Hello"));
    assert!(output.contains("\x1b[1m"));
    assert!(output.contains("\x1b[0m"));
}
