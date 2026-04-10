use ooxml_codegen::{CodegenConfig, FeatureMappings, NameMappings, generate, parse_rnc};
use std::fs;
use std::path::Path;

fn main() {
    let spec_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../spec/odf");
    let names_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../spec/odf-names.yaml");
    let features_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../spec/odf-features.yaml");

    let schema_12 = format!("{}/odf-1.2.rnc", spec_dir);
    let schema_13 = format!("{}/odf-1.3.rnc", spec_dir);

    println!("cargo::rerun-if-changed={}", schema_12);
    println!("cargo::rerun-if-changed={}", schema_13);
    println!("cargo::rerun-if-changed={}", names_path);
    println!("cargo::rerun-if-changed={}", features_path);
    println!("cargo::rerun-if-changed=build.rs");

    let dest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated.rs");
    let should_regenerate = std::env::var("ODF_REGENERATE").is_ok();

    if !should_regenerate {
        // Use committed generated.rs — nothing to do
        return;
    }

    // Prefer ODF 1.3 when available; fall back to 1.2
    let (schema_path, version) = if Path::new(&schema_13).exists() {
        (&schema_13, "1.3")
    } else if Path::new(&schema_12).exists() {
        (&schema_12, "1.2")
    } else {
        eprintln!(
            "Warning: No ODF schema found at {}. \
             Download with: curl -sL https://docs.oasis-open.org/office/OpenDocument/v1.3/os/schemas/OpenDocument-v1.3-schema.rng \
             -o /tmp/odf.rng && trang /tmp/odf.rng spec/odf/odf-1.3.rnc",
            spec_dir
        );
        return;
    };

    eprintln!("Regenerating src/generated.rs from ODF {} schema...", version);

    let schema_input = fs::read_to_string(schema_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", schema_path, e));
    let schema = parse_rnc(&schema_input)
        .unwrap_or_else(|e| panic!("failed to parse ODF {} schema: {:?}", version, e));

    eprintln!(
        "Parsed ODF {}: {} definitions, {} namespaces",
        version,
        schema.definitions.len(),
        schema.namespaces.len()
    );

    let name_mappings = load_yaml::<NameMappings>(names_path, "name mappings");
    let feature_mappings = load_yaml::<FeatureMappings>(features_path, "feature mappings");

    let config = CodegenConfig {
        strip_prefix: None, // ODF names already have namespace prefixes
        module_name: "odf".to_string(),
        name_mappings,
        feature_mappings,
        xml_serialize_prefix: None,
        warn_unmapped: true,
        // ODF doesn't use ooxml_xml for extra_children; use a plain Vec<u8> placeholder
        // until we have a proper odf_xml crate (or remove extra_children for ODF entirely)
        extra_children_type: Some("Vec<u8>".to_string()),
        ..Default::default()
    };

    let code = generate(&schema, &config);
    fs::write(&dest_path, &code).expect("failed to write generated types");
    eprintln!(
        "Generated {} bytes to src/generated.rs",
        dest_path.metadata().map(|m| m.len()).unwrap_or(0)
    );

    // Enable with ODF_GENERATE_PARSERS=1
    if std::env::var("ODF_GENERATE_PARSERS").is_ok() {
        use ooxml_codegen::generate_parsers;
        let dest = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated_parsers.rs");
        let code = generate_parsers(&schema, &config);
        fs::write(&dest, code).expect("failed to write generated parsers");
        eprintln!("Generated {} bytes to src/generated_parsers.rs", dest.metadata().map(|m| m.len()).unwrap_or(0));
    }

    // Enable with ODF_GENERATE_SERIALIZERS=1
    if std::env::var("ODF_GENERATE_SERIALIZERS").is_ok() {
        use ooxml_codegen::generate_serializers;
        let dest = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated_serializers.rs");
        let code = generate_serializers(&schema, &config);
        fs::write(&dest, code).expect("failed to write generated serializers");
        eprintln!("Generated {} bytes to src/generated_serializers.rs", dest.metadata().map(|m| m.len()).unwrap_or(0));
    }
}

fn load_yaml<T: serde::de::DeserializeOwned>(path: &str, label: &str) -> Option<T> {
    if !Path::new(path).exists() {
        return None;
    }
    match serde_yaml::from_str(&fs::read_to_string(path).unwrap_or_default()) {
        Ok(v) => {
            eprintln!("Loaded {} from {}", label, path);
            Some(v)
        }
        Err(e) => {
            eprintln!("Warning: Failed to load {}: {}", label, e);
            None
        }
    }
}
