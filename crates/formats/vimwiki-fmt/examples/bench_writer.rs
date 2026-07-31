//! Informal throughput benchmark for the streaming `Writer`, run manually
//! (not part of `cargo test`) to compare before/after the incremental
//! rewrite. Input construction happens outside the timed region, and
//! `std::hint::black_box` is applied to inputs, per
//! `docs/format-library-design.md`'s benchmarking convention.

use std::borrow::Cow;
use std::time::Instant;
use vimwiki_fmt::events::Event;
use vimwiki_fmt::writer::Writer;

fn events_for(n: usize) -> Vec<Event<'static>> {
    let mut evs = Vec::new();
    for i in 0..n {
        evs.push(Event::StartHeading { level: 2 });
        evs.push(Event::Text(Cow::Owned(format!("Section {i}"))));
        evs.push(Event::EndHeading);
        evs.push(Event::StartParagraph);
        evs.push(Event::Text(Cow::Owned(
            "plain text before bold ".to_string(),
        )));
        evs.push(Event::StartBold);
        evs.push(Event::Text(Cow::Owned("bold text".to_string())));
        evs.push(Event::EndBold);
        evs.push(Event::Text(Cow::Owned(" and after".to_string())));
        evs.push(Event::EndParagraph);
        evs.push(Event::StartList { ordered: false });
        for j in 0..3 {
            evs.push(Event::StartListItem { checked: None });
            evs.push(Event::Text(Cow::Owned(format!("item {j} text"))));
            evs.push(Event::EndListItem);
        }
        evs.push(Event::EndList);
    }
    evs
}

fn main() {
    const SECTIONS: usize = 20_000;
    const ITERS: usize = 10;

    let all_evs: Vec<Vec<Event<'static>>> = (0..ITERS).map(|_| events_for(SECTIONS)).collect();
    std::hint::black_box(&all_evs);

    let mut total_bytes = 0usize;
    let start = Instant::now();
    for evs in all_evs {
        let mut w = Writer::new(Vec::<u8>::new());
        for e in evs {
            w.write_event(e);
        }
        let out = w.finish();
        total_bytes += out.len();
        std::hint::black_box(out);
    }
    let elapsed = start.elapsed();

    println!(
        "vimwiki-fmt Writer: {ITERS} iters x {SECTIONS} sections, {:.3}s total, {:.0} ns/iter, {:.2} MB/s",
        elapsed.as_secs_f64(),
        elapsed.as_nanos() as f64 / ITERS as f64,
        (total_bytes as f64 / 1_000_000.0) / elapsed.as_secs_f64()
    );
}
