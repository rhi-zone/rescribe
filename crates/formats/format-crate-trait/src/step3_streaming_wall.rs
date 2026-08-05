//! Step 3: `StreamingParser<H: Handler>` — the hard part, and where the
//! spike hits a real wall.
//!
//! ## The concrete problem
//!
//! `opml_fmt::batch::StreamingParser<H: opml_fmt::batch::Handler>` and
//! `zip_fmt::batch::StreamingParser<H: zip_fmt::batch::Handler>` are each
//! generic over **that crate's own, distinct, concrete `Handler` trait**:
//!
//! - `opml_fmt::batch::Handler::handle(&mut self, event: opml_fmt::events::OwnedEvent)`
//! - `zip_fmt::batch::Handler::handle(&mut self, event: zip_fmt::batch::Event)`
//!   (note: **not** `zip_fmt::events::Event` — see below)
//!
//! A shared `FormatCrate` trait wants one generic signature, e.g.
//! `type StreamingParser<H: Handler<Self::BatchEvent>>`, where `Handler<E>`
//! is a trait *this* crate defines once. For that associated type to equal
//! the crate's real `StreamingParser<H>`, every `H` satisfying our bound
//! must also satisfy the crate's own `Handler` trait. Rust has no
//! "associated trait" mechanism to make the *bound itself* vary per impl
//! (only associated types/consts/fns) — so the only way to bridge our
//! generic `Handler<E>` to each crate's own concrete `Handler` trait is a
//! blanket impl:
//!
//! ```rust,ignore
//! impl<H: Handler<opml_fmt::events::OwnedEvent>> opml_fmt::batch::Handler for H {
//!     fn handle(&mut self, event: opml_fmt::events::OwnedEvent) {
//!         Handler::handle(self, event)
//!     }
//! }
//! ```
//!
//! This is a coherence/orphan-rule violation: `impl ForeignTrait for H`
//! where `H` is an uncovered generic type parameter is disallowed
//! regardless of trait bounds on `H` — the orphan check (RFC 1023) only
//! asks whether a *local type* appears before the first uncovered type
//! parameter; bounds don't count. See the commented-out block below for
//! the actual `E0210` this produces (left commented so the crate compiles;
//! uncomment to reproduce).
//!
//! ## What this rules out
//!
//! A single trait definition (in a third crate, unrelated to both format
//! crates) **cannot** retroactively unify two *already-published*,
//! independently-defined `Handler` traits behind one generic bound,
//! because bridging them requires a blanket impl that the orphan rule
//! forbids. This is fixable only by changing the format crates
//! themselves — either:
//!
//! (a) each crate's own `Handler` trait becomes generic
//!     (`pub trait Handler<E> { fn handle(&mut self, event: E); }`) and
//!     `StreamingParser<H: Handler<Self::Event>>` is expressed with the
//!     *shared* `Handler<E>` trait living in a common low-level crate that
//!     every `-fmt` crate depends on (not this spike's arrangement, where
//!     the format crates predate and know nothing of the trait); or
//!
//! (b) the shared trait's `StreamingParser` associated type is not
//!     generic over an arbitrary `H` at all, but is itself a concrete type
//!     erasing to `dyn` dispatch — which the task's own constraint (never
//!     `dyn`, preserve monomorphization) rules out.
//!
//! Option (a) is real but is a **different, larger** change than "add a
//! trait": it means either introducing a shared `Handler<E>` trait in a
//! new common crate that `opml-fmt`/`zip-fmt`/all 44 crates depend on and
//! *changing every crate's existing `Handler` trait* to be that one
//! (a breaking API change to each `-fmt` crate, not just an additive
//! trait layered on top) — or accepting that the unifying trait can cover
//! `parse`/`emit`/`events` but must leave `StreamingParser` out of the
//! unification (each crate keeps its own concrete, non-generic `Handler`,
//! documented by convention only, same as today).
//!
//! This spike does **not** attempt option (a) — rewriting two published
//! crates' `Handler` traits is out of scope for a feasibility spike and
//! was not requested. The wall reported here is specific: *retrofitting*
//! a shared generic `StreamingParser` bound onto crates whose `Handler`
//! traits already exist as distinct concrete types is not possible
//! without either an orphan-rule-violating blanket impl or a breaking
//! change to the format crates.

// Our own generic Handler trait, defined fresh in this crate (not
// borrowed from either format crate).
pub trait Handler<E> {
    fn handle(&mut self, event: E);
}

// Reproducing the orphan-rule wall for real. Uncomment to see E0210.
//
// impl<H: Handler<opml_fmt::events::OwnedEvent>> opml_fmt::batch::Handler for H {
//     fn handle(&mut self, event: opml_fmt::events::OwnedEvent) {
//         Handler::handle(self, event)
//     }
// }

/// A second, independent finding surfaced while investigating this: even
/// *within* `zip-fmt` alone, the pull-iterator `events()` API and the
/// push-based `StreamingParser`/`Handler` API do not share an event type.
/// `zip_fmt::events::Event<'a>` (borrowed, `Cow`-based, yielded by
/// `EventIter`) and `zip_fmt::batch::Event` (owned, yielded to `Handler`)
/// are two distinct enums with overlapping but not identical shape — per
/// that module's own doc comment, this is deliberate: a `Handler`-based
/// blanket impl over a lifetime-generic event type runs into an HRTB
/// closure-inference limitation, so `zip-fmt` (and `opml-fmt`) settled on
/// an owned-only event type for `Handler` specifically to keep
/// `StreamingParser::new(|ev| ...)` usable with a plain closure literal.
///
/// This means a trait with **one** `type Event<'a>` GAT shared between
/// `events()` and `StreamingParser`'s handler — as the task's sketch
/// implies — does not even match the *existing* API shape of `zip-fmt`
/// today: the real crate already needs two event types (one borrowed +
/// lifetime-generic for the pull API, one owned for the push API), not
/// one. Any shared trait must model that as two separate associated
/// types/GATs, not one `Event<'a>` reused for both.
#[cfg(test)]
mod evidence {
    #[test]
    fn zip_fmt_batch_event_is_a_distinct_type_from_the_pull_event() {
        // Type-level evidence only: this compiles iff the two are in fact
        // distinct types (no impl From/Into assumed, no shared trait).
        fn assert_distinct<A, B>() {}
        assert_distinct::<zip_fmt::events::Event<'static>, zip_fmt::batch::Event>();
    }
}
