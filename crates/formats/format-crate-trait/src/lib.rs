//! EXPLORATORY SPIKE — not wired into rescribe, not a public API.
//!
//! Prototype of a shared `FormatCrate` trait unifying the 5-API contract
//! documented in `docs/format-library-design.md` (parse/events/
//! StreamingParser/emit/Writer) across `-fmt` crates, tried against two
//! real crates: `opml-fmt` (simple, well-nested XML) and `zip-fmt` (real
//! streaming complexity, hand-rolled push parser).
//!
//! See the crate's accompanying report (delivered to the requester, not
//! committed here) for the full writeup. Summary of what actually
//! compiles vs. what hits a wall is in the module docs of `step1`..`stepN`.

pub mod step1_parse_emit;
pub mod step2_events;
pub mod step3_streaming_wall;
