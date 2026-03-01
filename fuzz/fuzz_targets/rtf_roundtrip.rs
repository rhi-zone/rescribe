#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let (ast1, _) = rtf_fmt::parse(s);
        let emitted = rtf_fmt::emit(&ast1);
        let (ast2, _) = rtf_fmt::parse(&emitted);

        // Structure and content must be identical; spans differ because the
        // emitted string has different byte offsets than the original.
        assert_eq!(
            ast1.strip_spans(),
            ast2.strip_spans(),
            "RTF round-trip changed content\n  input: {s:?}\n  emitted: {emitted:?}"
        );
    }
});
