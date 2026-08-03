//! Regression test for ADR 0013 open question 5 (OOXML slice/Cargo-feature collapse).
//!
//! `spec/ooxml-features.yaml` tags `Worksheet.drawingHF: [drawings, layout]` — two
//! pragmatic slices on one construct. A Cargo feature gate is a single `#[cfg]`
//! predicate, so the codegen must collapse the multi-membership list to one predicate.
//! The decided rule is OR-of-all: `#[cfg(any(feature = "sml-drawings", feature =
//! "sml-layout"))]`, so enabling *either* slice includes the construct.
//!
//! The bug this pins against: `primary_feature` (removed) kept only `tags.first()`
//! ("drawings") and silently discarded "layout" — enabling `sml-layout` alone would
//! never have compiled in `drawingHF`, with no diagnostic anywhere. If this test
//! starts failing, something has regressed back to first-tag-only collapse.
//!
//! Requires the ECMA-376 RNC schemas checked out at `spec/OfficeOpenXML-RELAXNG-Transitional`
//! (gitignored, fetched via `scripts/ooxml/download-spec.sh`) — skips with a message if absent,
//! since most checkouts won't have them (see ADR 0013 open question 3 on redistribution).

use ooxml_codegen::{
    CodegenConfig, FeatureMappings, NameMappings, generate, generate_parsers, parse_rnc,
};
use std::path::Path;

fn load_sml_schema() -> Option<ooxml_codegen::Schema> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../spec/OfficeOpenXML-RELAXNG-Transitional/sml.rnc"
    );
    if !Path::new(path).exists() {
        eprintln!("skipping: schema not found at {path} (run scripts/ooxml/download-spec.sh)");
        return None;
    }
    let input = ooxml_codegen::read_spec_file(path);
    Some(parse_rnc(&input).expect("failed to parse sml.rnc"))
}

fn load_feature_mappings() -> FeatureMappings {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../spec/ooxml-features.yaml"
    );
    FeatureMappings::from_yaml_file(Path::new(path)).expect("failed to load ooxml-features.yaml")
}

fn load_name_mappings() -> NameMappings {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../spec/ooxml-names.yaml"
    );
    NameMappings::from_yaml_file(Path::new(path)).expect("failed to load ooxml-names.yaml")
}

#[test]
fn drawing_hf_is_gated_on_the_or_of_both_its_tags_in_struct_codegen() {
    let Some(schema) = load_sml_schema() else {
        return;
    };
    let feature_mappings = load_feature_mappings();

    // Sanity: the fixture this test relies on hasn't drifted out from under it.
    let gates = feature_mappings
        .feature_gates("sml", "Worksheet", "drawingHF")
        .expect("Worksheet.drawingHF should still be tagged in ooxml-features.yaml");
    assert_eq!(
        gates,
        vec!["drawings", "layout"],
        "this test assumes Worksheet.drawingHF: [drawings, layout] in ooxml-features.yaml; \
         update the fixture (and this assertion) if the tags changed"
    );

    let config = CodegenConfig {
        strip_prefix: Some("sml_".to_string()),
        module_name: "sml".to_string(),
        name_mappings: Some(load_name_mappings()),
        feature_mappings: Some(feature_mappings),
        ..Default::default()
    };

    let code = generate(&schema, &config);
    assert!(
        code.contains("#[cfg(any(feature = \"sml-drawings\", feature = \"sml-layout\"))]"),
        "expected drawingHF's struct field to carry an OR-of-both-tags cfg predicate; \
         generated code did not contain it — the 'layout' tag may be silently inert again"
    );
    // The single-tag collapse bug produced exactly this predicate instead — assert its
    // absence so a regression back to first-tag-only is caught even if some other field
    // happens to also be sml-drawings-only gated elsewhere in the file.
    assert!(
        !code.contains("#[cfg(feature = \"sml-drawings\")]\n    pub drawing_hf"),
        "drawingHF must not be gated on 'drawings' alone — 'layout' would be silently inert"
    );
}

#[test]
fn drawing_hf_is_gated_on_the_or_of_both_its_tags_in_parser_codegen() {
    let Some(schema) = load_sml_schema() else {
        return;
    };
    let feature_mappings = load_feature_mappings();

    let config = CodegenConfig {
        strip_prefix: Some("sml_".to_string()),
        module_name: "sml".to_string(),
        name_mappings: Some(load_name_mappings()),
        feature_mappings: Some(feature_mappings),
        ..Default::default()
    };

    let parser_code = generate_parsers(&schema, &config);
    assert!(
        parser_code.contains("any(feature = \"sml-drawings\", feature = \"sml-layout\")"),
        "expected the generated parser to gate drawingHF on the OR of both tags"
    );
}
