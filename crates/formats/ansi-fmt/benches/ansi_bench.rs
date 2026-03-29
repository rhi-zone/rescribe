use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn build_sample() -> Vec<u8> {
    let mut buf = Vec::new();
    for i in 0..=255u8 {
        buf.extend_from_slice(b"\x1b[38;5;");
        buf.extend_from_slice(i.to_string().as_bytes());
        buf.extend_from_slice(b"m");
        buf.extend_from_slice(b"Color text ");
    }
    buf.extend_from_slice(b"\x1b[0m");
    buf.extend_from_slice(b"\n\x1b[1mBold\x1b[0m \x1b[3mItalic\x1b[0m\n");
    buf.extend_from_slice(b"\x1b]8;;https://example.com\x07Link\x1b]8;;\x07\n");
    buf.extend_from_slice(b"\x1b[5A\x1b[2J\x1b[10;20H");
    buf
}

fn bench_parse(c: &mut Criterion) {
    let sample = build_sample();
    c.bench_function("ansi_parse", |b| {
        b.iter(|| {
            let (doc, _) = ansi_fmt::parse(black_box(&sample));
            black_box(doc);
        });
    });
}

fn bench_emit(c: &mut Criterion) {
    let sample = build_sample();
    let (doc, _) = ansi_fmt::parse(&sample);
    c.bench_function("ansi_emit", |b| {
        b.iter(|| {
            let out = ansi_fmt::emit(black_box(&doc));
            black_box(out);
        });
    });
}

fn bench_events(c: &mut Criterion) {
    let sample = build_sample();
    c.bench_function("ansi_events", |b| {
        b.iter(|| {
            let evs: Vec<_> = ansi_fmt::events(black_box(&sample)).collect();
            black_box(evs);
        });
    });
}

fn bench_strip(c: &mut Criterion) {
    let sample = build_sample();
    let sample_str = String::from_utf8_lossy(&sample).into_owned();
    c.bench_function("ansi_strip", |b| {
        b.iter(|| {
            let out = ansi_fmt::strip_ansi(black_box(&sample_str));
            black_box(out);
        });
    });
}

fn bench_streaming_parser(c: &mut Criterion) {
    let sample = build_sample();
    c.bench_function("ansi_streaming_parser", |b| {
        b.iter(|| {
            let mut count = 0usize;
            let mut p = ansi_fmt::StreamingParser::new(|_ev| count += 1);
            for chunk in sample.chunks(64) {
                p.feed(black_box(chunk));
            }
            p.finish();
            black_box(count);
        });
    });
}

fn bench_writer(c: &mut Criterion) {
    let sample = build_sample();
    let evs: Vec<_> = ansi_fmt::events(&sample)
        .map(|e| e.into_owned())
        .collect();
    c.bench_function("ansi_writer", |b| {
        b.iter(|| {
            let mut w = ansi_fmt::Writer::new(Vec::<u8>::with_capacity(sample.len()));
            for ev in &evs {
                w.write_event(ev.clone());
            }
            let out = w.finish();
            black_box(out);
        });
    });
}

criterion_group!(
    benches,
    bench_parse,
    bench_emit,
    bench_events,
    bench_strip,
    bench_streaming_parser,
    bench_writer
);
criterion_main!(benches);
