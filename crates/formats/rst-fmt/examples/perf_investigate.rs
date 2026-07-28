//! Ad-hoc performance harness. NOT for commit — deleted after use.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

struct CountingAlloc;
static CUR: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let cur = CUR.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(cur, Ordering::Relaxed);
            ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        CUR.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

fn reset_counters() {
    PEAK.store(CUR.load(Ordering::Relaxed), Ordering::Relaxed);
    ALLOCS.store(0, Ordering::Relaxed);
}

fn peak_and_allocs() -> (usize, usize) {
    (PEAK.load(Ordering::Relaxed), ALLOCS.load(Ordering::Relaxed))
}

fn gen_doc(sections: usize) -> String {
    let mut s = String::new();
    for i in 0..sections {
        s.push_str(&format!("Section {i}\n{}\n\n", "=".repeat(20)));
        s.push_str("This is a **bold** paragraph with *italic*, ``code``, and a `link <https://example.com/page>`_ plus some plain text to pad it out a bit more so blocks are non-trivial in size.\n\n");
        s.push_str(
            "- item one with some text\n- item two with *emphasis*\n- item three with ``code``\n\n",
        );
        s.push_str(&format!(
            "See footnote [{i}]_.\n\n.. [{i}] Footnote body text number {i}.\n\n"
        ));
        s.push_str(".. code-block:: rust\n\n   fn f() {\n       println!(\"x\");\n   }\n\n");
        s.push_str("| Line one\n| Line two\n\n");
        s.push_str("+-----+-----+\n| A   | B   |\n+=====+=====+\n| C   | D   |\n+-----+-----+\n\n");
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("all");

    if mode == "all" || mode == "writer" {
        println!(
            "== Writer: build() (builder) vs streaming Writer, both from materialized input =="
        );
        println!("(writer numbers are net of the per-iteration event-clone baseline)");
        for sections in [50, 500, 2000] {
            let doc_src = gen_doc(sections);
            let ast = rst_fmt::parse(&doc_src).unwrap();
            let events: Vec<rst_fmt::OwnedEvent> =
                rst_fmt::events(&doc_src).map(|e| e.into_owned()).collect();
            let n = 30;

            // best-of-N timing, allocation count from a single clean pass.
            let mut build_time = f64::MAX;
            for _ in 0..n {
                let t0 = Instant::now();
                let out = rst_fmt::build(std::hint::black_box(&ast));
                let e = t0.elapsed().as_secs_f64();
                std::hint::black_box(out);
                build_time = build_time.min(e);
            }
            reset_counters();
            {
                let out = rst_fmt::build(std::hint::black_box(&ast));
                std::hint::black_box(out);
            }
            let (build_peak, build_allocs) = peak_and_allocs();

            // clone-only baseline: the harness must feed owned events, and
            // cloning them allocates; subtract that from the writer numbers.
            let mut clone_time = f64::MAX;
            for _ in 0..n {
                let t0 = Instant::now();
                let cloned: Vec<_> = events.to_vec();
                let e = t0.elapsed().as_secs_f64();
                std::hint::black_box(&cloned);
                clone_time = clone_time.min(e);
            }
            reset_counters();
            {
                let cloned: Vec<_> = events.to_vec();
                std::hint::black_box(&cloned);
            }
            let (_, clone_allocs) = peak_and_allocs();

            let mut w_time = f64::MAX;
            for _ in 0..n {
                let t0 = Instant::now();
                let mut out = Vec::new();
                {
                    let mut w = rst_fmt::writer::Writer::new(&mut out);
                    for ev in events.iter().cloned() {
                        w.write_event(ev);
                    }
                    w.finish();
                }
                let e = t0.elapsed().as_secs_f64();
                std::hint::black_box(&out);
                w_time = w_time.min(e);
            }
            reset_counters();
            {
                let mut out = Vec::new();
                {
                    let mut w = rst_fmt::writer::Writer::new(&mut out);
                    for ev in events.iter().cloned() {
                        w.write_event(ev);
                    }
                    w.finish();
                }
                std::hint::black_box(&out);
            }
            let (w_peak, w_allocs) = peak_and_allocs();

            let net_time = w_time - clone_time;
            let net_allocs = w_allocs.saturating_sub(clone_allocs);
            println!(
                "sections={sections:5} | build: {:>9.1}us allocs={:>7} peak={:>9}B | writer(net): {:>9.1}us allocs={:>7} | writer(raw): {:>9.1}us allocs={:>7} peak={:>9}B | time_ratio(net/build)={:.2} alloc_ratio(net/build)={:.2}",
                build_time * 1e6,
                build_allocs,
                build_peak,
                net_time * 1e6,
                net_allocs,
                w_time * 1e6,
                w_allocs,
                w_peak,
                net_time / build_time,
                net_allocs as f64 / build_allocs as f64,
            );
        }
    }

    if mode == "all" || mode == "events" {
        println!("\n== events() vs parse(): time, allocation count, borrowed-Cow fraction ==");
        for sections in [50, 500, 2000] {
            let doc = gen_doc(sections);
            let n = 20;

            let mut parse_time = f64::MAX;
            for _ in 0..n {
                let t0 = Instant::now();
                let d = rst_fmt::parse(std::hint::black_box(&doc)).unwrap();
                let e = t0.elapsed().as_secs_f64();
                std::hint::black_box(&d);
                parse_time = parse_time.min(e);
            }
            reset_counters();
            {
                let d = rst_fmt::parse(std::hint::black_box(&doc)).unwrap();
                std::hint::black_box(&d);
            }
            let (parse_peak, parse_allocs) = peak_and_allocs();

            let mut ev_time = f64::MAX;
            for _ in 0..n {
                let t0 = Instant::now();
                let c = rst_fmt::events(std::hint::black_box(&doc)).count();
                let e = t0.elapsed().as_secs_f64();
                std::hint::black_box(c);
                ev_time = ev_time.min(e);
            }
            reset_counters();
            {
                let c = rst_fmt::events(std::hint::black_box(&doc)).count();
                std::hint::black_box(c);
            }
            let (ev_peak, ev_allocs) = peak_and_allocs();

            let mut borrowed = 0usize;
            let mut owned = 0usize;
            for ev in rst_fmt::events(&doc) {
                match ev {
                    rst_fmt::Event::Text(c)
                    | rst_fmt::Event::Code(c)
                    | rst_fmt::Event::CodeBlockContent(c) => match c {
                        std::borrow::Cow::Borrowed(_) => borrowed += 1,
                        std::borrow::Cow::Owned(_) => owned += 1,
                    },
                    _ => {}
                }
            }

            println!(
                "sections={sections:5} bytes={:8} | parse: {:>9.1}us allocs={:>8} peak={:>10}B | events: {:>9.1}us allocs={:>8} peak={:>10}B | alloc_ratio={:.3} | cow borrowed={borrowed} owned={owned}",
                doc.len(),
                parse_time * 1e6,
                parse_allocs,
                parse_peak,
                ev_time * 1e6,
                ev_allocs,
                ev_peak,
                ev_allocs as f64 / parse_allocs as f64,
            );
        }
    }
}
