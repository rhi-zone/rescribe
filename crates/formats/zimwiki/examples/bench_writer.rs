//! Informal throughput benchmark for the streaming `Writer`, run manually
//! (not part of `cargo test`) to compare before/after the incremental
//! rewrite. Input construction happens outside the timed region, and
//! `std::hint::black_box` is applied to inputs, per
//! `docs/format-library-design.md`'s benchmarking convention.

use std::borrow::Cow;
use std::time::Instant;
use zimwiki::events::OwnedEvent;
use zimwiki::writer::Writer;

fn events_for(n: usize) -> Vec<OwnedEvent> {
    let mut evs = Vec::new();
    for i in 0..n {
        evs.push(OwnedEvent::StartHeading { level: 2 });
        evs.push(OwnedEvent::Text(Cow::Owned(format!("Section {i}"))));
        evs.push(OwnedEvent::EndHeading);
        evs.push(OwnedEvent::StartParagraph);
        evs.push(OwnedEvent::Text(Cow::Owned(
            "plain text before bold ".to_string(),
        )));
        evs.push(OwnedEvent::StartBold);
        evs.push(OwnedEvent::Text(Cow::Owned("bold text".to_string())));
        evs.push(OwnedEvent::EndBold);
        evs.push(OwnedEvent::Text(Cow::Owned(" and after".to_string())));
        evs.push(OwnedEvent::EndParagraph);
        evs.push(OwnedEvent::StartList { ordered: false });
        for j in 0..3 {
            evs.push(OwnedEvent::StartListItem { checked: None });
            evs.push(OwnedEvent::StartParagraph);
            evs.push(OwnedEvent::Text(Cow::Owned(format!("item {j} text"))));
            evs.push(OwnedEvent::EndParagraph);
            evs.push(OwnedEvent::EndListItem);
        }
        evs.push(OwnedEvent::EndList);
    }
    evs
}

fn main() {
    const SECTIONS: usize = 20_000;
    const ITERS: usize = 10;

    // Build all iterations' event vectors up front, outside the timed
    // region — `Event` has no `Clone` impl, so each iteration gets its own
    // freshly-built `Vec<OwnedEvent>` rather than cloning a shared one.
    let all_evs: Vec<Vec<OwnedEvent>> = (0..ITERS).map(|_| events_for(SECTIONS)).collect();
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
        "zimwiki Writer: {ITERS} iters x {SECTIONS} sections, {:.3}s total, {:.0} ns/iter, {:.2} MB/s",
        elapsed.as_secs_f64(),
        elapsed.as_nanos() as f64 / ITERS as f64,
        (total_bytes as f64 / 1_000_000.0) / elapsed.as_secs_f64()
    );
}
