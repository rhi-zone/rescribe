//! Static analysis: verify that all read-*/write-* features are wrapped in a
//! format-* feature, and every format-* feature is covered by at least one
//! formats-* group.
//!
//! This acts as a lint on the Cargo.toml feature table so that newly added
//! formats can't silently be missing from the group taxonomy.

const CARGO_TOML: &str = include_str!("../Cargo.toml");

/// Parse the `[features]` section of a Cargo.toml into a map of
/// `feature-name -> [dep, dep, ...]`.
///
/// Handles both single-line and multi-line array values.
fn parse_features(src: &str) -> std::collections::HashMap<String, Vec<String>> {
    let mut map = std::collections::HashMap::new();
    let mut in_features = false;
    let mut in_array = false;
    let mut current_key = String::new();
    let mut current_vals: Vec<String> = Vec::new();

    for line in src.lines() {
        let trimmed = line.trim();

        // Section headers
        if trimmed.starts_with('[') {
            in_features = trimmed == "[features]";
            in_array = false;
            continue;
        }
        if !in_features || trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        if !in_array {
            if let Some(eq_pos) = trimmed.find(" = [") {
                current_key = trimmed[..eq_pos].trim().to_string();
                let rest = trimmed[eq_pos + 4..].trim();
                current_vals = extract_vals(rest);
                if rest.ends_with(']') {
                    map.insert(current_key.clone(), current_vals.clone());
                    current_vals.clear();
                } else {
                    in_array = true;
                }
            }
        } else {
            current_vals.extend(extract_vals(trimmed));
            if trimmed.ends_with(']') {
                in_array = false;
                map.insert(current_key.clone(), current_vals.clone());
                current_vals.clear();
            }
        }
    }
    map
}

fn extract_vals(s: &str) -> Vec<String> {
    s.trim_end_matches(']')
        .split(',')
        .map(|v| v.trim().trim_matches('"').to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

#[test]
fn all_read_write_covered_by_format_feature() {
    let features = parse_features(CARGO_TOML);

    // Collect deps of all format-* features (e.g. format-markdown -> ["read-markdown", "write-markdown"])
    let format_deps: std::collections::HashSet<&str> = features
        .iter()
        .filter(|(k, _)| k.starts_with("format-"))
        .flat_map(|(_, deps)| deps.iter().map(|s| s.as_str()))
        .collect();

    let mut gaps: Vec<String> = Vec::new();
    for name in features.keys() {
        if (name.starts_with("read-") || name.starts_with("write-"))
            && !format_deps.contains(name.as_str())
        {
            gaps.push(format!(
                "  {name} is not referenced by any format-* feature"
            ));
        }
    }

    if !gaps.is_empty() {
        gaps.sort();
        panic!(
            "read-*/write-* features missing from format-* coverage:\n{}",
            gaps.join("\n")
        );
    }
}

#[test]
fn all_format_features_covered_by_formats_group() {
    let features = parse_features(CARGO_TOML);

    // Collect deps of all formats-* group features
    let group_deps: std::collections::HashSet<&str> = features
        .iter()
        .filter(|(k, _)| k.starts_with("formats-"))
        .flat_map(|(_, deps)| deps.iter().map(|s| s.as_str()))
        .collect();

    // Expand one level: formats-* may reference other formats-* (like `all` does)
    let mut expanded: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for dep in &group_deps {
        if dep.starts_with("formats-") {
            if let Some(inner) = features.get(*dep) {
                expanded.extend(inner.iter().map(|s| s.as_str()));
            }
        } else {
            expanded.insert(dep);
        }
    }
    let group_deps = expanded;

    let mut gaps: Vec<String> = Vec::new();
    for name in features.keys() {
        if name.starts_with("format-") && !group_deps.contains(name.as_str()) {
            gaps.push(format!("  {name} is not referenced by any formats-* group"));
        }
    }

    if !gaps.is_empty() {
        gaps.sort();
        panic!(
            "format-* features missing from formats-* group coverage:\n{}",
            gaps.join("\n")
        );
    }
}
