//! Shared pulldown-cmark `Options` construction and small value-conversion
//! helpers, factored out so they're available to every reader API mode
//! (`parse`, `events`, `batch`) regardless of which of those feature-gated
//! modules is compiled — none of them depend on each other.
//!
//! This module holds only *bit values* and trivial type conversions, not
//! event-translation logic — `parse.rs` and `events.rs` remain independent
//! implementations per the crate's "independent implementations, not derived
//! from one another" rule (see repository CLAUDE.md).

use pulldown_cmark::Options;

/// Distinguishes YAML (`---`) from TOML (`+++`) front-matter delimiters.
///
/// Defined here (not in `ast.rs`) for the same reason as [`ColumnAlignment`]:
/// available under `reader-streaming` without requiring `reader-ast`.
#[cfg(feature = "frontmatter")]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum FrontMatterKind {
    Yaml,
    Toml,
}

/// Build the pulldown-cmark [`Options`] bitset for the construct features
/// enabled at compile time.
pub(crate) fn build_options() -> Options {
    #[allow(unused_mut)]
    let mut opts = Options::empty();
    #[cfg(feature = "strikethrough")]
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    #[cfg(feature = "tables")]
    opts.insert(Options::ENABLE_TABLES);
    #[cfg(feature = "task-lists")]
    opts.insert(Options::ENABLE_TASKLISTS);
    #[cfg(feature = "frontmatter")]
    {
        opts.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        opts.insert(Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS);
    }
    opts
}

/// Per-column alignment for a GFM table.
///
/// Defined here (not in `ast.rs`) so it's available to `events.rs` under
/// `reader-streaming` even when `reader-ast` is disabled — neither reader API
/// mode module depends on the other.
#[cfg(feature = "tables")]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum ColumnAlignment {
    None,
    Left,
    Center,
    Right,
}

#[cfg(feature = "tables")]
pub(crate) fn pd_alignment_to_ast(a: &pulldown_cmark::Alignment) -> ColumnAlignment {
    match a {
        pulldown_cmark::Alignment::None => ColumnAlignment::None,
        pulldown_cmark::Alignment::Left => ColumnAlignment::Left,
        pulldown_cmark::Alignment::Center => ColumnAlignment::Center,
        pulldown_cmark::Alignment::Right => ColumnAlignment::Right,
    }
}
