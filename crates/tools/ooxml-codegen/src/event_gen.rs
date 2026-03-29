//! Streaming event type generator for OOXML formats.
//!
//! Generates the `WmlEvent<'a>` enum, `OwnedWmlEvent` alias, `into_owned()`,
//! and a SAX-dispatch table used by the hand-written `WmlEventIter`.

use serde::Deserialize;
use std::fmt::Write;

// ---------------------------------------------------------------------------
// Config types (deserialized from ooxml-events.yaml)
// ---------------------------------------------------------------------------

/// Top-level event configuration for one OOXML module.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EventConfig {
    /// Name of the generated event enum (e.g. `"WmlEvent"`).
    pub enum_name: String,

    /// Name of the generated container-kind enum (e.g. `"WmlStartKind"`).
    /// Defaults to `"{enum_name}StartKind"` if omitted.
    #[serde(default)]
    pub kind_enum_name: Option<String>,

    /// Additional `use` paths needed for cross-crate types referenced in the enum
    /// (e.g. `"ooxml_dml::types::*"` when DML types appear in PML events).
    #[serde(default)]
    pub cross_crate_imports: Vec<String>,

    /// Variant name for the start-of-document sentinel (e.g. `"StartDocument"`).
    #[serde(default)]
    pub document_start: Option<String>,

    /// Variant name for the end-of-document sentinel (e.g. `"EndDocument"`).
    #[serde(default)]
    pub document_end: Option<String>,

    /// Container elements: each produces a `Start…`/`End…` pair.
    #[serde(default)]
    pub containers: Vec<ContainerDef>,

    /// Leaf elements: each produces a single variant.
    #[serde(default)]
    pub leaves: Vec<LeafDef>,
}

impl EventConfig {
    /// Load from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Load from a YAML file.
    pub fn from_yaml_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        Ok(Self::from_yaml(&contents)?)
    }
}

/// A container element that emits `Start…` / `End…` events.
#[derive(Debug, Clone, Deserialize)]
pub struct ContainerDef {
    /// XML local name (e.g. `"p"`, `"r"`, `"tbl"`).
    pub xml_local: String,

    /// Rust variant name for the start event (e.g. `"StartParagraph"`).
    pub start_variant: String,

    /// Rust variant name for the end event (e.g. `"EndParagraph"`).
    pub end_variant: String,

    /// XML local name of the properties child element, if any (e.g. `"pPr"`).
    ///
    /// Mutually exclusive with `props_from_attrs`.
    #[serde(default)]
    pub props_xml_local: Option<String>,

    /// Rust type of the props struct, if any (e.g. `"ParagraphProperties"`).
    #[serde(default)]
    pub props_rust_type: Option<String>,

    /// If true, parse `props_rust_type` from the container element's own attributes
    /// (using `FromXml::from_xml(..., is_empty=true)`) instead of looking for a child
    /// element.  Used for XLSX `<row>` and `<c>` which carry all metadata as attributes.
    ///
    /// Mutually exclusive with `props_xml_local`.
    #[serde(default)]
    pub props_from_attrs: bool,

    /// Extra fields on the start variant, read from XML attributes of the container element.
    #[serde(default)]
    pub attrs: Vec<AttrFieldDef>,
}

/// A leaf element that emits a single event variant.
#[derive(Debug, Clone, Deserialize)]
pub struct LeafDef {
    /// XML local name (e.g. `"br"`, `"t"`).
    pub xml_local: String,

    /// Rust variant name (e.g. `"LineBreak"`, `"Text"`).
    pub variant: String,

    /// If set, this variant holds borrowed text content: `Cow<'a, str>`.
    /// The value is ignored; presence means the variant has a `Cow` payload.
    #[serde(default)]
    pub text_content: bool,

    /// Extra fields read from XML attributes.
    #[serde(default)]
    pub attrs: Vec<AttrFieldDef>,
}

/// A single named field parsed from an XML attribute.
#[derive(Debug, Clone, Deserialize)]
pub struct AttrFieldDef {
    /// XML attribute local name (e.g. `"id"`, `"embed"`).
    pub xml_attr: String,

    /// Rust field name in the variant (e.g. `"id"`, `"rel_id"`).
    pub field_name: String,

    /// Rust type as a string (e.g. `"i32"`, `"Cow<'a, str>"`,
    /// `"Option<Cow<'a, str>>"`).
    pub rust_type: String,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Generate streaming event types for the given config.
///
/// Returns a Rust source string suitable for writing to `src/generated_events.rs`.
pub fn generate_events(config: &EventConfig) -> String {
    let mut g = EventGenerator::new(config);
    g.run()
}

// ---------------------------------------------------------------------------
// Generator implementation
// ---------------------------------------------------------------------------

struct EventGenerator<'a> {
    config: &'a EventConfig,
    output: String,
}

impl<'a> EventGenerator<'a> {
    fn new(config: &'a EventConfig) -> Self {
        Self {
            config,
            output: String::new(),
        }
    }

    fn kind_enum_name(&self) -> String {
        self.config
            .kind_enum_name
            .clone()
            .unwrap_or_else(|| format!("{}StartKind", self.config.enum_name))
    }

    fn run(&mut self) -> String {
        self.write_header();
        self.gen_event_enum();
        self.gen_owned_alias();
        self.gen_into_owned();
        self.gen_start_kind_enum();
        self.gen_dispatch_start();
        self.gen_props_element();
        self.gen_props_strategy();
        self.gen_is_text_element();
        std::mem::take(&mut self.output)
    }

    fn write_header(&mut self) {
        writeln!(
            self.output,
            "// Streaming event types for OOXML WordprocessingML."
        )
        .unwrap();
        writeln!(
            self.output,
            "// DO NOT EDIT — generated by ooxml-codegen event_gen."
        )
        .unwrap();
        writeln!(self.output).unwrap();
        writeln!(self.output, "#![allow(unused_imports)]").unwrap();
        writeln!(self.output).unwrap();
        writeln!(self.output, "use std::borrow::Cow;").unwrap();
        writeln!(self.output, "use super::generated::*;").unwrap();
        for import in &self.config.cross_crate_imports {
            writeln!(self.output, "use {};", import).unwrap();
        }
        writeln!(self.output).unwrap();
    }

    // -----------------------------------------------------------------------
    // WmlEvent enum
    // -----------------------------------------------------------------------

    fn gen_event_enum(&mut self) {
        let name = &self.config.enum_name;
        writeln!(
            self.output,
            "/// Streaming events emitted by the DOCX SAX iterator."
        )
        .unwrap();
        writeln!(
            self.output,
            "/// Each container element produces a `Start…` / `End…` pair;"
        )
        .unwrap();
        writeln!(
            self.output,
            "/// leaf elements produce a single variant."
        )
        .unwrap();
        writeln!(self.output, "#[derive(Debug, Clone)]").unwrap();
        writeln!(self.output, "pub enum {}<'a> {{", name).unwrap();

        // Document sentinels
        if let Some(s) = &self.config.document_start {
            writeln!(self.output, "    {},", s).unwrap();
        }
        if let Some(e) = &self.config.document_end {
            writeln!(self.output, "    {},", e).unwrap();
        }

        // Container Start + End variants
        for c in &self.config.containers {
            // Start variant
            let has_props = c.props_rust_type.is_some();
            let has_attrs = !c.attrs.is_empty();

            if has_props || has_attrs {
                writeln!(self.output, "    {} {{", c.start_variant).unwrap();
                if has_props {
                    let pt = c.props_rust_type.as_ref().unwrap();
                    writeln!(self.output, "        props: Box<{}>,", pt).unwrap();
                }
                for attr in &c.attrs {
                    writeln!(
                        self.output,
                        "        {}: {},",
                        attr.field_name, attr.rust_type
                    )
                    .unwrap();
                }
                writeln!(self.output, "    }},").unwrap();
            } else {
                writeln!(self.output, "    {},", c.start_variant).unwrap();
            }

            // End variant (always a unit)
            writeln!(self.output, "    {},", c.end_variant).unwrap();
        }

        // Leaf variants
        for leaf in &self.config.leaves {
            let has_attrs = !leaf.attrs.is_empty();
            if leaf.text_content {
                writeln!(self.output, "    {}(Cow<'a, str>),", leaf.variant).unwrap();
            } else if has_attrs {
                writeln!(self.output, "    {} {{", leaf.variant).unwrap();
                for attr in &leaf.attrs {
                    writeln!(
                        self.output,
                        "        {}: {},",
                        attr.field_name, attr.rust_type
                    )
                    .unwrap();
                }
                writeln!(self.output, "    }},").unwrap();
            } else {
                writeln!(self.output, "    {},", leaf.variant).unwrap();
            }
        }

        writeln!(self.output, "}}").unwrap();
        writeln!(self.output).unwrap();
    }

    // -----------------------------------------------------------------------
    // OwnedWmlEvent alias
    // -----------------------------------------------------------------------

    fn gen_owned_alias(&mut self) {
        let name = &self.config.enum_name;
        writeln!(
            self.output,
            "/// Owned variant of [`{0}`] with `'static` lifetime.",
            name
        )
        .unwrap();
        writeln!(
            self.output,
            "pub type Owned{0} = {0}<'static>;",
            name
        )
        .unwrap();
        writeln!(self.output).unwrap();
    }

    // -----------------------------------------------------------------------
    // into_owned()
    // -----------------------------------------------------------------------

    fn gen_into_owned(&mut self) {
        let name = &self.config.enum_name;
        writeln!(self.output, "impl<'a> {}<'a> {{", name).unwrap();
        writeln!(
            self.output,
            "    /// Convert borrowed string slices to owned, yielding an `Owned{0}`.",
            name
        )
        .unwrap();
        writeln!(
            self.output,
            "    pub fn into_owned(self) -> Owned{} {{",
            name
        )
        .unwrap();
        writeln!(self.output, "        match self {{").unwrap();

        // Document sentinels
        if let Some(s) = &self.config.document_start {
            writeln!(
                self.output,
                "            {0}::{1} => {0}::{1},",
                name, s
            )
            .unwrap();
        }
        if let Some(e) = &self.config.document_end {
            writeln!(
                self.output,
                "            {0}::{1} => {0}::{1},",
                name, e
            )
            .unwrap();
        }

        // Containers
        for c in &self.config.containers {
            let has_props = c.props_rust_type.is_some();
            let has_attrs = !c.attrs.is_empty();

            if has_props || has_attrs {
                // Destructure
                let mut bindings = Vec::new();
                if has_props {
                    bindings.push("props".to_string());
                }
                for attr in &c.attrs {
                    bindings.push(attr.field_name.clone());
                }
                let binding_str = bindings.join(", ");
                writeln!(
                    self.output,
                    "            {}::{} {{ {} }} => {}::{} {{",
                    name, c.start_variant, binding_str, name, c.start_variant
                )
                .unwrap();
                if has_props {
                    writeln!(self.output, "                props,").unwrap();
                }
                for attr in &c.attrs {
                    let expr = self.owned_expr(&attr.field_name, &attr.rust_type);
                    if expr == attr.field_name {
                        writeln!(self.output, "                {},", attr.field_name).unwrap();
                    } else {
                        writeln!(
                            self.output,
                            "                {}: {},",
                            attr.field_name, expr
                        )
                        .unwrap();
                    }
                }
                writeln!(self.output, "            }},").unwrap();
            } else {
                writeln!(
                    self.output,
                    "            {0}::{1} => {0}::{1},",
                    name, c.start_variant
                )
                .unwrap();
            }
            // End variant (unit)
            writeln!(
                self.output,
                "            {0}::{1} => {0}::{1},",
                name, c.end_variant
            )
            .unwrap();
        }

        // Leaves
        for leaf in &self.config.leaves {
            let has_attrs = !leaf.attrs.is_empty();
            if leaf.text_content {
                writeln!(
                    self.output,
                    "            {0}::{1}(t) => {0}::{1}(Cow::Owned(t.into_owned())),",
                    name, leaf.variant
                )
                .unwrap();
            } else if has_attrs {
                let bindings = leaf
                    .attrs
                    .iter()
                    .map(|a| a.field_name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                writeln!(
                    self.output,
                    "            {}::{} {{ {} }} => {}::{} {{",
                    name, leaf.variant, bindings, name, leaf.variant
                )
                .unwrap();
                for attr in &leaf.attrs {
                    let expr = self.owned_expr(&attr.field_name, &attr.rust_type);
                    // Avoid redundant `field: field` when expr == field name
                    if expr == attr.field_name {
                        writeln!(self.output, "                {},", attr.field_name).unwrap();
                    } else {
                        writeln!(
                            self.output,
                            "                {}: {},",
                            attr.field_name, expr
                        )
                        .unwrap();
                    }
                }
                writeln!(self.output, "            }},").unwrap();
            } else {
                writeln!(
                    self.output,
                    "            {0}::{1} => {0}::{1},",
                    name, leaf.variant
                )
                .unwrap();
            }
        }

        writeln!(self.output, "        }}").unwrap();
        writeln!(self.output, "    }}").unwrap();
        writeln!(self.output, "}}").unwrap();
        writeln!(self.output).unwrap();
    }

    // -----------------------------------------------------------------------
    // WmlStartKind enum and dispatch
    // -----------------------------------------------------------------------

    fn gen_start_kind_enum(&mut self) {
        if self.config.containers.is_empty() {
            return;
        }
        let kname = self.kind_enum_name();
        writeln!(
            self.output,
            "/// Which container element was opened — used by the SAX state machine."
        )
        .unwrap();
        writeln!(self.output, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]").unwrap();
        writeln!(self.output, "pub enum {} {{", kname).unwrap();
        for c in &self.config.containers {
            let kind = c
                .start_variant
                .strip_prefix("Start")
                .unwrap_or(&c.start_variant);
            writeln!(self.output, "    {},", kind).unwrap();
        }
        writeln!(self.output, "}}").unwrap();
        writeln!(self.output).unwrap();
    }

    fn gen_dispatch_start(&mut self) {
        if self.config.containers.is_empty() {
            return;
        }
        let kname = self.kind_enum_name();
        writeln!(
            self.output,
            "/// Map an XML local element name to [`{0}`], if it is a tracked container.",
            kname
        )
        .unwrap();
        writeln!(
            self.output,
            "pub fn dispatch_start(local: &[u8]) -> Option<{0}> {{",
            kname
        )
        .unwrap();
        writeln!(self.output, "    match local {{").unwrap();
        for c in &self.config.containers {
            let kind = c
                .start_variant
                .strip_prefix("Start")
                .unwrap_or(&c.start_variant);
            writeln!(
                self.output,
                "        b\"{}\" => Some({}::{}),",
                c.xml_local, kname, kind
            )
            .unwrap();
        }
        writeln!(self.output, "        _ => None,").unwrap();
        writeln!(self.output, "    }}").unwrap();
        writeln!(self.output, "}}").unwrap();
        writeln!(self.output).unwrap();
    }

    fn gen_props_element(&mut self) {
        let has_any_props = self.config.containers.iter().any(|c| c.props_xml_local.is_some());
        if !has_any_props {
            return;
        }
        let kname = self.kind_enum_name();
        writeln!(
            self.output,
            "/// Return the XML local name of the properties child element for a container, if any."
        )
        .unwrap();
        writeln!(
            self.output,
            "pub fn props_element(kind: {0}) -> Option<&'static [u8]> {{",
            kname
        )
        .unwrap();
        writeln!(self.output, "    match kind {{").unwrap();
        for c in &self.config.containers {
            let kind = c
                .start_variant
                .strip_prefix("Start")
                .unwrap_or(&c.start_variant);
            if let Some(ref px) = c.props_xml_local {
                writeln!(
                    self.output,
                    "        {}::{} => Some(b\"{}\"),",
                    kname, kind, px
                )
                .unwrap();
            } else {
                writeln!(self.output, "        {}::{} => None,", kname, kind).unwrap();
            }
        }
        writeln!(self.output, "    }}").unwrap();
        writeln!(self.output, "}}").unwrap();
        writeln!(self.output).unwrap();
    }

    // -----------------------------------------------------------------------
    // Props-strategy enum
    // -----------------------------------------------------------------------

    /// Generate a `PropsStrategy` enum and a `props_strategy(kind) -> PropsStrategy` fn.
    ///
    /// Three strategies:
    /// - `ChildElement(&'static [u8])` — look for a named child element (pPr, rPr, …)
    /// - `FromAttrs` — parse `is_empty=true` from the container's own start tag
    /// - `None` — no props, container is purely structural
    fn gen_props_strategy(&mut self) {
        if self.config.containers.is_empty() {
            return;
        }
        let has_child = self
            .config
            .containers
            .iter()
            .any(|c| c.props_xml_local.is_some());
        let has_attrs = self
            .config
            .containers
            .iter()
            .any(|c| c.props_from_attrs && c.props_rust_type.is_some());

        if !has_child && !has_attrs {
            return;
        }

        writeln!(self.output, "/// How props are obtained for a container element.").unwrap();
        writeln!(self.output, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]").unwrap();
        writeln!(self.output, "pub enum PropsStrategy {{").unwrap();
        if has_child {
            writeln!(
                self.output,
                "    /// Buffer and parse this named child element."
            )
            .unwrap();
            writeln!(self.output, "    ChildElement(&'static [u8]),").unwrap();
        }
        if has_attrs {
            writeln!(
                self.output,
                "    /// Parse the container start tag's own attributes (is_empty=true)."
            )
            .unwrap();
            writeln!(self.output, "    FromAttrs,").unwrap();
        }
        writeln!(
            self.output,
            "    /// No props; container is purely structural."
        )
        .unwrap();
        writeln!(self.output, "    None,").unwrap();
        writeln!(self.output, "}}").unwrap();
        writeln!(self.output).unwrap();

        let kname = self.kind_enum_name();
        writeln!(
            self.output,
            "/// Return the props strategy for a container kind."
        )
        .unwrap();
        writeln!(
            self.output,
            "pub fn props_strategy(kind: {0}) -> PropsStrategy {{",
            kname
        )
        .unwrap();
        writeln!(self.output, "    match kind {{").unwrap();
        for c in &self.config.containers {
            let kind = c
                .start_variant
                .strip_prefix("Start")
                .unwrap_or(&c.start_variant);
            if let Some(ref px) = c.props_xml_local {
                writeln!(
                    self.output,
                    "        {}::{} => PropsStrategy::ChildElement(b\"{}\"),",
                    kname, kind, px
                )
                .unwrap();
            } else if c.props_from_attrs && c.props_rust_type.is_some() {
                writeln!(
                    self.output,
                    "        {}::{} => PropsStrategy::FromAttrs,",
                    kname, kind
                )
                .unwrap();
            } else {
                writeln!(
                    self.output,
                    "        {}::{} => PropsStrategy::None,",
                    kname, kind
                )
                .unwrap();
            }
        }
        writeln!(self.output, "    }}").unwrap();
        writeln!(self.output, "}}").unwrap();
        writeln!(self.output).unwrap();
    }

    // -----------------------------------------------------------------------
    // is_text_element dispatch
    // -----------------------------------------------------------------------

    /// Generate `is_text_element(local: &[u8]) -> bool` for leaves with `text_content: true`.
    ///
    /// These elements carry text content (start/text/end) rather than being self-closing.
    /// The SAX iterator must read their text rather than skipping them.
    fn gen_is_text_element(&mut self) {
        let text_leaves: Vec<_> = self
            .config
            .leaves
            .iter()
            .filter(|l| l.text_content)
            .collect();

        if text_leaves.is_empty() {
            return;
        }

        writeln!(
            self.output,
            "/// Return true if this XML local element name is a text-content leaf."
        )
        .unwrap();
        writeln!(
            self.output,
            "/// The SAX iterator reads the element's text content and emits a text event."
        )
        .unwrap();
        writeln!(
            self.output,
            "pub fn is_text_element(local: &[u8]) -> bool {{"
        )
        .unwrap();
        writeln!(self.output, "    matches!(local,").unwrap();
        for (i, leaf) in text_leaves.iter().enumerate() {
            let sep = if i + 1 < text_leaves.len() { " |" } else { "" };
            writeln!(self.output, "        b\"{}\"{}", leaf.xml_local, sep).unwrap();
        }
        writeln!(self.output, "    )").unwrap();
        writeln!(self.output, "}}").unwrap();
        writeln!(self.output).unwrap();
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Generate the `into_owned()` expression for a single field given its Rust type.
    fn owned_expr(&self, field_name: &str, rust_type: &str) -> String {
        if rust_type.contains("Cow") {
            if rust_type.starts_with("Option") {
                // Option<Cow<'a, str>>
                format!(
                    "{}.map(|v| Cow::Owned(v.into_owned()))",
                    field_name
                )
            } else {
                // Cow<'a, str>
                format!("Cow::Owned({}.into_owned())", field_name)
            }
        } else {
            // Plain Copy types (i32, bool, etc.)
            field_name.to_string()
        }
    }
}
