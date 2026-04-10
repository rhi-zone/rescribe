fn schema_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .join("spec/odf")
}

#[test]
fn test_parse_odf_13() {
    use ooxml_codegen::parse_rnc;
    let path = schema_root().join("odf-1.3.rnc");
    let schema_str = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("SKIP: ODF 1.3 schema not found at {path:?}");
            eprintln!("Run: curl -sL https://docs.oasis-open.org/office/OpenDocument/v1.3/os/schemas/OpenDocument-v1.3-schema.rng -o /tmp/odf-1.3.rng && trang /tmp/odf-1.3.rng spec/odf/odf-1.3.rnc");
            return;
        }
    };
    let schema = parse_rnc(&schema_str).expect("ODF 1.3 should parse");
    println!("ODF 1.3: {} definitions, {} namespaces", schema.definitions.len(), schema.namespaces.len());
}

#[test]
fn test_parse_odf_12() {
    use ooxml_codegen::parse_rnc;
    let path = schema_root().join("odf-1.2.rnc");
    let schema_str = std::fs::read_to_string(&path).expect("ODF 1.2 schema should be present");
    let schema = parse_rnc(&schema_str).expect("ODF 1.2 should parse");
    println!("ODF 1.2: {} definitions, {} namespaces", schema.definitions.len(), schema.namespaces.len());
}
