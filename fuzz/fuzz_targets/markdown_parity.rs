#![no_main]

use libfuzzer_sys::fuzz_target;
use rescribe_core::ParseOptions;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };

    let opts = ParseOptions {
        preserve_source_info: false,
        ..Default::default()
    };

    let Ok(pd_result) = rescribe_read_markdown::backend_pulldown::parse_with_options(s, &opts)
    else {
        return;
    };
    let Ok(ts_result) = rescribe_read_markdown::backend_treesitter::parse_with_options(s, &opts)
    else {
        return;
    };

    let mut pd = pd_result.value.content;
    let mut ts = ts_result.value.content;
    pd.strip_spans();
    ts.strip_spans();

    assert_eq!(pd, ts, "backend parity failure for input: {:?}", s);
});
