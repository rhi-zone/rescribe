//! Numbering (list) support for WordprocessingML.
//!
//! This module provides parsing of `numbering.xml` to determine whether
//! each concrete numbering definition (`<w:num>`) represents an ordered
//! (numbered) or unordered (bulleted) list.

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

/// Parse the `numbering.xml` bytes and return a map from `numId` to `is_ordered`.
///
/// For each concrete `<w:num>`, looks up the referenced abstract numbering's
/// level 0 `numFmt`. Decimal / roman / alphabetic → ordered (`true`);
/// bullet → unordered (`false`).
pub fn parse_numbering_order(xml: &[u8]) -> HashMap<i64, bool> {
    // abstract_num_id → is_ordered (based on level 0 numFmt)
    let mut abstract_ordered: HashMap<i64, bool> = HashMap::new();
    // num_id → abstract_num_id
    let mut num_to_abstract: HashMap<i64, i64> = HashMap::new();

    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    // State: current abstract num being processed
    let mut current_abstract_id: Option<i64> = None;
    let mut in_level0 = false;
    // Track nesting depth within abstractNum so we know when we exit
    let mut abstract_depth: usize = 0;
    // State: current num instance being processed
    let mut current_num_id: Option<i64> = None;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let e_name = e.name();
                let local = local_name(e_name.as_ref());
                match local {
                    b"abstractNum" => {
                        let id = attr_i64(&e, b"abstractNumId");
                        current_abstract_id = id;
                        abstract_depth = 1;
                        in_level0 = false;
                    }
                    b"num" if current_abstract_id.is_none() => {
                        current_num_id = attr_i64(&e, b"numId");
                    }
                    b"lvl" if current_abstract_id.is_some() => {
                        if abstract_depth == 1 {
                            // ilvl="0" means level 0
                            let ilvl = attr_i64(&e, b"ilvl").unwrap_or(99);
                            in_level0 = ilvl == 0;
                        }
                        abstract_depth += 1;
                    }
                    b"numFmt" if in_level0 => {
                        // val attr tells us the format
                        if let Some(val) = attr_str(&e, b"val") {
                            let ordered = is_ordered_num_fmt(&val);
                            if let Some(aid) = current_abstract_id {
                                abstract_ordered.insert(aid, ordered);
                            }
                        }
                    }
                    b"abstractNumId"
                        if current_num_id.is_some() && current_abstract_id.is_none() =>
                    {
                        // inside <w:num>: <w:abstractNumId w:val="N"/>
                        if let Some(val) = attr_i64(&e, b"val")
                            && let Some(nid) = current_num_id
                        {
                            num_to_abstract.insert(nid, val);
                        }
                    }
                    _ => {
                        if current_abstract_id.is_some() && abstract_depth > 0 {
                            abstract_depth += 1;
                        }
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let e_name = e.name();
                let local = local_name(e_name.as_ref());
                match local {
                    b"abstractNumId"
                        if current_num_id.is_some() && current_abstract_id.is_none() =>
                    {
                        if let Some(val) = attr_i64(&e, b"val")
                            && let Some(nid) = current_num_id
                        {
                            num_to_abstract.insert(nid, val);
                        }
                    }
                    b"numFmt" if in_level0 => {
                        if let Some(val) = attr_str(&e, b"val") {
                            let ordered = is_ordered_num_fmt(&val);
                            if let Some(aid) = current_abstract_id {
                                abstract_ordered.insert(aid, ordered);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let e_name = e.name();
                let local = local_name(e_name.as_ref());
                match local {
                    b"abstractNum" => {
                        current_abstract_id = None;
                        abstract_depth = 0;
                        in_level0 = false;
                    }
                    b"num" if current_abstract_id.is_none() => {
                        current_num_id = None;
                    }
                    b"lvl" if current_abstract_id.is_some() => {
                        abstract_depth = abstract_depth.saturating_sub(1);
                        if abstract_depth == 1 {
                            in_level0 = false;
                        }
                    }
                    _ => {
                        if current_abstract_id.is_some() && abstract_depth > 1 {
                            abstract_depth -= 1;
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    // Build num_id → is_ordered
    let mut result = HashMap::new();
    for (num_id, abstract_id) in &num_to_abstract {
        let ordered = abstract_ordered.get(abstract_id).copied().unwrap_or(false);
        result.insert(*num_id, ordered);
    }
    result
}

/// Returns `true` if the OOXML `numFmt` value represents an ordered (numbered) list.
///
/// Unordered: `"bullet"`, `"none"`, `"chicago"`, `"ordinalText"`, `"cardinalText"`.
/// Everything else (decimal, roman, letter, etc.) is ordered.
pub fn is_ordered_num_fmt(val: &str) -> bool {
    !matches!(
        val,
        "bullet" | "none" | "chicago" | "ordinalText" | "cardinalText"
    )
}

/// Strip the namespace prefix from an XML element name.
fn local_name(name: &[u8]) -> &[u8] {
    if let Some(pos) = name.iter().position(|&b| b == b':') {
        &name[pos + 1..]
    } else {
        name
    }
}

/// Extract an `i64` attribute value by local name.
fn attr_i64(e: &quick_xml::events::BytesStart<'_>, local: &[u8]) -> Option<i64> {
    for attr in e.attributes().flatten() {
        let key = local_name(attr.key.as_ref());
        if key == local {
            let val = std::str::from_utf8(&attr.value).ok()?;
            return val.parse().ok();
        }
    }
    None
}

/// Extract a `String` attribute value by local name.
fn attr_str(e: &quick_xml::events::BytesStart<'_>, local: &[u8]) -> Option<String> {
    for attr in e.attributes().flatten() {
        let key = local_name(attr.key.as_ref());
        if key == local {
            return std::str::from_utf8(&attr.value).ok().map(|s| s.to_string());
        }
    }
    None
}
