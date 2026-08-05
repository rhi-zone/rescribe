//! Deterministic round-trip regression test, covering the same property
//! the `fuzz_zip_fmt_roundtrip` fuzz target checks (see that target's
//! doc comment in `fuzz/fuzz_targets/zip_fmt_roundtrip.rs` for exactly
//! which `Entry` fields are — and are not — guaranteed to survive
//! `emit()`/`parse()`, and why), run here over many pseudo-random seeds
//! as a committed, non-fuzzer regression check.

use zip_fmt::{Archive, CompressionMethod, Entry};

/// Minimal xorshift PRNG — no external dependency needed for deterministic,
/// seeded pseudo-random test data.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(1))
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn byte(&mut self) -> u8 {
        (self.next_u64() & 0xff) as u8
    }

    fn range(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
}

fn random_entry(rng: &mut Rng, seen: &mut std::collections::HashSet<String>) -> Entry {
    let name_len = rng.range(12) as usize + 1;
    let mut name: String = rng
        .bytes(name_len)
        .iter()
        .map(|b| ((*b % 26) + b'a') as char)
        .collect();
    while !seen.insert(name.clone()) {
        name.push('x');
    }
    let compression = if rng.range(2) == 0 {
        CompressionMethod::Store
    } else {
        CompressionMethod::Deflate
    };
    let content_len = rng.range(500) as usize;
    Entry {
        name,
        is_utf8_name: true,
        compression,
        content: rng.bytes(content_len),
        ..Entry::default()
    }
}

#[test]
fn emit_parse_roundtrip_many_seeds() {
    for seed in 0u64..200 {
        let mut rng = Rng::new(seed);
        let count = rng.range(5) + 1;
        let mut seen = std::collections::HashSet::new();
        let entries: Vec<Entry> = (0..count)
            .map(|_| random_entry(&mut rng, &mut seen))
            .collect();
        let comment_len = rng.range(6) as usize;
        let archive = Archive {
            entries,
            comment: rng.bytes(comment_len),
            span: Default::default(),
        };

        let emitted = zip_fmt::emit(&archive);
        let (archive2, diags) = zip_fmt::parse(&emitted);
        assert!(diags.is_empty(), "seed {seed}: diagnostics: {diags:?}");
        assert_eq!(
            archive.comment, archive2.comment,
            "seed {seed}: comment mismatch"
        );
        assert_eq!(
            archive.entries.len(),
            archive2.entries.len(),
            "seed {seed}: entry count mismatch"
        );
        for (a, b) in archive.entries.iter().zip(archive2.entries.iter()) {
            assert_eq!(a.name, b.name, "seed {seed}: name mismatch");
            assert_eq!(
                a.compression, b.compression,
                "seed {seed}: compression mismatch"
            );
            assert_eq!(a.content, b.content, "seed {seed}: content mismatch");
        }
    }
}

/// Same property, driven through the hand-rolled `StreamingParser` instead
/// of `parse()` — this is the path `epub-fmt`/`ooxml-fmt` streaming
/// consumers will actually use, and it must agree with `parse()`'s view of
/// the same bytes (including for data-descriptor entries, which is every
/// entry `crate::Writer` produces — see `batch.rs`'s module docs).
#[test]
fn emit_then_streaming_parse_roundtrip_many_seeds() {
    for seed in 200u64..260 {
        let mut rng = Rng::new(seed);
        let count = rng.range(4) + 1;
        let mut seen = std::collections::HashSet::new();
        let entries: Vec<Entry> = (0..count)
            .map(|_| random_entry(&mut rng, &mut seen))
            .collect();
        let archive = Archive {
            entries: entries.clone(),
            comment: Vec::new(),
            span: Default::default(),
        };
        let emitted = zip_fmt::emit(&archive);

        let mut names = Vec::new();
        let mut current_content = Vec::new();
        let mut contents = Vec::new();
        {
            let mut p = zip_fmt::StreamingParser::new(|ev| match ev {
                zip_fmt::batch::Event::StartEntry { name, .. } => {
                    names.push(name);
                    current_content = Vec::new();
                }
                zip_fmt::batch::Event::Data(chunk) => current_content.extend_from_slice(&chunk),
                zip_fmt::batch::Event::EndEntry { .. } => {
                    contents.push(std::mem::take(&mut current_content));
                }
                zip_fmt::batch::Event::ArchiveComment(_) => {}
            });
            // Feed in small, arbitrary-sized chunks to exercise
            // chunk-boundary handling, not just one big feed().
            let mut offset = 0;
            while offset < emitted.len() {
                let take = (rng.range(37) as usize + 1).min(emitted.len() - offset);
                p.feed(&emitted[offset..offset + take]);
                offset += take;
            }
            let diags = p.finish();
            assert!(diags.is_empty(), "seed {seed}: diagnostics: {diags:?}");
        }

        let expected_names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
        assert_eq!(names, expected_names, "seed {seed}: name sequence mismatch");
        let expected_contents: Vec<Vec<u8>> = entries.iter().map(|e| e.content.clone()).collect();
        assert_eq!(
            contents, expected_contents,
            "seed {seed}: content sequence mismatch"
        );
    }
}
