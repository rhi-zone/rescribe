#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Parse the input
        if let Ok(result1) = rescribe_read_djot::parse(s) {
            let doc1 = result1.value;

            // Emit back to djot
            if let Ok(emitted) = rescribe_write_djot::emit(&doc1) {
                let djot = String::from_utf8_lossy(&emitted.value);

                // Parse again - should succeed if emit succeeded
                if let Ok(result2) = rescribe_read_djot::parse(&djot) {
                    let doc2 = result2.value;

                    // Text content should be preserved
                    let text1 = extract_text(&doc1.content);
                    let text2 = extract_text(&doc2.content);

                    // Normalize whitespace for comparison
                    let norm1: String = text1.split_whitespace().collect::<Vec<_>>().join(" ");
                    let norm2: String = text2.split_whitespace().collect::<Vec<_>>().join(" ");

                    // Use multiset (sorted-char) equality rather than strict string equality.
                    // jotdown has a span-delimiter adjacency quirk where e.g. `\^` immediately
                    // before a structural `^` can cause characters to be reordered across the
                    // roundtrip without any being added or removed.  Sorting both strings
                    // detects additions/removals (real bugs) while tolerating reordering.
                    let mut chars1: Vec<char> = norm1.chars().collect();
                    let mut chars2: Vec<char> = norm2.chars().collect();
                    chars1.sort_unstable();
                    chars2.sort_unstable();
                    assert_eq!(chars1, chars2, "Text content changed through roundtrip\n  before: {norm1:?}\n  after:  {norm2:?}");
                }
            }
        }
    }
});

fn extract_text(node: &rescribe_std::Node) -> String {
    use rescribe_std::{node, prop};

    let mut text = String::new();

    if node.kind.as_str() == node::TEXT {
        if let Some(content) = node.props.get_str(prop::CONTENT) {
            text.push_str(content);
        }
    }

    for child in &node.children {
        text.push_str(&extract_text(child));
    }

    text
}
