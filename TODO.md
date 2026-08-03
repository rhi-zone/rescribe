## Status Indicator
- Current: ◐ Fleshed Out — kept despite high commit count (112 commits, 207 Rust files)
- Needs hardening/verification work before upgrading to ● Potentially Mature
- Lots of code, but needs more verification to count as mature

# Rescribe Roadmap

Per-format status is tracked in `docs/format-audit.md` using the maturity pipeline
(0-Stub → 1-Partial → 2-Fixtures → 3-Harness → 4-Fuzz → 5-Production).
This file describes milestones, format tiers, and cross-cutting work.

**2026-08-01: rtf-fmt wired into the cross-API harness (`events`/`StreamingParser`/streaming
`Writer`); `rtf` removed from `streaming_harness::NOT_YET_AUDITED`.** `events()`
(`sem_events::events`) is a lazy frame-stack walk of the AST `parse()` already built — same
"not independently implemented reader" pattern already established as `Wired` for
t2t/pod/haddock/fountain/asciidoc/man — and passes an exact events()-vs-AST-projection check
across all 38 `fixtures/rtf/*` cases (`sem_events::Event` gained `#[derive(PartialEq)]`,
previously only `Debug`, to make the exact comparison possible;
`crates/formats/rtf-fmt/src/sem_events.rs:29`).

Two real, confirmed defects found and documented as new `streaming_harness::KNOWN_FAILURES`
entries (not fixed — both are structural, out of scope for a harness-wiring pass per the task's
"small, clearly in-scope" bar):

- `batch::StreamingParser` (`crates/formats/rtf-fmt/src/batch.rs:107-139`) is a confirmed
  buffer-then-finish stub: `feed()` only appends to an internal `Vec<u8>` and `finish()` calls
  `sem_events::events(&self.buf)` exactly once. The module's own doc argues this is an
  "inherent property of the RTF format" (font/color tables must be parsed before body content is
  interpretable) rather than an implementation shortfall — per this harness's explicit rule
  (a buffer-then-finish stub is always `KnownFailure`, never `NotApplicable`, regardless of the
  format's structural excuse) it is tracked as `KnownFailure` anyway. Directly verified: `feed()`
  alone, without `finish()`, delivers zero events to the handler for any input.
- `writer::Writer` (`crates/formats/rtf-fmt/src/writer.rs`) only accepts the low-level
  `TokenEvent` type (from `token_events()`), not the crate's own semantic `Event`/`OwnedEvent`
  type that `events()`/`StreamingParser` produce — unlike every other `Wired` format in this
  table, rtf-fmt has no writer that consumes its own semantic event stream at all. `Writer`
  itself writes directly to its sink on every `write_event()` call (genuinely incremental, not
  buffer-then-finish — the incrementality probe passes), but its delimiter-space policy
  (`writer.rs:57-68`: always a trailing space after a no-param `ControlWord`, never after a
  `Some(param)` one) systematically diverges from `emit()`'s own placement policy, confirmed on
  fixture `fixtures/rtf/adjacent_bold`: `build()` emits
  `\rtf1\ansi\deff0{\fonttbl{\f0 Times New Roman;}}` but re-tokenizing that same output via
  `token_events()` and feeding it back through `Writer` produces
  `\rtf1\ansi \deff0{\fonttbl {\f0Times New Roman;}}`.

`docs/format-audit.md`'s `rtf-fmt` row updated: `batch` ` ` → `~§`, `w-stream` ` ` → `✓§¥` (new
`¥` footnote explaining the TokenEvent/semantic-Event API-level mismatch). `multimarkdown`/`pdf`
remain in `NOT_YET_AUDITED` (confirmed via `crates/formats/` directory listing: neither has a
standalone `-fmt` crate — `rescribe-read-multimarkdown` depends on `pulldown-cmark` directly and
`rescribe-read-pdf` has no `pdf-fmt` sibling — same class of gap as `latex`, out of scope here).
`native`/`csv`/`tsv`/`ris` were not reached this pass; still `NOT_YET_AUDITED`.

**2026-07-31: texinfo `StreamingParser` hollow-reader defect fixed (reader-side counterpart to
the `texinfo` `streaming_writer` fix documented further below in this file, dated
2026-07-30/2026-07-31).** `texinfo::batch::StreamingParser::feed()`
(`crates/formats/texinfo/src/batch.rs`) used to just `self.buf.extend_from_slice(chunk)` and
only call `crate::events::events(&s)` inside `finish()` — implementing the feed/finish contract
while being architecturally O(full input), exactly the "buffer all input until finish()" pattern
CLAUDE.md explicitly rejects for hand-rolled parsers. The module's own doc comment claimed this
was required because "Texinfo requires the full input for correct parsing (forward references,
@set/@value, etc.)" — false: `@set`/`@value` are not implemented anywhere in `parse.rs` (grep
confirms `@set ` is a plain skip, no variable substitution ever happens), so that rationale did
not hold up.

Fix: `StreamingParser` now line-buffers input and splits it into top-level units — a paragraph,
a heading, or an `@directive ... @end directive`-delimited environment (list, definition list,
multitable, code block, quotation, menu, float, conditional block) — flushing each unit (via a
fresh `crate::events::events()` call on just that unit's text) to the handler as soon as its
boundary is confirmed by the input seen so far. Memory is O(largest unit), not O(full input).
`@settitle` (document-level metadata; `events()` always emits one `Title` event before all block
events, regardless of where `@settitle` appears in the source) is handled specially: buffered
and emitted once, immediately before the first content-producing flush — correct for the
universal case (and Texinfo's own authoring convention) where `@settitle` precedes all content,
which is the only pattern present in the fixture suite (one occurrence, always the first line).

Investigation finding, not fixed here (out of scope for this task): `parse.rs` itself has zero
nesting awareness for `@itemize`/`@enumerate`/etc. — nesting a same-type environment inside
itself (e.g. `@itemize` inside `@itemize`) causes the *outer* scan to stop at the *first* `@end
itemize`, whichever one that is, silently dropping the remaining content. Confirmed directly:
parsing `@itemize\n@item outer one\n@itemize\n@item inner one\n@end itemize\n@item outer
two\n@end itemize\n` produces one three-item list (with the inner `@itemize` line itself
misparsed as a malformed `@item` reading "ize") and silently drops `@item outer two` and the
final `@end itemize` entirely. The streaming splitter deliberately mirrors this exact flat
(non-nesting) boundary detection rather than second-guessing it with a proper nesting-depth
stack (the fix org-fmt's sibling `batch.rs` bug used) — a depth-aware splitter would produce
*different* top-level unit boundaries than a full-document `parse()` call does for nested
environments, which would make `StreamingParser` diverge from `events()` on such input: the
opposite of "genuinely incremental but behaviorally identical." Fixing nested-environment
parsing is a separate, larger `parse.rs` change (all 8 scanner functions), tracked here as a
known gap, not attempted in this fix.

Verified via: (1) `crates/rescribe-fixtures/tests/streaming_apis.rs`'s
`texinfo_streaming_parser_matches_events_under_adversarial_chunking` — `StreamingParser` under
whole/single-byte/chunk-of-N/mid-UTF-8-split chunking matches `events()` byte-for-byte over the
full `fixtures/texinfo/` suite (73 fixtures, including `comp-nested-lists`, which exercises the
flat-nesting behavior above); (2) a new deterministic
`texinfo_streaming_parser_delivers_events_incrementally` test proving `feed()` delivers a
completed unit's events before `finish()` is called, and that a second, still-incomplete unit
fed afterward does not retroactively change what was already delivered; (3) a new
`crates/formats/texinfo/tests/streaming_parser_memory.rs` peak-live-heap guard (thread-unique
`#[global_allocator]`, same pattern as `tikiwiki/tests/streaming_writer_memory.rs`) feeding a
synthetic multi-section document in 61-byte chunks. Measured peak live heap above baseline for a
10x increase in synthetic-document size (50 → 500 sections, 10,870 → 109,320 input bytes):
**before fix: 357,456 B → 3,117,042 B (8.72x — O(full input), confirmed by reverting to the old
implementation via `git stash`)**; **after fix: 3,429 B → 3,429 B (1.00x — flat, O(largest
unit))**. `streaming_harness.rs`'s `KNOWN_FAILURES` entry for `texinfo`/`streaming_parser` was
removed and `CAPABILITIES`' `streaming_parser` promoted to `ApiState::Wired`; `BatchParser` and
`BatchSink` are unchanged (both remain deliberate buffer-then-`finish()` shims per their own doc
comments, matching `org-fmt`/`rst-fmt`'s sibling `BatchParser`/`BatchSink`).

---

**2026-07-31: xwiki `streaming_parser` hollow-reader defect fixed (reader-side counterpart to
the writer fix below).** `xwiki::batch::StreamingParser::feed()` was a bare
`self.buf.extend_from_slice(chunk)`; all parsing happened inside `finish()`, which called
`parse::parse` over the full reassembled buffer then walked it with `events::events` — the
"buffer all input until finish()" shape CLAUDE.md explicitly rejects for hand-rolled parsers.
Tracked as a `KnownFailure` (format: "xwiki", api: "streaming_parser") in
`crates/rescribe-fixtures/src/streaming_harness.rs`.

Rewrote `StreamingParser` (`crates/formats/xwiki/src/batch.rs`) to a line-buffered,
block-boundary-aware state machine mirroring the boundaries `xwiki::parse::Parser::parse`'s own
dispatch loop uses: one top-level block at a time (paragraph, heading, horizontal rule, list,
table, code block, `{{quote}}`/`{{name}}` macro block, self-closing macro). `feed()` accumulates
lines until a block's boundary is confirmed, then flushes it — reparsed in isolation via
`parse::parse` and walked with `events::events`, forwarding every event to the handler — before
the next block starts accumulating. Only the current incomplete trailing block is buffered, not
the whole document. `BlockState::MacroBlock(String)`/`CodeBlock`/`QuoteBlock` track macro/code
bodies as an explicit state (not a boolean flag), so a blank line inside a multi-line macro body
doesn't cause a premature flush — matching `try_parse_block_macro_start`/
`try_parse_self_closing_macro`, both changed from private to `pub(crate)` so `batch.rs` could
reuse them instead of duplicating detection logic. XWiki's own recursive-descent parser doesn't
track macro nesting depth either (it closes on the first line containing the matching
`{{/name}}`, regardless of nested same-name macros), so the streaming parser intentionally
matches that exact behavior rather than introducing a depth counter that would diverge from
`parse()`/`events()`. Confirmed no cross-block state exists in xwiki's grammar (no
reference-style link definitions or similar forward references), so reparsing one block at a
time is safe.

Measured (test-local `thread_local!` tracking allocator, shared with `writer.rs`'s existing
`alloc_guard` module since only one `#[global_allocator]` may exist per test binary; 5000-section
~490KB synthetic document fed in 64-byte chunks, vs. 500 sections as baseline): peak memory
**1,969 bytes flat regardless of document size** (500 sections and 5000 sections both measured
exactly 1,969 bytes above baseline) for the new `StreamingParser`, vs. the old
buffer-then-finish implementation's **1,296,864 bytes at 500 sections → 12,738,976 bytes at
5000 sections** (~9.8x growth, near-linear with input size, confirming `O(full document)`).

Adversarial-chunking tests (whole input, single-byte, 3/7/16/64-byte chunks, and a dedicated
mid-UTF-8-character case) added to `xwiki`'s own test module (`batch.rs`), covering every
top-level construct including a macro/quote block with an internal blank line — all pass
byte-for-byte identical to `events()` on the whole input. No formatting or block-boundary bugs
were surfaced (unlike the org/asciidoc/djot/t2t/fountain-fmt streaming-writer rewrites earlier
this session, which each found at least one real bug); xwiki's parser dispatch logic ported
over cleanly to line-by-line boundary detection on the first attempt.

Updated `crates/rescribe-fixtures/src/streaming_harness.rs`: promoted xwiki's
`streaming_parser` from `ApiState::KnownFailure(...)` to `ApiState::Wired`, removed the matching
`KnownFailure { format: "xwiki", api: "streaming_parser", ... }` entry, and updated four stale
"unlike xwiki/muse-fmt" comparison comments (zimwiki/markua capability comments and their
matching `streaming_apis.rs` doc-comments) that had cited xwiki alongside muse-fmt as a
buffer-then-finish counter-example — now correctly say "like xwiki's (fixed 2026-07-31) but
unlike muse-fmt's." Also replaced `xwiki_streaming_parser_matches_events_and_is_incremental`'s
old per-fixture mid-input-cutoff incrementality probe (which fed exactly the first half of every
fixture's bytes and asserted at least one event delivered) with a probe against a dedicated
synthetic two-block document: the old per-fixture version had a structural false-positive risk
that surfaced immediately — the real `blockquote` fixture is a single indivisible `{{quote}}`
block spanning its entire ~39-byte file, so its first half necessarily lands mid-body before the
closing `{{/quote}}` is seen, and zero events at that point is correct (the block genuinely
isn't finished yet), not a buffer-then-finish defect. This mirrors the same fixture-shape caveat
already documented for the jats-fmt entry above. The adversarial-chunking correctness loop
(comparing `StreamingParser` output to `events()` over the whole fixture suite) is retained
unchanged and now exercises real per-block state transitions rather than passing trivially by
construction.

---

**2026-07-31: fb2-fmt `StreamingParser` rewritten from buffer-everything to true incremental
XML draining (reader-side counterpart of the streaming-writer sweep).** `StreamingParser::feed`
used to just `extend_from_slice` into a `Vec<u8>` and parse the whole thing in `finish()` —
`O(full input)` memory despite the crate's own `events()`/`EventIter` already being a genuine
incremental `quick_xml` pull parser underneath. Fixed by splitting the semantic dispatch logic
(`crates/formats/fb2-fmt/src/events.rs`) out of `XmlEventIter` (which still holds a whole-slice
`Reader`, used by `events()`) into a reusable `SemanticState::dispatch(&XmlEvent)` that consumes
one already-decoded token at a time with no dependency on how its bytes were obtained.
`StreamingParser::drain` then reuses `SemanticState` while rebuilding a fresh `quick_xml::Reader`
over only the unconsumed tail of its byte buffer on each call — the same "rebuild over the tail,
`Err(Syntax)` means wait for more bytes" technique `docbook-fmt::batch::StreamingParser`
pioneered, including the same ambiguous-trailing-text handling (plain text terminates at either
`<` or genuine EOF, indistinguishable from a slice-bounded reader, so a text run consuming every
currently-buffered byte waits for confirmation via a later `feed()`/`finish()` call).

One real bug surfaced by the adversarial-chunking fixture test during this rewrite: disabling
quick-xml's own `check_end_names`/`allow_unmatched_ends` (architecturally required — each
`drain()`'s fresh `Reader` has no memory of Start tags consumed by a previous call) silently
dropped the tag-balance validation `events()` gets for free from its single long-lived `Reader`,
so `StreamingParser` continued past a mismatched end tag that `events()` correctly treats as
fatal and stops on (fixture `adv-malformed-xml`: an unterminated `<FictionBook xmlns="..."`
start tag swallows the following `<body>` open tag, leaving `</body>`/`</FictionBook>` as
unmatched closes) — producing a spurious extra `EndFictionBook` event `events()` never emits.
Fixed via a hand-tracked `open_stack: Vec<String>` + `failed` flag replicating quick-xml's
validation by hand, same shape as docbook-fmt's own `open_stack`.

A second real defect was found in the *test*, not the production code: the harness's original
incrementality probe fed an arbitrary 50%-byte split of each real fixture and asserted at least
one event was delivered — but a 50% split of `fixtures/fb2/adv-empty` (138 bytes) lands
mid-attribute-value inside the still-open root `<FictionBook xmlns="...` start tag, so zero
events at that exact split point is the correct, spec-conforming answer, not a defect (the same
probe-naivety class already documented for jats-fmt). Replaced with a hand-built probe input
with a guaranteed unambiguous complete-prefix boundary (a complete
StartFictionBook/StartBody/StartSection/StartParagraph/Inline/EndParagraph sequence followed by
deliberately unterminated trailing tags), matching the pattern bbcode-fmt's equivalent probe
already used.

Confirmed via a new adversarial-chunking test in `events.rs` (whole/single-byte/chunks-of-3/7/13/
mid-UTF-8-character-split, byte-for-byte equal to `events()` on a synthetic multi-section
document) and a new peak-memory guard using the `thread_local!` allocator-tracking pattern
(per-thread, not a shared `AtomicUsize`, so a concurrently-running unrelated test on another
thread can't inflate the measurement): peak allocated bytes stayed flat (~3090 bytes @200
synthetic sections vs ~3091 bytes @2000 sections, a 10x input-size increase) — confirming
`O(largest token)`, not `O(full input)`.

`streaming_harness::KNOWN_FAILURES`'s `fb2`/`streaming_parser` entry removed;
`CAPABILITIES.fb2.streaming_parser` promoted to `Wired`. `fb2`/`events` and
`fb2`/`streaming_writer` KnownFailure entries are untouched — both are downstream of a
different, still-open defect (`events()` silently drops `Metadata` for input lacking a literal
`<description>` element), unrelated to this fix.

---

**2026-07-31: muse-fmt `streaming_parser` hollow-reader defect fixed.** `muse_fmt::batch::
StreamingParser::feed()` was a bare `buf.extend_from_slice()`; all parsing happened in
`finish()`, which called `parse::parse()` then walked the result with `events::events()` — the
crate's own module docs admitted this outright ("Muse's block-level structure makes true
incremental parsing difficult without a dedicated state machine"). That claim did not hold up:
Muse's top-level grammar (headings, paragraphs, lists, tables, definition lists, indented code,
footnote defs, comments, and `<example>`/`<verse>`/`<quote>`/`<center>`/`<right>`/`<literal>`/
`<src ...>`/`<comment>` tag blocks) has **no cross-block state** — every `parse_*` method in
`parse.rs` only ever looks at its own block's lines, never references anything before or after
it (footnote *references* don't resolve against footnote *definitions* either — `events()`
just emits the label). The only genuine document-wide state is the `#title`/`#author`/`#date`/
`#desc`/`#keywords` header, which is only meaningful at the very start of the document.

Rewrote `StreamingParser::feed()` into a genuine line-buffered block splitter: it accumulates
lines only until a top-level block boundary is confirmed (a blank line, a line starting a
different kind of block, a tag block's own closing tag, or a single-line construct like a
heading/comment/footnote-def/horizontal-rule), then immediately re-parses just that block's
text via a new `crate::parse::parse_blocks` — which runs `Parser::parse_block_loop` *without*
the document-header phase, so a `#`-led line appearing mid-document (not at the true start) is
never misread as a header directive by the isolated re-parse — and forwards its events to the
handler, before `finish()` is ever called. The document header itself is handled by a small
dedicated state in `StreamingParser` (accumulating `#`-directive lines, skipping blanks,
ending on the first non-matching line) that emits exactly one `Metadata` event, matching
`events()`'s single always-present `Metadata` event.

To keep the splitter's block-boundary decisions from drifting out of sync with `Parser::
parse_block_loop`'s own dispatch order, extracted the dispatch's `if`/`else if` conditions into
shared pure predicate functions in `parse.rs` (`heading_level`, `is_over_leveled_heading`,
`is_horizontal_rule`, `is_unordered_list_start`, `is_ordered_list_item`,
`is_definition_list_line`, `is_indented_code_start`, `is_footnote_def_start`,
`tag_open_close`, plus `is_table_row`/`is_inline_tag_line` promoted to `pub(crate)`) and
refactored `Parser::parse_block_loop`/`parse_paragraph`/`try_parse_heading`/
`try_parse_footnote_def` to call them instead of duplicating the conditions inline — so both
the parser's own dispatch and the streaming splitter's boundary classification are now the
*same* function calls, not two hand-copied implementations that could silently diverge. (Two
existing inconsistencies in the original code were deliberately preserved rather than "fixed"
during this refactor, since fixing them wasn't in scope: `parse_paragraph`'s horizontal-rule
break check used `line.starts_with("----")`, not the trimmed `is_horizontal_rule`; and its
unordered-list break check only covered `" - "`, not `"  - "` — both are documented inline at
the call site, and neither changes behavior since indented-code's own break check already
subsumes the `"  - "` case.) Muse's tag blocks do not support nesting in `parse()` itself
(each looks for the *first* occurrence of its own closing tag, regardless of any nested
same-tag open) — `StreamingParser` intentionally reproduces that rather than "fixing" it, to
stay aligned with `events()`.

Verified via adversarial chunking (whole input, single-byte, chunks-of-7, chunks-of-37,
mid-UTF-8-character, mid-tag-block, header-only, and a `#`-led non-header line mid-document) —
`StreamingParser` output matches `events()` byte-for-byte in every case, including this
harness's own fixture-suite incrementality probe (`crates/rescribe-fixtures/tests/
streaming_apis.rs::muse_streaming_parser_matches_events_and_is_incremental`, 87 fixtures).
Peak memory (test-local `thread_local!` tracking allocator, feeding 64-byte chunks of a
synthetic multi-block document, discarding sink): **before** (buffer-then-finish) 624,826
bytes at a ~11KB (50-section) input growing to 5,684,358 bytes at a ~113KB (500-section, 10x)
input — scaling almost exactly linearly with input size, confirming O(full document); **after**
(this fix) 7,680 bytes at *both* sizes — flat regardless of document size, confirming
O(largest block). That's roughly an 81x reduction at the smaller size and ~740x at the larger
one.

`crates/rescribe-fixtures/src/streaming_harness.rs`'s `CAPABILITIES` table flips muse's
`streaming_parser` from `ApiState::KnownFailure` to `ApiState::Wired`; the matching
`KNOWN_FAILURES` entry was removed. `crates/rescribe-fixtures/tests/streaming_apis.rs`'s
doc comments and in-test descriptions referencing muse-fmt's (formerly fake) `StreamingParser`
— including three other formats' comparison comments ("unlike xwiki/muse-fmt") — were updated
to reflect that muse-fmt's `StreamingParser` is now also genuinely incremental.
`docs/format-audit.md`'s muse row `streaming_parser` column was updated to match. Verified
`cargo clippy -p muse-fmt --all-targets --all-features -- -D warnings`,
`cargo test -p muse-fmt -q`, `cargo test -p rescribe-fixtures -q`, `cargo fmt --check -p
muse-fmt`, and the full workspace `cargo clippy --all-targets --all-features -- -D warnings &&
cargo test -q && cargo fmt --check` all pass clean.

---

**2026-07-31: pod-fmt `StreamingParser` (reader side) rewritten from buffer-then-finish to
genuine incremental block parsing.** `crates/formats/pod-fmt/src/batch.rs`'s `StreamingParser`
was, until this fix, explicitly self-documented as buffer-then-finish ("POD documents are
always small enough to buffer fully, so this implementation accumulates all input and parses
on finish()") — `feed()` only did `self.buf.extend_from_slice(chunk)`, and all parsing/event
delivery happened inside `finish()`. Per CLAUDE.md, a crate's own "small enough to buffer"
rationale is not a sanctioned exemption (only commonmark-fmt's pulldown-cmark wrapping is), so
this was tracked as a `KnownFailure` in `rescribe-fixtures/src/streaming_harness.rs`
(`format: "pod", api: "streaming_parser"`), pinned via a `feed()`-before-`finish()`
incrementality probe. The companion streaming *writer* fix landed the same day (see below);
this is the reader-side counterpart.

Design: a line-buffered block-splitting state machine (`State::{Idle,Paragraph,Verbatim,
List{depth},BeginEnd}` in `batch.rs`) mirroring `pod_fmt::parse::Parser::parse_blocks`'s own
per-line dispatch order exactly — headings/`=for`/`=encoding`/stray `=item`/`=back`/`=end`/
unknown commands flush as single-line blocks; `=over`...`=back` lists are tracked by nesting
depth (incremented on each nested `=over`, decremented on each `=back`; a `=cut` at *any*
depth unwinds and flushes the whole list immediately, matching `parse_list`'s own
unconditional `=cut` break propagating up through every recursion level — traced by hand
through the nested-item-body-loop-then-outer-loop control flow); `=begin FORMAT`...`=end
FORMAT` regions accumulate to their literal `=end` line without matching the format identifier
and without being `=cut`-aware (raw content is never POD-interpreted, matching
`parse_begin_end`'s behavior exactly); ordinary paragraphs end on a blank line, a command
line, or an indented line; verbatim blocks continue through blank lines and only end on a
non-blank, non-indented line. Each confirmed-complete block is re-parsed standalone via
`crate::parse::parse()` and its events forwarded via `EventIter` — the same
re-parse-in-isolation shape rst-fmt/other formats' batch.rs use.

One piece of state genuinely spans blocks: `in_pod` (POD's own cross-block flag, set by
`=pod`/`=head*`/`=over`, cleared by `=cut`; content lines outside POD mode are dropped
entirely by `parse::parse` itself). `StreamingParser` tracks the same flag across `feed()`
calls at its `Idle`-state top-level dispatch (matching `parse_blocks`'s own gate order line by
line), and — since a standalone re-parse of just a paragraph/verbatim block's lines would
otherwise start with `in_pod = false` and silently drop the content — prefixes those two block
kinds' isolated re-parse text with a synthetic leading `=pod` line (itself producing no event)
to reproduce the same state. List/`=begin` blocks start with a `=` line and need no such
prefix (POD's own `in_pod` gate only applies to non-`=` lines).

Unlike the org/asciidoc/t2t/djot bug class this session's other streaming-parser rewrites hit
(cross-block context loss, or a document-wrapper event pair duplicated per block), pod-fmt's
own `Event` enum has no `StartDocument`/`EndDocument` wrapper at all, so that failure mode
doesn't apply here. Confirmed via 15 hand-built adversarial-chunking tests in `batch.rs`'s own
test module (chunk sizes 0/1/3/7/13, including a mid-UTF-8-character split, nested lists,
verbatim-with-internal-blank-lines, an unclosed `=begin` region, an `=over` block terminated by
`=cut` with no matching `=back`, and stray `=back`/`=item` commands) plus the pre-existing
fixture-driven equivalence check in `rescribe-fixtures/tests/streaming_apis.rs`
(`pod_streaming_parser_matches_events_and_is_incremental`), which still passes over the full
pod fixture suite — no divergence from `events()` was introduced.

**Peak memory measured via a `thread_local!` allocator probe** (`crate::alloc_probe`,
extracted out of `writer.rs`'s pre-existing one so both modules' test binaries — which `cargo
test`'s `--lib` harness links into a single binary — share the one `#[global_allocator]` a
process may define; `writer.rs`'s tests already had this pattern from a real flake found this
session, a shared `AtomicUsize` counter picking up 407x noise from concurrently-running
threads under full-workspace `cargo test -q`, fixed by making the counters `thread_local!`).
`batch.rs::tests::test_streaming_parser_peak_memory_bounded` feeds a synthetic 200- vs
2000-section (10x) multi-block document (headings + paragraphs with bold/link inlines) in
64-byte chunks and asserts the peak-to-peak ratio stays under 20x — measured ~2,253 bytes peak
@200 sections vs ~2,261 bytes @2000 sections (ratio ~1.00), confirming O(largest block) not
O(document). A throwaway before/after comparison (a temporary `examples/mem_compare.rs`, not
committed — reproduced the exact prior `StreamingParser` implementation from git history
side-by-side with the new one against the same synthetic documents) measured:

| sections | doc bytes | OLD peak (buffer-then-finish) | NEW peak (incremental) | reduction |
|---|---|---|---|---|
| 200 | 25,470 | 328,007 B (~12.9x doc) | 2,253 B | ~146x |
| 2,000 | 260,670 | 3,104,721 B (~11.9x doc) | 2,261 B | ~1,373x |
| 20,000 | 2,666,670 | 35,208,659 B (~13.2x doc) | 2,285 B | ~15,408x |

The old implementation's peak scaled linearly with input size (~12-13x the raw byte count, the
full input buffer plus the parsed `PodDoc` AST plus the collected `Vec<OwnedEvent>`); the new
implementation's peak stays flat regardless of document size, as expected for O(largest
block).

**The original `feed()`-before-`finish()` incrementality probe in `streaming_apis.rs` (a fixed
50%-of-total-bytes split of each real fixture) reported `Err` for 35 of 36 checked fixtures —
a probe-methodology gap, not a real defect, since fixed by replacing the probe rather than
accepted as a permanent `KnownFailure`.** The pod fixture suite is overwhelmingly one block
per fixture by design (`fixtures/spec.md`'s single-focused-construct convention — one heading,
one paragraph, or one `=over`/`=back` list per file). A single block's events cannot be
emitted until that block's own boundary (a blank line, the matching `=back`, or EOF) is
reached — architecturally true for *any* correct block-granular incremental parser, not a
defect — so the probe's fixed 50%-of-total-bytes split lands mid-block on such fixtures
regardless of implementation quality. Measured directly (a temporary
`examples/probe_incrementality.rs`, not committed): of the 36 pod fixtures large enough for
the probe to run (input length > 32 bytes, non-empty `events()` output), only 1
(`heading-levels`, which has 6 independently-flushing single-line heading blocks) delivered
any event before the halfway-byte mark; the other 35 are single-block documents where the
probe's fixed split can never succeed by construction. This is the same probe-methodology gap
already fixed this session for fb2-fmt/texinfo/xwiki (see
`fb2_streaming_parser_matches_events_and_is_incremental` in `tests/streaming_apis.rs`, and its
`fix(fb2-fmt): make StreamingParser genuinely incremental` commit, for the precedent) — a
fixed-byte-count split of a real fixture is not a valid incrementality test when the fixture
corpus is dominated by single-block documents; the correct fix is a hand-built probe with a
guaranteed-complete prefix, not accepting the failure permanently. Applied that fix here too:
`pod_streaming_parser_matches_events_and_is_incremental`'s per-fixture 50%-split check was
removed and replaced with a single hand-built probe fed once (not per fixture) — a full
`=head1` heading (a single-line block, flushes on its own newline) followed by a full ordinary
paragraph (flushes on the blank line after it), followed by deliberately unterminated trailing
content with no closing newline at all (so it never reaches `feed_line` and stays harmlessly
buffered, since `finish()` is never called in this probe). This passes cleanly. `pod`'s
`streaming_parser` is now `ApiState::Wired` in `streaming_harness.rs`'s `CAPABILITIES` table,
and the matching `KNOWN_FAILURES` entry (`format: "pod", api: "streaming_parser"`) was
removed.

---

**2026-07-31: textile-fmt `StreamingParser` (reader side) rewritten to be genuinely
incremental — the counterpart to the same-day `Writer` streaming fix.** Before this fix,
`textile_fmt::batch::StreamingParser::feed()` just appended to a `Vec<u8>` and did all real
parsing inside `finish()` (the crate's own module doc admitted this: "It also buffers all
input ... so memory is likewise O(full input)"), tracked as a `KnownFailure` in
`streaming_harness::KNOWN_FAILURES` (format: "textile", api: "streaming_parser").

Design: `parse.rs`'s block dispatch loop (`parse_blocks()`) was refactored into a step
function, `Parser::parse_next_block()`, that parses exactly one top-level block starting at
the current line position (or returns `None` once only blank lines/EOF remain) — `parse_blocks`
now just loops it to EOF, unchanged in behavior. A new `pub(crate) parse::BlockCursor` wraps
it for `batch.rs`'s use. `StreamingParser::feed()` accumulates complete lines (buffering
partial trailing bytes and mid-UTF-8-character splits internally, RST-batch.rs-style) into a
small `pending_lines` buffer, and after every new line re-runs `BlockCursor` over just that
pending tail: a block is "confirmed complete" — and its events flushed to the handler — the
moment the cursor's parse stops short of the buffered tail's end, since every block-parsing arm
in `parse_next_block()` only ever inspects lines up to and including the one it stops on, never
further ahead (proved by inspection of every arm: paragraph/list/table/definition-list/code-
block/pre-block all break out of their own `while self.pos < self.lines.len()` loop on the
first line that doesn't belong, without reading past it; `bq..`'s extended blockquote — the one
construct whose boundary isn't decidable from blank lines alone, since it swallows
blank-line-separated paragraphs until an explicit block-start line — behaves identically,
just with a later boundary line). Only the still-open block's lines remain buffered, so memory
is O(largest block), not O(full input). `finish()` runs the real `parse()` once over whatever
small tail remains pending, reusing the exact same grammar rather than a duplicated state
machine (avoiding the bug class hit by several other crates' streaming-parser rewrites this
session — see the org/asciidoc/djot/t2t `KNOWN_FAILURES`/fixed-comments for cross-block-context
and duplicate-document-wrapper pitfalls; textile's `TextileEvent` has no `StartDocument`/
`EndDocument` pair and no cross-block state like link-reference resolution, so per-block
re-derivation is safe here).

Measured peak memory (synthetic multi-block documents, fed in 64-byte chunks, `thread_local!`
allocator probe — see `crates/formats/textile-fmt/src/lib.rs`'s new shared `alloc_probe`
module, reused by both `writer.rs`'s and `batch.rs`'s memory-guard tests since a process may
only register one `#[global_allocator]`):

| sections | doc bytes | before (buffer-then-`finish()`) peak | after (incremental) peak | ratio |
|----------|-----------|---------------------------------------|----------------------------|-------|
| 20       | 2,280     | 377,495 bytes                          | 13,597 bytes               | 27.8x |
| 200      | 23,180    | 3,385,324 bytes                        | 59,258 bytes                | 57.1x |
| 2,000    | 235,780   | 29,556,929 bytes                       | 473,088 bytes               | 62.5x |

The "after" peak still grows somewhat with document size — inspection shows this is the test
harness's own `synthetic_doc(n)` input-construction cost (an O(n) `String`/`Vec<u8>` built
before feeding), not the parser: the parser itself only ever holds one block's lines pending.

Added adversarial-chunking tests (whole/single-byte/chunks-of-N/mid-UTF-8-character-split,
including one exercising `bq..`'s cross-blank-line boundary specifically) and a peak-memory
regression guard (ratio < 20x across a 10x section-count increase) to
`crates/formats/textile-fmt/src/batch.rs`'s own test module. Updated
`crates/rescribe-fixtures/tests/streaming_apis.rs`'s
`textile_streaming_parser_matches_events_and_is_incremental`: its incrementality probe used to
require *every* qualifying fixture (>32 bytes) to show partial delivery at exactly the halfway
byte offset, which is structurally impossible for a block-granular streaming parser (matching
every other hand-rolled line-oriented crate in this codebase — `rst-fmt`, `org-fmt`, etc.) when
a fixture's first block/line alone exceeds half the file's byte length (the `acronym` fixture:
first line is 66 of 124 bytes) — that's an inherent property of block granularity, not a bug.

Replaced it with the **hand-built synthetic-document probe** that fb2-fmt, xwiki, texinfo,
muse-fmt and pod-fmt all independently converged on for this identical defect (textile is the
sixth and last of that set): the per-fixture 50%-split check is dropped from the fixture loop
entirely (with a comment recording *why* it can't work here), and a single probe runs once
after the loop, feeding a hand-built input whose prefix is provably complete (`h1.` and `h2.`
headings — single-line blocks that flush on their own newline — plus a full paragraph that
flushes on its following blank line) followed by deliberately unterminated trailing content
(a partial line with no trailing newline, so it never reaches `feed_line` and stays buffered).
That guarantees an unambiguous complete-block boundary, so the probe tests the parser rather
than the fixture corpus's shape. It passes. `streaming_harness::CAPABILITIES`'s `textile` row
promotes `streaming_parser` from `ApiState::KnownFailure` to `ApiState::Wired`; the matching
`KNOWN_FAILURES` entry is removed. `docs/format-audit.md`'s textile row (`batch`, `w-stream`
columns) and cross-API harness inventory entry updated to match — the `w-stream` column had
also gone stale (writer already fixed same-day, table not updated until now).

Follow-up (deliberately *not* done here, queued centrally): the six inline copies of this
hand-built probe should collapse into one shared helper taking a per-format sample document.
textile's copy is intentionally left in the same shape as the other five so that extraction is
mechanical.

---

**2026-07-31: org-fmt `streaming_writer` hollow-writer defect fixed (and this harness's own
Wired claim corrected).** Before this fix, `streaming_harness.rs`'s `CAPABILITIES` table
declared `org`'s `streaming_writer` as `ApiState::Wired`, but its own adjacent comment admitted
"Writer is still not incrementally streaming" — a documentation-accuracy gap that existed
because the harness's `org_streaming_writer_matches_builder_over_all_fixtures` test only ever
checked byte-identical *content*, never wiring the same incrementality probe djot/texinfo/t2t's
equivalent tests already had. Both problems are fixed together: `org_fmt::writer::Writer`
(`crates/formats/org-fmt/src/writer.rs`) was rewritten from buffer-all-events-then-reconstruct
to a single shared `String` output buffer (mirroring `rst-fmt`'s `Writer`), and the missing
incrementality probe was added to the fixture-suite test in
`rescribe-fixtures/tests/streaming_apis.rs`.

Construct classification (see the module's doc comment for the full writeup): most constructs
are write-straight-through, using `emit.rs`'s own two idempotent tail operations
(`ensure_newline`/`ensure_blank_line`, both operating on the *whole* buffer tail, not a scoped
span — `emit.rs` itself has no per-construct sub-buffers except `Table`'s cell-width
measurement) ported directly onto `Writer::out`. This forced a genuinely different
invalid-context strategy than the other three rewrites: since `ensure_blank_line`'s
`trim_end()` isn't scoped to a mark, the usual "write speculatively, truncate on invalid
parent" pattern could incorrectly eat trailing whitespace belonging to earlier, valid content.
Instead, `accepts_blocks()`/`accepts_inline()` are checked *before* writing anything, pushing a
`Frame::Discard` marker (which itself doesn't accept blocks/inlines, cascading discard to
descendants) rather than a real frame when invalid. `List`/`ListItem` need a `list_depth`
counter (mirroring `BuildContext::list_depth`) plus per-child position/type dispatch (mirroring
`build_list_item`'s `first`/`Paragraph`/`List`/other match, including a bare-inline-run case
for list items with no `StartParagraph` wrapper — `events()` emits this for
`ListItemContent::Inline`). `Table` is genuinely content-dependent (column widths from every
cell's trimmed formatted-markup length), collecting cells the same way `rst-fmt` captures
heading plain text.

**Document metadata (`Event::Metadata`) is a documented, deliberate partial divergence from
`build()`'s exact semantics — not an oversight.** `build()` always moves *all* `OrgDoc.metadata`
to the very top of the document regardless of where in the source it appeared (confirmed via
`parse.rs`: `parse_next_block` can pick up a `#+KEY: value` line at any point in the document,
not just the start). A genuinely incremental writer cannot losslessly replicate "move
everything to the top" without unbounded lookahead (metadata could appear immediately before
the *last* block), so this `Writer` instead emits each `Metadata` line write-through, wherever
it arrives, with the single blank-line-before-next-block rule applied once metadata stops.
Audited every org fixture for this: none currently has generic metadata after body content
starts (two apparent counterexamples, `dynamic-block`'s `#+BEGIN:` and `figure`'s
`#+CAPTION:`/`#+NAME:`, both go through dedicated non-generic-metadata code paths in
`parse.rs`), so the divergence is currently unobservable in the byte-identical-to-builder
check — documented rather than silently assumed safe.

Verified byte-identical to `build()` over all 89 org fixtures on the first implementation
attempt (no formatting bugs found, unlike djot-fmt's two), with the newly-added incrementality
probe confirming bytes reach the sink before `finish()`. Measured (test-local tracking
allocator, release build, 5000-section ~759KB synthetic document, discarding sink): peak memory
4,480 bytes for the streaming Writer vs 1,720,320 bytes for `parse()`+`build()` (384x smaller),
but throughput went the other way (2.40ms vs 1.33ms, streaming ~1.8x slower) — the same
per-event-dispatch-overhead-vs-lightweight-builder tradeoff shape found for t2t, not a harness
artifact (events/AST both built outside the timed window). `CAPABILITIES`'s `org`
`streaming_writer` comment is corrected to state the fix accurately instead of contradicting
its own `ApiState::Wired` value.

---

**2026-07-31: t2t `streaming_writer` hollow-writer defect fixed.**
`t2t::writer::Writer` (`crates/formats/t2t/src/writer.rs`) was rewritten from
buffer-all-events-then-reconstruct-the-AST to a single shared `String` output buffer,
mirroring `rst-fmt`'s `Writer`. Every t2t construct turned out to be write-straight-through —
reading `emit.rs` end to end found no content-dependent prefix anywhere (heading rule width is
fixed by `level.min(5)`, not text length; table cells get no column-width padding at all) —
and, unlike RST/djot, there is no generic "blank line between siblings" rule to implement
either: every block variant's own `emit.rs` arm already writes its complete trailing
whitespace, so consecutive children simply concatenate with zero separator logic. The one real
subtlety: `Paragraph` gets three *different* framings depending on its parent's type (plain
top-level `"\n\n"`; a `"\t"` prefix + single `"\n"` inside `Blockquote`; no prefix and *no*
trailing newline at all inside `ListItem`, since `emit.rs`'s item loop concatenates all of an
item's blocks and writes exactly one `"\n"` after the whole item; no prefix + single `"\n"`
inside `DefinitionDesc`) — decided at `Paragraph`'s own `Start`/`End` events purely by
inspecting the parent frame, the same "known at open, applied at close" shape as `rst-fmt`'s
list-item dispatch. Non-`Paragraph` children of `Blockquote`/`ListItem`/`DefinitionDesc` (a
nested `List`, `Table`, etc.) get their own *unmodified* top-level formatting — confirmed by
reading `emit.rs`'s `else { build_block(child, ctx) }` branch, not assumed by analogy with
RST/djot's blockquotes (which *do* re-indent their full subtree; t2t deliberately does not).

Passed the byte-identity check against `emit()` over every fixture on the first implementation
attempt — no formatting bugs surfaced during this rewrite (unlike djot-fmt's two). Verified via
`t2t_streaming_writer_matches_builder_over_all_fixtures` in
`rescribe-fixtures/tests/streaming_apis.rs`, including the document header (`Event::Header`),
with bytes reaching the sink before `finish()`. Measured (test-local tracking allocator,
release build, 5000-section ~769KB synthetic document, discarding sink): peak memory 4,244
bytes for the streaming Writer vs 1,179,648 bytes for `parse()`+`emit()` (278x smaller) — but
throughput went the *other* way: 1.78ms streaming vs 0.64ms builder (streaming ~2.8x *slower*).
Both events and the AST were built outside the timed window (same discipline as
texinfo/djot-fmt's benchmarks), so this isn't the same harness artifact caught earlier — it's
architecture: t2t's builder (`build_block`) is an unusually lightweight direct tree recursion
with almost no per-node work, while the streaming Writer pays a `match` over ~30 event variants
plus `accepts_blocks()`/`accepts_inline()` frame-stack lookups per *event*, and t2t's event
granularity (~17 events per synthetic section) is much finer than its block granularity (~5
blocks per section) — so the per-event dispatch constant factor dominates for this
particular, very cheap-to-build format. A genuine memory/throughput tradeoff, not a defect —
noted here rather than silently reported as an unambiguous win. The `t2t/streaming_writer`
`KNOWN_FAILURES` entry is removed; `CAPABILITIES` now declares
`streaming_writer: ApiState::Wired`.

---

**2026-07-31: djot-fmt `streaming_writer` hollow-writer defect fixed.**
`djot_fmt::writer::Writer` (`crates/formats/djot-fmt/src/writer.rs`) was rewritten from
buffer-all-events-into-`Vec<OwnedEvent>`-then-`events_to_doc()`-then-`emit()` to a single
shared `String` output buffer, mirroring `rst-fmt`'s `Writer` (and the texinfo rewrite
immediately below). Construct classification (see the module's doc comment for the full
writeup): most constructs are write-straight-through; `Blockquote`/`DefinitionDesc` (uniform
per-line `"> "`/`"  "` prefix) and `ListItem`/`FootnoteDef` (per-line `"  "` prefix on every
line but the first, which continues the marker/label line) are deferred per-line
re-indentation via a pooled scratch buffer; `Table` is genuinely content-dependent (the header
separator's column count/alignments need every row seen first), so rows collect
already-*formatted* cell markup (captured under a mark and sliced out, since Djot cells can
hold inline spans, not just plain text) and render at `EndTable`. `Div` turned out to be
write-through, not deferred, on inspection of `emit.rs` — unlike `Blockquote`, its children
are written directly via `emit_blocks`, no sub-emitter/re-indentation. `Event::TableCaption`
carries an atomic `Vec<Inline>` payload (not streamed sub-events, by `events()`'s own design),
so rendering it needed one small independent AST-fragment-to-markup helper
(`render_inlines_ast`) — documented in the module doc as the one deliberate exception to
"never reconstruct via the tree/emit path," forced by the event vocabulary's shape.

Two real bugs surfaced and were fixed while getting the byte-identity check to pass (not
content bugs in the *old* buffer-then-emit writer, which round-tripped correctly by
construction — bugs in getting the *new* write-through design to match `emit()` exactly):
(1) `ListItem`'s between-items separator was double-counted (writing an unconditional `"\n"`
before both the marker's own newline logic and the reindented content's own trailing newline),
producing a blank line between list items where `emit()` uses exactly one newline — fixed by
popping the reindented content's trailing newline (matching `emit.rs`'s explicit
`if buf.ends_with('\n') { buf.pop(); }`) and letting the *next* item's marker write the sole
separating newline. (2) `Div`'s closing `":::"` fence needs a blank line before it even when
the last child already ended with its own single trailing newline (`emit.rs` writes an
unconditional extra `newline()` there) — the streaming writer initially only ensured a single
trailing newline. Both are pinned by `test_writer_byte_identical_to_builder` in `writer.rs`.

Verified byte-identical to `emit()` over every djot fixture (`djot_streaming_writer_matches_\
builder_over_all_fixtures` in `rescribe-fixtures/tests/streaming_apis.rs`), including
link-reference definitions and table captions, with bytes reaching the sink before `finish()`.
Measured (test-local tracking allocator, release build, 5000-section ~760KB synthetic
document, discarding sink): peak memory 4,469 bytes for the streaming Writer vs 1,720,320
bytes for `parse()`+`emit()` (385x smaller), 1.68x faster throughput. The `djot/streaming_writer`
`KNOWN_FAILURES` entry is removed; `CAPABILITIES` now declares `streaming_writer: ApiState::Wired`.

---

**2026-07-31: texinfo `streaming_writer` hollow-writer defect fixed.** The remaining half of
the `texinfo/streaming_writer` `KnownFailure` (see the 2026-07-30 entry immediately below —
title-loss was already fixed; the buffer-then-reconstruct architecture was not) is closed.
`texinfo::writer::Writer` (`crates/formats/texinfo/src/writer.rs`) was rewritten from
buffer-all-events-into-`Vec<OwnedEvent>`-then-`events_to_doc()`-then-`emit()` to a single
shared `String` output buffer written straight through per event, mirroring `rst-fmt`'s
`Writer` design (`f87b3d62ef`, `de67174ddd`). Construct classification: every Texinfo
construct turned out to be write-straight-through — no heading-underline-width or
table-column-width style deferred prefix exists anywhere in `emit.rs` (unlike RST). The three
subtleties (invalid-context discard via write-then-truncate-on-close, `TableCell`'s `" @tab "`
separator via a `cell_count` scalar on the parent frame, `Link`'s lazy `", "` before optional
link text via a `wrote_any` flag) are documented in the module's doc comment. Frame stack is
`O(nesting depth)`; each top-level block flushes to the sink and clears the shared buffer
(capacity retained) as soon as the frame stack empties. Verified via
`texinfo_streaming_writer_byte_identical_to_builder_over_all_fixtures` (byte-identical to
`emit()` over every fixture) plus its incrementality probe (bytes reach the sink before
`finish()`), both now passing — the `texinfo`/`streaming_writer` `KnownFailure` entry and its
matching `KNOWN_FAILURES` array entry are removed from `streaming_harness.rs`; `CAPABILITIES`
now declares `texinfo` `streaming_writer: ApiState::Wired`. Measured (test-local tracking
allocator, `test_writer_peak_memory_and_throughput_report` in `writer.rs`, `#[ignore]`d,
release build, 5000-section ~1MB synthetic document, discarding sink to avoid attributing the
caller's own output-retention choice to the Writer): peak memory 4,180 bytes for the streaming
Writer vs 1,966,080 bytes for `parse()`+`emit()` (470x smaller — the streaming Writer's peak is
now bounded by the largest top-level block, not the whole document), 1.54x faster throughput.
An earlier version of this benchmark fed the Writer via `events()` and a `Vec<u8>` sink inside
the timed/tracked window, which produced a misleadingly *worse* number for the Writer (higher
peak, slower) — a harness artifact from two unrelated sources: (a) `texinfo::events()` itself
eagerly parses the full document into an AST and materializes the complete `Vec<OwnedEvent>`
before yielding the first event (a separate, pre-existing non-incremental `events()` gap, out
of scope for this writer-only change — `CAPABILITIES`'s `texinfo` `events` field is still
`ApiState::Wired` with no caveat, which is arguably now inaccurate and worth a follow-up
audit), and (b) a `Vec<u8>` sink retains the whole flushed output regardless of how the Writer
itself buffers internally. Both were corrected (event vec built before the tracked window,
discarding sink) before trusting the final numbers.

---

**2026-07-31: merged the texinfo/djot-fmt/t2t/org-fmt streaming-writer sweep (above) with the
16-format wiki/lightweight-markup streaming-writer sweep** (see the "Central bookkeeping" entry
further below covering bbcode..fountain). The two sweeps were done in parallel worktrees against
the same 50-entry `streaming_harness::KNOWN_FAILURES` baseline and touched disjoint format sets,
so both sides' removals apply cleanly: 3 `streaming_writer` entries removed by the first sweep
(texinfo, djot, t2t — org-fmt's `streaming_writer` entry had already been deleted in an earlier,
pre-existing fix, so there was nothing left for this sweep to remove; it added the missing
incrementality probe instead) plus 16 removed by the second, leaving **31** entries (50 − 3 −
16, not the naively-expected 30). `CAPABILITIES` and `NOT_YET_AUDITED` are unaffected in size
(still 35 / 28) since both sweeps only flipped `ApiState` on existing rows. Post-merge
table self-consistency (every format in exactly one of `CAPABILITIES`/`NOT_YET_AUDITED`, every
`KNOWN_FAILURES` entry matching a `CAPABILITIES` row, 1:1 correspondence with
`ApiState::KnownFailure(_)` occurrences) reverified via `streaming_harness`'s own test. See
`docs/format-audit.md`'s "Third merge reconciliation" note for the full writeup.

---

**2026-07-30: texinfo `@settitle` title-loss gap closed.** One of the two `KnownFailure`s
tracked against `texinfo/streaming_writer` (see the 2026-07-30 cross-API harness entry below)
was an `Event`-enum expressiveness gap: `texinfo::events::Event` had no variant carrying
`TexinfoDoc::title`, so `events_to_doc()` in the streaming writer always reconstructed
`title: None`, silently dropping `@settitle` (`fixtures/texinfo/settitle-header`). Fixed by
adding `Event::Title(String)`, emitted by `events()`/`EventIter` (and thus by
`StreamingParser`, which is `events()`-backed) whenever `TexinfoDoc::title.is_some()`, and
handled by the streaming `Writer`'s `DocBuilder` to set the reconstructed doc's `title` field.
The **separate**, still-open half of that `KnownFailure` entry — `texinfo::batch::StreamingParser`
and `texinfo::writer::Writer` both being architecturally hollow (buffer-all-input/buffer-all-events
and only parse/emit inside `finish()`, not genuinely incremental) — is untouched; the
`KNOWN_FAILURES`/`CAPABILITIES` entries for `texinfo/streaming_parser` and
`texinfo/streaming_writer` in `streaming_harness.rs` remain, with descriptions trimmed to
reflect only the incrementality gap.

---

**2026-07-28: every "5-Production" / "done" claim below (and in `docs/format-audit.md`,
`README.md`, crate doc comments, and every `fixtures/*/COVERAGE.md` header) carries an
unverified construct-completeness caveat.** The hand-written construct checklists these
claims are partly based on have not been checked against any spec-derived source; see
`docs/format-audit.md`'s new "Construct Coverage (CC)" section for the evidence and the
"Status reset: construct-completeness marked unverified pending registry" entry below for
what this does and doesn't change. The reader/writer/API/fuzz work behind each "5" is real
and not being retracted — only the construct-list-completeness component of it is unverified.

---

**2026-07-30: djot-fmt `Event` enum expressiveness gap closed — `Event::LinkDef` added,
carried through `events()`/`StreamingParser`/the streaming `Writer`.** Fixes the gap named in
the 2026-07-30 cross-API harness entry below ("djot's `Event` has no `LinkDef` variant, drops
link-reference definitions"). `Event::LinkDef { label, url, title, id, classes, kv }` mirrors
`ast::LinkDef` field-for-field (flattening `Attr` the same way every other attribute-carrying
`Event` variant in `events.rs` does). `EventIter::next`'s `None` arm now emits one `LinkDef`
event per entry once top-level blocks are exhausted, taking `self.link_defs` at that point —
landing right after the body and before footnote defs in the stream. `writer.rs::DocBuilder`
gained a matching arm pushing to `DocBuilder.link_defs` (previously declared, initialized to
`vec![]`, and never written to — the concrete bug). `collect_doc_from_iter` (`events.rs`) was
updated to recover `link_defs` from the reconstructed `BlockFrame::Document` (which now
carries a `link_defs` field, populated via the new `Event::LinkDef` handler in `handle_event`)
rather than reaching into `EventIter::link_defs` directly — that field is now drained by
`next()` itself before iteration completes, so the old direct-field read would return empty.
`crates/rescribe-fixtures/tests/streaming_apis.rs`'s hand-written `dj_ast_to_events` AST→Event
projection (used to check `events()` against `parse()`'s own AST independently) was updated to
project `doc.link_defs` into the same `LinkDef` events at the same stream position.

Confirmed **not** fixed (out of scope, tracked separately): djot's `StreamingParser`
(`batch.rs`) still resolves a `[label]: url` definition to `url: ""` when the definition and
its reference live in different top-level blocks (fixture `link-reference` is exactly this
shape) — `emit_block()` re-parses each flushed block in isolation via `crate::events()`, so
that block's own `pre_scan()` never sees a definition sitting in a sibling block. `Event` now
being able to carry a `LinkDef` doesn't change that `StreamingParser` never gets to run
`pre_scan()` over more than one block's text at a time; this is a distinct batch.rs
cross-block-context bug, still tracked in `streaming_harness::KNOWN_FAILURES` under
`djot`/`streaming_parser`. Also confirmed **not** fixed: `writer.rs`'s `Writer` is still
architecturally hollow (buffers all events into a `Vec<OwnedEvent>`, only reconstructs the AST
+ calls `emit()` inside `finish()`) — a separate, still-open concern from the `LinkDef` gap.
Added an incrementality probe to `djot_streaming_writer_matches_builder_over_all_fixtures`
(same idiom as texinfo/commonmark/bbcode/creole's writer tests) so this stays a tracked
`KnownFailure` under `djot`/`streaming_writer` rather than silently going green — the fixture
content-equivalence half of that test now passes (the `LinkDef` gap was its only cause of
failure), but the probe still finds zero bytes reach the sink before `finish()`.
`streaming_harness::CAPABILITIES`/`KNOWN_FAILURES` and `docs/format-audit.md`'s djot row were
updated accordingly; do not fix the hollow-writer performance rework here, it needs its own
pass.

---

**2026-07-30: t2t-fmt — `Event::Header` closes the title/author/date expressiveness gap.**
Fix pass on the `t2t` `KnownFailure`s from the follow-up-pass entry below. Added
`Event::Header { title: Option<String>, author: Option<String>, date: Option<String> }` to
`crates/formats/t2t/src/events.rs` (one dedicated variant, not three, mirroring `T2tDoc`'s own
fixed 3-field header shape) and wired it through all three reader APIs plus the streaming
writer:
- `events()`/`EventIter` emits it right after `StartDocument` when any of title/author/date is
  `Some`.
- `StreamingParser` (`batch.rs`'s new `try_emit_header`) recognizes the header directly via
  `Parser::try_parse_header` on the stream's first accumulated block, instead of falling
  through to the generic `crate::events::events(&text)` re-parse path that used to
  spuriously re-trigger `try_parse_header()` on an isolated block and produce an extra *empty*
  `StartDocument`/`EndDocument` pair with the header silently dropped. This closes the
  document-header-specific half of the `streaming_parser` `KnownFailure`.
- The streaming `Writer`/`DocBuilder` (`writer.rs`) now tracks `title`/`author`/`date` fields,
  set by a new `Event::Header` arm in `process()` and threaded through `finish()`, so
  `t2t_streaming_writer_matches_builder_over_all_fixtures`'s content-equality check now passes
  on every fixture including `document-header`.

What's still open, deliberately not touched by this pass (out of scope — different defect
class): the **general per-block `StartDocument`/`EndDocument` duplication** in
`StreamingParser` (bulk `events()` emits one pair for the whole document, `StreamingParser`
emits one per accumulated block since `emit_block()` re-parses each block via
`crate::events::events()`, which always wraps its own pair) — this is much broader than the
two fixtures originally named in the `KnownFailure` text (definition-list and document-header);
it also reproduces on heading-h2, horizontal-rule, path-many-sections, comp-heading-list, and
any other multi-block fixture. The `definition-list` fixture's blank-line-splits-a-multi-item-
list symptom is one instance of this general issue and remains tracked. Also untouched: the
streaming `Writer`'s **buffer-then-emit-in-`finish()` non-incrementality** (the "hollow writer
performance" concern) — content is now correct but the writer still isn't a genuine incremental
streamer; `t2t_streaming_writer_matches_builder_over_all_fixtures`'s incrementality probe still
(correctly) fails on this and remains tracked in `KNOWN_FAILURES`.
`streaming_harness::KNOWN_FAILURES` entries for `t2t`/`streaming_parser` and
`t2t`/`streaming_writer` were both updated (not removed — both `Err` results persist for the
reasons above) to describe only the now-narrower remaining defects.

---

**2026-07-30: cross-API harness wired for the 4 wiki formats (mediawiki, tikiwiki, twiki,
vimwiki); 5 new tracked defects found.** Follow-up to the entry below — those 4 formats sat in
`streaming_harness::NOT_YET_AUDITED`, explicitly named as suspects for the
"`events()` implemented as `parse()`-then-tree-walk" pattern. Confirmed true for all four
(`EventIter::new`/`events()` calls `crate::parse::parse` then walks the result), but — like
asciidoc's narrower-Wired-claim precedent, and unlike html-fmt's `events_from_doc` (a generic,
structure-free tree walk) — each crate's walk makes real per-`Block`/`Inline`-variant mapping
decisions, so an independently-derived `ast_to_events` projection is not guaranteed to pass by
construction and the equivalence check has real teeth. Full per-format, per-API breakdown is in
`docs/format-audit.md`'s "Cross-API harness inventory" table; the executable source of truth is
`crates/rescribe-fixtures/src/streaming_harness.rs::CAPABILITIES`/`KNOWN_FAILURES` (26 entries
total, 5 new this session).

Results:
- **`events()` and `StreamingParser`: Wired, no divergence found**, for all of mediawiki,
  tikiwiki, and twiki, over their full fixture suites (50/37/48 fixtures respectively) including
  adversarial chunking (whole input, single-byte, 3/7/13-byte chunks, mid-UTF-8-char split).
  Despite all three `StreamingParser`s re-parsing each accumulated block in isolation via
  `crate::events::events(&text)` (the same "re-parse per block" architecture that split
  cross-block constructs for rst/org/asciidoc), no fixture in any of these three formats'
  suites exercises a cross-block construct that the isolation actually breaks.
- **twiki's `events()` has a non-standard signature**: `fn events(doc: &TwikiDoc) ->
  EventIter<'_>` takes an already-parsed AST, not raw input (`&str`/`&[u8]`) — a real deviation
  from the vertical-completion checklist's `events(input: &[u8]) -> impl Iterator<Item =
  Event>` contract (CLAUDE.md). Not fixed here; a caller must call `parse()` first, unlike
  every other format checked in this harness.
- **`vimwiki`'s `StreamingParser` genuinely diverges from `events()`**, and not from a
  chunk-boundary issue: it reproduces even feeding the whole input as a single `feed()` call
  (fixture `oracle`). Root cause: `parse()`/`events()` treat a blank-line-separated run of an
  unordered list, then an ordered list, then an unordered checklist (no other content between
  them) as **one** `Block::List` with a single `ordered: bool` for all 8 items — silently
  losing the ordered/unordered distinction for the second and third groups, since `Block::List`
  has one `ordered` flag for the whole list, not per-item. `StreamingParser`'s `emit_block`
  hard-splits on every blank line (batch.rs's `feed_line`), so it instead emits three separate,
  correctly-typed `StartList`/`EndList` pairs. The two implementations disagree about where one
  list ends and the next begins — arguably `StreamingParser`'s behavior is the *more* correct
  one here, but that's a call for whoever fixes `parse()`'s list-continuation logic, not this
  audit pass. Tracked as `KnownFailure { format: "vimwiki", api: "streaming_parser" }`.
- **All four formats' streaming writers are architecturally hollow buffer-then-emit**,
  confirmed via the harness's `ObservableSink` incrementality probe (a complete
  `StartParagraph`/`Text`/`EndParagraph` sequence writes zero bytes to the sink before
  `finish()`), not merely inferred from module docs (none of these four crates' writer.rs
  module docs self-admit this the way org/djot/commonmark-fmt's do). Content round-trips
  correctly against `build()`/`emit()` on every fixture in all four formats — this is purely an
  incrementality defect, not a content-correctness one. Tracked as `KnownFailure` for
  `mediawiki`/`tikiwiki`/`twiki`/`vimwiki` `streaming_writer`.

Not fixed here (by design — this was a wiring/audit pass, not a fix pass): none of the 5 new
`KnownFailure`s were fixed; each needs its own fix pass in its crate. No new fixtures were
needed — the existing suites already exercised the divergences found. 45 formats remain in
`NOT_YET_AUDITED`.

**2026-07-30 (second pass): cross-API harness wired for 4 more formats (xwiki, zimwiki,
markua, muse); 8 new tracked defects found, including a `parse()`-level bug shared verbatim
between two crates.** Follow-up to the first 2026-07-30 entry below, which left xwiki,
zimwiki, markua, and muse in `streaming_harness::NOT_YET_AUDITED`. This pass moved all four
into real, fixture-driven `CAPABILITIES` entries. Full per-format, per-API breakdown is in
`docs/format-audit.md`'s "Cross-API harness inventory" subsection; the executable source of
truth is `crates/rescribe-fixtures/src/streaming_harness.rs::CAPABILITIES` and
`KNOWN_FAILURES` (29 entries total, 8 new this pass).

Headline findings:
- **A `parse()`-level bug, not a streaming-specific one, found identically in two
  independently-written crates**: `zimwiki::parse::Parser::parse_list` and
  `markua::parse::Parser::parse_list` both accept either a bullet or a numbered marker in
  the same loop with no check for a marker-type transition, and both skip blank lines with
  `continue` instead of breaking on them. The result: a blank-line-separated unordered list
  immediately followed by an ordered list (`fixtures/zimwiki/oracle`, `fixtures/markua/oracle`)
  gets merged by the *whole-document* `parse()` into ONE `Block::List`, tagged with the first
  item's `ordered` value — silently mislabeling the later numbered items as unordered. This
  is not a `StreamingParser` regression: `StreamingParser`'s blank-line block splitter
  hard-splits at that same blank line *before* re-parsing each half in isolation, so it
  actually produces the more correct two-list output — the adversarial-chunking equivalence
  check catches this as a *divergence from `events()`*, but the underlying defect is in
  `parse()`. Needs a fix in both crates' `parse_list` (track the active marker type and break
  on a transition, and break rather than skip on a blank line unless immediately followed by
  a continuation at the same indent).
- **xwiki's `events()` is the first genuinely lazy pull-iterator found in this format
  family**: `EventIter::next()` walks a frame stack on demand (events.rs:168-385), unlike
  zimwiki/markua/muse's eager materialize-then-walk. Confirmed by reading the source, not
  assumed from the crate's docs.
- **Two more architecturally-hollow `StreamingParser`s**, confirmed by reading `feed()`/
  `finish()` directly (not inferred from doc comments alone): xwiki's and muse's both do a
  bare `buf.extend_from_slice()` in `feed()` with all parsing deferred to `finish()`. Verified
  via this harness's `ObservableSink`/handler incrementality probe (feed half the input,
  assert the handler received at least one event before `finish()` — it received zero for
  both). zimwiki's and markua's `StreamingParser`s are, by contrast, genuinely incremental
  (real per-line block-boundary state machines) — confirmed by reading `feed_line()`.
- **Four architecturally-hollow streaming `Writer`s** (xwiki, zimwiki, markua, muse), all
  confirmed via the same incrementality probe: `write_event()` only pushes to a `Vec`, all
  reconstruction + emission happens in `finish()`.
- **muse-fmt's streaming writer has a genuine `Event`-expressiveness gap**, the same bug
  class already tracked for org-fmt/texinfo: `MuseEvent` has no variant carrying document
  metadata, so `DocBuilder::finish` always reconstructs with `..Default::default()`,
  permanently dropping `#title`/`#author`/`#date`/`#desc`/`#keywords`. Unlike markua's
  superficially similar `title`/`author`/`description` fields (which `parse()` never
  populates from any syntax at all, so the analogous drop in markua's writer is unreachable
  via any fixture), muse-fmt's `parse()` genuinely parses these directives
  (`parse.rs:240-249`), so this is reachable via the `document-header` fixture and was
  confirmed failing before being added to `KNOWN_FAILURES`.
- **markua's `Writer` has a real `Figure`/`Caption` reconstruction bug** (`EndFigure` takes
  the wrong buffered child as the figure body and always resets `caption: vec![]`), found by
  reading `writer.rs:315-330` — but it is *not* reachable via any fixture, because
  `markua::parse()` never constructs `Block::Figure` from any Markua syntax (confirmed by
  grepping `parse.rs`/`emit.rs` for the only two call sites, both non-parsing). Documented in
  the `KnownFailure` description for the record; not pinned by a fixture, since no valid
  `input.markua` can reach it.

Not fixed here (by design — this was a wiring/audit pass, not a fix pass): none of the 8 new
`KnownFailure`s were fixed; each needs its own fix pass in its `-fmt` crate. The
`parse_list()` block-merging bug (zimwiki + markua) is the highest-value fix candidate since
it's a real parser correctness bug independent of streaming, not just an architectural
streaming-API gap. 45 formats remain in `NOT_YET_AUDITED`.

**2026-07-30 (follow-up pass): cross-API harness wired for t2t, pod, haddock, fountain;
7 new tracked defects found.** Follow-up to the entry directly below, picking 4 more names
off `streaming_harness::NOT_YET_AUDITED`. Full per-format, per-API breakdown is in
`docs/format-audit.md`'s "Cross-API harness inventory" table; the executable source of truth
is `crates/rescribe-fixtures/src/streaming_harness.rs::CAPABILITIES` and `KNOWN_FAILURES`
(28 entries total now, 7 new this pass).

Headline findings:
- **t2t-fmt StreamingParser**: `emit_block()` re-parses each accumulated block in isolation
  (same root cause as org/asciidoc's `StreamingParser` bugs from the prior pass) — splits a
  multi-item `DefinitionList` into one list per item (definition-list fixture), and
  spuriously re-triggers `try_parse_header()`'s 3-line lookahead on any 3+ line block that
  happens to look like a document header out of context (document-header fixture), producing
  an extra spurious `StartDocument`/`EndDocument` pair.
- **fountain-fmt StreamingParser**: the dominant bug of this pass, not an edge case — its
  `emit_block()` forwards every event from its own per-block re-parse *verbatim*, including
  that call's own `StartDocument`/`EndDocument` pair, with no filtering. Bulk `events()`
  emits exactly one such pair for the whole document; `StreamingParser` emits one pair *per
  block*, diverging on the majority of multi-block fixtures. A second, narrower defect
  shares the re-parse-in-isolation root cause: `parse_title_page()` has no
  document-position guard, so a body line matching a title-page field name is misread as
  metadata when re-parsed in isolation.
- **pod-fmt StreamingParser** is explicitly self-documented buffer-then-finish in its own
  module doc — a different shape from t2t/fountain's per-block-reparse bug: `feed()` only
  extends a `Vec<u8>`, all parsing happens in `finish()`, so it does NOT diverge from
  `events()` under adversarial chunking (no per-block reparse to disagree with events()) —
  only the incrementality probe (feed-before-finish delivery) fails.
- **Four more architecturally hollow streaming writers** (buffer-all-events-then-emit-in-
  `finish()`, a fake streaming API per CLAUDE.md): t2t, pod, haddock, fountain. t2t's writer
  additionally always drops `title`/`author`/`date` since `t2t::Event` has no variant
  carrying document-level metadata (an expressiveness gap, same class as org/texinfo/djot
  from the prior pass) — pod/haddock/fountain's writers don't have this compounding gap.
- **haddock-fmt StreamingParser is the one fully clean result this pass**: no `KnownFailure`
  needed. Unlike t2t/fountain, every haddock block-termination rule in `parse.rs` depends
  only on the content of lines within the block being scanned, never on cross-block state or
  document position, so its per-block isolated re-parse recovers exactly the same boundaries
  `events()` finds inline — verified empirically via the adversarial-chunking check, not
  assumed from reading the code alone.
- **Also noted, out of scope for this pass**: fountain-fmt's `events.rs` defines a second,
  un-exported *borrowed* `EventIter<'a>` (distinct from the `OwnedEventIter` that
  `fountain_fmt::events()` actually returns) that a direct reading of its `Blocks`-phase
  match arms suggests double-emits `Event::PageBreak` and never emits a `Text` event for any
  non-Character/Dialogue/Parenthetical block. Not wired into `CAPABILITIES` since it isn't
  the API this harness's "events" state tracks — flagged here for a future look.

Not fixed here (by design — this was a wiring/audit pass, not a fix pass): none of the 7 new
`KnownFailure`s were fixed; each needs its own fix pass in its `-fmt` crate. 45 formats
remain in `NOT_YET_AUDITED`.

---

**2026-07-30: cross-API harness wired for 11 more formats (org, html, asciidoc, djot,
texinfo, fb2, textile, commonmark, gfm, markdown); 18 new tracked defects found.** Follow-up
to the 2026-07-29 entry below — that session left ~55 formats in
`streaming_harness::NOT_YET_AUDITED` as an honest "nobody has looked yet" placeholder. This
session moved 11 of them into real, fixture-driven `CAPABILITIES` entries, prioritizing the
formats with the largest fixture suites. Full per-format, per-API breakdown (Wired /
KnownFailure with root cause / NotApplicable with citation) is in `docs/format-audit.md`'s
new "Cross-API harness inventory (2026-07-30)" subsection; the executable source of truth is
`crates/rescribe-fixtures/src/streaming_harness.rs::CAPABILITIES` and `KNOWN_FAILURES`
(21 entries total, 18 new this session — 3 predate it: docx/events, pptx/events,
rst/streaming_parser).

Headline findings (see `KNOWN_FAILURES` for full descriptions of all 18):
- **Architecturally hollow ("buffer-then-`finish()`/buffer-then-emit") implementations**,
  not just wrong output: texinfo's `StreamingParser` and streaming writer, fb2's
  `StreamingParser`, textile's `StreamingParser` and streaming writer, org's streaming
  writer, djot's streaming writer, commonmark-fmt's streaming writer. These pass the
  feed/finish contract but deliver zero incremental output before `finish()` — a fake
  streaming API per CLAUDE.md.
- **`Event` enum expressiveness gaps** that make round-tripping through `events()`/the
  streaming writer lossy even when both are otherwise correct: org's `Event` has no
  document-metadata variant at all (drops `#+TITLE:`/`#+AUTHOR:`/keyword lines);
  texinfo's `Event` has no variant for `TexinfoDoc::title` (drops `@settitle`); djot's
  `Event` has no `LinkDef` variant (drops link-reference definitions).
- **fb2's `events()` silently drops the `Metadata` event** whenever input lacks a literal
  `<description>` element — affects the majority (34/58) of fb2 fixtures, not an edge case.
- **commonmark-fmt's `events()`** (shared by commonmark/gfm/markdown) has two real bugs:
  image alt-text `Text` events fire before `StartImage` instead of between
  `StartImage`/`EndImage` (duplicates alt text in output), and consecutive `Text` events
  aren't coalesced the way `parse()`'s AST deliberately does.
- **`StreamingParser` divergences from `events()` under adversarial chunking**, each with
  multiple distinct root causes, on: org (3/89 fixtures), asciidoc (8/85), djot (6/79) — see
  `KNOWN_FAILURES` for the per-bug breakdown of each.
- **html-fmt's `events()`/`StreamingParser` are genuinely, documentedly `NotApplicable`**,
  not just unaudited: HTML5 tree construction (foster parenting, adoption agency) makes true
  incremental delivery impossible per the crate's own docs and CLAUDE.md's html5ever
  out-of-scope carve-out. The streaming writer is independent and passes cleanly.
- Also fixed in this session (not a new defect, a harness bug): the rst-fmt
  `StreamingParser` equivalence check's `checked > N` sanity floor was
  `read_dir`-iteration-order-dependent — it incremented `checked` only on the non-divergent
  path and hard-broke the whole loop on first divergence, so whether the floor was ever
  reached depended on filesystem directory order rather than test content. Fixed in
  `crates/rescribe-fixtures/tests/streaming_apis.rs` to count fixtures as checked as soon as
  their `events()` baseline is computed and to keep iterating past a single divergence.

Not fixed here (by design — this was a wiring/audit pass, not a fix pass): none of the 18
new `KnownFailure`s were fixed; each needs its own fix pass in its `-fmt` crate. 49 formats
remain in `NOT_YET_AUDITED`; wiring the rest is tracked, unstarted follow-up work.

**2026-07-30: cross-API harness wired for bbcode-fmt — the first format audited whose
`StreamingParser` turned out to be genuinely, not just nominally, Wired.** Follow-up to the
entry above, picking one more format out of `NOT_YET_AUDITED`. Full breakdown in
`docs/format-audit.md`'s "Cross-API harness inventory" table; source of truth is
`streaming_harness::CAPABILITIES`'s new `"bbcode"` entry.

- **`events()` — Wired, but honestly scoped like asciidoc's entry, not an independent
  parser.** `bbcode_fmt::events()` (`events.rs`) literally calls `crate::parse::parse(input)`
  and then walks the resulting `BbcodeDoc` — the same shape as html-fmt's
  `events_from_doc(&parse(input).0)`. Unlike html-fmt there is no format-spec reason forcing
  this (no foster-parenting/adoption-agency equivalent for BBCode), so it's a real
  `CLAUDE.md` "three independent implementations" violation, not a documented structural
  absence — `NotApplicable` would have been the wrong call. Wired anyway per this task's
  brief: the check still pins the AST↔`Event` correspondence and would catch a field
  silently dropped by the tree walk.
- **`StreamingParser` — Wired, for real, no `KnownFailure` needed.** `batch.rs`'s
  `StreamingParser` is a genuine incremental line-buffered state machine: `feed_line`
  advances real state and `emit_block()` flushes events to the handler as soon as a blank
  line or a recognized block tag's close line is seen, not only inside `finish()`. Verified
  two ways: (1) an incrementality probe (feed a complete `[b]Hello[/b]` paragraph + blank
  line + unterminated trailing text, confirm events arrive before `finish()`) passes; (2) the
  adversarial-chunking equivalence check against `events()` passes over all 53 bbcode
  fixtures. Also hand-checked several adversarial cases the fixture suite doesn't happen to
  cover, since `detect_block_tag` (batch.rs:200-224) is visibly coarser than `parse.rs`'s
  `is_block_start` (missing heading/`[hr]` tags; returns `None` — no boundary — when a block
  tag's close is on the same line as its open, batch.rs:217-219): a same-line-closed
  `[code]...[/code]` immediately followed by more text with no blank line, a blank line
  inside an `InBlock` quote, and nested same-tag quotes (`[quote=A][quote=B]...[/quote]
  [/quote]`) all converge with `events()`. They converge because `StreamingParser`'s
  boundary detection is only ever coarser-or-equal to `parse()`'s, never finer — whatever
  text it accumulates into one flushed chunk gets handed to the same `parse::parse()` call `events()`
  itself would run on that span, re-deriving identical fine-grained structure. (The nested-
  quote case actually converges for a less happy reason: `parse_quote`/`detect_block_tag`
  both close on the *first* line containing `[/quote]` regardless of nesting depth, so both
  sides independently mishandle nested quotes the same way — a real pre-existing `parse()`-
  level bug, but out of scope for this task, which only compares `StreamingParser` against
  `events()`, not either against a spec.)
- **streaming `Writer` — `KnownFailure`, architecturally hollow.** `writer.rs`'s own module
  doc says outright: "this implementation buffers all events, reconstructs the AST, then
  emits." `write_event()` (writer.rs:42-44) only pushes onto a `Vec<OwnedEvent>`; all real
  work (`events_to_doc` + `emit::emit`) happens inside `finish()`. Content still matches
  `build()` byte-for-byte over all 53 fixtures (same reason as texinfo/commonmark: `finish()`
  ultimately drives the same `emit()` path), but the incrementality probe (write a complete
  paragraph, check for any bytes reaching the sink before `finish()`) gets zero bytes. Same
  fix shape as `ooxml-wml`/`ooxml-sml`'s writers: rewrite `write_event()` to push into a fixed
  output window instead of a `Vec`, driven directly off `crate::emit`'s per-node string logic.

Not fixed here (by design): the streaming-writer `KnownFailure` above was found, not fixed;
it needs its own fix pass in `bbcode-fmt`. 48 formats remain in `NOT_YET_AUDITED`.

**2026-07-30: cross-API harness wired for creole — the second format (after bbcode) whose
`StreamingParser` turned out to be genuinely, not just nominally, Wired.** Follow-up to the
bbcode entry above, picking one more format out of `NOT_YET_AUDITED`. Full breakdown in
`docs/format-audit.md`'s "Cross-API harness inventory" table; source of truth is
`streaming_harness::CAPABILITIES`'s new `"creole"` entry.

- **`events()` — Wired, but honestly scoped like bbcode's entry, not an independent parser.**
  `creole::events::EventIter::new` (`events.rs:123-127`) literally calls
  `crate::parse::parse(input)` and then walks the resulting `CreoleDoc` via `collect_events` —
  the same non-independent shape as bbcode-fmt's and html-fmt's `events()`. No format-spec
  reason forces this (no HTML5-tree-construction equivalent for Creole), so `NotApplicable`
  would have been the wrong call; wired anyway per the bbcode/asciidoc precedent, since the
  check still pins the AST↔`Event` correspondence.
- **`StreamingParser` — Wired, for real, no `KnownFailure` needed.** `batch.rs`'s
  `StreamingParser` is architecturally near-identical to bbcode's: `feed_line` advances real
  state and `emit_block()` (which re-parses just the accumulated block text via
  `crate::events::events()`) flushes to the handler as soon as a blank line or a nowiki
  block's `}}}` close is seen, not only inside `finish()`. Verified two ways: (1) an
  incrementality probe (feed a complete `= Hello` heading + blank line + unterminated
  trailing text, confirm events arrive before `finish()`) passes; (2) the adversarial-chunking
  equivalence check against `events()` passes over all 35 creole fixtures, both under whole-
  input and single-byte-at-a-time chunking. One edge case inspected by hand but not exercised
  by any fixture: `feed_line`'s in-nowiki close test (`batch.rs`, `is_end = line.trim() ==
  "}}}"`) requires the closing marker to be the *entire* trimmed line, while `parse.rs`'s
  `parse_nowiki_block` finds `"}}}"` anywhere in the line (silently dropping any trailing text
  after it, a separate pre-existing `parse()`-level quirk out of scope here). A nowiki block
  closed by a line like `"tail}}}"` never trips the streaming splitter's boundary, so
  everything from that opener onward is swept into one oversized block delivered only at
  `finish()` — confirmed by hand with `"{{{\ncode\nsome}}}\nmore\n"` that this degrades
  *incrementality*, not *correctness*: the oversized block is still handed whole to
  `crate::events::events()`, which re-derives the identical block split a bulk `parse()` over
  that span would produce, so the final event sequence still matched `events()` exactly. Not
  filed as a `KnownFailure` since nothing observable diverges — noted here in case a future
  incrementality-specific probe wants to target it directly.
- **streaming `Writer` — `KnownFailure`, architecturally hollow, same shape as bbcode's.**
  `write_event()` (`writer.rs:38-40`) only pushes onto a `Vec<OwnedEvent>`; all real work
  (`events_to_doc` + `crate::emit::build`) happens inside `finish()` (`writer.rs:43-48`).
  Content still matches `build()` byte-for-byte over all 35 fixtures (`finish()` ultimately
  drives the same `build()` path), but the incrementality probe (write a complete
  `StartParagraph`/`StartBold`/`Text`/`EndBold`/`EndParagraph` sequence, check for any bytes
  reaching the sink before `finish()`) gets zero bytes. Same fix shape as bbcode's writer:
  rewrite `write_event()` to push into a fixed output window instead of a `Vec`, driven
  directly off `crate::emit`'s per-node string logic.

Not fixed here (by design): the streaming-writer `KnownFailure` above was found, not fixed;
it needs its own fix pass in `creole`. 47 formats remain in `NOT_YET_AUDITED`.

**2026-07-30: cross-API harness wired for dokuwiki — the third format (after bbcode, creole)
whose `StreamingParser` turned out to be genuinely, not just nominally, Wired, and the first
of the three with a clean equivalence check needing no coarser-boundary caveat at all.**
Follow-up to the bbcode/creole entries above, picking one more format out of
`NOT_YET_AUDITED`. Full breakdown in `docs/format-audit.md`'s "Cross-API harness inventory"
table; source of truth is `streaming_harness::CAPABILITIES`'s new `"dokuwiki"` entry.

- **`events()` — Wired, but honestly scoped like bbcode's/creole's entry, not an independent
  parser.** `dokuwiki::events()` (`lib.rs:33-35`, delegating to `events::events`) is
  `InputEventIter::new`, which calls `crate::parse::parse(input)` and walks the resulting
  `DokuwikiDoc` with the crate's own lazy `EventIter` (`events.rs:705-731`) — the same
  non-independent "parse() then walk the tree" shape as bbcode-fmt's and creole's `events()`.
  No format-spec reason forces this, so `NotApplicable` would have been the wrong call; wired
  anyway per the bbcode/creole precedent, since the check still pins the AST↔`Event`
  correspondence. Also confirmed a positive: every `Block`/`Inline` variant and field in
  `ast.rs` has a corresponding `Event` variant/field in `events.rs` (`FileBlock`'s `filename`,
  `RawBlock`'s `format`, `Macro`'s `name`, `Image`'s `alt` all round-trip) — no `Event` enum
  expressiveness gap, unlike org-fmt (no metadata variant) or djot-fmt (no `LinkDef` variant).
- **`StreamingParser` — Wired, for real, no `KnownFailure` needed, and no caveat needed
  either.** `batch.rs`'s `StreamingParser` is a genuine incremental line-buffered state
  machine: `feed_line` advances real per-line state
  (`BlockState::{Between,Accumulating,InSpecialBlock}`) and `emit_block()` (which re-parses
  just the accumulated block text via `crate::events::events()`) flushes to the handler as
  soon as a blank line or a recognized `<code>/<file>/<html>/<php>` block's close is seen, not
  only inside `finish()`. Verified two ways: (1) an incrementality probe (feed a complete
  `**Hello**` paragraph + blank line + unterminated trailing text, confirm events arrive
  before `finish()`) passes; (2) the adversarial-chunking equivalence check against `events()`
  passes over every dokuwiki fixture, under every chunking in `adversarial_chunkings`
  (whole/single-byte/chunks-of-3/7/13/mid-UTF-8-char). Unlike bbcode's and creole's entries,
  no coarser-boundary caveat was needed: `parse.rs`'s `Parser` has *no* cross-block state at
  all — no loose-list joining across blank lines (`parse_list_items` already stops at any
  non-`"  "`-prefixed line, which includes blank lines) and no forward/backward reference
  resolution of any kind — so every boundary `StreamingParser::feed_line` can flush on is one
  `parse.rs`'s own top-level dispatch loop would also treat as a valid, self-contained block
  split point.
- **streaming `Writer` — `KnownFailure`, architecturally hollow, same shape as bbcode's/
  creole's.** `write_event()` (`writer.rs:27-29`) only pushes onto a `Vec<OwnedEvent>`; all
  real work (`events_to_doc` + `crate::emit::build`) happens inside `finish()`
  (`writer.rs:32-37`), self-admitted in the module doc: "Buffers all events, reconstructs the
  AST, then emits." Content still matches `build()` byte-for-byte over every fixture
  (`finish()` ultimately drives the same `build()` path), but the incrementality probe (write
  a complete `StartParagraph`/`StartBold`/`Text`/`EndBold`/`EndParagraph` sequence, check for
  any bytes reaching the sink before `finish()`) gets zero bytes. Same fix shape as bbcode's/
  creole's writers: rewrite `write_event()` to push into a fixed output window instead of a
  `Vec`, driven directly off `crate::emit`'s per-node string logic.

Not fixed here (by design): the streaming-writer `KnownFailure` above was found, not fixed;
it needs its own fix pass in `dokuwiki`. 46 formats remain in `NOT_YET_AUDITED`.

**2026-07-30: cross-API harness wired for jira-fmt — the fourth format (after bbcode, creole,
dokuwiki) whose `StreamingParser` turned out to be genuinely, not just nominally, Wired, and
the second (after dokuwiki) with a clean equivalence check needing no coarser-boundary caveat
at all.** Follow-up to the bbcode/creole/dokuwiki entries above, picking one more format out
of `NOT_YET_AUDITED`. Full breakdown in `docs/format-audit.md`'s "Cross-API harness inventory"
table; source of truth is `streaming_harness::CAPABILITIES`'s new `"jira"` entry.

- **`events()` — Wired, but honestly scoped like bbcode's/creole's/dokuwiki's entry, not an
  independent parser.** `jira_fmt::events()` (`lib.rs:24-26`, delegating to `events::events`)
  calls `crate::parse::parse(input)` and walks the resulting `JiraDoc` with
  `emit_doc_events`/`emit_block_events`/`emit_inline_events` (`events.rs:141-149`) — the same
  non-independent "parse() then walk the tree" shape as bbcode-fmt's/creole's/dokuwiki's
  `events()`. No format-spec reason forces this, so `NotApplicable` would have been the wrong
  call; wired anyway per that precedent, since the check still pins the AST↔`Event`
  correspondence. Also confirmed a positive: every `Block`/`Inline` variant and field in
  `ast.rs` has a corresponding `Event` variant/field in `events.rs` — no `Event` enum
  expressiveness gap, unlike org-fmt (no metadata variant) or djot-fmt (no `LinkDef` variant).
- **`StreamingParser` — Wired, for real, no `KnownFailure` needed, and no caveat needed
  either.** `batch.rs`'s `StreamingParser` is a genuine incremental line-buffered state
  machine: `feed_line` advances real per-line state (`BlockState::{Between,Accumulating,
  InDelimitedBlock}`) and `emit_block()` (which re-parses just the accumulated block text via
  `crate::events::events()`) flushes to the handler as soon as a blank line or a
  `{code.../{quote}/{noformat}/{panel...` delimited block's close is seen, not only inside
  `finish()`. Verified two ways: (1) an incrementality probe (feed a complete `*Hello*`
  paragraph + blank line + unterminated trailing text, confirm events arrive before
  `finish()`) passes; (2) the adversarial-chunking equivalence check against `events()` passes
  over every jira fixture, under every chunking in `adversarial_chunkings`
  (whole/single-byte/chunks-of-3/7/13/mid-UTF-8-char). Like dokuwiki's entry, no
  coarser-boundary caveat was needed: `parse.rs`'s `Parser` has no cross-block state at all —
  no loose-list joining across blank lines (`parse_list_at_depth` already breaks on any
  non-`*`/`#` line, which includes blank lines), no reference resolution, and critically no
  decorator-line-preceding-a-fence construct of the kind that caused real bugs in org-fmt
  (`#+NAME:` before `#+BEGIN_`) and asciidoc (`[source,...]`/`.Title` before a delimited
  block): Jira's `{code:lang}` language and `{panel:title=...}` title are both encoded on the
  fence line itself, so `feed_line`'s "flush pending content before starting a delimited
  block" step (`batch.rs`, the four `{code/{quote}/{noformat}/{panel` branches) never has
  anything decorator-shaped to strand. Every boundary `StreamingParser::feed_line` can flush
  on is one `parse.rs`'s own `parse_paragraph`/`parse_list_at_depth`/`parse_table` stop
  conditions would also treat as a valid, self-contained block split point.
- **streaming `Writer` — `KnownFailure`, architecturally hollow, same shape as bbcode's/
  creole's/dokuwiki's.** `write_event()` (`writer.rs:40-42`) only pushes onto a
  `Vec<OwnedEvent>`; all real work (`events_to_doc` + `crate::emit::build`) happens inside
  `finish()` (`writer.rs:45-50`), self-admitted in the module doc: "this implementation
  buffers all events, reconstructs the AST, then emits." Content still matches `build()`
  byte-for-byte over every fixture (`finish()` ultimately drives the same `build()` path), but
  the incrementality probe (write a complete `StartParagraph`/`StartBold`/`Text`/`EndBold`/
  `EndParagraph` sequence, check for any bytes reaching the sink before `finish()`) gets zero
  bytes. Same fix shape as bbcode's/creole's/dokuwiki's writers: rewrite `write_event()` to
  push into a fixed output window instead of a `Vec`, driven directly off `crate::emit`'s
  per-node string logic.

Not fixed here (by design): the streaming-writer `KnownFailure` above was found, not fixed;
it needs its own fix pass in `jira-fmt`. 45 formats remain in `NOT_YET_AUDITED`.

---

**2026-07-29: fixture harness extended to exercise `events()`/`StreamingParser`/streaming
writer directly, not just the rescribe adapter's `parse()`/`emit()`.** Previously
`crates/rescribe-fixtures/tests/run.rs` only ever drove the rescribe adapter's `parse()`
(reader) / `emit()` (writer) — never any format crate's `events()`, `StreamingParser<H>`, or
streaming writer. New infrastructure: `crates/rescribe-fixtures/src/streaming_harness.rs`
(adversarial chunking helper, `FormatCapabilities`/`ApiState` declaration table,
`KnownFailure`/`KNOWN_FAILURES` tracked-failure mechanism) and
`crates/rescribe-fixtures/tests/streaming_apis.rs` (the fixture-driven checks). Full reader
verticals wired: rst-fmt (`events()` vs. AST projection, `StreamingParser` vs. `events()`
under adversarial chunking, streaming `Writer` vs. `build()` byte-identical — all over the
whole `fixtures/rst/` suite) and `events()` checks for `ooxml-wml`/`ooxml-pml`/`ooxml-sml`,
plus a streaming-writer fidelity check for `ooxml-sml`. See `fixtures/spec.md`'s new
"Cross-API harness" section for the equivalence definitions. Every other format tested in
`tests/run.rs` got an explicit capability declaration (`NotYetWired`, an honest "nobody has
individually audited this format's events()/StreamingParser/streaming-writer status yet"
placeholder) rather than silent absence from the harness — auditing and wiring real checks
for those is tracked, unstarted follow-up work.

Confirmed status of the four bugs this work was tasked with verifying (three were already
fixed by prior sessions; verify against source, not this summary, before relying on it):
1. **rst-fmt's `events()`/`StreamingParser`/`Writer` modules orphaned by a bad merge**:
   already fixed (`bbf1d7ffc6`, `3bb80ac74e`) with a permanent regression guard
   (`crates/formats/rst-fmt/tests/no_orphan_modules.rs`). Confirmed still fixed: all three
   modules compile, are `pub use`-re-exported from `lib.rs`, and the new harness exercises
   all three directly against the full `fixtures/rst/` suite.
2. **`ooxml-wml` `events()` treating `<w:document>` as unknown and swallowing the body**:
   this specific description is false today — `is_transparent_wrapper`
   (`crates/formats/ooxml-wml/src/events.rs:80-98`) explicitly descends into
   `document`/`body`. However, a real, previously-undocumented bug was found while wiring
   the new `wml_events_reaches_and_correctly_orders_paragraph_text` check: for the most
   common real-DOCX paragraph shape (`<w:p><w:r><w:t>…</w:t></w:r></w:p>`, no `<w:pPr>`
   before the run), `events()` drops the `Text` event entirely and emits `EndParagraph`
   before `EndRun` (violating well-nestedness). Root cause traced to `read_props`
   (`events.rs:386-463`)'s lookahead recursing into `self.open(child_kind)` when it meets a
   nested container before finding the expected props element, and that recursive `open()`
   call's `self.queue(...)` (`events.rs:146-149`) unconditionally overwriting
   `pending[0]`/`pending[1]` set by the outer call — plus the child's `ContextFrame` getting
   pushed onto `stack` before the parent's own, inverting end-tag pop order. **`ooxml-pml`'s
   `events.rs` shares the identical `queue()`/`read_props` pattern
   (`events.rs:99-118,480-495`) and reproduces the same Text-drop/reversal bug.** Tracked as
   `KnownFailure { format: "docx"/"pptx", api: "events" }` in the new harness — not fixed
   here (out of scope for the harness-wiring task; needs its own fix pass in `ooxml-wml`'s
   `read_props`/`queue`, then porting the fix to `ooxml-pml`, which is presumably
   copy-derived from the same codegen template).
3. **`ooxml-pml` rewriting every non-rectangular shape as a rectangle**: already fixed
   (documented at `TODO.md` "ooxml-pml geometry threading" entry, dated 2026-07-29);
   confirmed by the harness's `pml_events_reaches_slide_text` groundwork (blocked by finding
   #4 below before it can independently re-verify the geometry fix through `events()`
   end-to-end from real slide XML).
4. **`ooxml-pml` `events.rs` cannot reach slide text at all** (`<p:txBody>` not in
   `dispatch_start`, falls to `skip_element`): confirmed still open, already documented at
   `TODO.md:160-175` (untouched here per the task's explicit instruction not to fix it).
   `pml_events_reaches_slide_text` in the new harness reproduces this directly: feeding a
   `<p:sld>`-wrapped fragment through `ooxml_pml::events::events` yields
   `[StartPresentation, Text("\n"), EndPresentation]` — the shape/text never appears.
   Tracked as `KnownFailure { format: "pptx", api: "events" }` (the same entry as #2, since
   once txBody-reachability is fixed the shared queue-clobber bug would still corrupt the
   result).
5. **New finding, not in the original bug list**: wiring
   `rst_streaming_parser_matches_events_under_adversarial_chunking` across the *entire*
   `fixtures/rst/` suite (rst-fmt's own pre-existing streaming tests covered only 6
   hand-picked chunk-splitting cases, none a multi-item definition list) found that
   `StreamingParser` closes and reopens a multi-item RST definition list as one
   `StartDefinitionList`/`EndDefinitionList` pair *per item*, instead of one list spanning
   all items the way `events()` produces. Tracked as `KnownFailure { format: "rst", api:
   "streaming_parser" }`; not fixed here.
6. **`ooxml-sml` streaming writer dropping row properties and cell `style_index`**: already
   fixed and pinned by `crates/formats/ooxml-sml/tests/streaming_writer.rs`'s
   `row_and_cell_attributes_pass_through`; the new harness reproduces the same property
   independently in `sml_streaming_writer_preserves_row_and_cell_attributes` (writes a row
   with `height`/reference `7` and a cell with `style_index: 3` through `SmlWriter`, unzips
   the resulting XLSX package, and asserts `xl/worksheets/sheet1.xml` contains both
   attributes) — passes.

**`docs/format-audit.md`'s docx/pptx/xlsx `5†`/`5†` Office-table entries do not currently
carry a footnote or caveat naming findings #2 and #5 above** (the `†` there is defined for
Pandoc-oracle-harness limitations, unrelated). Not changed in this pass — the "5-Production"
claim concerns whole-document `parse()`/`emit()` fidelity via the adapter, which these
`events()`/`StreamingParser`-specific bugs don't directly contradict, but a reviewer auditing
those rows should be aware `events()` is not yet at parity for `docx`/`pptx`, and rst-fmt's
`StreamingParser` has one open construct gap.

## Open Threads

- **Footnote IR shape: embedded-content vs. linked-by-label are both live, unreconciled
  conventions (found during 2026-07-28 ADR audit).** `docs/adr/0001-footnote-ref-def-
  separate-node-kinds.md` originally claimed `footnote_ref`/`footnote_def` are always linked
  by an id/label property, never one embedding the other. That's false as a description of
  the codebase: `rescribe-read-rtf`, `rescribe-read-docx`, and JATS's `<xref ref-type="fn">`
  handling in `rescribe-read-jats` all embed the footnote body directly as children of
  `footnote_ref`, with no `footnote_def` node ever created. Only `rescribe-read-odt` and
  `rescribe-read-docbook`'s `<footnote>`/`<footnoteref>` pair implement the claimed
  linked-by-label shape. The ADR has been rewritten to describe both shapes as they actually
  exist, rather than asserting a uniform rule that isn't followed. **Not resolved**: whether
  the IR should converge on one shape (most likely always-linked, since it can represent both
  inline-at-marker and hoisted-elsewhere placement, while the embedded shape cannot express
  hoisting) is a real design fork with real migration cost across at least three readers — not
  decided here, and not something to guess past. Pick this up as its own task if/when a
  consumer actually needs uniform footnote handling across formats.

- **`ooxml-wml`'s streaming writer is real now; its `StreamingParser` is fenced, not fixed
  (2026-07-29).** `WmlWriter` no longer buffers `OwnedWmlEvent`s or reconstructs a
  `Paragraph`/`Run`/`Table` AST — each event goes straight into the open
  `word/document.xml` ZIP entry through a fixed 64 KiB window. Peak live heap for 100k
  paragraphs went 160.3 MB → 0.49 MB (85.9x growth → 1.00x), and throughput went from
  7.74x slower than `DocumentBuilder` to 0.52x (1.9x faster). Numbers, methodology and the
  construct-by-construct deferred/straight-through split are in `docs/format-audit.md`'s
  streaming inventory; `tests/streaming_writer_memory.rs` and
  `examples/streaming_writer_throughput.rs` reproduce them.

  **What is fenced, precisely.** `ooxml-wml`'s chunk-driven *reader* still buffers the
  whole DOCX (`BatchParser::feed` appends to a `Vec<u8>`; `finish()` calls
  `Document::from_reader`). Two separate facts, so the fence is not mistaken for a
  structural impossibility:
  - **The ZIP container is not the blocker.** The central-directory-at-the-end layout does
    *not* force reading the tail first: zip 7 ships `zip::read::stream::ZipStreamReader`,
    which walks local file headers sequentially. So an entry can be located and inflated
    before the central directory is seen.
  - **The API shape is the blocker.** `ZipStreamReader::visit` *pulls* from a `Read`, while
    `feed(chunk)` *pushes*. Bridging them needs either a thread + pipe, or a hand-rolled
    local-header state machine driving `flate2::Decompress` incrementally. And
    `BatchParser::finish()` returns a materialised `Document<Cursor<Vec<u8>>>`, so even a
    bounded feed path could not satisfy the current signature. A genuine bounded reader is
    a new `StreamingParser<H: Handler>` surface — which `ooxml-wml` does not expose at all
    today — not a patch to `BatchParser`.
  - One further wrinkle any such reader must decide, rather than discover late: the main
    part's path comes from `_rels/.rels`, which is conventionally but not normatively
    stored before `word/document.xml`. A sequential reader either assumes the
    `word/document.xml` convention or buffers until the rels part appears.

  Not started, deliberately: `ooxml-sml` and `ooxml-pml` writers (see below).

- [x] **`ooxml-sml`'s writer made genuinely incremental (2026-07-29) — resolves the previous
  entry.** `SmlWriter` no longer holds a `WorkbookBuilder`; each `SmlEvent` is written
  straight into the open `xl/worksheets/sheetN.xml` ZIP entry through the same fixed 64 KiB
  output window `ooxml-wml` uses, reusing `Row`/`Cell`'s own generated
  `ToXml::write_attrs` to open tags rather than reconstructing an AST. Measured, release
  build, 100k rows x 3 cells (20 distinct strings), inputs prepared outside the timed
  region:

  | | peak live heap | wall time |
  |---|---|---|
  | before (streaming, via `WorkbookBuilder`) | 233,578,753 B (222.76 MB) | 456.9 ms |
  | after (streaming, incremental) | 484,831 B (0.46 MB) | 137.3 ms |
  | `WorkbookBuilder` path itself (reference) | 296,247,374 B (282.5 MB) | 390.9 ms |

  481.8x less peak memory, 3.3x faster than the old streaming writer, 2.9x faster than the
  `WorkbookBuilder` path it used to wrap. No per-tag throughput regression was hit (unlike
  `ooxml-wml`'s intermediate attempt) — the fixed output window was applied from the start.

  Genuinely deferred, O(bounded) not O(cells): `xl/workbook.xml` + `.rels` (O(sheet count) —
  sheet list only complete once every `StartWorksheet` is seen); `xl/sharedStrings.xml`
  (string *values* are interned incrementally with stable indices the moment each is first
  seen — O(distinct strings) — but the part listing every distinct string can only be
  written once streaming ends, the same bound SST dedup always costs). `<dimension>` and
  `<row spans="...">` are omitted outright (optional per ECMA-376 §18.3.1.35, would require
  buffering a whole sheet's cell references before the first `<row>`). Styles, charts,
  comments, pivot tables, and merged cells have no `SmlEvent` representation at all
  (`WorkbookBuilder`-only), so they are out of scope for this writer, not deferred by it.

  Two pre-existing fidelity gaps fixed as a side effect: row attributes (including the row
  number) and cell `style_index` were previously dropped entirely by the event-driven writer
  (it only ever read `props.reference`/`props.cell_type`, with `StartRow { .. }` grouped
  into a no-op match arm); both now pass through, since the incremental writer has the full
  `Row`/`Cell` props in hand at exactly the point it needs to emit them.

  Tests added: `tests/streaming_writer.rs` (round trip + attribute pass-through + SST
  dedup), `tests/streaming_writer_memory.rs` (permanent memory-guard test, test-local
  `#[global_allocator]`, fails if buffer-everything behavior returns),
  `tests/bench_streaming.rs` (`#[ignore]`-gated manual throughput/memory comparison vs
  `WorkbookBuilder`, inputs pre-built outside the timed region per
  `docs/format-library-design.md`'s benchmarking convention).

  Out of scope, separately filed: `ooxml-sml`'s reader side (`StreamingParser<H>` surface
  does not exist yet — same boundary as `ooxml-wml`'s reader-batch gap above). `ooxml-pml`
  untouched (see below).

- **`ooxml-pml`'s geometry loss is fixed (2026-07-29); the writer is still hollow
  (buffer-then-delegate-to-builder), fenced below.** `PmlEvent::StartShape` previously
  carried only `ShapeTransform` (bounding box); `<a:prstGeom>`/`<a:custGeom>` were never
  read by `events.rs` and `PmlWriter` always emitted `PresetGeometry::Rect` regardless of
  the source shape. Confirmed via direct read (not guessed) that this loss was confined to
  the events()/`PmlWriter` pair: `Presentation`/`Slide` (the AST/`parse()` path) wrap the
  generated `types::Shape` directly and round-trip full `spPr` fidelity through the
  generated FromXml/ToXml — `CTPresetGeometry2D.av_lst` and `CTCustomGeometry2D` are both
  modeled there already. The builder path (`ShapeBuilder`) had a related but smaller gap:
  `set_geometry` could set a preset name but hardcoded `av_lst: None` and had no custGeom
  support at all — fixed in the same commit.

  Added `ShapeGeometry` (`generated_events.rs`): `Preset { preset, adjustments }` for
  `<a:prstGeom>`/`<a:avLst>` (modeled — this is a small, well-defined attribute list, not
  format-specific enough to warrant raw-only treatment), and `Custom(RawXmlElement)` for
  `<a:custGeom>` (raw-preserved verbatim — no cross-format equivalent, per CLAUDE.md's
  raw-preservation pattern). `events.rs`'s `extract_xfrm_from_sppr` now reads both;
  `ShapeBuilder` gained `set_geometry_adjustments`/`set_custom_geometry`; `PmlWriter`'s
  `process_slide` threads `ShapeGeometry` through and calls the full `ShapeBuilder` (not
  `add_text_at`) whenever geometry is present. 8 new tests in
  `ooxml-pml/tests/streaming_writer_geometry.rs` cover reader extraction and writer
  emission for ellipse, roundRect+avLst, and custGeom.

  **Two further bugs found while writing the round-trip tests, left fenced rather than
  fixed here — both are cross-cutting, not specific to the geometry fix:**

  1. **ooxml-dml's generated `CTPath2D` parser/serializer put `x`/`y` directly on
     `<a:moveTo>`/`<a:lnTo>`, not on a nested `<a:pt>`.** Real ECMA-376 (and every
     PowerPoint-authored PPTX) writes `<a:moveTo><a:pt x=".." y=".."/></a:moveTo>`; the
     codegen output (`ooxml_dml::generated_parsers::CTPath2D::from_xml`,
     `generated_serializers.rs`'s matching `ToXml` impl) instead reads/writes those
     attributes on `moveTo`/`lnTo` themselves. This is internally self-consistent for
     ooxml-dml's own `parse(emit(x)) == x`, so existing ooxml-dml/ooxml-wml/ooxml-sml
     tests don't catch it, but it means `RawXmlElement::parse_as::<CTCustomGeometry2D>()`
     fails on any real-shaped custGeom fed through `PmlWriter`'s custGeom write path
     (`streaming.rs`). Verified with `MissingAttribute("x")` from a real-shaped fixture in
     a scratch test. Landed as designed-for gracefully: `PmlWriter` falls back to the
     default `Rect` (keeping the shape's text) rather than corrupting output when this
     happens, locked in by `pml_writer_falls_back_gracefully_on_unparseable_cust_geom`.
     Root-causing and fixing this belongs in `ooxml-codegen`'s handling of the
     `EG_Path2DMoveTo`-style group-to-type mapping — cross-cutting across every consumer
     of `CTPath2D`, not an `ooxml-pml`-scoped fix.
  2. **`events.rs`'s true SAX reader was never exercised by any test until now, and does
     not treat `<p:txBody>` as a transparent container.** `dispatch_start` only recognizes
     `sp`/`graphicFrame`/`tbl`/`tr`/`tc`/`p`/`r`/`hyperlink`; any other `Start` element
     (including `<p:txBody>`, `<p:nvSpPr>`, `<p:style>`) falls through to
     `skip_element`, which skips the *entire* subtree. Since a shape's paragraphs/runs
     live inside `<p:txBody>`, real slide XML fed through `events()` never reaches its own
     text — `PmlWriter`'s shape-filtering-by-nonempty-text (`process_slide`) means such
     shapes are silently dropped from output entirely. Confirmed directly: an end-to-end
     `<p:sld>`-wrapped fixture through `pml_events()` produced zero shapes. Worked around
     for the new geometry tests by driving `events()` with `<p:sp>` fragments directly
     (bypassing the txBody descent this exposes) rather than full slide parts. Fixing this
     requires `events.rs` to either add `txBody` (and the wrapper elements
     `p:sld`/`p:cSld`/`p:spTree`/`p:nvGrpSpPr`/`p:grpSpPr` needed to drive it from a real
     slide part at all) as transparent pass-through containers, or restructure
     `dispatch_start`'s container model — general `events.rs` rework, explicitly out of
     scope for the geometry fix per the task that produced it.

  **What remains unfenced from the earlier writer-hollowness finding, still open:**
  `PmlWriter::finish()` still buffers every `OwnedPmlEvent` into a `Vec` and replays it
  through `process_pml_events`/`process_slide`'s hand-rolled little state machine to
  reconstruct calls against `PresentationBuilder`, which then does the actual `write()` —
  O(full input) memory, not O(nesting depth), and delegates to the builder's emit path
  exactly as CLAUDE.md forbids. Not started, deliberately — the `ooxml-wml` rework
  (`849480a98c`) is the ready-made template: `PackageWriter::start_part`/`write_part_data`
  + the `Write` impl on `PackageWriter` (added in `ooxml-opc` for `ooxml-wml`) make
  "stream one XML part into the ZIP, accumulate only the relationship table, write rels +
  content-types at finish" mechanical. PPTX-specific complications the next agent should
  work out before starting (not yet analyzed): slide/layout/master relationship IDs and
  `[Content_Types].xml` entries, `presentation.xml`'s `sldIdLst` (needs relationship IDs
  assigned as slides are created — mirrors the WML rework's rels-table-accumulated-until-
  finish pattern), and whether `PmlWriter`'s current "flatten paragraphs into a `String`
  and only add a shape once its container closes with non-empty text" model can survive
  becoming truly incremental (it currently must buffer at least one shape's worth of
  events to know if a shape ends up with any text — that's fine, O(nesting depth), but the
  per-slide `Vec<(String, Option<ShapeTransform>, Option<ShapeGeometry>)>` buffering
  across *all* shapes on a slide before calling `add_slide()` should collapse to
  streaming each shape into the open slide part as its `EndShape` arrives).

- **`rst-fmt`'s streaming/batch/writer-streaming APIs recovered (2026-07-28) — resolves the
  previous entry.** Root cause pinned exactly: not commit `79ea2ce7af` itself but merge
  commit `383d4e6adf` (`Merge: 395a7ee532 79ea2ce7af`), which took the `79ea2ce7af` topic
  branch's entire `lib.rs` verbatim instead of merging it — discarding 1443 lines mainline
  had gained in parallel (`mod events;`/`mod batch;`/`mod writer;`, the `EventIter` struct,
  and the `Block::LineBlock`→`Block::Div{class:"line-block"}` migration). Confirmed via
  `git log --format=%H --follow --reverse -- .../lib.rs` showing `pub mod events` present at
  every commit through `395a7ee532` and absent from `79ea2ce7af` onward, then
  `git show 383d4e6adf --stat` showing the 1443-line loss landed in the merge, not the
  topic-branch commit.
  What was done:
  - `events()`/`EventIter`: **not** a resurrection of the old ~1300-line duplicate grammar
    (the salvaged `events.rs` embedded a full second copy of every `try_parse_*` method,
    operating on its own state instead of `Parser`'s). Rebuilt as a thin composition —
    `EventIter` holds a `Parser<'a>` and calls its `try_parse_block()` one top-level block
    at a time, then `expand_block`/`expand_inline` lazily turn that already-parsed `Block`
    into a `Vec<Frame>` stack (`O(nesting depth)` to drain, not `O(full document)`). `parse()`
    and `events()` now share the one parser implementation, satisfying ADR 0003.
  - `batch::StreamingParser`: unchanged — it already re-parses each accumulated blank-line-
    delimited block through `events()`, genuinely `O(largest block)`.
  - `writer::Writer`: rewritten. The salvaged version buffered the whole `Vec<Event>` and
    only called `build()` at `finish()` — the "fake streaming, wraps the tree builder"
    pattern CLAUDE.md explicitly names and rejects. Replaced with a frame stack that flushes
    each completed **top-level** block to the sink immediately (via the shared, now
    `pub(crate)` `build_block`/`BuildContext`), `O(largest top-level block + nesting depth)`.
  - Tests: `events()`≡`parse()` shape-equivalence over 11 inputs covering every `Block`/
    `Inline` variant (compares discriminant-tag sequences, not concrete types, so it doesn't
    require `events()` and `parse()` to produce identical Rust values — just the same
    constructs in the same order); 6 `StreamingParser` tests feeding input one byte at a
    time with deliberately awkward splits (mid-directive-keyword, mid-heading-underline,
    mid-list/blank-line boundary, mid-footnote-continuation, mid-UTF-8-character,
    mid-table-border); one round-trip test through `Writer` covering headings, inline
    formatting, both list kinds, code blocks, definition lists, grid tables, footnotes,
    block quotes, and a transition rule in one document. 45 unit tests + 3 doctests pass;
    `cargo clippy --all-targets --all-features -- -D warnings` clean.
  - `crates/formats/rst-fmt/tests/no_orphan_modules.rs` added as the recurrence guard: walks
    `src/` from `lib.rs` following file-backed `mod name;` declarations, fails if any `.rs`
    file is unreachable. Verified it actually catches this bug class: with a genuinely
    unreferenced new file added to `src/`, `cargo build` stays green (proving the blind
    spot) while this test fails. A one-off workspace-wide sweep with the same logic found
    no real orphans elsewhere in `crates/` — 3 files flagged, all confirmed false positives
    from heuristic gaps (two crates with a sibling `lib.rs` + `main.rs` in one `src/` dir;
    one `#[path = "registry_generated.rs"]` redirect in `jats-fmt` that isn't a plain
    `mod name;`).
  - Two **pre-existing, unrelated** `build_block` bugs found via the new Writer round-trip
    test (not fixed — logged here, out of scope for this pass): (1) admonition directives
    (`.. note::`, `.. warning::`, etc., represented as `Block::Div{class: Some(name), ..}`)
    lose their directive wrapper entirely on write-back — the builder's `Block::Div { children,
    .. } => build_blocks(children, ctx)` arm ignores `class`, so `.. note::\n\n   text\n`
    round-trips to bare `text` with no way to tell it was ever an admonition. (2)
    `Block::FootnoteDef`'s builder emits only a single trailing `\n`, not a blank-line
    separator, so a block immediately following a footnote definition with ≥3-space
    indentation (e.g. a block quote) gets swallowed into the footnote body as a continuation
    line on re-parse. Both reproduce with plain `crate::build()` — no streaming API involved,
    confirmed by isolating each with a throwaway `#[test]` calling `parse()`+`build()` only.
  - **Not promoted to 5-Production**: `fuzz_rst_reader`/`fuzz_rst_roundtrip` only ever drove
    `parse()`/`build()` via the `rescribe-read-rst`/`rescribe-write-rst` adapters; no fuzz
    target exercises `events()`, `StreamingParser`, or `Writer` directly. Adding
    no-panic + chunked-split fuzz targets for those three specifically is the concrete
    remaining step. `docs/format-audit.md`'s rst row is now R:4/W:4 (up from R:4/W:2 —
    reader-ast/writer-builder were already fine and unaffected; all five API modes are now
    real and tested, but not yet fuzzed as such).
  - **Cross-cutting gap found, explicitly out of scope here**: `crates/rescribe-fixtures/
    tests/run.rs` tests every format crate via `parse()`/`emit()` only — for every format,
    not just rst, no fixture is wired through `events()`/`StreamingParser`/a streaming
    writer. This is the same blind-spot class the markdown suite hit (`backend_pulldown`
    vs. the default backend). Fixing it is a horizontal sweep across every format crate and
    does not belong to the rst vertical; flagging it here so it isn't lost.
  - Two of the two-pre-existing-bug fixes above (admonition wrapper, footnote blank-line
    separator) remain **open follow-up work** for whoever next touches `rst-fmt`'s
    writer-builder.

- **Status reset: construct-completeness marked unverified pending a construct registry
  (2026-07-28).** This session's DocBook/JATS/TEI work (see the entries below and in
  `docs/format-audit.md`'s change log) produced four pieces of evidence that no format's
  hand-written construct checklist can be trusted as a completeness measurement:
  (1) `fixtures/docbook/COVERAGE.md` and `fixtures/jats/COVERAGE.md` denominators moved
  94→105→117 and 106→109→133 across one session purely from incidentally-noticed gaps
  (commit `c2d6028c9a`, 265/216 element names found enumerated nowhere against the
  authoritative DocBook 5.2 / JATS 1.3 element indexes); (2) the `is_block_element`
  "schema-verification" methodology used on DocBook (`abd6dd447d`), JATS (`20c27d032e`),
  and TEI (`3e3d84bcef`) only re-checked elements already on each list against the spec —
  it never asked which elements were absent from the list entirely, and a later full
  re-check against DocBook's ~392-element index (`be578fb98c`) found 17 more genuine
  misclassifications beyond the 4 the audit had named, meaning the JATS/TEI "verified,
  zero/one misclassifications" results carry the same unchecked blind spot; (3) a
  COVERAGE.md checkmark has only ever asserted one of `fixtures/spec.md`'s six coverage
  dimensions (one basic fixture per named construct), never the Adversarial/Pathological
  dimensions per-construct; (4) `crates/rescribe-fixtures/tests/run.rs` was found
  validating `backend_pulldown::parse` for markdown while the actual default backend
  silently misparsed front matter (fixed in `1574db80e8`) — a green suite had not been
  proof the default path worked.

  None of this says any specific format's coverage is wrong — it says the ratio was never
  a measurement to begin with, for any format, because it was hand-typed against memory
  rather than checked against the format's own spec. Per CLAUDE.md, "5-Production requires
  100% construct coverage — not enough for common cases," so this is a real gap in every
  existing 5-Production sign-off, not just the three XML formats audited this session.

  **What changed:** `docs/format-audit.md` gained a `CC` (Construct Coverage) column,
  `U` (unverified) for every format, with a section explaining what would close it out (a
  spec-derived construct registry — see below) and citing the four findings above. Every
  `fixtures/*/COVERAGE.md` gained an identical boilerplate header note stating the
  denominator is hand-curated and unverified. `docs/adr/0004-xml-classifier-schema-
  verification-methodology.md` records the corrected methodology (check the full spec
  element index for elements that *should* be classified and aren't, before re-checking
  entries already listed). **2026-07-28 ADR-audit note**: this ADR was later rewritten in
  place rather than kept as an amendment layered on the original insufficient method — the
  original method was wrong from the moment it was written, not something that became
  insufficient later, so the amendment convention didn't fit. See the ADR itself for the
  current, single, corrected methodology.

  **What did NOT change:** no reader/writer/classifier code, no `R`/`W` stage numbers, no
  fuzz results, no fixture content. The API-modes/fuzz/fixture-suite work behind every
  existing "5" is real and stays exactly as recorded; only the construct-list-completeness
  claim is now explicitly flagged unverified instead of implicitly assumed true.

  **What closes `CC` out:** a construct registry — machine-readable, generated or checked
  against each format's actual spec/schema/DTD, not hand-typed from memory or "typical
  usage" — is being designed as a separate effort (an ADR + pilot; check `docs/adr/` for
  whether it has landed by the time this is picked up, and coordinate with it rather than
  re-deriving the same design). `CC` moves from `U` to `✓` per format only once that
  registry (or an equivalent spec-derived check) has actually been run against the format's
  construct list.

  **Concrete pending work, stated so it isn't repeated with the flawed method:** the JATS
  and TEI `is_block_element` classifiers need the same full re-verification DocBook already
  got in `be578fb98c` — enumerate every block-level element name from each format's own
  authoritative reference (JATS 1.3 Tag Library alpha-index; TEI P5 Guidelines element
  index), diff against every element `is_block_element` (and the reader's dedicated match
  arms) already handles, and confirm each candidate miss against the spec directly — not a
  re-check of entries already on the list. This is unstarted; do not mark JATS/TEI
  `is_block_element` "re-verified" on the basis of the original `20c27d032e`/`3e3d84bcef`
  passes, which used the insufficient method.

- **Construct registry designed and piloted on JATS; ADR 0013 landed (2026-07-28).**
  This is the "what closes `CC` out" work the Status-reset entry above points at. The
  design is recorded in `docs/adr/0013-per-format-construct-registry.md`; read it before
  extending the registry to another format, and do not re-derive the design.

  **Shape, in one line:** each `-fmt` crate carries a committed, spec-derived,
  machine-readable catalog of every construct its format defines, behind an opt-in
  `registry` Cargo feature, runtime-queryable, with constructs annotated by *slice*. The
  registry is spec-pure: support status is never a field, only a caller-side join
  (`Registry::not_handled`), so it doesn't churn when reader/writer work lands.

  **2026-07-28 amendment: slices are now two separate fields, not one.** ADR 0013's
  original decision 3 claimed OOXML's slices come from "namespace/part schemas" — false;
  `spec/ooxml-features.yaml`'s ~20 tags (`core`, `styling`, `charts`, …) are hand-chosen by
  this project, not spec-derived. That error produced a rule that outlawed OOXML's own
  shipping tags while blocking DocBook. Corrected in the ADR's amendment (read it) and
  implemented in `jats-fmt` (commit range starting `73c040bd`): `Registry`/`Construct` now
  carry `normative_slices` (spec-published, may be empty) and `pragmatic_slices` (ours,
  always permitted, explicitly non-normative). `registry_version` bumped 1 → 2. JATS's
  pilot populates only `normative_slices`; `pragmatic_slices` is empty throughout, since
  JATS's own modularization already does the decomposition job and inventing a second one
  would be noise, not value. Everything below in this entry that says "slice" without
  qualification is describing the *pre-amendment* single-field design as it stood at
  landing time (2026-07-28, same day) — read it as history, and see the ADR amendment for
  what actually ships now.

  **2026-07-28, later the same day: a second amendment adds content models and replaces
  hand-curation with scripted extraction.** Two human decisions resolved ADR 0013's open
  questions 4 and 2; both are implemented in `jats-fmt` (commits `af657562fd` for the ADR
  text, `ce5d98b054` for the implementation).

  - **Content models** (open question 4): `Construct::content_model: Option<ContentModel>`
    now records, per element, the *flattened* set of permitted direct children (each tagged
    `repeatable`), permitted attributes (each tagged `required`), and whether character data
    is allowed directly (`mixed`) — deliberately **not** the source schema's ordering, choice,
    group, or interleave structure (see the registry module docs and the ADR amendment for
    why flattening was chosen over a full grammar tree, and open question 6 for whether a
    richer representation is worth building later). `None` for attribute constructs, which
    have a value type rather than a content model. `registry_version` bumped 2 → 3.
    `derive-registry` resolves this by building a global `<define name>` table across every
    parsed RNG module and walking each `<element>`'s body, with a cycle guard on `<ref>`
    resolution. New `Construct` methods: `permits_child`, `requires_attribute`,
    `permits_attribute`.

    **Measured size impact**: the committed JATS registry grew from 4,079 lines / 90 KB to
    39,946 lines / 885 KB (~10×), split roughly evenly between JATS's own 305 elements
    (7,558 permitted-child entries) and the 181 embedded MathML elements (5,404 entries) — not
    a MathML-specific artifact, an inherent property of the flattened form at this vocabulary
    size (~25-30 permitted children per element on average). **Decided: no new Cargo
    sub-feature for content models** — they stay inside the existing `registry` feature.
    Justified in the ADR amendment (existing 3.6 MB `ooxml-wml` `generated.rs` precedent;
    `registry` already gates whether a consumer pays for the catalog at all; splitting would
    fork `--check`/provenance/derivation into two artifacts for a modest benefit). Flagged as
    a **per-format**, not general, call — re-check the size before assuming the same answer
    holds once TEI (614 elements) or DocBook (414 elements) get registries.

  - **Hand-curation replaced by scripted extraction** (open question 2): `SourceKind::HandCurated`
    is **removed outright** — not kept as a marked-unreliable fallback — and replaced by
    `SourceKind::ScriptedExtraction`: a format with no machine-readable schema derives its
    construct list via a committed, re-runnable script that extracts it from a published
    prose artifact (an HTML element index, etc.), the same reproducibility property a schema
    derivation has. No new top-level `Provenance` fields were needed — `source_base_url`,
    `derived_on`, `derived_by`, and `source_digests` already cover "source URL(s), retrieval
    date, checksum, extraction-script reference"; `SourceDigest` gained one small addition,
    an optional per-entry `url`, for extractions spanning several distinct published pages
    that don't share one base URL. **This retires "schema-derived vs. hand-curated" as a
    framing** — it was a false dichotomy; the real axis was always reproducible-vs-not, and
    a schema was only ever one way to be reproducible. Practical consequence: DocBook's
    *construct-list* rollout (not just its slice rollout, already unblocked by the first
    amendment) is now tractable without lowering the bar, once someone writes the extraction
    script against DocBook's own published element reference. Not resolved here: which
    specific artifact/script DocBook (or any other schema-less format) should use — that's
    rollout work, not a design decision.

  **Pilot: JATS 1.3 Archiving, end to end.** `crates/formats/jats-fmt/registry/jats-1.3-archiving.json`
  (human-readable derived source; see the 2026-07-28 runtime-representation entry below —
  this was `.yaml` until that change), `crates/formats/jats-fmt/src/registry.rs` (runtime
  API), `crates/formats/jats-fmt/src/registry_generated.rs` (committed generated Rust
  statics the runtime API reads), `crates/formats/jats-fmt/src/registry_derive.rs` (owned
  model, JSON I/O, Rust codegen — `registry-derive` feature),
  `crates/formats/jats-fmt/src/bin/derive-registry.rs` (schema walk + CLI wrapper around the
  above; `--check` drift verification), `scripts/jats/download-spec.sh` (fetches the schema;
  `spec/` is gitignored so it is never committed), and one real consumer,
  `crates/readers/rescribe-read-jats/tests/registry_coverage.rs`.

  **2026-07-28, later the same day: runtime representation changed from YAML/`serde_yaml` to
  committed generated Rust statics; source-of-truth format changed from YAML to JSON.**
  Implemented in `jats-fmt` only; DocBook/TEI have not been rolled out yet (rollout plan
  below updated to target this shape from the start).

  - `serde_yaml` is removed from `jats-fmt` entirely, in every feature (`registry` and
    `registry-derive`). No YAML parser of any kind is now in `jats-fmt`'s dependency graph
    (verified via `cargo tree -p jats-fmt --features registry-derive -e normal`).
  - `crate::registry`'s types (`Construct`, `ContentModel`, `Registry`, etc.) now hold
    `&'static str` / `&'static [T]` fields instead of owned `String`/`Vec`. `registry()`
    returns a reference to a `static REGISTRY: Registry` compiled directly into the binary
    (`crates/formats/jats-fmt/src/registry_generated.rs`, committed) — no parsing, no
    `OnceLock`, no allocation at call time.
  - The human-readable source moved to `registry/jats-1.3-archiving.json`
    (`serde_json`, not YAML), read only by the offline `registry-derive` tool, never at
    runtime. `serde_json` is an existing workspace dependency (used by `rescribe query`),
    gated behind `registry-derive` only — not part of a normal `registry`-feature build.
  - New module `crates/formats/jats-fmt/src/registry_derive.rs` (feature `registry-derive`)
    holds the owned model types (mirroring `crate::registry`'s shapes with `String`/`Vec`),
    `Registry::from_json`/`to_json`, and `emit_rust` (the codegen that produces
    `registry_generated.rs`'s text, with content-model deduplication — see below).
    `src/bin/derive-registry.rs` is now a thin CLI: schema walk (unchanged logic) →
    `registry_derive::Registry` → write both the JSON source and the generated Rust file.
    New `--emit-rust-only` flag regenerates `registry_generated.rs` from the committed JSON
    alone, with no schema involved.
  - **Two independent drift checks**, per the ADR's existing schema-vs-source pattern
    extended one level: (1) `derive-registry --check` — schema vs. committed JSON, needs the
    schema fetched locally, unchanged from before; (2)
    `registry_derive::drift_tests::generated_rust_matches_committed_source` — committed JSON
    vs. committed `registry_generated.rs`, needs **no schema**, runs as an ordinary
    `cargo test -p jats-fmt --features registry-derive`. A third test,
    `committed_source_round_trips`, confirms the JSON model survives a
    serialize/deserialize round trip.
  - **Content-model deduplication**: distinct `ContentModel` values (children set +
    attributes set + `mixed`) are emitted once each as a named `static` (`CM_0`, `CM_1`, …)
    and referenced by pointer from every construct sharing that shape, instead of each
    construct carrying its own copy. Measured on the current derivation: 486 elements have a
    content model; only 270 distinct shapes exist (44.4% would be duplicate data without
    this). `crate::registry::tests::content_models_are_deduplicated` pins this by pointer
    identity.
  - **Measured rlib size** (`cargo build -p jats-fmt --release`, clean builds,
    `target/release/libjats_fmt.rlib`): no `registry` feature, 440,032 bytes (unchanged —
    this path never touched YAML). With `registry` feature: was 4,244,748 bytes under the
    old YAML/`serde_yaml` runtime-parsed design; now 2,275,366 bytes under committed Rust
    statics. Growth over baseline dropped from 3,804,716 bytes to 1,835,334 bytes (~52%
    reduction). The remaining growth is the construct data itself (734 constructs, ~12,900
    permitted-child/attribute entries even after dedup) compiled into rodata, not parser
    machinery.
  - **File sizes**: `registry/jats-1.3-archiving.json` is 76,735 lines / ~1.66 MB (was
    39,943 lines / 885 KB as YAML — JSON's punctuation overhead and one-entry-per-line
    formatting account for the growth; this file is not read by any normal build).
    `src/registry_generated.rs` is 47,572 lines / ~1.02 MB after this repo's pre-commit
    hook's `rustfmt` pass (one struct literal per multi-line block, not the denser one-line
    form `emit_rust` originally produced) — `emit_rust` now runs its output through
    `rustfmt --edition 2024` itself (shelling out, `registry-derive`-only), so its output is
    already byte-identical to what the hook would otherwise produce, and the
    source→generated drift test stays meaningful regardless of whether the hook runs.
    Committed and is what `registry`-feature builds actually compile.
  - **MathML sharing across formats (JATS/DocBook/TEI/BITS all embed the same MathML
    vocabulary) was assessed and explicitly deferred, not built.** 181 of JATS's 486
    registry elements are MathML (confirmed again this session via
    `registry_coverage.rs`'s `denominator_is_plausible` test: 486 total − 305 JATS-native =
    181). Deferred because no second format has a registry yet to prove the sharing shape
    against — building a shared crate now would be speculative. If picked up later: the
    shape is a small crate (e.g. `mathml-registry-fmt`) exporting the same
    `&'static`-statics shape as this rollout, consumed as an ordinary Cargo dependency by
    `jats-fmt`/`docbook-fmt`/`tei-fmt`'s own `registry_generated.rs`-equivalents; cost of
    deferring is committed-file duplication only (each format's generated file would embed
    its own copy of the ~181-element/~5,400-entry MathML block) — zero runtime cost either
    way, since both shapes are statics.
  - Flow-style/compact-array source formatting was considered and not implemented: the JSON
    source is pretty-printed one entry per line (not compacted), so that adding or removing
    one child/attribute is a one-line diff. This is a JSON-vs-YAML question now, not a
    YAML-flow-style question, since the source format itself changed.
  - All prior tests pass unchanged in behavior: `cargo clippy --all-targets --all-features --
    -D warnings` and `cargo test --all-features -q` both clean;
    `registry_coverage.rs`'s regression guard for the 176-element gap
    (`registry_contains_the_elements_the_hand_written_checklist_missed`) still passes.

  **It works.** 305 JATS elements derived against the ~306 the Tag Library's alpha index
  lists; 176 of them are never mentioned anywhere in `rescribe-read-jats`'s source, grouped
  by slice in the test output. Every element the 2026-07-28 hand audit found — `hr`,
  `sub-article`, `response`, `ruby`/`rb`/`rt`/`rp`, `chem-struct`, `array`, `index-term`,
  `media`, `alt-text` — falls out mechanically, and a regression test pins them.

  **JATS `CC` is not yet `✓`.** The registry exists and the gap list is real, but nothing
  has been *closed* — 176 unmentioned elements is the starting number, not the ending one.
  `docs/format-audit.md`'s `CC` column for JATS should move from `U` only once those gaps
  are triaged (many will be legitimately covered by the catch-all; that triage is the work).

  ### Rollout plan

  1. **TEI next, and it should be easy.** TEI P5 has the best schema story of any format
     here: `p5subset.xml` (ODD) declares 22 `<moduleSpec>` modules and 614
     `<elementSpec module="…" xml:id="gi-…">` — module membership is a *literal attribute*,
     so slices need no inference at all. Dual CC BY 3.0 / BSD-2-Clause, so vendorable if
     ever wanted. Citation: `xml:id="gi-<element>"` plus the `ref-<element>.html` reference
     page. Derivation reads XML, so `tei-fmt`'s own parser suffices — same shape as the
     JATS pilot. Note the ODD declares 22 modules while the Guidelines' ST chapter prose
     says 23; resolve that discrepancy rather than picking one. **Target the JATS registry's
     current shape from the start** (2026-07-28 runtime-representation change, see above):
     committed generated Rust statics as the runtime artifact
     (`tei-fmt/src/registry_generated.rs`), a JSON human-readable source
     (`tei-fmt/registry/tei-p5.json`), an owned model + codegen module
     (`tei-fmt/src/registry_derive.rs`), and the two-tier drift check (schema-vs-source,
     source-vs-generated) — do not build a YAML/runtime-parsed version first and migrate it
     later. With 614 elements (vs. JATS's 486 constructs total), measure the content-model
     dedup ratio and rlib size before assuming JATS's numbers (44.4% duplicate shapes, ~52%
     rlib-growth reduction vs. the old design) transfer.

  2. **DocBook is unblocked for rollout (2026-07-28 amendment; was "blocked on an open
     question, not on effort").** DocBook 5.2's *normative* OASIS artifact
     (`docs.oasis-open.org/docbook/docbook/v5.2/os/rng/docbook.rnc`) is a flattened
     monolith: 414 distinct elements in 420 **anonymous** `div { }` blocks with no module
     identity anywhere. So DocBook still cannot source *normative* slices from its
     normative schema — that fact hasn't changed. What changed is that this no longer
     blocks anything: under the two-field model, DocBook ships with `normative_slices: []`
     and a recorded reason immediately, no decision needed first. The only genuinely open
     piece is narrower and optional — whether to also populate `pragmatic_slices`, and if
     so from what (invent fresh, or borrow the shape of the non-normative Codeberg TC
     source's ~35 `.rnc` modules, codeberg.org/docbook/docbook,
     `schemas/docbook/src/main/docbook/` — as an idea, not a redistribution, so its
     unverified license doesn't need resolving for this use). See ADR 0013 open question 1
     (as amended) for the full statement. Separately, unaffected by any of this: DocBook's
     schema is RNC, not RNG, so deriving its *construct list* (the denominator) still needs
     a compact-syntax reader — see item 4. **Also target committed generated Rust statics
     from the start**, same as the TEI note above — no interim YAML/runtime-parsed version.

  3. **ooxml migration onto the uniform design.** Concretely:
     - **What stays:** `crates/tools/ooxml-codegen`'s `lexer.rs`/`parser.rs`/`ast.rs` (they
       are format-agnostic and already run against both ECMA-376 and ODF); the
       committed-artifact + env-var-gated regeneration pattern; the `"*"` wildcard escape
       hatch; `analysis.rs`'s skip-list reasoning about `AG_`/`EG_` groups.
     - **What moves:** `spec/ooxml-features.yaml`, `spec/ooxml-names.yaml` shard into
       per-crate `crates/formats/ooxml-{wml,sml,pml,dml}/registry/`. This deletes the
       hard-coded `match module { "sml" => … }` in `NameMappings::for_module` /
       `FeatureMappings::for_module` — note the latter currently defaults *unknown* modules
       to `sml`, a latent bug. It also makes `odf-fmt` a first-class citizen; it currently
       points at `spec/odf-names.yaml` and `spec/odf-features.yaml`, **neither of which
       exists**, so ODF codegen runs entirely unmapped today.
     - **What must be added:** provenance (the existing YAMLs have *none* — no ECMA edition,
       no Strict-vs-Transitional marker even though `build.rs` hard-codes Transitional, no
       checksums, no derivation date); a declared slice vocabulary as data rather than a
       header comment (the current comment documents `revisions`, which has 0 uses, while
       the data uses `track-changes`, which has 70 — already drifted, nothing checks it);
       and promotion of `analyze_schema` from `OOXML_ANALYZE`-gated stderr to a real test
       (`has_unmapped()` is already written and is dead code).
     - **Corrected by the 2026-07-28 amendment: OOXML's existing ~20 tags are legitimate
       `pragmatic_slices` as-is, not a violation to fix.** ADR 0013 originally claimed
       OOXML's slices come from "namespace/part schemas" — that was factually wrong; the
       tags in `spec/ooxml-features.yaml` are hand-chosen by this project. The corrected
       migration: the existing `core`/`styling`/`charts`/… tags move into each shard's
       `pragmatic_slices`, explicitly marked non-normative, with no further justification
       needed. The *real* namespace/part decomposition (21 namespace schemas, ~59 parts)
       remains available as a separate, future `normative_slices` source if someone does
       that derivation work — it is not required for this migration to land.
     - **Resolved (2026-07-28, ADR 0013 Amendment 3): the slice/Cargo-feature collapse rule
       is OR-of-all.** `primary_feature` (which silently kept only the first tag, so
       `drawingHF: [drawings, layout]` gated on `sml-drawings` alone and `layout` was inert
       with no diagnostic) is replaced by `FeatureMappings::feature_gates` (returns every
       non-core tag) plus a shared `cfg_predicate` helper used identically by
       `codegen.rs`/`parser_gen.rs`/`serializer_gen.rs`: a construct now gates on
       `#[cfg(any(feature = "sml-drawings", feature = "sml-layout"))]` — enabling *either*
       slice includes it. 76 non-core constructs across `sml`/`wml`/`pml`/`dml` were
       multi-tagged (the blast radius of the original bug); all four crates' committed
       `generated.rs`/`generated_parsers.rs`/`generated_serializers.rs` were regenerated
       against the checked-out ECMA-376 schemas (present in this working tree) and now carry
       the OR'd predicates. Guarded against recurrence by unit tests in `ooxml-codegen`
       (`codegen.rs`'s `tests` module) and an integration test
       (`ooxml-codegen/tests/multi_tag_feature_gates.rs`) that runs the real spec files
       through the full pipeline and asserts the real `Worksheet.drawingHF` construct's
       generated code contains the `any(...)` predicate verbatim. Verified under partial
       feature sets (e.g. `cargo build -p ooxml-sml --no-default-features --features
       sml-layout,...` now correctly compiles in `drawingHF` without `sml-drawings`
       enabled). **Follow-up, not yet done**: hand-written (non-generated) adapter files in
       each crate (`writer.rs`, `workbook.rs`, `ext.rs`, etc.) that construct or read these
       same struct fields carry their own copied `#[cfg]` gates, which must be updated to
       match the widened OR predicate wherever they reference a multi-tagged field, or
       partial-feature builds fail with missing/unknown-field errors — there is no
       compile-time link between a hand-written `#[cfg]` and the codegen's `cfg_predicate`
       output for the same field, so this is a grep-and-fix sweep per crate, not a design
       question. See ADR 0013 Amendment 3's closing note for the exact mechanism gap; a
       structural fix (codegen-emitted constant/macro hand-written code could reference) is
       a possible future improvement, not built here.
     - **Licensing constrains the construct list and any future normative-slice citation,
       not the pragmatic tags.** No copyright or license statement was found in the
       ECMA-376 schema files or in Parts 1/2 (ADR 0013 open question 3). Treat as
       non-redistributable. `spec/` is already gitignored, which is exactly why the
       committed-artifact + external-citation design was built the way it is. This gates
       deriving the *element list itself* from ECMA-376 text and any future
       `normative_slices` sourced from the namespace/part schemas; it does not gate
       `pragmatic_slices`, which carries no redistribution claim.
     - **Effort shape:** the sharding is mechanical and large-diff/low-risk. The genuinely
       uncertain part is that the registry must describe constructs `#[cfg]`'d *out* of the
       current build — get that wrong and the catalog lies about what the binary can parse.
       Also: 3.6 MB of committed generated code in `ooxml-wml` alone means any generator
       change produces unreviewable diffs; establish a generate-and-diff checkpoint before
       starting or a refactor will be indistinguishable from a regression.

  4. **`parse_rnc` needs work before any RNC-schema format (DocBook, ODF) can be derived.**
     `ooxml-codegen`'s parser is a pragmatic subset: no `grammar`/`div`/`include`, no
     `|=`/`&=` combine operators, no `notAllowed`, name classes collapsed to a `_any`
     placeholder, no source spans, and `strip_rnc_annotations` textually deletes the `##`
     documentation *before lexing* (so `Definition.doc_comment` is always `None` — the field
     exists and nothing populates it). Recovering annotations is the highest-value single
     change: it is where a construct's human-readable description lives. Consider splitting
     `lexer`/`parser`/`ast` into an `rnc-parse` crate — it is already used cross-format and
     the `ooxml-codegen` name is the misnomer, not the design.

  5. **Formats with no machine-readable schema — resolved (2026-07-28, second amendment).**
     `SourceKind::HandCurated` is removed; `SourceKind::ScriptedExtraction` is the answer:
     a format with no machine-readable schema derives its construct list via a committed,
     re-runnable script against a published prose artifact (an HTML element index, etc.),
     never by hand-typing. See this file's registry-rollout entry above and ADR 0013
     Amendment 2, Decision B, for the full reasoning and what `ScriptedExtraction`'s
     provenance must carry. What's left as rollout work, not a design decision: writing the
     actual extraction script for the first schema-less format that wants one (DocBook is
     the leading candidate, since Amendment 1 already unblocked its *slice* rollout, leaving
     only its construct-list rollout open — which this closes the design gap for). **Not the
     same question as a hand-curated *slice*** — Amendment 1 already settled that a
     `pragmatic_slices` grouping is fine when explicitly marked non-normative, independent of
     this.

- **DocBook's two confirmed code gaps from the 2026-07-28 element-index audit fixed;
  audit re-verified and extended, not just patched (2026-07-28).** Follow-up to the
  "DocBook and JATS COVERAGE.md audited against the full format schema element lists"
  entry below, which flagged exactly two genuine code gaps (as opposed to bookkeeping/
  checklist gaps) in DocBook: (1) `<book>`/`<chapter>`/`<part>`/`<appendix>`/sectN/
  `<simplesect>` mapping to a bare untagged `DIV`, losing element identity on
  round-trip; (2) `<glossentry>`/`<indexentry>`/`<indexdiv>`/`<refnamediv>`/
  `<refsynopsisdiv>`/`<refmeta>`/`<entrytbl>` absent from `is_block_element`,
  misclassified as inline. Both confirmed real (not audit false-positives) by reading
  the reader/writer source and round-tripping concrete documents through
  `rescribe-cli convert --from docbook --to docbook`, then fixed. JATS not started
  this session — DocBook alone was a full vertical's worth of work; see the JATS scope
  note at the end of this entry.

  **Bug 1 fix — element identity.** `rescribe-read-docbook::convert_element`'s
  "Document level"/"Sections" arms now tag every one of these `DIV`s with
  `docbook:tag = <original element name>` — the same convention `generic_div` already
  used for every other raw-preserved block element (this reader has used that
  convention consistently since it was introduced; these fourteen elements were simply
  never migrated onto it, being handled by dedicated match arms predating the
  convention rather than falling through the generic catch-all). On the writer side,
  a new `write_sectioning_container` helper (in `rescribe-write-docbook`) re-emits the
  tagged `DIV` as its real element, extracting the container's own leading `HEADING`
  child back into a plain `<title>` in natural position — replacing the old behavior
  where the generic `node::HEADING` write arm always synthesized a fresh `<sectN>`/
  `<section>` wrapper around *just* the title, both losing the real tag (could never
  produce `<book>`/`<chapter>`/`<part>`/`<appendix>`) and leaving the container's actual
  body content as siblings *outside* that synthesized wrapper. Fixing this also fixes,
  for these specific tags, the previously-disclosed "DIV containing a HEADING plus
  following block siblings doesn't reassemble into one shared element on write" writer
  bug (see the "docbook-fmt fixture suite closed to 88/94" entry below) — a side effect
  of the identity fix, not separately re-engineered. Also fixed the same title-
  double-wrap bug on `BIBLIOGRAPHY` (a dedicated node kind, not a `docbook:tag`-carrying
  `DIV`, so it needed the same treatment applied directly) — found while fixing the
  `DIV` case, same root cause, not named in the original audit.

  Fixing this exposed a second bug the audit didn't anticipate: `rescribe-write-docbook`'s
  top-level `emit()` always hardcoded `<article>` as the XML document root regardless of
  `doc.content`'s actual shape — harmless before this fix (since book/chapter/part/
  appendix/article DIVs were always untagged and flattened away, so `<article>` was the
  *only* source of a root tag), but once these DIVs carry their real tag, the previous
  hardcoding produced a doubled root (`<article><book>...`) for every document whose top
  level actually was `<book>`/`<part>`/etc — caught by 8 failing existing writer round-trip
  tests, not missed. Fixed: `emit()` now reuses the single top-level structural
  container's own tag as the XML root when `doc.content` is exactly one such `DIV`,
  falling back to the old synthesized-`<article>`-wrapper behavior otherwise.

  **Bug 2 fix — block/inline misclassification, re-verified beyond the audit's four
  named element families.** The task brief asked *why* the earlier `is_block_element`
  schema-verification pass (commit `abd6dd447d`, same session as the audit) missed
  these seven elements at all, since it was explicitly a schema-verification pass.
  Reading that commit: **it checked every element name already present in
  `is_block_element` against the DocBook 5.2 reference to see if any were wrongly
  classified — it never asked the inverse question, whether the format defines block
  elements absent from the list entirely.** A presence-checking pass over an
  incomplete list can only find "have but shouldn't"; it structurally cannot find
  "should have but don't" no matter how carefully each listed entry is re-verified.
  This is a real methodology gap, not a one-off oversight — the JATS and TEI
  classifiers were verified the same session with the same method (per the task brief),
  so the same class of gap plausibly exists there too and has not been re-checked by
  this session (DocBook-only scope; noted for a future JATS/TEI pass).

  Given that finding, this session did the full check rather than patching only the
  four named families: extracted every DocBook 5.2 element name from
  `tdg.docbook.org/tdg/5.2/ref-elements.html` (392 raw names, matching the original
  audit's ~390 estimate) and diffed against every tag `rescribe-read-docbook` already
  handles explicitly (`convert_element`'s match arms) or lists in `is_block_element`,
  narrowing candidates by DocBook-domain knowledge (most misses are legitimately
  phrase-level, e.g. bibliographic citation fields consumed by the separate
  `convert_biblio_field` path) and confirming the remainder against tdg.docbook.org
  directly (each element's own reference page, not just the audit's original claim).
  Result: the seven elements the audit named were all confirmed genuinely block-shaped
  and genuinely absent, and seventeen more were found the same way: `<glossdef>`
  (glossentry's own definition body — the audit named `<glossentry>` but not this
  direct child, which holds the actual paragraph/list content), `<refsection>` (the
  generic recursive refentry subsection, sibling of `<refsect1>`/`<refsect2>`/
  `<refsect3>`, likely missed by the audit for the same "wasn't already on the list to
  re-check" reason), `<bibliodiv>`, `<bibliolist>`, `<glossdiv>`, `<glosslist>`,
  `<qandadiv>`, `<simplelist>`, `<partintro>`, `<setindex>`, `<toc>`/`<tocdiv>`/
  `<tocentry>`, `<productionset>`/`<production>`/`<productionrecap>`,
  `<constraintdef>`, `<msgset>`. All added to `is_block_element`. Not claimed
  exhaustive — this was one more thorough pass, not a formal proof of completeness;
  a handful of rarer families (deeper Message Set fields, `<colgroup>`/`<col>`/
  `<spanspec>` in table/entrytbl headers) were assessed as plausible-but-unverified and
  left out rather than guessed at, see `fixtures/docbook/COVERAGE.md`.

  Fixture-testing the `<entrytbl>` fix (a table nested inside a table cell) surfaced a
  third, independent writer bug: `write_inline` (used by `TABLE_CELL`/`TABLE_HEADER`
  for their children) had no arm for a `DIV` landing in inline position, so its generic
  "unknown inline - recurse" catch-all recursed via `write_inline` into the `DIV`'s
  block children too — which also have no `write_inline` arm — silently flattening the
  entire nested table down to bare text and losing every intermediate tag. This bug
  predates this session (the same collapse would have hit `<entrytbl>`'s block children
  even under its old, wrong inline classification) but was only surfaced by round-trip-
  testing the new fixture. Fixed with a `node::DIV => write_node(node)` arm in
  `write_inline`.

  **Not fixed, disclosed, out of scope:** `<cmdsynopsis><command>grep</command>...`
  round-trips as `<cmdsynopsis><para><code>grep</code></para></cmdsynopsis>` — a
  bare `CODE` inline node landing directly under a block-position `DIV` (not inside a
  `paragraph`) gets `<para>`-wrapped by `write_node`'s generic "inline nodes appearing
  at block level" arm, which is correct for genuine prose but wrong for phrase-level
  children of structured verbatim elements like `<cmdsynopsis>`/`<funcsynopsis>` per
  their DocBook 5.2 content models. Pre-existing (this session didn't touch that code
  path), found incidentally while round-trip-verifying the new `refentry-structure`
  fixture, not gating any box closed this session.

  **Fixtures added** (all in `fixtures/docbook/`, reader-only per `fixtures/spec.md`,
  round-trip-verified manually via `rescribe-cli convert --from docbook --to docbook`
  in addition to the automated reader-only assertions): `book-chapter-part-appendix`,
  `glossary-glossentry`, `index-indexentry`, `refentry-structure`, `table-entrytbl`,
  `rare-additional-block-elements`. `fixtures/docbook/COVERAGE.md` updated:
  **112/118** (was 101/117 after the audit; +1 to the denominator from a new
  consolidated line for the seventeen additional `is_block_element` finds beyond the
  audit's four named families). The six still-open boxes are unrelated to this
  session's two bugs (front-matter/back-matter division elements, programming-language
  synopsis family, indexterm/primary/secondary, person/org detail phrases,
  technical/UI phrase elements, keyword/keywordset — all pre-existing bookkeeping gaps
  where the generic fallback already round-trips losslessly, just unenumerated).

  Full test suite (`cargo clippy --all-targets --all-features -- -D warnings && cargo
  test -q`) green at commit boundaries.

  **JATS scope, not started this session:** the task brief's JATS list — `<hr>`,
  `<sub-article>`, `<response>`, the ruby family (`<ruby>`/`<rb>`/`<rt>`/`<rp>`), Q&A
  elements, `<chem-struct>`, `<array>`, and the `ali:` license namespace — needs the
  same treatment: confirm each is a real code gap (not just unenumerated) by reading
  `rescribe-read-jats`/`rescribe-write-jats` and round-tripping concrete documents,
  fix what's genuinely broken, raw-preserve or model what's merely unhandled, fixture
  everything. Per CLAUDE.md's one-vertical-at-a-time rule, this is a fresh vertical
  slice, not a continuation of the DocBook work above — start it fresh rather than
  assuming any DocBook finding (e.g. the `is_block_element` methodology gap) transfers
  without re-verification, since JATS's `convert_element` catch-all is architecturally
  different (universal, no destructive drop arm, per the audit) from DocBook's.

- **JATS tag-set scope audited and documented (2026-07-28): Archiving stays the
  reference tag set, no crate split, no validation modes — ADR only, no code change.**
  An audit flagged that `jats-fmt`/`fixtures/jats/` targeting the Archiving and
  Interchange Tag Set had never been an explicit decision. Investigated whether the
  `ooxml-wml`/`ooxml-sml`/`ooxml-pml` (schema-specific crates + `ooxml-opc`/`ooxml-xml`/
  `ooxml-dml`/`ooxml-omml` shared crates) precedent should apply. It does not: wml/sml/
  pml are genuinely different vocabularies for different document types (word
  processing/spreadsheet/presentation share almost no element names), whereas JATS's
  Archiving (~306 elements)/Publishing (~298 elements)/Authoring tag sets are the *same*
  vocabulary with progressively tighter content-model constraints — Publishing and
  Authoring are validity subsets of Archiving per JATS's own documentation, not
  divergent element sets. Also confirmed `jats-fmt`'s parser is already fully generic
  XML with zero tag-set awareness (no DTD/schema validation anywhere in the crate; any
  element round-trips, unrecognized ones raw-preserve via the existing `jats:tag`
  mechanism) — so "which tag set" only ever affected fixture/mapping-table scope, never
  parser behavior, making this a documentation question rather than an architecture
  fork. BITS (Book Interchange Tag Suite) is the one JATS relative that *would* fit the
  ooxml pattern if ever undertaken (genuinely additive book-specific elements, not just
  tighter constraints) — noted as future scope, not implemented. See
  `docs/adr/0012-jats-archiving-tag-set-scope.md` for the full writeup;
  `crates/formats/jats-fmt/src/lib.rs` and `fixtures/jats/COVERAGE.md` now cite it.

- **`commonmark-fmt` construct-feature vertical (2026-07-28): frontmatter/tables/task-lists/
  strikethrough landed; footnotes/definition-lists/math deferred.** Fixed the headline bug
  this session existed to chase: `rescribe_read_markdown::parse()` (default path) was
  silently misparsing YAML front matter as a bogus `horizontal_rule` + setext `heading`
  in the document body, with `doc.metadata` left empty and zero fidelity warning; TOML
  front matter merged into a plain paragraph. Both now populate `doc.metadata` correctly.
  See `docs/adr/0011-commonmark-extension-feature-gating.md` for the feature-gating design
  (individual `tables`/`task-lists`/`strikethrough`/`frontmatter` Cargo features, off by
  default, `gfm`/`extensions` umbrella aliases) and rationale for why "autolinks" is not a
  feature (pulldown-cmark 0.13's `ENABLE_GFM` doesn't gate bare-URL autolinking — angle-
  bracket autolinks are core CommonMark and already unconditional; verified by reading
  pulldown-cmark 0.13.1's source directly, not guessed).

  **Gap found during 2026-07-28 ADR audit, not previously tracked here:** pulldown-cmark
  0.13.1 exposes six more real `Options` bits this crate neither gates nor reserves a feature
  name for: `ENABLE_SMART_PUNCTUATION`, `ENABLE_HEADING_ATTRIBUTES`, `ENABLE_SUPERSCRIPT`,
  `ENABLE_SUBSCRIPT`, `ENABLE_WIKILINKS`, and `ENABLE_OLD_FOOTNOTES` (an alternate footnote
  syntax alongside the already-reserved `ENABLE_FOOTNOTES`/`footnotes` feature). None of these
  have a Cargo feature today, not even an inert reserved one the way `footnotes`/
  `definition-lists`/`math` already are. See `docs/adr/0011-commonmark-extension-feature-
  gating.md`'s "Not yet covered" section. Also, an earlier draft of that ADR claimed
  `Options::ENABLE_GFM` affects footnote-reference *kind* internally at `firstpass.rs:231` —
  false, that line builds a `BlockQuoteKind`, unrelated to footnotes; `has_gfm_footnotes()`
  doesn't check `ENABLE_GFM` at all. Corrected in the ADR; flagging here so the miscount isn't
  silently repeated in future construct-coverage work on this crate.

  **Fully complete (all API modes — AST/events/batch/emit/writer — both backends verified
  to agree via the new `markdown_backends_agree` fixture-parity test in
  `crates/rescribe-fixtures/tests/run.rs`):** YAML + TOML front matter, GFM tables, GFM
  task lists, GFM strikethrough (refactored from always-on to feature-gated).

  **Explicitly deferred, not silently dropped:** footnotes, definition lists, math. These
  three remain unimplemented in `commonmark-fmt` (feature names `footnotes`/
  `definition-lists`/`math` are reserved in `Cargo.toml` — inert today, no `Options` bit,
  no AST variant — so downstream `Cargo.toml`s can request them now without a future
  breaking rename). Concretely still open:
  - `commonmark-fmt`: wire `Options::ENABLE_FOOTNOTES` → `Block`/`Inline`/`Event` variants
    for `FootnoteReference`/`FootnoteDefinition` (mirror the `tables` implementation shape
    in `parse.rs`/`events.rs`/`emit.rs`/`writer.rs`); `Options::ENABLE_DEFINITION_LIST` →
    `DefinitionList`/`DefinitionListTitle`/`DefinitionListDefinition`; `Options::ENABLE_MATH`
    → `InlineMath`/`DisplayMath` (check whether to model as `math_inline`/`math_display`
    IR nodes, matching `pulldown.rs`'s existing mapping, or raw-preserve — undecided, a
    real design fork, not resolved here).
  - The markdown `footnote` fixture is excluded from the default-backend `markdown` test
    in `crates/rescribe-fixtures/tests/run.rs` via a new `run_format_fixtures_excluding`
    helper (and from `markdown_backends_agree`'s comparison) — tracked here, not silently
    passing. Once footnotes land in `commonmark-fmt` + the adapters, remove both
    exclusions.
  - Per CLAUDE.md, a reader that drops a semantic construct without warning is incorrect.
    The default backend currently has **no fidelity warning at all** for footnote/
    definition-list/math source syntax — unlike YAML/TOML front matter (which was
    actively misparsed into wrong nodes, now fixed), these three constructs currently
    degrade *gracefully* to plain prose (CommonMark has no syntax collision with them, so
    nothing is corrupted structurally) — but the semantic construct is still silently
    lost. A heuristic best-effort detector was deliberately NOT added in this session
    beyond a light source-text scan for footnote-style `[^label]:` and definition-list-
    style `term\n: definition` lines (in
    `crates/readers/rescribe-read-markdown/src/commonmark.rs::detect_unsupported_extensions`)
    — a `$`-based math heuristic was considered and rejected as too false-positive-prone
    (matches ordinary currency amounts in prose) without further refinement (e.g. requiring
    a matched pair of `$…$` with no whitespace immediately inside). Revisit before
    `math` fidelity-warning coverage is claimed complete.

  **Two real, pre-existing bugs found (not introduced) and fixed while building the
  `markdown_backends_agree` parity test, since that's what parity tests are for:**
  1. `commonmark-fmt`'s tight-list-item builder (`parse.rs`) only flushed accumulated
     leading inline text into an implicit paragraph at `End(Item)` — but a sibling nested
     block (e.g. a sublist) reaches `push_block` *before* `End(Item)` fires, so
     `- outer\n  - inner\n` produced `list_item → [list, paragraph("outer")]` instead of
     `[paragraph("outer"), list]`, silently reordering source content. Fixed by flushing
     tight-inline accumulation at `push_block` time (see `flush_tight_inlines`), using the
     flushed inlines' own span bounds rather than borrowing the item's start/end.
  2. `rescribe-read-markdown/src/pulldown.rs` never inserted
     `Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS`, so `backend_pulldown::parse`
     (the "legacy, supports everything" backend) silently didn't support TOML front
     matter at all, despite already having the `Tag::MetadataBlock(PlusesStyle)` handling
     arm sitting unreachable. One-line fix (add the `Options` bit next to the existing
     `ENABLE_YAML_STYLE_METADATA_BLOCKS`).

  **Fixture-harness sequencing:** `crates/rescribe-fixtures/tests/run.rs`'s `markdown`
  test was switched from `rescribe_read_markdown::backend_pulldown::parse` to the default
  `rescribe_read_markdown::parse` in the same commit as the `commonmark-fmt` construct
  features + adapter wiring landed (not a separate "flip it red then fix it" commit,
  since both bugs above were found and fixed before that commit closed) — the fixture
  suite is green at every commit boundary in this vertical's history, per the
  pre-commit-hook constraint.

  **Not attempted, explicitly out of scope for this session, pre-existing and already
  tracked in `docs/format-audit.md` (row 252):** `rescribe-write-commonmark` hand-rolls
  CommonMark text generation entirely rather than calling `commonmark_fmt::emit()` — a
  "PARTIAL MIGRATION" architecture violation of CLAUDE.md's "adapter layer must never
  contain parsing or writing logic" rule that predates this session. This session added
  front-matter emission and task-list-checkbox emission to that hand-rolled writer
  (matching its existing style, functionally useful) rather than fixing the underlying
  architecture violation — that rewrite (route `rescribe-write-commonmark` through
  `commonmark_fmt::emit()`, including wiring up the new construct features on the writer
  side: `emit.rs`/`writer.rs` already support tables/task-lists/frontmatter/strikethrough
  as of this session) is real, separately-scoped work, not done here.

  **Fuzz coverage not extended:** `fuzz/fuzz_targets/commonmark_roundtrip.rs`'s
  hand-rolled `Gen` (arbitrary-`CmDoc`-first generator) was NOT extended to cover the new
  `Block::Table`/`ListItem::checked`/`Inline::Strikethrough`/`CmDoc::frontmatter`
  variants — only updated minimally so it still compiles against the now-larger `CmDoc`/
  `ListItem` struct shape (feature-unified through `rescribe-read-commonmark`'s
  `Cargo.toml` dependency, which requests `tables`/`task-lists`/`strikethrough`/
  `frontmatter`). `fuzz_commonmark_reader` (no-panic gate on arbitrary bytes) already
  exercises the new code paths for free once features are on, since it doesn't construct
  a `CmDoc` — only `fuzz_commonmark_roundtrip`'s AST-first generator needs extending.

- **`docbook-fmt` fixture suite closed to 88/94** (checked 2026-07-27, this session) —
*Open threads from a previous session. Treat as starting context, not instructions — verify relevance before acting.*

- **`docbook-fmt` fixture suite closed to 88/94** (checked 2026-07-27, this session) —
  up from 30/94. Real reader/writer work, not just fixture-writing: added CALS table
  attributes (frame/colsep/rowsep/colspec/spanning), formal-table titles, list
  numeration/spacing, xml:lang/xlink attrs applied uniformly via
  `attach_generic_attrs`, `procedure`/`step`/`substeps` -> ordered list mapping,
  `screen`/`literallayout`/`synopsis`/`address` -> tagged `code_block`,
  `epigraph`/`attribution`, `bridgehead`, `footnoteref` -> `footnote_ref`,
  `mediaobject`/`textobject` -> image `alt` folding, and ~20 phrase-level semantic
  inlines (abbrev/acronym/trademark/keycap/guilabel/etc) verified individually
  against the DocBook 5.2 reference (tdg.docbook.org) and closed via the existing
  `generic_span` raw-preservation mechanism. Full commit list on `master`, newest
  first: `feat(docbook): fold mediaobject alt text into image; close composition
  fixtures`, `feat(docbook): map example/screen/synopsis/procedure/epigraph/
  bridgehead/address`, `fix(docbook): stop corrupting <title> round-trip in
  non-sectioning containers`, `test(docbook): close the Adversarial and
  Pathological COVERAGE dimensions`, `feat(docbook): map footnoteref; fixture the
  phrase-level semantic inlines`, `test(docbook): fixtures for xref/anchor/
  personname/filename/revhistory/pubdate`, `feat(docbook): model CALS table
  attributes, formal tables, cell spanning`, `feat(docbook): add xml:lang, link
  xlink attrs, list numeration/spacing`.

  **Real bugs found and fixed along the way** (discovered while verifying
  parse -> emit -> parse round-trips for new fixtures, not just one-way reader
  assertions — docbook has no dedicated writer fixture suite, so these were
  latent): (1) any `<title>` whose parent wasn't a genuine sectioning container
  always became a `HEADING`, which the writer always wraps in a fresh `<sectN>`
  on emit — so e.g. `<example><title>T</title>...` round-tripped as a spurious
  nested `<sect1>` inside the example, corrupting every non-sectioning titled
  container (example, figure, admonitions, qandaset, refentry, ...). Fixed via
  `heading_level_for_parent` + a new `CAPTION` node kind/write arm. (2) a
  `generic_span` landing directly in a raw-preserved block container's children
  (e.g. `<arg>` inside `<cmdsynopsis>`) silently lost its tag on write. (3)
  `<abstract>` (the one dedicated DIV mapping without `docbook:tag`) was dropped
  entirely by the writer's DIV arm. (4) `FOOTNOTE_DEF` embedded inline (e.g. in a
  table cell) had no `write_inline` arm and silently lost its `<footnote>`
  wrapper. All four fixed this session.

  **[Superseded 2026-07-28 — see "MathML resolved for DocBook and JATS" below: the
  `equation`/`inlineequation` MathML fork described here was NOT actually a fork (the
  HTML precedent transferred cleanly) and is now implemented/closed; the fixture count
  below is stale, now 101/105. The other forks named here are re-verified as still
  genuinely open, with sharpened writeups in that same section.]**

  **Left open, genuine design forks (not lookup-resolvable), 6 of 94 boxes**:
  `qandaset`/`qandaentry` (no Q&A-list IR shape attempted — still raw-preserves
  generically via `generic_div`, just unverified with a fixture); `equation`/
  `inlineequation` (MathML modeling choice — reuse `rescribe-math`'s
  `math_inline`/`math_display` with the MathML captured as raw content, or
  something else — genuinely undecided); `programlistingco`/`co`/"callout listing
  + callout list" (three boxes, all paired: `co` only has meaning alongside a
  `<calloutlist>` that references it back, so designing one without the other
  would be premature).

  **Found but NOT fixed this session** (real, disclosed, out of scope for the
  fixture-closing pass): a `DIV` containing a `HEADING` plus following block
  siblings (any section with more than just a title) does not reassemble into
  one shared `<sectN>` on write — `write_node`'s `HEADING` arm always wraps only
  the title itself in a fresh `<sectN>`, leaving the section's actual body
  content as siblings *outside* that new element on round-trip. Exposed by the
  `nested-section` fixture (whose *reader* output is correct — fixtures only
  test the reader per `fixtures/spec.md`, so the fixture was still added). Fixing
  this needs the writer's section-boundary detection redesigned (recognizing "a
  DIV whose first child is a HEADING" as one section unit to serialize together)
  — a real architecture decision, not a quick patch. Also found: `<figure>`'s
  `<caption>` child (mapped to a custom `figcaption` node kind, pre-existing,
  unrelated to this session's changes) has no writer arm and silently drops the
  `<caption>` wrapper on round-trip, leaving a bare `<para>` — pre-existing,
  not fixed, not gating any box closed this session.

- **`jats-fmt` fixture suite closed to 99/106** (checked 2026-07-27, this session) —
  up from 32/106. Built on top of the classifier-verification pass below (same
  session): `crates/formats/jats-fmt/` itself needed no changes (JATS's AST is
  generic XML), all work landed in the AST↔IR adapter layer
  (`crates/readers/rescribe-read-jats/src/lib.rs`,
  `crates/writers/rescribe-write-jats/src/lib.rs`) plus 73 new fixture pairs under
  `fixtures/jats/*/`. Added: table properties (id/lang, `ext-link-type`, spanning),
  the inline `code`/`monospace` distinction, `xref` variants, generic-fallback
  attribute preservation, nested-section depth tracking, `abstract` metadata
  capture, front-matter metadata fields, adversarial-dimension fixtures,
  back-matter structural elements, `underline-style`, and the composition/
  pathological dimensions. Commits, newest first: `1eb2ffc14d` (stop folding
  disp-formula label into math:source; close composition/pathological dims),
  `c133f49562` (adversarial + back-matter + underline-style), `3f4db4a90d`
  (front-matter metadata), `4b3b2c1604` (nested-section depth + abstract-drop
  fix), `65245a6a12` (inline code/monospace/xref variants), `7dfacccd47` (table
  properties + table-wrap double-wrap fix).

  **Real bugs found and fixed along the way** (via parse→emit→parse round-trip
  checks, same discovery pattern as docbook's session): (1) the `TABLE` write arm
  always synthesized its own `<table-wrap>`, double-wrapping tables that
  originated from a `<table-wrap>` — fixed via `jats:tag="table-wrap"` tagging
  plus a shared `table_element()` helper. (2) block-position `SPAN` (e.g.
  `<label>` inside `<fig>`) and the `figcaption` node kind had no `write_node`
  arm and silently dropped their tags — fixed with dedicated arms. (3)
  `<abstract>` was dropped entirely: its `DIV` mapping never set `jats:tag`, so
  it missed the front-matter capture path — fixed. (4) nested `<sec>` heading
  level was hardcoded to `2` regardless of depth (JATS reuses `<sec>` at every
  nesting level) — fixed by threading a real depth counter. (5) `math:source`
  for `<disp-formula>`/`<inline-formula>` absorbed the `<label>` text into the
  math content — fixed with a `split_label()` helper.

  **[Superseded 2026-07-28 — see "MathML resolved for DocBook and JATS" below: the
  MathML fork described here was NOT actually a fork and is now implemented/closed
  (citation/ref-list was already separately closed by a later session, see the JATS
  citation vertical entry below). Current count: 108/109, `<alternatives>` only.]**

  **Left open, genuine design forks (not lookup-resolvable), 7 of 106 boxes**:
  MathML `<math>` as an alternative to `<tex-math>` inside
  `disp-formula`/`inline-formula` — the same math-modeling fork docbook's
  `equation`/`inlineequation` hit, genuinely undecided; citation/reference-list
  IR shape (`ref-list`/`ref`/`mixed-citation`/`element-citation`, 5 of the 7
  boxes including two dependents — no dedicated bibliography IR shape attempted,
  still raw-preserves generically, unverified with a fixture); `<alternatives>`
  (JATS's own Tag Library page states it "is neither inherently block nor
  inherently inline in nature... determined by context and usage" — JATS itself
  declines to commit, so no IR classification was guessed).

  **Found but NOT fixed this session** (real, disclosed, out of scope): a
  titleless `<sec>` loses its wrapper on write — mirror of docbook's disclosed
  section-writer gap (a DIV without a leading HEADING doesn't reassemble into
  one shared element on emit). `generic_div` wraps bare-PCDATA children in a
  synthetic `<p>` even for elements whose content model forbids it (e.g.
  `<verse-line>`) — pre-existing, not gating any box closed this session.
  `<journal-meta>` fields get spliced into a reconstructed `<article-meta>` on
  write, losing origin-wrapper distinction — pre-existing, the flat metadata
  namespace has no origin tracking.

- **`jats-fmt` `is_block_element` classifier schema-verified against JATS 1.3**
  (checked 2026-07-27, this session, commit `20c27d032e`) — following the same
  pattern as `docbook-fmt` (docbook.org, three misclassifications corrected) and
  `tei-fmt` (TEI P5 Guidelines, zero misclassifications, three missing elements
  added). Checked every element in
  `crates/readers/rescribe-read-jats/src/lib.rs`'s `is_block_element` against the
  JATS 1.3 (NISO Z39.96-2019) Tag Library at jats.nlm.nih.gov, fetching each
  element's actual page (expanded content model + "May be contained in" list)
  rather than relying on memory. **Found and fixed**: `related-article` was
  misclassified as block — its Tag Library page documents it as a phrase-level
  link element (like `xref`/`ext-link`) that can appear inside `<p>`, `<italic>`,
  `<sub>`, etc. — removed from the block list. **Added, previously missing**:
  `speech`, `speaker`, `supplementary-material`, `block-alternatives`, all
  confirmed block-shaped via their JATS content models. Plain `<alternatives>`
  was deliberately left unclassified (defaults to inline) since JATS's own docs
  decline to classify it either way (see the fixture-suite bullet above).
  **Verified correct, no change**: nine metadata-container entries
  (`contrib-group`/`aff`/`pub-date`/`permissions`/`history`/
  `custom-meta-group`/`custom-meta`/`product`/`sig`/`sig-block`) plus
  `statement`/`verse-group`/`table-wrap-group`/`tfoot`/`disp-formula-group`/
  `kwd-group`/`ack`/`app-group`/`app`/`notes` — full citation trail in the doc
  comment above `is_block_element`.
- **[Superseded 2026-07-28: the counts and residue list below are stale — citation/
  ref-list IR shape was closed for all three formats in later 2026-07-28 sessions
  (see the citation-vertical entries below), and MathML was resolved for docbook/jats
  the same day (see "MathML resolved for DocBook and JATS" below). Current state:
  docbook 101/105 (3 remaining: qandaset/qandaentry, programlistingco/co/calloutlist),
  jats 108/109 (1 remaining: alternatives). Both re-verified as genuine, not
  lookup-resolvable, with sharpened writeups in that section.]**
- **The docbook/jats/tei extraction-and-closing arc is essentially wound down**
  as of 2026-07-27. All three verticals have had their fixture suites deepened
  (tei 118/118, docbook 88/94, jats 99/106) and their `is_block_element`
  classifiers schema-verified against each format's authoritative reference,
  with real round-trip bugs found and fixed along the way in every case. The
  actual residue for a future session, accurately inventoried (not urgent):
  docbook's 6 open design-fork boxes (qandaset/qandaentry, equation/
  inlineequation MathML, programlistingco/co/calloutlist) plus its 2 disclosed
  writer bugs (section reassembly, figure caption drop); jats's 7 open
  design-fork boxes (MathML, citation/ref-list IR shape, alternatives) plus its
  3 disclosed writer gaps (titleless-sec reassembly, generic_div bare-PCDATA
  wrapping, journal-meta origin tracking); and DTD-aware entity resolution
  across all three (see below). The docbook/jats fuzz campaigns (previously
  only a ~60s validation run) were brought to parity with tei's multi-hour
  campaign in a later session (2026-07-27): `fuzz_docbook_fmt_reader`
  8,918,090 runs clean, `fuzz_docbook_fmt_roundtrip` 6,012,874 runs clean,
  `fuzz_jats_fmt_reader` 8,162,993 runs clean, `fuzz_jats_fmt_roundtrip`
  5,696,913 runs clean — no crashes, no panics, no artifacts, no bugs found.
  All four fuzz targets across all three verticals (tei/docbook/jats) are now
  at the same extended-campaign scale. None of the remaining residue items
  block calling any of the three verticals 3-Harness; they gate 5-Production
  only.
- **Oracle harness not yet run for `docbook-fmt`/`jats-fmt`; applicability confirmed.**
  `pandoc --list-input-formats` (checked 2026-07-27) includes both `docbook` and `jats`, so per
  TODO.md's Tier B oracle-harness guidance ("skip for formats Pandoc can't read") the harness step
  applies to both and is still open — `docs/format-audit.md` shows both at oracle-harness status
  "harness" (applicable, not yet done). `tei` is **not** in pandoc's input-formats list (only
  output); `docs/format-audit.md` now marks TEI's oracle-harness step N/A (2026-07-27), the same
  way `asciidoc`'s was.
- **`docbook-fmt`/`jats-fmt` fuzz targets initially only had a ~60s validation run each**
  (docbook: 1.69M reader / 573K roundtrip runs; jats: 1.61M / 553K; both clean, one fuzz-harness
  generator bug fixed, no library bugs) — not the multi-hour/multi-million-run campaign
  `commonmark-fmt` got before its 5-Production sign-off. `tei-fmt`'s fuzz targets got that longer
  campaign in an earlier session (2026-07-27): `fuzz_tei_fmt_reader` 7,518,438 runs clean,
  `fuzz_tei_fmt_roundtrip` 6,611,996 runs clean (15 min/target via `cargo fuzz run <target> --
  -max_total_time=900`), no crashes/panics/artifacts. **Closed (2026-07-27, later same day)**:
  docbook and jats got the same extended campaign — `fuzz_docbook_fmt_reader` 8,918,090 runs
  clean, `fuzz_docbook_fmt_roundtrip` 6,012,874 runs clean, `fuzz_jats_fmt_reader` 8,162,993
  runs clean, `fuzz_jats_fmt_roundtrip` 5,696,913 runs clean. No crashes, no panics, no
  artifacts, no roundtrip mismatches, no bugs found — all three XML verticals' fuzz targets
  are now at the same extended-campaign scale. The fixture-suite gaps above remain open but
  are independent of this closed item.
- **DTD-aware entity resolution — implemented (2026-07-28)** via a new standalone crate,
  `crates/formats/xml-entities` (no rescribe dependency, workspace member `xml-entities`).
  Scope: (1) a narrow DTD internal-subset `<!ENTITY ...>` declaration parser
  (`DtdEntities::parse_doctype`/`parse_subset`) — general *and* parameter entities,
  `SYSTEM`/`PUBLIC` external entities recorded (name + identifiers) but never fetched (no
  network/filesystem access anywhere in the crate), numeric char refs expanded at
  declaration time per the XML spec, internally-declared parameter entities expanded
  in-place (the "combine several `<!ENTITY %`-declared fragments" idiom), external
  parameter-entity references diagnosed rather than silently skipped. Deliberately **not**
  a DTD validator — `<!ELEMENT>`/`<!ATTLIST>`/`<!NOTATION>` are recognized only well enough
  to skip correctly. (2) `EntityResolver`, layering those document-declared entities over
  the WHATWG HTML5 standard table (via the `entities` crate, ~7M downloads/wk) as a
  fallback — HTML5's table absorbed nearly all of the ISO 8879/9573 sets (`ISOlat1`,
  `ISOnum`, `ISOpub`, `ISOtech`, `mmlalias`) DocBook/JATS/TEI lean on, which in practice is
  what resolves most entities from those DTDs anyway since the real-world idiom pulls them
  in via an *externally*-fetched parameter entity (e.g. DocBook's
  `%isolat1; PUBLIC "ISO 8879-1986//ENTITIES Added Latin 1//EN" "isolat1.ent"`) that this
  crate does not fetch. Resolution is recursive (an entity's value referencing another
  entity) with cycle/depth guards. Unknown/external names resolve to a non-error variant
  (`Resolution::Unknown`/`ExternalUnresolved`) so callers raw-preserve rather than drop
  them. 30 unit tests + 1 doctest, clippy clean, no-panic fuzz target
  `fuzz_xml_entities_reader` registered (compiles clean; not run as an extended campaign —
  `cargo-fuzz` isn't installed in this repo's dev shell).

  **Wired into all three format crates' `parse()`, `EventIter` (SAX), and
  `StreamingParser` (batch)** — all three independent reader API surfaces per
  CLAUDE.md's "each API mode is independently implemented" rule, not just the AST path.
  Named entities beyond the 5 XML-predefined ones now try `EntityResolver` (built from
  that document's own DOCTYPE, if any) before falling back to the pre-existing
  `Node::EntityRef` raw-preservation. Malformed DOCTYPE internal subsets surface as
  diagnostics (prefixed `"DOCTYPE internal subset: ..."`) instead of being silently
  discarded. 3 new fixtures per vertical (`dtd-entity-resolution`,
  `rare-named-entity-standard-table`, `adv-unresolvable-entity`) plus 3 new unit tests
  per format crate; COVERAGE.md updated for all three (docbook/jats/tei).

  **Known gap, disclosed rather than silently left**: the rescribe adapter layer
  (`rescribe-read-{docbook,jats,tei}`) is unaffected by this change other than seeing
  fewer `raw_inline`/`*:entity` nodes and more resolved `text` nodes — no adapter code
  changed, since resolution now happens entirely inside the `-fmt` crates before the
  adapter ever sees a `Node::EntityRef`. External (`SYSTEM`/`PUBLIC`) DTD entities and
  entities declared only in an external subset pulled in via a parameter-entity reference
  remain genuinely unresolvable without fetching that external file — this is a
  deliberate, disclosed scope boundary (no network/filesystem access from this crate),
  not a bug; such entities still raw-preserve losslessly exactly as before.

- **ADR 0007's open parser-swap question — resolved (2026-07-28).** ADR 0007 originally
  shipped `xml-entities` without ever weighing "switch the underlying parser to `xml-rs`/
  `roxmltree` to get internal-subset entity discovery natively" against building a standalone
  crate — that comparison is now done and the ADR rewritten with the evidence in place.
  `roxmltree` is disqualified outright: `Document::parse(&str)` needs the whole document
  pre-allocated, no `Read`/chunked entry point, no streaming per its own docs — building
  `StreamingParser` on it would require the buffer-everything-then-wrap anti-pattern CLAUDE.md
  bans by name. `xml-rs` is a real contender on capability but loses on two measured,
  non-negotiable axes: zero-copy (owned `String` per token by design, confirmed via its own
  README) and throughput (19–21x slower than quick-xml on an out-of-repo synthetic-DocBook
  benchmark, quick-xml 0.39.4 vs. `xml` 0.8.28 vs. `roxmltree` 0.20.0, release build,
  best-of-3; quick-xml's own README shows ~50x on its input). Whether `xml-rs`'s error model
  can support the same truncation-vs-malformed distinction `docbook-fmt::batch::StreamingParser`
  exploits for chunked feeding is unverified on top of that. Net: quick-xml stays; see ADR 0007
  for the full writeup.

  **Also corrected in the same pass**: the ADR previously mischaracterized quick-xml as not
  resolving entities at all. Verified against quick-xml 0.39.4 source and the live GitHub
  thread (`gh issue view 258 --repo tafia/quick-xml --comments`): quick-xml resolves the 5
  predefined entities and numeric char refs today (`resolve_xml_entity`, `unescape`/
  `unescape_with`, opt-in per-call), and has exposed a custom-entity resolution hook
  (`unescape_with`'s closure) since contributor `pchampin`'s PR #261 shipped years ago — the
  *resolution* half of issue #258. What's still missing, confirmed by maintainer `Mingun` on
  that thread, is DOCTYPE internal-subset *discovery*: quick-xml's DTD skip-state-machine
  (`src/parser/dtd.rs`) only tracks nesting well enough to find the closing `>` ("we simply
  count `<` and `>`"), never builds an entity table from a document's own DOCTYPE. Mingun
  called a `reader::Config`-level entity map with transparent expansion "the plan for further
  improvements" — unshipped, no committed timeline. `xml-entities` fills exactly that gap
  (`decl.rs`), not "entity resolution" in general, and doesn't duplicate quick-xml's
  `unescape_with` (none of docbook-fmt/jats-fmt/tei-fmt call it; they dispatch
  `Event::GeneralRef` manually instead).

  **Open, future work, not a defect**: if quick-xml ships that `reader::Config` entity map,
  `xml-entities` could drop `decl.rs` and shrink to a pure table + resolution layer at zero
  performance cost. Upstreaming DOCTYPE internal-subset discovery to quick-xml directly (rather
  than waiting) would serve the wider Rust ecosystem per CLAUDE.md's priority hierarchy and is
  worth someone picking up, but isn't scheduled here.

- **DocBook and JATS `COVERAGE.md` audited against the full format schema element
  lists (2026-07-28)** — prompted by the observation that both checklists' denominators
  had grown ad hoc during this session (docbook 94→105, jats 106→109) purely from gaps
  noticed incidentally, with no evidence either was complete against the actual spec.
  Two parallel research agents fetched the authoritative element indexes and diffed
  them against every element name mentioned anywhere in the corresponding COVERAGE.md
  (checked or not), then triaged each miss against the reader/writer source.

  **DocBook**: authoritative list fetched from `https://tdg.docbook.org/tdg/5.2/ref-elements.html`
  (~390 elements; the separate Assembly profile — `assembly`/`resource`/`structure` — was
  excluded as a distinct DocBook 5.1+ profile, not core narrative markup). 265 raw element
  names were absent from COVERAGE.md's text; collapsed by construct family (not enumerated
  1:1, since many are narrow phrase-level elements already covered losslessly by the
  generic-span/div catch-all) into **16 new unchecked lines** (12 block-construct family
  lines + 4 inline-construct family lines). Two are genuine code gaps, not just checklist
  gaps: (1) `<book>`/`<chapter>`/`<part>`/`<appendix>` and the `sectN`/`simplesect` family
  map to a bare `DIV` with no `docbook:tag`, so on round-trip they collapse into
  heading-level-inferred nesting with the specific element identity lost — broader than
  the already-disclosed nested-section writer bug, same root cause; (2) `<glossentry>`,
  `<indexentry>`/`<indexdiv>`, `<refnamediv>`/`<refsynopsisdiv>`/`<refmeta>`, and
  `<entrytbl>` are block-shaped in the real content model but absent from
  `is_block_element`, so they're misclassified as inline spans by the catch-all — a real
  fidelity risk, not just an unenumerated element. No stale/invented entries found in
  DocBook's COVERAGE.md. **Corrected honest ratio: 101/117** (was reported as 101/105).

  **JATS**: authoritative list fetched from
  `https://jats.nlm.nih.gov/archiving/tag-library/1.3/alpha-index/alpha-index.html`
  (~306 elements). 216 element names absent from COVERAGE.md's text, collapsed into
  **25 new unchecked lines**. Unlike DocBook, `rescribe-read-jats`'s `convert_element` has
  a truly universal catch-all (verified no destructive `=> None` arm exists outside pure
  pass-through wrappers), so most misses are bookkeeping gaps, not data loss — but ~12
  elements are genuinely unhandled beyond even the catch-all's shape-classification:
  `<hr>` (rescribe-std already defines `horizontal_rule` but no reader arm exists and it's
  absent from `is_block_element`, so it misclassifies as inline), `<sub-article>`/
  `<response>` (whole nested front/body/back substructures defaulting to an inline span
  wrapping a block subtree — the riskiest single gap found), the ruby annotation family
  (`ruby`/`rb`/`rt`/`rp`, no rescribe-std node kind exists for it at all), the Q&A family,
  `chem-struct`, `array`, `index-term`, the `media`/`alt-text` accessibility family, and
  the `ali:` open-access-license namespace. No stale/invented entries found in JATS's
  COVERAGE.md. **Corrected honest ratio: 108/133** (was reported as 108/109).

  **JATS tag-set target — confirmed never an explicit decision.** JATS defines three tag
  sets (Archiving/"green" — broadest; Publishing/"blue"; Authoring/"orange" — narrowest),
  and "100% coverage" means a different denominator depending which is the target.
  COVERAGE.md's header cites the Archiving tag library URL, and every prior session's
  schema-verification pass (the `is_block_element` audit, MathML resolution) also fetched
  pages under `/archiving/tag-library/1.3/...` — but nowhere in TODO.md or the crate
  source is there a sentence that actually chose Archiving over Publishing/Authoring. It's
  an inherited default from whoever first pasted that URL into COVERAGE.md's header, not a
  reasoned choice. This audit used Archiving as the working target since that's what's
  consistently cited, but the choice itself remains undocumented as a decision.

  **What the checkmarks actually mean, re: `fixtures/spec.md`'s six test dimensions
  (Happy path / Integration / End-to-end / Rare / Adversarial / Pathological)**: per-format,
  the spec's bar is "all six dimensions have meaningful coverage for all constructs," but
  in practice a `[x]` in the Block/Inline/Metadata/Properties sections of either
  COVERAGE.md means only that **one happy-path fixture demonstrates recognition** — the
  Adversarial and Pathological dimensions are covered *globally* (a handful of fixtures at
  the bottom of the file, e.g. `adv-empty`, `path-large-table`) rather than per-construct,
  and Integration/e2e is likewise a handful of composition fixtures touching a few
  constructs each, not full cross-product coverage. So a checked box asserts "this
  construct round-trips in isolation," not "this construct has been tested against all six
  dimensions" — a materially weaker claim than the ratio implies at face value. This gap
  between what the ratio suggests and what it actually verifies is itself worth tracking,
  independent of the denominator-completeness issue this entry addresses.

  **Scope note**: per the audit's brief, this was audit-only — the deliverable is the
  corrected enumeration (36 new unchecked boxes total across both files), not
  implementations. The two genuine-code-gap findings above (DocBook's book/chapter/part/
  appendix tag loss and block-misclassification; JATS's `<hr>`/`<sub-article>` gaps) are
  the highest-priority items if this audit becomes follow-up implementation work.

---

## Near-term mode of working: finish one vertical before starting the next

The fixture suite is the primary deliverable. A format's fixtures should be comprehensive
enough that any implementation in any language could use them as a complete correctness
test — every construct, every edge case, every adversarial input a real implementation
might get wrong.

Work **one format at a time**, completing the full vertical before touching the next.
**Do not start a new format until the current one reaches 5-Production.**

A vertical has these steps, in order — complete each before moving to the next:

1. **Fixture suite complete** — `fixtures/{format}/COVERAGE.md` all boxes checked. Covers
   all six dimensions: happy path, integration, end-to-end, rare, adversarial, pathological.
   Fixtures assert correct behavior; the Rust implementation is fixed to pass them (dogfooding).
   Required for both reader and writer.
2. **Oracle harness** (where applicable — skip for formats Pandoc can't read) — run against
   Pandoc or another reference implementation. No numeric threshold; all differences must be
   understood and documented. The goal is zero unexplained differences.
3. **Fuzz clean** — both no-panic gate and roundtrip property, run until no failures.
   Required for both reader and writer.
4. **All API modes complete** — reader: ast + stream + batch; writer: w-build + w-stream.
5. **5-Production sign-off** in `docs/format-audit.md`

**The anti-pattern to avoid:** completing step 1 for format A, then starting format B at
step 1. That's a horizontal sweep in disguise. Finish A through step 5 first.

Horizontal sweeps are explicitly out of scope. The measure of progress is finished verticals.

---

## Completed

- [x] CLI tool (`rescribe-cli`)
- [x] Metadata handling (YAML frontmatter, HTML meta tags)
- [x] Resource embedding (images, data URIs)
- [x] ParseOptions / EmitOptions implementation
- [x] Transforms crate (ShiftHeadings, StripEmpty, MergeText, etc.)
- [x] Pandoc JSON compatibility layer
- [x] DOCX reader/writer (via `ooxml-wml`)
- [x] PDF reader (text extraction via `pdf-extract`)
- [x] PPTX reader/writer (migrated to `ooxml-pml`)
- [x] XLSX reader/writer (via `ooxml-sml`)
- [x] 54 readers, 64 writers — comprehensive format coverage
- [x] Pandoc harness — 25/25 parsers, 20/25 at ≥90% coverage

---

## Format Tiers

Tiers determine how much investment a format gets. Higher tiers reach production first;
lower tiers get fixtures and correctness but not necessarily fuzz hardening.

### Tier A — Production priority

The formats people actually use for document authoring and conversion.
Target: **5-Production**.

Markdown family (commonmark, gfm, markdown, markdown-strict, multimarkdown), HTML,
DOCX, EPUB, AZW3, Org, RST, AsciiDoc, Djot, ODT, PPTX, XLSX, PDF

### Tier A (read-limited) — Production priority, last in queue

Formats where the **write direction is high quality** (IR → LaTeX/Typst produces correct,
well-structured output) but the **read direction is extraction-only**: the authoring
language is Turing-complete, so arbitrary user-defined macros/functions cannot be resolved
without full execution. Round-trip fidelity is architecturally impossible in the read
direction; the write direction is fine.

Read strategy: known constructs (standard packages/builtins) → IR; unknown constructs
→ `raw_inline`/`raw_block` with a fidelity warning. No round-trip fuzz target (the read
direction cannot guarantee it). Quality bar for reading is extraction fidelity for
real-world documents using common packages.

These are last in the Tier A queue because the reader surface area is enormous (just the
common LaTeX packages — amsmath, biblatex, hyperref, geometry, listings — is months of
work) and the reader quality ceiling is fundamentally lower than interchange formats.

LaTeX, Typst

### Tier B — Correctness, not urgent

Formats with real use cases but lower conversion frequency.
Target: **3-Harness** (or 2-Fixtures where harness is N/A), fuzz as bandwidth allows.

MediaWiki, DocBook, JATS, TEI, FB2, RTF, Man,
BibTeX, BibLaTeX, CSL-JSON, RIS, EndNote XML,
CSV, TSV, OPML, iPynb, Pandoc JSON, Native,
MOBI, KFX

### Tier C — Best-effort

Niche formats; fixtures are sufficient, no production guarantee.
Target: **2-Fixtures**.

Creole, DokuWiki, VimWiki, ZimWiki, XWiki, TWiki, TikiWiki, Jira,
ANSI, Haddock, Markua, Texinfo, POD
(Fountain: advanced to 4-Fuzz 2026-03-21; Muse: 5-Production; t2t: 4-Fuzz;
BBCode: advanced to 4-Fuzz 2026-03-21;
All 8 wiki formats advanced to 4-Fuzz 2026-03-21;
csv-fmt, tsv-fmt, ris, texinfo advanced to 4-Fuzz 2026-03-21)

### Tier D — Output-only, low investment

Write-only presentation formats. Correctness is hard to verify programmatically.
Target: **2-Fixtures** (round-trip not required).

Beamer, reveal.js, Slidy, S5, DZSlides, Slideous, ConTeXt, ms, ICML,
Chunked HTML, Plaintext

---

## Architecture: Format Crate Split (M0-style, ongoing)

### Motivation

`rescribe-read-{format}` and `rescribe-write-{format}` should be **thin IR adapters only** —
they translate between rescribe's `Document` IR and the format, nothing more.

Hand-rolled format logic (tokenizer, AST, builder) belongs in a **standalone crate** with
no rescribe dependency, so it can be used, tested, and fuzzed independently.

Library-backed formats (pulldown-cmark, html5ever, ooxml-*, etc.) already follow this
pattern — we wrap them. Hand-rolled formats should look the same from the outside.

### Naming convention

- `{format}` when the crates.io name is available (e.g. `asciidoc`, `odt`, `docbook`)
- `{format}-fmt` when taken (e.g. `rst-fmt`, `rtf-fmt`, `latex-fmt`)

### Crate layout (target state)

```
crates/
├── formats/             ← standalone format libraries, no rescribe dep
│   ├── rst-fmt/         # RST parser + builder API
│   ├── asciidoc/        # AsciiDoc parser + builder API
│   ├── rtf-fmt/         # RTF tokenizer + builder API
│   ├── org-fmt/         # Org-mode parser + builder API
│   ├── latex-fmt/       # LaTeX parser + builder API
│   └── ...              # one per hand-rolled format
├── readers/             ← thin IR adapters: {format} → rescribe Document
└── writers/             ← thin IR adapters: rescribe Document → {format}
```

### Name availability (checked 2026-03-01)

Available (use plain name): asciidoc, t2t, markua, texinfo, creole, dokuwiki, zimwiki,
xwiki, twiki, tikiwiki, docbook, native, ris, endnotexml, odt

Need `-fmt` suffix: rst, org, rtf, textile, mediawiki, muse, fountain, bbcode, pod,
haddock, ansi, man, vimwiki, jira, fb2, opml, tsv, tei, typst (already `typst-syntax`),
djot (already `jotdown`), latex

### What each standalone crate exposes

See **[`docs/format-library-design.md`](docs/format-library-design.md)** for the
full design spec and per-vertical checklist. Short version:

- Owned AST with source spans on every node
- `parse(input) -> (Ast, Vec<Diagnostic>)` + `events()` pull iterator
- `emit(ast) -> String` with round-trip guarantee
- No `Document`, `Node`, or `Properties` anywhere in the standalone crate
- Rescribe adapter does only AST↔IR translation (no format parsing/writing)

---

## Strategy: Verticals, not sweeps

The primary development model is **vertical slices**, not horizontal sweeps.

For each format in priority order:
1. Build the standalone library (`formats/{name}/`) — parser + builder API, publishable independently
2. Thin rescribe adapter (`rescribe-read-{fmt}`, `rescribe-write-{fmt}`)
3. Owned fixture suite (2-Fixtures)
4. Pandoc/oracle harness (3-Harness)
5. Fuzz targets (4-Fuzz): **both** no-panic gate **and** round-trip property, run until clean
6. Production sign-off (5-Production)

**A vertical is not done until step 5 passes.** Fixtures + harness without fuzz is only
3-Harness. The round-trip fuzz harness is mandatory for standalone library verticals
because it's the only way to catch emitter bugs at scale. See
`docs/format-library-design.md` for the full per-vertical checklist.

**Why verticals:** rescribe's goal is to *be* the Rust format ecosystem for formats
that currently lack good libraries. Each vertical produces a publishable, standalone
crate that fills a real ecosystem gap — the rescribe adapter is almost incidental.
Horizontal sweeps (all formats to stage N, then loop) delay shipping anything useful
and accumulate half-finished work across many formats simultaneously.

The format tiers below determine priority order within this model.

### Vertical priority order (Tier A)

**CURRENT TOP PRIORITY: `commonmark-fmt` — see below.**

0. `commonmark-fmt` — write from scratch; tree-sitter-md is explicitly not for
   correctness-critical parsing (its README says so); pulldown-cmark is events-only
   with no proper AST; the Rust ecosystem has no quality CommonMark AST crate.
   This fills the most important ecosystem gap. See "commonmark-fmt vertical" below.
1. `rtf-fmt` — highest risk, most isolated, no viable crate exists ✓
2. `rst-fmt` — large parser, complex spec, `docutils` is the reference ✓
3. `asciidoc` — similar scope; `asciidoctor` as oracle ✓
4. `org-fmt` — reader at 4-Fuzz (2026-03-21); writer still at 2-Fixtures; coverage gaps remain ✓
5. `djot-fmt` — jotdown has confirmed bugs; djot spec is clean and small ✓
6. `odt` — no library; hand-rolled; ODF is a real interchange format
7. `epub` — library-backed (epub/epub-builder)
8. `azw3` — not yet implemented
9. LaTeX, Typst — read-limited; deferred until all other tiers complete; writer is
   high quality but reader quality ceiling is bounded by package recognition.
   See "Tier A (read-limited)" above.

### commonmark-fmt vertical (CURRENT)

**Why wrapping pulldown-cmark, not from scratch:**
pulldown-cmark has 77M+ downloads; it IS the Rust CommonMark ecosystem (used by
mdBook, rustdoc). It already exposes `into_offset_iter()` yielding `(Event, Range<usize>)`
pairs — spans on every event, explicitly designed for AST construction (see its README:
"quite straightforward to construct an AST"). The tree-sitter backend was solving a
problem pulldown already solved; we just weren't using the right API.

**Crate:** `crates/formats/commonmark-fmt/`
Depends on pulldown-cmark. No rescribe dependency. Exposes:
- `parse(&[u8]) -> (Ast, Vec<Diagnostic>)` — drives pulldown's offset iterator,
  assembles (Event, Range) pairs into a full tree with Span on every node
- `emit(ast: &Ast) -> Vec<u8>` — round-trip correct
- `events(&[u8]) -> impl Iterator<Item = Event>` — thin re-export of pulldown events
- Feature flags: ast, streaming, batch, writer-streaming, writer-builder (all default=true)

**Architecture:** `commonmark-fmt` wraps pulldown-cmark. The three reader APIs:
- `parse()` — `TreeBuilder` over pulldown's `into_offset_iter()`. Direct and fast.
- `events()` — thin wrapper over pulldown's iterator; translates `pulldown_cmark::Event`
  to `commonmark_fmt::Event<'_>` with `Cow::Borrowed` slices from the input. Standard
  `Iterator`. Max perf — pulldown IS a true pull parser.
- `StreamingParser<H>` — buffers all chunks, runs pulldown on `finish()`. **Known
  limitation: not true chunked streaming.** Documented in the crate. Superseding
  pulldown-cmark is a non-goal; see `docs/format-library-design.md`.

**Build order:**
1. [x] Complete `fixtures/commonmark/` — all 74 COVERAGE.md boxes checked (2026-03-25)
2. [x] `ast.rs` — Block/Inline enums with Span on every node (2026-03-25)
3. [x] `parse.rs` — TreeBuilder over pulldown offset iterator (2026-03-25)
4. [x] `emit.rs` — Ast → bytes, round-trip guarantee (2026-03-25)
5. [x] `events.rs` — `Event<'a>` with `Cow<'a, str>`; `EventIter` wraps pulldown iterator (2026-03-25)
6. [x] `batch.rs` — `StreamingParser<H>` buffering wrapper; `Handler` trait; limitation documented (2026-03-25)
7. [x] `writer.rs` — `Writer<W: Write>` streaming writer (2026-03-25)
8. [x] No-panic fuzz gate (`fuzz_commonmark_reader`) (2026-03-25)
9. [x] Round-trip fuzz (`fuzz_commonmark_roundtrip`) — compile-verified (2026-03-25)
10. [x] `rescribe-read-markdown` + `rescribe-read-commonmark`: tree-sitter backend dropped; both now use commonmark-fmt (2026-03-25)
11. [x] 5-Production sign-off — fuzz_commonmark_reader 342K runs clean; fuzz_commonmark_roundtrip 4+ hours / ~2M+ runs clean after 12 crash artifacts fixed (2026-03-25)

**GFM extensions** (after base complete):
Tables, strikethrough (`~~text~~`), task list items (`- [x]`), extended autolinks

### Milestone: M1 ✓

- [x] Write fixture runner (`rescribe-fixtures`, `tests/run.rs`)
- [x] Hook fixture runner into CI (`cargo test --all-targets`)
- [x] Fill gaps: all formats at ≥2-Fixtures
- [x] Presentation writers (Tier D): writer fixture infrastructure + one fixture each
- [x] Fixture spec v1.2: writer fixture format documented

### Milestone: M2 — Tier A verticals complete

Each Tier A format at 5-Production with a published standalone crate.

- [x] `rtf-fmt` vertical — **5-Production** (2026-03-03)
  - All 9 coverage gaps closed; 3 fuzz bugs found and fixed during final fuzz run
  - [x] **Ignored-list cleanup** — drawing-obj + Asian typography words added; 0% diagnostic rate
  - [x] **Font face** — `\fonttbl` pre-scan; `Inline::Font`; `style:font` in IR
  - [x] **Background color** — `\cb<N>`; `Inline::BgColor`; `style:background` in IR
  - [x] **Language tags** — `\lang<N>`; `Inline::Lang`; LCID→BCP-47 adapter
  - [x] **Code page** — `\ansicpg` pre-scan; CP1250/1251/1253/1254 dispatch
  - [x] **Tables** — `\intbl`/`\cell`/`\row` → `Block::Table`
  - [x] **Footnotes** — `{\footnote...}` sub-parsed; `Inline::Footnote`; `footnote_ref` in IR
  - [x] **Lists** — `{\*\pn\pnlvlblt}`/`{\*\pn\pnlvlbody}` → `Block::List`
  - [x] **Zero-diagnostic corpus gate** — `#[ignore]` test; 1125 files, 0% diagnostics
  - [x] **Fuzz clean** — reader/roundtrip/writer all clean; 3 bugs fixed (slice panic, OOM, UTF-8 boundary)
- [x] `rst-fmt` vertical — orphan-API recovery done, **R:4/W:4** (recovered 2026-07-28; see
  `docs/format-audit.md`'s rst row and "RST reader" section, and the top-of-file entry above
  for the full root-cause writeup)
  - [x] No-panic fuzz gate (`fuzz_rst_reader`); roundtrip fuzz (`fuzz_rst_roundtrip`) —
    covers `parse()`/`build()` only, see gap below
  - [x] Fixtures: 80 total; COVERAGE.md all boxes checked (2 N/A items: include directive, hard break)
  - [x] Oracle harness: 100% word coverage on rst-reader.rst (ref=618)
  - [x] Benchmarks: rst_parse_small 3.3µs, rst_parse_medium 30µs, rst_emit_medium 2.5µs
  - [x] reader-ast (`parse()`) and writer-builder (`build()`) — real, tested, unaffected throughout
  - [x] Table parsing — grid and simple tables with header support (2026-03-29)
  - [x] Footnote parsing — numbered, auto-symbol, auto-numbered, multi-line continuation (2026-03-29)
  - [x] **reader-streaming (`events()`/`EventIter`)** — rebuilt 2026-07-28 as a thin wrapper
    composing `Parser` (not a duplicate grammar): `expand_block`/`expand_inline` lazily turn
    one already-parsed top-level `Block` into a `Vec<Frame>` stack, `O(nesting depth)`.
    Covers every current construct including tables/footnotes (added after the old iterator
    was deleted) and the `Div{class:"line-block"}` representation.
  - [x] **reader-batch (`StreamingParser<H>`)** — needed no changes; already re-parses each
    accumulated blank-line-delimited block through `events()`, `O(largest block)`.
  - [x] **writer-streaming (`Writer<W>`)** — rewritten 2026-07-28; the salvaged version
    buffered the entire event stream and only emitted at `finish()` (the fake-streaming
    pattern CLAUDE.md rejects). Now flushes each top-level block to the sink the moment its
    `End*` event arrives, `O(largest top-level block + nesting depth)`.
  - [x] Tests: events↔parse shape equivalence (11 inputs, all constructs); 6 chunked
    `StreamingParser` tests with awkward mid-token/mid-line/mid-UTF-8/mid-construct splits;
    full-construct-mix round-trip through `Writer`. 45 unit + 3 doc tests pass; clippy clean.
  - [x] Orphan-recurrence guard: `crates/formats/rst-fmt/tests/no_orphan_modules.rs` — walks
    `src/` from `lib.rs` over `mod` declarations, fails on any unreachable `.rs` file.
    Verified it actually catches this bug class (confirmed `cargo build` stays green with a
    genuinely-unreferenced file present, while this test fails). Workspace-wide sweep with
    the same logic found no real orphans elsewhere (3 flagged, all confirmed heuristic false
    positives — sibling `lib.rs`+`main.rs` crates, one `#[path=...]` redirect in jats-fmt).
  - [ ] **Remaining for true 5-Production**: no fuzz target exercises `events()`,
    `StreamingParser`, or `Writer` directly — `fuzz_rst_reader`/`fuzz_rst_roundtrip` only
    ever drove `parse()`/`build()` via the adapters. Add no-panic + chunked-split fuzz
    coverage for the three restored APIs specifically.
  - [ ] Two pre-existing, unrelated `build_block` bugs found via the new Writer round-trip
    test (not fixed, logged for whoever next touches the writer-builder): admonition
    directives (`.. note::` etc.) lose their wrapper on write-back (`Block::Div{class,..}`'s
    builder arm ignores `class`); `Block::FootnoteDef`'s builder emits only `\n` instead of
    a blank-line separator, so a following ≥3-space-indented block gets swallowed into the
    footnote body on re-parse. Both reproduce with plain `crate::build()`, no streaming
    involved.
  - [ ] Cross-cutting, explicitly out of scope for the rst vertical: `crates/rescribe-
    fixtures/tests/run.rs` tests every format via `parse()`/`emit()` only — no format's
    fixtures exercise `events()`/`StreamingParser`/a streaming writer, for any crate. Same
    blind-spot class the markdown suite hit. A horizontal sweep, not part of this vertical.
- [x] `asciidoc` vertical — **5-Production** (2026-03-29)
  - [x] No-panic fuzz gate (`fuzz_asciidoc_reader`); roundtrip fuzz (`fuzz_asciidoc_roundtrip`)
  - [x] Fixtures: 84 total; COVERAGE.md all boxes checked
  - [x] Oracle harness: N/A (pandoc can't read asciidoc)
  - [x] Benchmarks: asciidoc_parse_small 6.6µs, asciidoc_parse_medium 48µs, asciidoc_emit_medium 1.9µs
  - [x] All API modes: ast + stream + batch + w-build + w-stream
  - [x] Table parsing — with header row detection (2026-03-29)
  - [x] Footnote parsing — anonymous + named + back-reference forms (2026-03-29)
  - [x] Math parsing — `stem:[...]` inline + `[stem]\n++++` block (2026-03-29)
- [x] `textile-fmt` vertical — **5-Production** (2026-03-29)
  - [x] Split monolith lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span on every AST node; Diagnostic type; strip_spans()
  - [x] parse() infallible → (TextileDoc, Vec<Diagnostic>)
  - [x] build() renamed to emit() returning String
  - [x] No-panic fuzz gate (`fuzz_textile_reader`) — 1.6M runs clean (2026-03-21)
  - [x] Roundtrip fuzz target (`fuzz_textile_roundtrip`) — 923K runs clean (2026-03-21)
  - [x] Fixed infinite loop bug: list parser on `** ` (level-2 marker with no level-1 items)
  - [x] Fixtures: table, image, superscript, subscript added (2026-03-21); COVERAGE.md all checked
  - [x] Footnotes — FootnoteDef block + FootnoteRef inline (2026-03-28)
  - [x] Definition lists — DefinitionList block with term/desc pairs (2026-03-28)
  - [x] Oracle harness (`pandoc_textile_corpus` #[ignore]) — pandoc_harness.rs (2026-03-29)
  - [x] Benchmarks: textile_parse_small ~1.9µs, textile_parse_medium ~47µs (2026-03-29)
  - [x] All API modes: ast + stream + batch + w-build + w-stream (2026-03-29)
- [x] `org-fmt` vertical — **5-Production** (2026-03-29)
  - [x] No-panic fuzz gate (`fuzz_org_reader`) — 1.25M runs clean; roundtrip fuzz clean
  - [x] Fixtures: 88 total; COVERAGE.md all boxes checked
  - [x] Oracle harness: 100% word coverage on writer.org (ref=919)
  - [x] Benchmarks: org_parse_small 3.4µs, org_parse_medium 53µs, org_emit_medium 2.9µs
  - [x] All API modes: ast + stream + batch + w-build + w-stream
  - [x] Nested blockquote parsing (depth counter) — (2026-03-29)
  - [x] Footnote definitions — `[fn:label]` block-level (2026-03-29)
  - [x] Figure/caption blocks — `#+CAPTION:`/`#+NAME:` wrapping image/table/code (2026-03-29)
- [ ] `muse-fmt` vertical — **4-Fuzz** → needs re-fuzz after construct expansion
  - [x] Split monolith lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span on every AST node; Diagnostic type; strip_spans()
  - [x] parse() infallible → (MuseDoc, Vec<Diagnostic>)
  - [x] No-panic fuzz gate (`fuzz_muse_reader`) — 1.65M runs clean (2026-03-21); needs re-run after expansion
  - [x] Roundtrip fuzz target (`fuzz_muse_roundtrip`) — 1.15M runs clean (2026-03-21); needs re-run
  - [x] Constructs: tables, verse, footnotes, centered/right/literal/src blocks, underline, strikethrough, sup, sub, image, anchor, line-break (2026-03-29)
  - [x] Fixtures: COVERAGE.md fully checked (2026-03-29); composition + adversarial + pathological added
  - [x] Oracle harness: `pandoc_muse_corpus` #[ignore] + `parse_sample_no_panic` CI test (2026-03-29)
  - [x] Benchmarks: muse_parse_small, muse_parse_medium, muse_emit_medium (2026-03-29)
  - [x] All API modes: ast + stream + batch + w-build + w-stream (2026-03-29)
  - [ ] Re-run fuzz clean after construct expansion (pre-req for 5-Production)
- [ ] `man-fmt` vertical — **4-Fuzz** → needs re-fuzz after expansion (2026-03-29)
  - [x] Split monolith lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span on every AST node; Diagnostic type; strip_spans()
  - [x] parse() infallible → (ManDoc, Vec<Diagnostic>)
  - [x] No-panic fuzz gate (`fuzz_man_reader`) — 2M runs clean; needs re-run
  - [x] Roundtrip fuzz target (`fuzz_man_roundtrip`) — 855K runs clean; needs re-run
  - [x] New constructs: IndentedParagraph, ExampleBlock, Comment blocks; Code/Superscript/Subscript inlines; special char escapes; .TH full metadata (2026-03-29)
  - [x] All API modes: ast + stream + batch + w-build + w-stream (2026-03-29)
  - [x] Oracle harness + benchmarks (2026-03-29)
  - [x] Fixtures: COVERAGE.md mostly checked (few N/A items: .SY/.RS synopsis, \fP, \f[name]) (2026-03-29)
  - [ ] Re-run fuzz clean after construct expansion
- [x] `djot-fmt` vertical — **5-Production** (2026-03-29; writer signed off)
  - [x] All API modes: ast + stream + batch + w-build + w-stream
  - [x] Oracle harness: 100% word coverage on djot-reader.djot (ref=931)
  - [x] Fixtures: 79 total; COVERAGE.md all boxes checked
  - [x] Benchmarks: djot_parse_small 7.8µs, djot_parse_medium 49µs, djot_emit_medium 9.8µs
  - [x] Fuzz reader: fuzz_djot_fmt_reader + fuzz_djot_fmt_roundtrip — 21M runs clean
  - [x] Fuzz writer: fuzz_djot_roundtrip — 1M runs clean
  - [x] Writer: no construct gaps vs reader; all Block+Inline variants handled in emit.rs + writer.rs

---

## Standalone crate API completion (level 2 & 3)

Goal: every format crate ships all five API modes as separate Cargo features (all on by
default). This is the "Rust ecosystem (any consumer)" deliverable — useful entirely outside
rescribe. See CLAUDE.md vertical completion checklist for the full spec.

Five modes: `ast` · `stream` · `batch` · `w-stream` · `w-build`

### `djot-fmt` — complete (2026-03-23)

jotdown had a confirmed char-reordering bug and unfriendly API. `djot-fmt` was written
from scratch as a proper standalone library.

- [x] Create `crates/formats/djot-fmt/` with `ast.rs` / `parse.rs` / `emit.rs` / `events.rs`
- [x] AST covering full Djot spec: all block types, all inline types, attributes, footnotes,
  definition lists, math, raw blocks, task lists, tables
- [x] `parse(input: &str) -> (DjotDoc, Vec<Diagnostic>)` — infallible, Span on every node
- [x] `emit(ast: &DjotDoc) -> String` — builder writer
- [x] `events(input: &str) -> impl Iterator<Item = Event>` — streaming, no full AST,
  smart punctuation folded into text (not separate variants)
- [x] Fuzz: `fuzz_djot_fmt_reader` (no-panic) + `fuzz_djot_fmt_roundtrip` (parse(emit(ast))==ast)
  - 21M roundtrip runs clean; 4 parse bugs found and fixed
- [x] Fuzz: `fuzz_djot_reader` (rescribe-level) + `fuzz_djot_roundtrip` (updated: strict equality)
- [x] Update `rescribe-read-djot` to use `djot-fmt` instead of jotdown
- [x] Pandoc harness 100% after migration (ref=931, ours=943)
- [x] Benchmarks: djot_parse_small 7.8µs, djot_parse_medium 49µs, djot_emit_medium 9.8µs
- [x] `batch` chunk-driven parser (BatchParser + BatchSink) — 2026-03-23
- [x] Streaming writer (`w-stream`) — Writer<W: Write> with write_event/finish — 2026-03-23
- [x] Fix events() — now a true pull iterator (2026-03-24)
- [x] StreamingParser<H: Handler> + Handler trait — 2026-03-25
- [x] events() frame-stack fix — O(nesting depth), not O(block subtree) (2026-03-28)
- [x] parse() direct recursive descent — independent of events() (2026-03-28)
- [x] StreamingParser<H> Tier 2 — O(largest block) streaming (2026-03-28)
- [x] `Cow::Borrowed` zero-copy text for headings and paragraphs (2026-03-28)
  - `Frame::InlineText { span, content }` carries absolute span + owned fallback
  - `ParseContext::line_offset_at()` provides line→byte mapping (0 for SubParser)
  - `push_heading_frames` / `push_paragraph_frames` pass real base_offset to parse_inlines
  - EventIter::next() checks `&input[span] == content` before borrowing; falls back to Owned
  - Smart punctuation (e.g. `--` → `–`) correctly returns Cow::Owned (content ≠ input slice)
  - SubParser events always Cow::Owned (no input reference available)

### `rtf-fmt` — API modes (2026-03-28)

- [x] `ast`: `parse(input: &[u8]) -> (RtfDoc, Vec<Diagnostic>)` — Span on every node
- [x] `ast`: `emit(ast: &RtfDoc) -> Vec<u8>` — builder writer
- [x] `stream` (token level): `token_events(input: &[u8]) -> TokenEventIter` — raw RTF tokens
- [x] `stream` (semantic): `events(input: &[u8]) -> SemanticEventIter` — document-semantic events;
  internally calls `parse()` first (RTF group/property inheritance requires full context);
  walks parsed RtfDoc with frame-stack; documented limitation
- [x] `batch`: `StreamingParser<H: Handler>` + `Handler` trait (2026-03-28)
  RTF is O(full input) — structural constraint (font/color tables must precede body);
  documented as inherent format limitation, not an implementation shortcut.
- [x] `w-build`: `emit()` builder writer
- [x] `w-stream`: Writer<W: Write> streaming writer — exists as writer::Writer<W> (token-level; 2026-03-28)

### DEBT: Adapter crates containing format parsing logic — identified 2026-04-10

The rule: adapter production code must not contain format parsing/writing.
Large line counts from AST↔IR translation are acceptable (DOCX, PPTX are genuinely complex).
The violation is format-parsing deps (quick-xml, zip, etc.) called from production functions.

- **`rescribe-read-docx`**: CLEAN — `parse_numbering_order()` moved to `ooxml-wml` (fixed 2026-04-10).
- **`rescribe-read-odt`**: CLEAN — rewritten to use `odf_fmt::parse()` (fixed 2026-04-10).
- **`rescribe-write-odt`**: CLEAN — rewritten to use `odf_fmt::emit()` (fixed 2026-04-10).
- **`rescribe-read-pptx`**: `zip` in `[dependencies]` but only used by `gen_fixtures`
  binary and `#[cfg(test)]`. Production parsing path is clean. Acceptable.
- **`rescribe-read-fb2`**: CLEAN — uses `fb2-fmt` (fixed 2026-04-10).
- **`rescribe-write-fb2`**: CLEAN — uses `fb2-fmt` (fixed 2026-04-10).
- **`rescribe-read-docbook`**: CLEAN — uses `docbook-fmt` (fixed 2026-07-26).
- **`rescribe-write-docbook`**: CLEAN — uses `docbook-fmt` (fixed 2026-07-26).
- **`rescribe-read-jats`**: CLEAN — uses `jats-fmt` (fixed 2026-07-26).
- **`rescribe-write-jats`**: CLEAN — uses `jats-fmt` (fixed 2026-07-26).
- **`rescribe-read-tei`**: CLEAN — uses `tei-fmt` (fixed 2026-07-26).
- **`rescribe-write-tei`**: CLEAN — uses `tei-fmt` (fixed 2026-07-26).

Fix each when doing that format's vertical. Do NOT fix all at once (horizontal sweep).

**Superseded by a full sweep, 2026-07-28.** The list above was incidental (only the
crates that happened to come up during other work). Every reader and writer adapter has
now been audited against the rule; the complete inventory lives in
`docs/format-audit.md` § "Adapter parsing/emitting-logic inventory" — 65 formats,
38 clean, 14 violating, 13 uncertain. Not repeated here. Headlines only:

- **Three PARTIAL MIGRATION cases** — `commonmark`, `djot`, `ansi`: the `-fmt` crate
  exists and the *reader* uses it, but the writer hand-rolls emission and never calls
  the crate's `emit()`. These look done from `Cargo.toml` and are not; `djot`'s writer
  doesn't even declare the dependency. Highest-value fixes (backing crate already there).
- **Worst violation: `latex`** — `rescribe-read-latex/src/handwritten.rs` is an 895-line
  recursive-descent LaTeX parser living inside the reader adapter, plus a 662-line
  tree-sitter backend and a 717-line hand-written emitter in the writer. No `latex-fmt`.
- **No standalone crate at all**: latex, opml, endnotexml, bibtex, biblatex, csl-json,
  pandoc-json, ipynb, typst (writer side).
- **New findings beyond the bibliographic four**: `pandoc-json` has the same
  schema-in-adapter shape as `csl-json` (not previously listed); the whole markdown
  writer family (`markdown`, `gfm`, `markdown-strict`, `multimarkdown`) plus `typst`'s
  writer are hand-rolled emitters with no backing crate.
- **Open policy call, not decided**: whether the 11 output-only rendering targets
  (beamer, revealjs, slidy, s5, dzslides, slideous, context, ms, icml, chunkedhtml,
  plaintext) fall under the rule at all — they have no reader and no round-trip
  consumer, so a `beamer-fmt` crate may serve no real ecosystem user. Recorded as
  uncertain in the audit doc rather than guessed either way.
- Confirmed clean on re-verification: the previously-listed docx/odt/pptx/fb2/docbook/
  jats/tei entries above, plus `ris` (the one bibliographic vertical done right).

### `docbook-fmt` crate created (2026-07-26)

Standalone DocBook/generic-XML AST + parser + emitter (`crates/formats/docbook-fmt`),
wrapping `quick-xml` — no rescribe dependency. `rescribe-read-docbook` and
`rescribe-write-docbook` rewired to thin AST↔IR translators over `docbook_fmt::Node`
(no `quick-xml` left in either adapter's production code).

- [x] AST: `DocBookDoc { xml_decl, nodes: Vec<Node> }`; `Node::{Element, Text, Cdata,
  Comment, ProcessingInstruction, Doctype, EntityRef}`, `Span`, `Diagnostic`, `strip_spans()`
- [x] `parse(&[u8]) -> (DocBookDoc, Vec<Diagnostic>)` — direct recursive-descent build
  over `quick_xml::Reader`, never panics (malformed input recovered best-effort + diagnostics)
- [x] `events(&[u8]) -> EventIter` — **true SAX streaming**, not derived from `parse()`.
  Unlike `html-fmt` (which must build the full tree because HTML5 tree construction can
  rearrange nodes), XML is well-nested by construction, so `EventIter` wraps
  `quick_xml::Reader` directly and is genuinely O(largest token) memory.
- [x] `StreamingParser<H: Handler>` (`batch.rs`) — genuinely incremental: dispatches every
  event to the handler as soon as it's provably complete and drops the consumed prefix
  from its buffer, so memory is bounded by the largest in-progress token, not the whole
  document. The one non-obvious case: quick-xml can't distinguish "text run ended because
  `<` was found" from "text run ended because the buffer ran out" — a `Text` event is only
  dispatched once it's terminated by an actual `<` boundary or `finish()` confirms EOF.
  Verified with chunk-boundary-splitting tests (text split mid-word, tag split mid-name).
- [x] Entity handling: quick-xml 0.39 tokenizes `&name;`/`&#N;` as its own `GeneralRef`
  event rather than folding it into `Text`. The 5 predefined XML entities and numeric
  character refs are resolved and merged into the surrounding text; any other named
  (DTD-defined) entity is preserved verbatim as `Node::EntityRef` / IR `raw_inline` with
  `docbook:entity` — never silently dropped, per CLAUDE.md's raw-preservation rule.
- [x] `emit(&DocBookDoc) -> Vec<u8>` builder writer + `Writer<W: Write>` streaming writer,
  both via `quick_xml::Writer` for correct escaping
- [x] Full construct parity with the pre-split adapter logic preserved (all node kinds the
  old hand-rolled reader/writer handled still map the same way); `xlink:href` link
  attribute matching now actually works (previously dead code — the old reader stripped
  namespace prefixes before matching the literal string `"xlink:href"`, so it could never
  match; `docbook-fmt` keeps the raw prefixed attribute name)
- [x] `cargo clippy --all-targets --all-features -p docbook-fmt -p rescribe-read-docbook
  -p rescribe-write-docbook -- -D warnings` and full test suite (incl. fixture suite) clean
- [x] Fuzz targets added (2026-07-26): `fuzz_docbook_fmt_reader` (no-panic gate on
  `parse()`/`events()`, 1.69M runs clean in initial 60s validation) and
  `fuzz_docbook_fmt_roundtrip` (arbitrary `DocBookDoc` → `emit()` → `parse()` →
  `strip_spans()` equality, per CLAUDE.md's arbitrary-AST-first direction; 573K runs
  clean). Only a fuzz-harness bug found (duplicate attribute names on one element —
  invalid XML, fixed by suffixing generated names with their index), no library bugs.
  Initial validation only, not an exhaustive campaign — see `docs/format-audit.md`.
  **Superseded (2026-07-27)**: extended campaign run, `-max_total_time=900` (15 min)
  per target via `cargo fuzz run <target> -- -max_total_time=900` in the
  `nix develop .#fuzz` shell — `fuzz_docbook_fmt_reader` 8,918,090 runs clean,
  `fuzz_docbook_fmt_roundtrip` 6,012,874 runs clean. No crashes, no panics, no
  artifacts written, no roundtrip mismatches. No bugs found this pass — now at
  parity with `tei-fmt`'s campaign scale.
- [x] **Bug found and fixed (2026-07-27)**: two silent-drop bugs closed,
  mirroring the tei fix (same audit, applied to this vertical). (1) The
  reader's final `_ => None` catch-all arm silently unwrapped *any*
  unrecognized element into its parent, discarding the fact that the tag
  ever existed with no warning. `rescribe-read-docbook` gained
  `is_block_element(tag: &str) -> bool` (a DocBook block-level vocabulary
  allow-list, mirroring `rescribe-read-tei`/`rescribe-read-html`) plus
  `generic_div`/`generic_span` helpers; the catch-all now raw-preserves an
  unrecognized element as a `docbook:tag`-tagged `div` (block-shaped) or
  `span` (inline-shaped) instead of dropping the tag. New fixtures
  `adv-unknown-block-element` (`<sidebar>`) and `adv-unknown-inline-element`
  (`<quote>` nested in running text) regression-test both branches.
  (2) `<info>`/`<articleinfo>`/`<bookinfo>` front-matter beyond `title`
  (author, authorgroup, date, copyright, legalnotice, pubdate, releaseinfo,
  revhistory, revision, or any other unmodeled field) was silently dropped —
  `extract_metadata` only ever extracted `title`. `docbook-fmt` gained a
  `Node::Raw { content, span }` AST variant plus
  `emit_fragment(nodes: &[Node]) -> Vec<u8>` (mirroring `tei-fmt`/`html-fmt`).
  `convert_children` now threads an `in_header: bool` through its recursion
  (true once inside `<info>`/`<articleinfo>`/`<bookinfo>` or any descendant)
  and, for any child not in the new `is_modeled_header_field` allow-list
  (just `title` — the only field with dedicated semantic extraction before
  this fix, per the current-code check this pass started from), captures
  the whole subtree's original XML via `docbook_fmt::emit_fragment` and
  stores it as `{tag}_raw` metadata (e.g. `author_raw`) alongside a
  flattened `{tag}` text convenience copy — generalized directly to the
  `{tag}_raw` naming from the start (not the two-hardcoded-names
  intermediate step tei's own fix went through first, since docbook had no
  prior per-field modeling to preserve compatibility with).
  `extract_metadata` matches both `span` and `div` node kinds and skips
  recursing into an already-raw-captured subtree's children. A repeatable
  field (e.g. more than one `<author>`) joins its flattened text with `"; "`
  and concatenates its raw XML (valid, since concatenated sibling XML
  elements are themselves valid XML content) rather than a later occurrence
  silently overwriting an earlier one. The fidelity warning path only fires
  if raw capture genuinely fails (non-UTF8 content — not expected for XML
  that parsed at all). `rescribe-write-docbook` now emits an `<info>`
  wrapper (title plus any spliced-back `*_raw` fields, sorted by tag for
  deterministic output) instead of writing a bare `<title>` only. New
  fixture `header-author` (`<info><author>` with nested `<personname>`)
  regression-tests the general fallback. `cargo clippy --all-targets
  --all-features -p docbook-fmt -p rescribe-read-docbook
  -p rescribe-write-docbook -- -D warnings` and full test/fixture suite
  clean.
- [ ] DTD-aware entity resolution and closing the remaining
  `fixtures/docbook/COVERAGE.md` gaps are follow-up work — out of scope for this pass per
  CLAUDE.md (Tier B target is 3-Harness, not 5-Production)

### `jats-fmt` crate created (2026-07-26)

Standalone JATS/generic-XML AST + parser + emitter (`crates/formats/jats-fmt`),
wrapping `quick-xml` — no rescribe dependency. Mirrors `docbook-fmt`'s architecture
exactly since JATS, like DocBook, is well-nested XML with no format-specific AST needs
(element semantics live entirely in the rescribe adapter, not the crate). `rescribe-read-jats`
and `rescribe-write-jats` rewired to thin AST↔IR translators over `jats_fmt::Node`
(no `quick-xml` left in either adapter's production code).

- [x] AST: `JatsDoc { xml_decl, nodes: Vec<Node> }`; `Node::{Element, Text, Cdata,
  Comment, ProcessingInstruction, Doctype, EntityRef}`, `Span`, `Diagnostic`, `strip_spans()`
- [x] `parse(&[u8]) -> (JatsDoc, Vec<Diagnostic>)` — direct recursive-descent build
  over `quick_xml::Reader`, never panics (malformed input recovered best-effort + diagnostics)
- [x] `events(&[u8]) -> EventIter` — true SAX streaming, not derived from `parse()`
  (XML is well-nested, so no tree needs to be built first, unlike `html-fmt`'s HTML5 case)
- [x] `StreamingParser<H: Handler>` (`batch.rs`) — genuinely incremental: dispatches every
  event to the handler as soon as it's provably complete and drops the consumed prefix
  from its buffer, memory bounded by the largest in-progress token. Verified with
  chunk-boundary-splitting tests (text split mid-word, tag split mid-name).
- [x] Entity handling: the 5 predefined XML entities and numeric character refs are
  resolved and merged into surrounding text; any other named (DTD-defined) entity is
  preserved verbatim as `Node::EntityRef` / IR `raw_inline` with `jats:entity` — never
  silently dropped. The pre-split reader had **no** entity handling at all, so this is
  a net fidelity improvement, not just parity.
- [x] `emit(&JatsDoc) -> Vec<u8>` builder writer + `Writer<W: Write>` streaming writer,
  both via `quick_xml::Writer` for correct escaping
- [x] Full construct parity with the pre-split adapter logic preserved (all node kinds the
  old hand-rolled reader/writer handled still map the same way); one incidental fidelity
  fix — `<xref ref-type="…">` now preserves `jats:ref-type` for both the self-closing
  and full-element (`<xref ...>text</xref>`) shapes, where the old reader only attached
  it for the self-closing case
- [x] `cargo clippy --all-targets --all-features -p jats-fmt -p rescribe-read-jats
  -p rescribe-write-jats -- -D warnings` and full test suite (incl. fixture suite) clean
- [x] Fuzz targets added (2026-07-26): `fuzz_jats_fmt_reader` (no-panic gate on
  `parse()`/`events()`, 1.61M runs clean in initial 60s validation) and
  `fuzz_jats_fmt_roundtrip` (arbitrary `JatsDoc` → `emit()` → `parse()` →
  `strip_spans()` equality; 553K runs clean). No library bugs found. Initial
  validation only, not an exhaustive campaign — see `docs/format-audit.md`.
  **Superseded (2026-07-27)**: extended campaign run, `-max_total_time=900` (15 min)
  per target via `cargo fuzz run <target> -- -max_total_time=900` in the
  `nix develop .#fuzz` shell — `fuzz_jats_fmt_reader` 8,162,993 runs clean,
  `fuzz_jats_fmt_roundtrip` 5,696,913 runs clean. No crashes, no panics, no
  artifacts written, no roundtrip mismatches. No bugs found this pass — now at
  parity with `tei-fmt`'s campaign scale.
- [x] **Bug found and fixed (2026-07-27)**: two silent-drop bugs closed,
  mirroring the docbook/tei fix (same audit, applied to this vertical; docbook's
  final generalized form used directly as the template, not the two-hardcoded-
  names intermediate step tei went through first). (1) The reader's final
  `_ => None` catch-all arm silently unwrapped *any* unrecognized element into
  its parent, discarding the fact that the tag ever existed with no warning.
  `rescribe-read-jats` gained `is_block_element(tag: &str) -> bool` (a JATS
  block-level vocabulary allow-list, mirroring `rescribe-read-docbook`) plus
  `generic_div`/`generic_span` helpers; the catch-all now raw-preserves an
  unrecognized element as a `jats:tag`-tagged `div` (block-shaped) or `span`
  (inline-shaped) instead of dropping the tag. New fixtures
  `adv-unknown-block-element` (`<statement>`) and `adv-unknown-inline-element`
  (`<styled-content>` nested in running text) regression-test both branches.
  (2) `<article-meta>`/`<journal-meta>` front-matter beyond `title`/
  `article-title` (contrib-group, pub-date, volume, issue, fpage, lpage,
  permissions, history, or any other unmodeled field) was silently dropped —
  `extract_metadata` only ever extracted `title`. `jats-fmt` gained a
  `Node::Raw { content, span }` AST variant plus
  `emit_fragment(nodes: &[Node]) -> Vec<u8>` (mirroring `docbook-fmt`/
  `tei-fmt`). `convert_children` now threads an `in_header: bool` through its
  recursion (true once inside `<article-meta>`/`<journal-meta>` or any
  descendant) and, for any child not in the new `is_modeled_header_field`
  allow-list (just `title`/`article-title` — the only fields with dedicated
  semantic extraction before this fix), captures the whole subtree's original
  XML via `jats_fmt::emit_fragment` and stores it as `{tag}_raw` metadata
  (e.g. `contrib-group_raw`) alongside a flattened `{tag}` text convenience
  copy. `<title-group>` gained an explicit pass-through arm (it wraps
  `<article-title>`/journal `<title>` under both `<article-meta>` and
  `<journal-meta>`) so the already-modeled title reaches `extract_metadata`
  as a direct sibling instead of being buried inside a `jats:raw` blob
  `extract_metadata` never recurses into. `extract_metadata` matches both
  `span` and `div` node kinds and skips recursing into an already-raw-captured
  subtree's children. A repeatable field (e.g. more than one
  `<contrib-group>`) joins its flattened text with `"; "` and concatenates its
  raw XML rather than a later occurrence silently overwriting an earlier one.
  The fidelity warning path only fires if raw capture genuinely fails
  (non-UTF8 content — not expected for XML that parsed at all).
  `rescribe-write-jats` now emits an `<article-meta>` wrapper (title-group
  plus any spliced-back `*_raw` fields, sorted by tag for deterministic
  output) instead of writing a bare `<title-group>` only. New fixture
  `header-contrib-group` (`<contrib-group>` with nested `<contrib>`/`<name>`)
  regression-tests the general fallback. `cargo clippy --all-targets
  --all-features -p jats-fmt -p rescribe-read-jats -p rescribe-write-jats --
  -D warnings` and full test/fixture suite clean.
- [ ] DTD-aware entity resolution and closing the remaining
  `fixtures/jats/COVERAGE.md` gaps are follow-up work — out of scope for this pass per
  CLAUDE.md (Tier B target is 3-Harness, not 5-Production)

### `tei-fmt` crate created (2026-07-26)

Standalone TEI/generic-XML AST + parser + emitter (`crates/formats/tei-fmt`),
wrapping `quick-xml` — no rescribe dependency. Mirrors `docbook-fmt`/`jats-fmt`'s
architecture exactly since TEI, like DocBook and JATS, is well-nested XML with no
format-specific AST needs (element semantics live entirely in the rescribe adapter, not
the crate). `rescribe-read-tei` and `rescribe-write-tei` rewired to thin AST↔IR
translators over `tei_fmt::Node` (no `quick-xml` left in either adapter's production
code).

- [x] AST: `TeiDoc { xml_decl, nodes: Vec<Node> }`; `Node::{Element, Text, Cdata,
  Comment, ProcessingInstruction, Doctype, EntityRef}`, `Span`, `Diagnostic`, `strip_spans()`
- [x] `parse(&[u8]) -> (TeiDoc, Vec<Diagnostic>)` — direct recursive-descent build
  over `quick_xml::Reader`, never panics (malformed input recovered best-effort + diagnostics)
- [x] `events(&[u8]) -> EventIter` — true SAX streaming, not derived from `parse()`
  (XML is well-nested, so no tree needs to be built first, unlike `html-fmt`'s HTML5 case)
- [x] `StreamingParser<H: Handler>` (`batch.rs`) — genuinely incremental: dispatches every
  event to the handler as soon as it's provably complete and drops the consumed prefix
  from its buffer, memory bounded by the largest in-progress token. Verified with
  chunk-boundary-splitting tests (text split mid-word, tag split mid-name).
- [x] Attribute keys are kept exactly as written (including namespace prefix, e.g.
  `xml:id`, `xml:lang`) rather than local-name-stripped — TEI leans heavily on
  `xml:id`/`xml:lang`, and adapter-layer matching against the literal prefixed name only
  works if the prefix survives parsing.
- [x] Entity handling: the 5 predefined XML entities and numeric character refs are
  resolved and merged into surrounding text; any other named (DTD-defined) entity is
  preserved verbatim as `Node::EntityRef` / IR `raw_inline` with `tei:entity` — never
  silently dropped. The pre-split reader had **no** entity handling at all, so this is
  a net fidelity improvement, not just parity.
- [x] `emit(&TeiDoc) -> Vec<u8>` builder writer + `Writer<W: Write>` streaming writer,
  both via `quick_xml::Writer` for correct escaping
- [x] Full construct parity with the pre-split adapter logic preserved (all node kinds the
  old hand-rolled reader/writer handled still map the same way), plus one real fidelity
  bug fixed: the old reader captured `xml:id` and `n` attributes into a `FrameAttrs`
  struct on every element but never read either field back out when building IR nodes —
  both attributes were parsed and then silently discarded on every element that carried
  them (dead-code capture bug, same family as docbook's `xlink:href` issue). `xml:id`
  now round-trips as the standard `id` property; `n` round-trips as `tei:n`. Comments/PIs
  inside content flow (previously bare-dropped with no signal at all) now emit a
  `Minor` fidelity warning instead of vanishing silently.
- [x] `cargo clippy --all-targets --all-features -p tei-fmt -p rescribe-read-tei
  -p rescribe-write-tei -- -D warnings` and full test suite (incl. fixture suite) clean
- [x] Fuzz targets added (2026-07-26): `fuzz_tei_fmt_reader` (no-panic gate on
  `parse()`/`events()`, 1.59M runs clean in initial 60s validation) and
  `fuzz_tei_fmt_roundtrip` (arbitrary `TeiDoc` → `emit()` → `parse()` →
  `strip_spans()` equality; 527K runs clean). No library bugs found. Initial
  validation only, not an exhaustive campaign — see `docs/format-audit.md`.
  **Superseded (2026-07-27)**: extended campaign run, `-max_total_time=900`
  (15 min) per target via `cargo fuzz run <target> -- -max_total_time=900`
  in the `nix develop .#fuzz` shell — `fuzz_tei_fmt_reader` 7,518,438 runs
  clean, `fuzz_tei_fmt_roundtrip` 6,611,996 runs clean. No crashes, no
  panics, no artifacts written, no roundtrip mismatches. No bugs found this
  pass (the one fuzz-harness generator bug from the initial 2026-07-26 run
  — duplicate attribute names — was already fixed before this campaign).
- [ ] DTD-aware entity resolution is follow-up work — out of scope for this pass
  per CLAUDE.md (Tier B target is 3-Harness, not 5-Production)

### `fixtures/tei/COVERAGE.md` closed to 118/118 (2026-07-27)

Fixture suite completeness (vertical checklist step 1) reached: every item in
`fixtures/tei/COVERAGE.md` now has a passing fixture (85 new `fixtures/tei/*`
directories added across block, inline, teiHeader-metadata, property,
integration/e2e, adversarial, and pathological categories). This closing pass
required real reader/writer changes, not just fixture-writing:

- [x] `rescribe-read-tei`/`rescribe-write-tei`: ~35 new element mappings (`sp`,
  `speaker`, `stage`, `epigraph`, `argument`, `byline`, `dateline`/`salute`/`signed`,
  `trailer`, `castList`/`castItem`, `ab`, `gap`/`space`, `div5`/`div6`, list
  `type` variants, `<label>` items) plus a generic `span`-tagged (`tei:tag=`)
  raw-preservation path for editorial/named-entity apparatus (`choice`,
  `abbr`/`expan`, `orig`/`reg`, `sic`/`corr`, `add`/`del`/`supplied`/`unclear`,
  `persName`/`placeName`/`orgName`/`name`, `date`/`title`/`num`/`measure`,
  `anchor`/`milestone`/`seg`/`w`/`pc`, `foreign`, `bibl`) — this is the same
  `span` node kind already used for exactly this purpose.
- [x] `xml:lang`, `corresp`, `sameAs` added to `attach_generic_attrs` (alongside
  the existing `xml:id`/`n`); `style:align` now derived from alignment-only
  `rend` values (`center`/`right`/`left`/`justify`) on `p`/`div` rather than
  overloading the `<hi>`-only `tei:rend` fallback.
  `<formula type="inline">` now maps to `math_inline` instead of always
  `math_display`; bare `<code>` now maps to inline `code` (previously only
  `<eg>` was reachable, and both aliased to `code_block`).
- [x] teiHeader metadata extraction substantially deepened: `author`/`editor`
  (repeatable, `"; "`-joined), `publisher`/`idno`, `profileDesc/langUsage/language`
  (`ident` → `language`), `abstract`, `textClass/keywords`, `revisionDesc/change`
  (repeatable, timestamped) all now populate `Document::metadata` and round-trip
  through the writer (which previously only ever wrote/read `title`).
  `encodingDesc`/`msDesc` are flattened to plain-text metadata with an explicit
  `Minor` fidelity warning (structure genuinely not modeled — a tracked gap, not
  a silent drop). **Superseded**: see the `2026-07-27` entries below —
  `encodingDesc`/`msDesc`, and every other unmodeled teiHeader field, are now
  raw-preserved byte-for-byte instead of flattened-with-warning.
- [x] **Bug found and fixed**: the reader's final `_ => None` fallback arm
  silently unwrapped *any* unrecognized element into its parent — dropping the
  fact that e.g. `<foo>` ever existed, not just layout. Changed to raw-preserve
  as a generic tagged `span` (`adv-unknown-element` fixture regression-tests
  this). Same fix category added a catch-all fidelity warning for teiHeader
  fields with no known metadata key (previously silently scanned-and-discarded
  with zero signal).
- [x] `cargo clippy --all-targets --all-features -p tei-fmt -p rescribe-read-tei
  -p rescribe-write-tei -- -D warnings` and full test/fixture suite clean
  (all 111 `fixtures/tei/*` fixtures + all existing unit tests pass)
- [x] **Fixed (2026-07-27)**: teiHeader sub-structure (`msDesc`, `encodingDesc`)
  is now raw-preserved byte-for-byte, not just flattened to text. `tei-fmt`
  gained a `Node::Raw { content, span }` AST variant (mirroring `html-fmt`'s)
  plus `emit_fragment(nodes: &[Node]) -> Vec<u8>` (mirroring
  `html_fmt::emit_fragment`, used there to raw-capture inline MathML).
  `rescribe-read-tei` captures the `<msDesc>`/`<encodingDesc>` subtree's
  original XML via `emit_fragment` at the point it still holds the raw
  `tei_fmt::Node` (before IR conversion) and stores it as `ms_desc_raw`/
  `encoding_desc_raw` string metadata, alongside the existing flattened
  `ms_desc`/`encoding_desc` text kept for convenience. `rescribe-write-tei`
  prefers the raw metadata when present, splicing it back in via a
  `tei_fmt::Node::Raw` node; the fidelity warning now only fires if raw
  capture genuinely fails (should not happen for any XML that parsed
  successfully). `fixtures/tei/header-ms-desc` and `header-encoding-desc`
  updated to assert the new `*_raw` metadata keys and the no-warning case.
  **Superseded below (2026-07-27)**: this pass hand-picked only `msDesc`/
  `encodingDesc` for raw-preservation; the generalization that closes the
  gap for *every* unmodeled teiHeader field is the next entry.
- [x] **Fixed (2026-07-27)**: generalized the `msDesc`/`encodingDesc`
  raw-preservation above from a two-element special case to *any* teiHeader
  child element `convert_element` has no dedicated semantic mapping for.
  `convert_children` now threads an `in_header: bool` through its recursion
  (true once inside `<teiHeader>` or any of its descendants) and, for any
  such child not in the new `is_modeled_header_field` allow-list
  (`author`/`editor`/`publisher`/`idno`/`language`/`abstract`/`keywords`/
  `change`/`title` — the fields that already have dedicated semantic
  extraction), captures the whole subtree's original XML via
  `tei_fmt::emit_fragment`, same mechanism as before. The hardcoded
  `msDesc`/`encodingDesc` arms in `convert_element` were removed — they were
  already producing the exact same `generic_span`/`generic_div` node the
  generic catch-all does, so removal is a no-op for those two and the
  general path now covers them plus everything else (`particDesc`,
  `projectDesc`, or any other TEI header element). Metadata key naming
  generalized from the old ad hoc snake_case (`ms_desc_raw`,
  `encoding_desc_raw`) to `{tag}_raw` using the element's actual XML tag
  name (`msDesc_raw`, `encodingDesc_raw`, `particDesc_raw`, ...), plus a
  `{tag}` flattened-text convenience copy — `extract_metadata` now matches
  both `span` and `div` node kinds (previously `div`-shaped unrecognized
  header children were silently invisible to it) and skips recursing into
  an already-raw-captured subtree's children (nothing further to extract;
  previously this could double-process msDesc's internal
  msIdentifier/physDesc/etc. as spurious separate warnings). The old
  per-field fidelity warning only fires now if raw capture genuinely fails
  (non-UTF8 content — not expected for XML that parsed at all).
  `rescribe-write-tei` generalized correspondingly: instead of two hardcoded
  `ms_desc_raw`/`encoding_desc_raw` checks, it scans `Document::metadata`
  for any `*_raw`-suffixed key and splices each back via `tei_fmt::Node::Raw`
  as a `teiHeader` child, sorted by tag for deterministic output.
  `fixtures/tei/header-ms-desc`, `header-encoding-desc`, and
  `path-full-header` updated to the new key names; new fixture
  `header-partic-desc` (`<profileDesc><particDesc>`, an element with no
  explicit semantic mapping and not one of the previously-hardcoded two)
  regression-tests the general path directly. Residual gap: none known —
  every teiHeader child without a dedicated semantic mapping now
  raw-preserves rather than warn-and-drop; the warning path is reachable
  only in the (currently unexercised) genuine-raw-capture-failure case.
- [x] **Fixed (2026-07-27)**: an unrecognized element at block level no
  longer round-trips wrapped in an extra `<p>`. `rescribe-read-tei` gained
  `is_block_element(tag: &str) -> bool` (mirroring
  `rescribe-read-html::is_block_element`) listing TEI's block-level
  vocabulary, plus a `generic_div` helper (the block-level counterpart to
  `generic_span`). The catch-all fallback in `convert_element` now branches:
  unrecognized block-level elements become a `div` tagged `tei:tag`
  (`generic_div`), unrecognized inline elements keep the existing `span`
  path (`generic_span`). `rescribe-write-tei`'s `node::DIV` arm now falls
  back to `tei:tag` (re-emitting the original element name) when no
  `tei:type` matches, mirroring `rescribe-write-html::convert_div`'s
  `html:tag` handling. New fixtures `adv-unknown-block-element` (`<closer>`)
  and `adv-unknown-inline-element` (`<mysteryTag>` nested in running text)
  regression-test both branches; `adv-unknown-element` (the pre-existing
  fixture, an unrecognized element that is *not* in the block-level
  vocabulary, sitting at block-dispatch position) continues to assert the
  bare-span behavior for that case, which is correct — the classification is
  by TEI content-model shape, not by dispatch position.
- [ ] DTD-aware entity resolution remains out of scope for this pass (Tier B
  target is 3-Harness, not 5-Production; fixture-suite-complete is step 1 of 5
  in the vertical checklist — reader/writer completeness beyond the fixture
  suite, the oracle harness, and a longer fuzz campaign are still open).

### DEBT: Streaming architecture — COMPLETED 2026-03-28

**`events()` frame-stack — DONE:**
All four crates use `Vec<Frame>` frame-stack. Memory O(nesting depth). `parse()` is
direct recursive descent, independent of events().

**`StreamingParser<H>` Tier 2 — DONE (line-oriented crates):**
- org-fmt: blank-line separation + #+BEGIN_*…#+END_* (O(largest block))
- rst-fmt: blank-line separation + directive body (O(largest block))
- asciidoc: blank-line separation + delimited blocks (O(largest block))
- djot-fmt: blank-line separation + fenced code / div (O(largest block))
- rtf-fmt: O(full input) — documented structural constraint; cannot be improved
  without significant parser refactoring (font/color table dependency)
- commonmark-fmt: O(full input) — pulldown-cmark requires full `&str`; exemption documented

**`Cow::Borrowed` — DONE for djot-fmt (2026-03-28):**
`Text` events for headings and paragraphs now yield `Cow::Borrowed` when the span maps
cleanly to the original input (no escape processing). Implementation: `Frame::InlineText`,
`ParseContext::line_offset_at()`, real base_offset in push_heading/paragraph_frames.

**Remaining (other crates):**
**Does the rst-fmt pattern generalize? Spot-checked 2026-07-29 (org-fmt, djot-fmt,
mediawiki-fmt; read, not inferred).** Yes — rst-fmt was the *median* case, not an outlier.
The forward-declared-but-unwired `Cow<'a, str>` event enum is a house convention (org,
djot, mediawiki, muse, asciidoc share the enum shape and in three cases the verbatim "so
that future optimisations can" comment), so the AST-lifetime step is mechanical everywhere
and the API-compat story validated here carries over. The cost is concentrated in the
tokenizer rewrite and the line-joining decision, and both are genuinely per-crate:
- `org-fmt` — closest analogue to pre-change rst-fmt. `Vec<char>` tokenizer; `Span` fields
  exist but every construction site passes `Span::NONE`. Paragraphs already tokenize
  per-line (so its borrow ceiling should land near rst-fmt's 93%), but list items and code
  bodies join. Extra wrinkle: `find_inline_span` returns an owned `String` that is
  recursively re-parsed, so nested emphasis re-allocates at every level — a rewrite, not an
  index-type swap.
- `djot-fmt` — the one good outlier: `EventIter::next` already yields `Cow::Borrowed`
  slices guarded by a span-validity check. But it still has a `Vec<char>` tokenizer, and
  `current_byte_offset()` recomputes a prefix sum on every call — O(n²) in inline length,
  paid *to buy* those borrows. Its paragraph join silently demotes wrapped paragraphs to
  `Owned` via the equality guard. A djot port likely shows a *larger* throughput delta than
  rst-fmt's, despite starting further along.
- `mediawiki-fmt` — the floor. `Inline` has no `Span` field at all, so spans must be added
  as a prerequisite. `Vec<char>` in both inline and block paths, and block parsers take
  `&[&str]` and `join("\n")` at ≥6 sites, so nearly every block body is allocated before
  tokenizing. Expect a materially lower borrow ceiling until those signatures change.
- Range-setter: `textile-fmt` has no `Cow` anywhere and no lifetime on its event type, so
  the forward-declared-`Cow` convention is not even universal — the tail is not uniform.

- [ ] `Cow::Borrowed` for org-fmt — inline parser uses `Span::NONE`; needs span tracking in parse_inline_content before base_offset approach works
- [x] `Cow::Borrowed` for rst-fmt — **DONE (2026-07-29)**, see the dedicated entry below.
- [ ] `Cow::Borrowed` for asciidoc — same shape as rst-fmt was; the rst-fmt commit is the
  worked reference for the lifetime-generic-AST + byte-indexed-tokenizer pattern.
- [ ] `Cow::Borrowed` for djot-fmt Verbatim/Math — Verbatim trimming means span ≠ content slice; would need a content-only span separate from the full backtick-construct span

### `rst-fmt` — API modes: **complete and tested, 2026-07-28 (fuzz coverage still open)**

This section previously (correctly, at the time) recorded all five API modes as deleted/
non-functional after merge commit `383d4e6adf` discarded `mod events;`/`mod batch;`/
`mod writer;`, `EventIter`, and the `Block::LineBlock`→`Div{class:"line-block"}` migration
as collateral damage (see the top-of-file 2026-07-28 entry and `docs/format-audit.md`'s
"RST reader" section for the full root-cause writeup). All five are now real:

- [x] `ast`: `parse() direct recursive descent` — unaffected throughout (2026-03-28)
- [x] `stream`: `events(input: &str) -> EventIter` — rebuilt as a thin composition over
  `Parser` (calls `Parser::try_parse_block()` one top-level block at a time, then lazily
  expands that block into a `Vec<Frame>` stack), not a resurrection of the deleted
  ~1300-line duplicate-grammar `EventIter`. Covers tables/footnotes (added after the old
  iterator was deleted) and the current `Div{class:"line-block"}` representation.
- [x] `batch`: `BatchParser` (feed/finish) + `BatchSink<F>` — unaffected; both just call
  `crate::parse()`/`crate::events()` respectively, which work again now.
- [x] `batch`: `StreamingParser<H: Handler>` — needed no changes; its O(largest block)
  design (re-parses each accumulated blank-line-delimited block through `events()`) was
  already sound, just uncompilable while `events()` didn't exist.
- [x] `w-stream`: `Writer<W: Write>` — rewritten (not just fixed): the salvaged version
  buffered the whole event stream and only emitted at `finish()`, the fake-streaming
  pattern CLAUDE.md rejects; now flushes each completed top-level block to the sink
  immediately via the shared `build_block`, O(largest top-level block + nesting depth).
- [x] Feature flags in `Cargo.toml` (`reader-streaming`, `reader-batch`, `writer-streaming`)
  now gate real, working code.
- [x] Table/footnote parsing — covered by `events()`/`Writer` (see above); no longer a gap.
- [x] Tests: events↔parse shape equivalence (11 inputs); 6 chunked `StreamingParser` tests
  with awkward splits; full-construct-mix `Writer` round-trip. 45 unit + 3 doc tests pass.
- [x] `tests/no_orphan_modules.rs` — recurrence guard, verified it catches this bug class.
- [ ] **Still open**: no fuzz target drives `events()`/`StreamingParser`/`Writer` directly
  (existing `fuzz_rst_reader`/`fuzz_rst_roundtrip` only ever exercised `parse()`/`build()`
  via the adapters) — needed before re-claiming 5-Production.
- [ ] Two pre-existing `build_block` bugs found (unrelated to streaming, not fixed): `Div`
  admonitions lose their directive wrapper on write-back; `FootnoteDef` is missing a
  blank-line separator after its body, so a following indented block gets swallowed into
  it on re-parse.

### `rst-fmt` — streaming Writer subtree-reconstruction fix + events() zero-copy scoped (2026-07-29)

A benchmark investigation (methodology + numbers: `docs/format-audit.md`'s 2026-07-29 entry)
found `Writer::process` was still funneling through `build_block` — it reconstructed a full
`Block`/`Inline` subtree per top-level block via the `Frame` stack, then called the exact
function `build()` uses, on top of that reconstruction. Fixed:

- [x] `Writer` rewritten to emit RST text directly from events — every frame accumulates
  already-formatted `String` buffers (plus a parallel plain-text buffer only where genuinely
  read: `Heading` underline sizing, `TableCell` content) and each `End*` renders+splices once;
  no `Block`/`Inline` is ever constructed. Commit `01472e3027`.
- [x] Follow-up: gated an unconditional plain-text-tracking cost (`heading_depth`/
  `table_cell_depth` counters) — commit `4daecb99`. Measured before/after: **no wall-clock
  change** (this was not the dominant cost).
- [x] `test_writer_roundtrip_nested_lists`, `test_writer_no_subtree_reconstruction_blowup`
  (allocation-growth regression guard) added; all pre-existing tests still pass.
- [x] **Buffer-per-frame allocation cost closed — commit `f87b3d62ef`.** The `Frame` stack
  now holds *marks* (a `usize` offset into one shared `Writer::out` buffer) instead of
  buffers; children write straight through and a frame post-processes its own `out[mark..]`
  range in place. Constructs classified three ways: **write-through** (paragraphs, lists,
  list items — `build_list_item`'s per-child dispatch is decidable when the *child* opens,
  so continuation indents are emitted then, not reconstructed later — divs, definition
  lists, footnote defs, every inline span); **write-through + one in-place insert once the
  content is known** (heading underline width, figure caption lead-in); **deferred per-line
  transform** (blockquote/admonition/code-block re-indent, via a *pooled and reused* scratch
  buffer, so the pool costs `O(nesting depth)` allocations per document rather than one per
  construct). Tables still collect cells — column widths genuinely cannot be known before
  the last cell — but render straight into the shared buffer and borrow the collected cells
  for the width pass. Measured (release, best-of-30, same harness both sides, net of the
  harness's own event clone-and-drop baseline): allocations `3,560 → 425` @50 sections,
  `35,914 → 4,029` @500, `143,916 → 16,031` @2000 — ~9x fewer, and now **0.73x of
  `build()`'s**, so allocation count is no longer what separates the two paths.
- [x] **Residual wall-clock gap — profiled for real, 2026-07-29 (was asserted, not
  measured).** Used `perf record`/`perf report` (`nix-shell -p linuxPackages.perf`; not in
  the project flake) with `--call-graph fp` (dwarf unwinding produced corrupted stacks in
  this sandbox) against a temporary release build with `CARGO_PROFILE_RELEASE_STRIP=none`
  (the workspace `.cargo/config.toml` sets `strip = "symbols"` for release, which silently
  makes `perf report` useless otherwise — worth remembering for the next crate profiled).
  Re-measured ratio on this harness/machine: **~1.5-2.0x, not 2.6-2.7x** (different harness,
  construct mix and machine than the original figure; not a claim the original number was
  wrong, just what this investigation's own harness produces). Breakdown: ~29-36% of
  `Writer`-loop self-time in `Vec<Frame>::push`, ~18-22% in buffer-growth/reindent memmove,
  everything else (dispatch, `escape_text`, table border emission) under 3% each — i.e.
  `Vec<Frame>::push` (writing the `Frame` enum's bytes once per event that opens a
  construct) is the one real, measured hotspot, not "dispatch" broadly. **Attempted fix**
  (shrink `Frame`'s widest variants — `CloseDelim` enum replacing `Inline.close: &'static
  str`, dead `content` field removed from `Heading`/`Blockquote` since it's provably always
  `== mark`): `size_of::<Frame>()` measured 40 bytes **before and after every combination
  tried**, because `Table`/`TableRow`/`Link` are tied (or nearly tied) for the size
  ceiling, so shrinking any one variant doesn't lower the enum's total size — confirmed via
  a temporary `size_of` test, removed before commit. Two other attempted shrinks
  (`Link.url: String → Box<str>`; `Table.rows`/`TableRow.cells` boxed) were reverted:
  `into_boxed_str()` forces a shrink-realloc when `cap != len` (net allocation regression
  for zero measured benefit), and `Box::new(vec![])` allocates immediately even for an
  empty `Vec` (a real allocation-count regression, caught by the existing
  `test_writer_no_subtree_reconstruction_blowup` guard + a manual size check before commit,
  not by inspection). Kept: `CloseDelim` (a real representational improvement — 6-valued
  field no longer stored as a 16-byte fat pointer — regardless of the null perf result) and
  the dead-field removal (pure hygiene, not a behavior change). **Re-measured wall-clock
  after the kept changes: no change**, consistent with `size_of::<Frame>()` being
  byte-identical — a validated null result, not a manufactured improvement. **Conclusion,
  reported honestly: the residual gap is structural to the event-driven writer shape, not
  a contained rst-fmt bug** — a direct tree walk gets "what construct am I in" for free
  from the CPU call stack; an event-driven writer surviving across separate
  `write_event()` calls must reify that as an explicit `Vec<Frame>` stack with its own
  push/pop and discriminant tag, which a recursive walker never pays. Actually lowering
  `Frame`'s size floor would need `Link`/`Table` to stop storing their payload inline (e.g.
  index into a side arena) — real architectural work, correctly out of scope here, not
  attempted. Full writeup with the perf methodology (including the two sandbox-specific
  obstacles — stripped binary, dwarf-unwind corruption — and their fixes, useful for
  profiling any of the other ~25 crates next) at
  `/tmp/claude-1000/-home-me-git-rhizone-rescribe/7cb7fb03-d716-4725-8ce3-b1417c88f03e/scratchpad/rst-fmt-writer-profile.md`.
  **Recommend accepting this as the event-API tax** — the streaming writer's reason to
  exist is the `O(largest block + nesting depth)` memory bound, which it delivers, and the
  allocation-count discriminator (the thing that *was* fixable) is already fixed.
- [x] `test_writer_byte_identical_to_builder` added (commit `f87b3d62ef`): the streaming
  path must produce **byte-identical** output to `build()` across 18 construct-mix inputs.
  The pre-existing tests only compared re-parsed *block shapes*, which cannot catch
  formatting drift between the two independent emission paths.
- [x] **The missing backslash-escape handling turned out to be a real correctness bug, and
  is fixed — commit `1c430173f4`.** It was found while scoping the zero-copy work but is
  independent of it: the inline tokenizer had *no* backslash handling at all, so
  `\*not emphasis\*` — the RST spec's own example — parsed as live `Emphasis`. Silent
  misparse of valid input, no diagnostic. Reader now resolves escapes in the text scanner
  (escaped whitespace removed, per the `word\ *markup*` adjacency idiom);
  `find_closing`/`find_closing_char` refuse to close a span on an escaped delimiter and copy
  the escape *through* so it resolves exactly once, at the level that emits the text; inline
  literals pass `escapes: false` per the spec's exemption, and `:math:` content stays
  verbatim. Writer re-escapes `\`, `*`, `` ` `` on emit (a reader-only fix would break
  `parse(emit(parse(x))) == parse(x)`), and `collect_text_from_inlines` counts the escaped
  form so heading underline widths and table column widths match the emitted bytes.
  Fixtures `escaped-markup` + `escaped-whitespace`, and the two `COVERAGE.md` rows they
  close — escaping was enumerated nowhere in that checklist before.
- [x] **`events()` is now zero-copy where the format allows — DONE 2026-07-29.** Previously
  `Cow::Borrowed` fired for exactly 0 of 50,000 text events; the AST used owned `String`
  everywhere, so nothing downstream could borrow even in principle. Resolved by making the
  AST lifetime-generic (`RstDoc<'a>`/`Block<'a>`/`Inline<'a>`/`DefinitionItem<'a>`/
  `TableRow<'a>`/`Event<'a>` with `Cow<'a, str>` payloads) and rewriting the shared inline
  tokenizer to be byte-indexed over the input. The three blockers scoped here resolved as:
  1. **AST ownership** — fixed head-on by the breaking `<'a>` change (`rst-fmt` is
     unpublished, so the cost was paid now rather than later). Note the earlier inference
     that this "means `events()` must stop deriving from the tree" was **wrong**: once the
     tree can borrow, deriving events from it costs nothing extra, and the two paths keep
     sharing exactly one grammar.
  2. **Line-joining contiguity** — kept the joined-string representation (`join_cow` /
     `join_words` return `Cow::Borrowed` when exactly one source line survives). The
     per-line-spans alternative was rejected on correctness, not effort: RST inline markup
     may span a soft line break, so tokenizing per line changes what parses. Consequence:
     single-line paragraphs/headings/table cells/list items/line-block lines/definition
     terms borrow; genuinely multi-line ones are owned. Event-stream shape is unchanged, so
     `test_events_matches_parse_shape` and every consumer are unaffected.
  3. **Char-indexed tokenizer** — rewritten byte-indexed. `find_closing`/`find_closing_char`
     now return a byte offset rather than a rebuilt `String` (they already passed escapes
     through verbatim, so the span text was always exactly `content[start..end]` and the
     buffer was pure waste), and `merge_text_nodes`'s post-pass became merge-on-push that
     widens the borrowed slice for adjacent borrowed runs.

  **Measured** (synthetic construct-mix doc, release, temporary global-allocator harness,
  deleted after measuring; 200 and 2000 sections = 99KB / 1.0MB):

  | | allocs before | allocs after | MB/s before | MB/s after |
  |---|---|---|---|---|
  | `parse()` @200 | 25,826 | 8,925 | 109.4 | 144.2 |
  | `events()` @200 | 25,818 | 8,917 | 93.8 | 115.2 |
  | `parse()` @2000 | 252,633 | 88,132 | 108.9 | 137.0 |
  | `events()` @2000 | 252,621 | 88,120 | 96.6 | 118.0 |

  −65% allocations and +22-32% throughput on both paths, and **93.0% of emitted spans are
  `Cow::Borrowed`**. `parse()` and `events()` still have near-identical allocation counts —
  expected and correct: the remaining allocations are the `Vec<Inline>`/`Vec<Block>` nodes
  themselves, which both paths build, not per-span `String`s.

  The 7% that stay owned, by construct: the multi-line wrapped paragraph (one `Text` per
  section) and `CodeBlockContent` (the directive content collector keeps a trailing blank
  line inside the body, so the content is joined from two collected lines). Escape-bearing
  text runs and synthesised `:ref:`/`:doc:` URLs are the other two owned cases, absent from
  this corpus. Pinned by `test_events_are_zero_copy_where_the_format_allows` and
  `test_events_own_only_where_the_format_forces_it`, which assert on the `Cow` **variant**
  (a value-only comparison would pass unchanged against a fully-owned implementation).

  Side effects worth knowing: `Event::into_owned` is now an exhaustive match rather than an
  unsafe lifetime transmute; `Writer::write_event` takes `Event<'_>` instead of requiring
  `'static`; `BatchParser::finish` returns `RstDoc<'static>` via the new `into_owned()`;
  and `rescribe-write-rst` now builds an RST AST that *borrows from the `Document`* instead
  of copying every string out of it.

### `rst-fmt` — streaming Writer `Frame` shrunk via side-stack, `out` pre-reserved, benchmark-clone-artifact found (2026-07-29)

Follow-up to the profiling entry above (full numbers: `docs/format-audit.md`'s 2026-07-29
entry of the same name; methodology writeup:
`/tmp/claude-1000/-home-me-git-rhizone-rescribe/7cb7fb03-d716-4725-8ce3-b1417c88f03e/scratchpad/rst-fmt-writer-profile.md`).
Tried both avenues that entry left open.

- [x] **`Frame` shrunk 40→32 bytes, allocation-neutral, real ~6-10% wall-clock win.**
  `Table`/`TableRow`/`Link` (32-40 bytes each) were tied for `Frame`'s size ceiling.
  First tried boxing them together behind one `Frame::Wide(Box<WideFrame>)` — this DID
  shrink `size_of::<Frame>()` to 32, but wall-clock was measurably unchanged (two A/B'd
  builds, ~1300µs/iter either way at 2000 sections) while allocations on a table/link-heavy
  synthetic rose 4,518→6,518 (+44%, `Box::new` allocates eagerly per push where the old
  inline `Vec::new()` didn't). **Reverted** — a real, measured regression for zero benefit.
  Replaced with a side-stack design: `Frame::Wide` is now a zero-payload marker; the real
  `Table`/`TableRow`/`Link` payloads live on a new `Writer::side: Vec<WideFrame>`. No index
  needs to be stored in `Frame::Wide` — events are well-nested, so the subsequence of
  wide-frame opens/closes is itself a valid push/pop order, and `side.pop()` always returns
  the payload matching whichever `Frame::Wide` is on top of the main stack. `side` grows via
  ordinary amortized `Vec::push`, so no per-push allocation is added. Measured:
  `size_of::<Frame>() = 32` (was 40), allocation count on the table/link-heavy synthetic
  4,519 (baseline 4,518 — no regression), isolated writer-only wall-clock (git-stash A/B,
  4 trials each, 4000 sections) dropped from a ~2,571-2,917µs/iter baseline to
  ~2,373-2,735µs/iter (**~6-10%** across repeated trials under varying machine noise —
  the ratio, not the absolute numbers, is what's stable). `perf` confirms the mechanism:
  `Vec<Frame>::push`'s share of self-time fell from ~29-36% (prior entry) to ~12%.
- [x] **`Writer::out` pre-reserved — small additional win, also allocation-neutral-to-fewer.**
  Added `Writer::with_capacity(sink, out_capacity)` and a `DEFAULT_OUT_CAPACITY = 4096`
  used by `Writer::new` (previously `String::new()`, matching `BuildContext::output`, which
  also doesn't pre-reserve). ~1-3% further wall-clock reduction on top of the side-stack
  change, and fewer allocations on the table-heavy synthetic (4,515 vs 4,519) since fewer of
  `out`'s early doublings are needed.
- [ ] **Tried and rejected**: pre-sizing the *sink* `Vec<u8>` (not `out`) to `input.len()`
  made the writer-only loop ~20% **slower**, not faster — a single large upfront allocation
  plausibly crosses into a different allocator path (mmap + lazy page faults) that a series
  of smaller incremental reallocs avoids. Not pursued; not part of what was kept.
- [x] **Benchmark-methodology finding, reported because it changes how every absolute
  ratio in this file and `docs/format-audit.md` should be read.** Profiling the final
  (side-stack + pre-reserve) build showed `<Event as Clone>::clone` — the harness's own
  per-iteration clone of pre-materialized events, used to replay one parsed stream across
  many timed iterations in every `Writer`-vs-`build()` measurement to date — consuming
  **~40% of measured "Writer" self-time**. Isolated directly (a `clone_only` harness mode
  cloning just the event `Vec`): ~1,095-1,135µs/iter out of a ~2,664-2,710µs/iter total at
  4000 sections. This clone never happens in real `Writer` usage (a live event stream is
  consumed once). It was present uniformly across every configuration A/B'd here, so the
  *relative* improvements above are unaffected, but it inflates every *absolute* ratio
  reported in both files. Subtracting it from the raw measurements gives an adjusted ratio
  of roughly **0.90-1.39x** at 500/2000/4000 sections (occasionally faster than `build()`)
  instead of the previously reported 1.5-2.6x — noisy, not a precise number, but strong
  enough to say the real zero-clone production gap is materially smaller than every prior
  absolute figure suggested. **Not fixed here** (the harness was temporary and deleted, not
  committed, per convention) — flagged for whoever next benchmarks a streaming writer
  against a tree builder with a similar "clone pre-materialized events per iteration"
  harness shape: measure the clone cost in isolation and subtract it, or avoid cloning
  altogether, before trusting the ratio.
- [x] All tests (including `test_writer_byte_identical_to_builder`,
  `test_writer_no_subtree_reconstruction_blowup`, the events()≡parse() equivalence test,
  and the chunked `StreamingParser` tests) still pass; `cargo clippy --all-targets
  --all-features -D warnings` and `cargo fmt --check` clean. Stage numbers unchanged
  (R:4/W:4) — this is a perf-only pass, no construct or API-mode change.

### Workspace-wide benchmark-artifact sweep — 2026-07-29

Follow-up to the clone-artifact finding above: does the same defect class (setup work
inside the timed region, unequal setup cost between two compared paths, missing
`black_box`) appear in any *committed* benchmark, and does any recorded conclusion rest
on one? The rst-fmt harness that produced the 1.5–2.6x figures was a temporary, deleted
`examples/perf_writer.rs`, never committed — so nothing in the repo needed fixing for
that specific number; TODO.md/`docs/format-audit.md` already carry the honest
correction inline (adjusted ratio ~0.90–1.39x), by design not rewritten to a clean
retraction since the surrounding entries are a legitimate investigation log.

- [x] **Inventoried every benchmark in the workspace**: 24 `criterion` `benches/*.rs`
  files (one per `crates/formats/*` crate, plus `rescribe-read-djot` and
  `ooxml-sml/benches/parse_benchmark.rs`), one non-criterion timed-throughput example
  (`ooxml-sml/examples/bench_throughput.rs`), one corpus-analysis tool with wall-clock
  totals (`crates/tools/ooxml-corpus/src/main.rs`), and one test-with-a-timing-guard
  (`bbcode-fmt`'s `test_timeout_case`, a 100ms panic-avoidance smoke test, not a
  performance benchmark). The quick-xml-vs-`xml-rs`-vs-`roxmltree` numbers cited in
  `docs/adr/0007-dtd-entity-resolution-build-vs-buy.md` (884–1088 MB/s vs 46–51 MB/s,
  19–21x) come from an out-of-repo, one-off benchmark with no harness committed here —
  not auditable directly, but cross-checked against quick-xml's own published README
  benchmark (~50x on its own input, same order of magnitude) and it's a same-input
  cross-*library* comparison rather than a shared-harness A/B replaying a cloned event
  stream, so the specific clone-in-loop defect class doesn't apply to its shape. Left
  as uncertain-but-low-risk, not re-measured (no harness to fix).
- [x] **Found one real instance of the same defect, committed**:
  `crates/formats/ansi-fmt/benches/ansi_bench.rs`'s `bench_writer` cloned each
  pre-materialized `OwnedEvent` (`Event<'static>`, `Cow::Owned` string payloads on
  `Text`/`Hyperlink`) once per timed iteration to replay one parsed stream — same shape
  as the rst-fmt harness, actually worse (an owned `Cow` clone allocates and copies,
  where rst-fmt's borrowed-`Cow` clone was allocation-free). **Fixed**: switched to
  `criterion::Bencher::iter_batched` — the per-iteration `evs.clone()` now happens in
  the untimed setup closure, and the routine consumes the batch's owned copy directly
  (no clone in the timed region at all). Re-measured (release,
  `CARGO_PROFILE_RELEASE_STRIP=none`, same machine): `ansi_writer` **27.4µs → 28.7µs**
  (within criterion's own noise threshold, not a real change) — at this bench's scale
  (~264 events, mostly short strings like `"Color text "`), the clone's absolute cost
  turned out to be below the noise floor, unlike rst-fmt's ~50,000-event/iteration
  corpus where it was ~40% of measured time. **No recorded conclusion anywhere in
  TODO.md/`docs/format-audit.md` cited `ansi_writer`'s numbers** (grepped for
  `ansi_bench`/`ansi_writer`/`ansi.fmt.*writer` — the only hits are unrelated
  "streaming-writer audit: clean" checklist entries, not throughput claims), so there
  is nothing to retract — this was a hygiene fix, not a record correction. Still fixed,
  since the defect would resurface at a larger corpus size and the fix is free.
- [x] **Two benches (`bbcode-fmt`, `xwiki`) had zero `black_box` calls** — every other
  bench file in the workspace wraps routine inputs (`black_box(SMALL)`,
  `black_box(&doc)`, etc.); these two passed bare `&'static str` constants and
  once-built `Doc` values straight into `parse`/`build`/`events`, the "missing
  `black_box`, dead-code-elimination risk" class named as a concern. Checked whether
  this was live: re-measured both crates' `parse_small`/`parse_medium` before touching
  them — non-zero, input-size-scaled numbers (bbcode: 2.7µs/15.5µs; xwiki:
  1.3µs/10.6µs), not the near-zero result loop-invariant hoisting would produce across
  an opaque cross-crate function call, so no live defect, but no recorded conclusion
  depended on it either way. **Fixed for defense-in-depth and workspace consistency**:
  added `black_box` around every routine input in both files, matching the other 22.
  Re-measured after: bbcode `parse_small` 2.74µs (unchanged, "no change in performance
  detected"), `parse_medium` 15.24µs (-2%, within measurement noise) — confirms the fix
  was precautionary, not a correction.
- [x] **Guard added**: `docs/format-library-design.md` gained a "Benchmarking
  convention" section (after "Fuzz harness") naming the three failure shapes (setup
  inside the timed closure, unequal setup cost across an A/B comparison, missing
  `black_box`) with the rst-fmt/ansi-fmt cases as worked examples, plus the
  `iter_batched` fix pattern for routines that must consume owned input.
- [x] All three touched crates (`ansi-fmt`, `bbcode-fmt`, `xwiki`): `cargo clippy
  --all-targets --all-features -D warnings`, `cargo test -q`, `cargo fmt --check` all
  clean.
- [ ] **Not done**: no attempt to re-verify the out-of-repo quick-xml/xml-rs/roxmltree
  numbers in ADR 0007 with an in-repo harness — flagged as uncertain above, not fixed,
  since there is no committed benchmark to correct and reproducing it was out of scope
  for this pass.

### DEBT: Fake-streaming writer/reader audit across all `crates/formats/` — identified 2026-07-28

The rst-fmt writer bug above (buffer-all → reconstruct AST via frame stack → delegate
to the builder's own `build_block`) turned out not to be isolated. Full sweep of all
43 format crates for the same shape, on both the streaming writer and the reader side
(`StreamingParser`, `events()`). Method and full per-crate detail:
`docs/format-audit.md` § "Streaming reader/writer fidelity inventory" — not repeated
here. Headlines only:

- **26 of 43 crates (60%) have a hollow streaming writer** — identical shape to the
  rst-fmt bug: `bbcode-fmt`, `creole`, `dokuwiki`, `asciidoc`, `djot-fmt`,
  `commonmark-fmt`, `fountain-fmt`, `haddock-fmt`, `jira-fmt`, `man-fmt`, `markua`,
  `mediawiki-fmt`, `muse-fmt`, `pod-fmt`, `org-fmt`, `ooxml-pml`, `ooxml-wml`,
  `odf-fmt`, `textile-fmt`, `texinfo`, `tikiwiki`, `twiki`, `vimwiki-fmt`, `xwiki`,
  `zimwiki`, `t2t`.
- **Highest-consequence single finding: `ooxml-wml` (DOCX)** — CLAUDE.md names OOXML
  as the priority target for the full three-API architecture (DOCX/XLSX/PPTX exceed
  RAM on large corpora), yet its writer is hollow and its `StreamingParser` also
  buffers the whole input. `ooxml-pml`'s hollow writer additionally drops shape
  geometry (lossy, not just slow). `ooxml-sml` is the counter-example proving a
  genuinely incremental writer is achievable for the same zip/OPC container shape.
- **`StreamingParser` buffers O(full input)** (contract violation) in `bbcode-fmt`,
  `creole`, `dokuwiki`, `muse-fmt`, `texinfo`, `textile-fmt`, `xwiki`, `ooxml-wml`,
  `pod-fmt`, `fb2-fmt` (this last one documented but with no CLAUDE.md exemption,
  and `docbook-fmt` proves bounded XML streaming is achievable with the same
  `quick_xml` library). `html-fmt` also buffers-all but has a structurally honest,
  documented justification (HTML5 tree construction can rearrange already-seen
  nodes) so it is not counted as a violation.
- **`events()` derived from `parse()`+walk** (ADR 0003 violation) in 14 crates:
  `bbcode-fmt`, `dokuwiki`, `tikiwiki`, `creole`, `fountain-fmt`, `haddock-fmt`,
  `jira-fmt`, `man-fmt`, `markua` (doc comment claims independence, code
  contradicts it), `mediawiki-fmt`, `muse-fmt`, `pod-fmt` (dead `unreachable!()`
  stub), `odf-fmt`, `texinfo`, `textile-fmt`, `t2t`. **This architectural gap is
  still open for all 14** — none of them has a genuine standalone incremental
  parser at the free-function `events(input)` entry point; see the dedicated
  memory-safety fix below for the subset where this also caused a real
  soundness bug or a leak (now fixed, but the derived-from-`parse()` shape
  itself is unchanged).
- **No feature-declared-but-module-missing cases found** — every declared streaming
  feature has some code behind it everywhere. But real `#[cfg(feature = ...)]`
  gating of `mod events`/`batch`/`writer` exists only in `commonmark-fmt` and
  `creole`; every other crate compiles all three modules unconditionally regardless
  of the Cargo.toml flags.
- **Honest gaps, not violations**: `csv-fmt`, `ris`, `tsv-fmt`, `native` have no
  streaming API at all and claim none.
- **Clean**: `docbook-fmt`, `jats-fmt`, `tei-fmt`, `ansi-fmt`, `ooxml-sml`, `rst-fmt`
  (per the fix above), `html-fmt` (one documented exception).
- This is an audit only — nothing was fixed in this pass. Given "work one vertical to
  completion" (CLAUDE.md), the next streaming-fix vertical should be chosen from this
  list rather than swept horizontally; `ooxml-wml` is highest-priority by consequence,
  the wiki/small-format Tier-1 group is highest-count by breadth.

#### FIXED (2026-07-29): the memory-safety/leak subset of the `events()` audit

Of the 14 `events()`-derived-from-`parse()` crates above, 4 were flagged as
memory-safety or leak issues (not just architecture/perf) and were fixed in a
follow-up pass — each was first independently re-verified against the actual
code before changing anything, since the original audit's mechanism claims
were not all accurate as stated:

- **`dokuwiki`** — **confirmed genuinely unsound.**
  `events::InputEventIter::new` took `&doc` where `doc` was a local `DokuwikiDoc`,
  built an `EventIter<'_>` borrowing it, `transmute`d that iterator to
  `EventIter<'static>`, and *then* moved `doc` into the returned struct — so the
  transmuted reference could be left dangling relative to the doc's post-move
  location. Fixed (Option B — sound but still derived): `InputEventIter::new`
  now walks the doc with the already-sound `EventIter<'a>` (borrows a real
  `&'a DokuwikiDoc`, O(depth), unaffected) and eagerly collects into an owned
  `Vec<OwnedEvent>` before returning, all within one function scope — no
  self-reference, no `unsafe`. `crates/formats/dokuwiki/src/events.rs`.
- **`man-fmt`** — **confirmed genuine leak, not UB.** `events::events()` used
  `Box::leak(Box::new(doc))` to manufacture a `'static` reference for
  `EventIter::new`, permanently leaking one `ManDoc` (and its scratch Vec) on
  every call — safe Rust, but a real unbounded resource leak. Fixed the same
  way as dokuwiki: eager collection into an owned `Vec` inside the function so
  `doc` drops normally. `crates/formats/man-fmt/src/events.rs`.
- **`bbcode-fmt`** — **audit's characterization was wrong for `events()` itself.**
  `events()` has no `unsafe` and is not self-referential — every event it
  builds is already `Cow::Owned`, so the `'a` on the returned `EventIter<'a>`
  was vacuous (nothing actually borrows the input). The real (and only)
  `unsafe` was in `Event::into_owned()`'s catch-all arm, transmuting
  `Event<'a>` to `Event<'static>` for variants known (by elimination) to hold
  no `'a` data — currently sound given the current variant set, but a latent
  hazard: a future `Cow`-bearing variant added without updating that arm would
  silently mis-convert instead of failing to compile. Replaced the catch-all
  with an exhaustive explicit match. `crates/formats/bbcode-fmt/src/events.rs`.
- **`tikiwiki`** — **checked and found sound, but pointless.**
  `EventIter::new` used `unsafe { transmute::<Event<'static>, Event<'a>> }` —
  widening a `'static` lifetime to any `'a` cannot dangle, so this was never
  UB, just unnecessary lifetime laundering (every pushed event already owns
  its data). Removed by making `emit_block`/`emit_inlines` generic over the
  output lifetime and building `Vec<Event<'a>>` directly.
  `crates/formats/tikiwiki/src/events.rs`.

All four crates now have `#![deny(unsafe_code)]` at the crate root (man-fmt's
test-only `GlobalAlloc` harness carries a narrowly-scoped
`#[allow(unsafe_code)]`), and each has a regression test:
`man-fmt::events::tests::test_events_no_per_call_leak` is an allocation-growth
guard (modeled on rst-fmt's `test_writer_no_subtree_reconstruction_blowup`)
verified to fail against the reintroduced `Box::leak` code (662KB net growth
over 200 calls vs. a 50KB bound); `dokuwiki`/`tikiwiki` have iterator-churn
tests exercising many short-lived borrowing iterators dropped out of creation
order; `bbcode-fmt` has an exhaustive `into_owned()` round-trip test across
every event family. `cargo miri` was not available in the dev shell (no
`rustup`, `cargo miri` not otherwise installed) — not run; soundness was
verified by code inspection and reliance on the borrow checker instead.

**Still open for all four** (unchanged by this fix, tracked above): the
`events(input: &str)` free function in each crate is still a parse-then-walk
convenience wrapper, not a genuine streaming parser — ADR 0003 non-compliant.
The already-sound `EventIter::new(&doc)` two-step path (parse yourself, then
walk) is the closest thing to real O(depth) streaming available in
`dokuwiki`/`man-fmt`/`tikiwiki`/`bbcode-fmt` today; a true single-call
streaming rewrite (Option A, rst-fmt-style) was judged out of scope for this
memory-safety pass per "scope discipline: fix minimally, fence the rest."

### `org-fmt` — API modes complete (2026-03-23)

- [x] `stream`: pull iterator (events())
- [x] `batch`: BatchParser + BatchSink
- [x] `batch`: StreamingParser<H: Handler> + Handler trait (2026-03-25)
- [x] `w-stream`: Writer<W: Write> streaming writer
- [x] Feature flags added
- [x] Fix events() — now a true pull iterator (2026-03-24)
- [x] events() frame-stack fix — O(nesting depth), not O(block subtree) (2026-03-28)
- [x] parse() direct recursive descent — independent of events() (2026-03-28)
- [x] StreamingParser<H> Tier 2 — O(largest block) streaming (2026-03-28)
- [ ] Parser/writer gaps: blockquote nesting, footnote definitions, figure/caption blocks

### `asciidoc` — API modes complete (2026-03-23)

- [x] `stream`: pull iterator (events())
- [x] `batch`: BatchParser + BatchSink
- [x] `batch`: StreamingParser<H: Handler> + Handler trait (2026-03-25)
- [x] `w-stream`: Writer<W: Write> streaming writer
- [x] Feature flags added
- [x] Fix events() — now a true pull iterator (2026-03-24)
- [x] events() frame-stack fix — O(nesting depth), not O(block subtree) (2026-03-28)
- [x] parse() direct recursive descent — independent of events() (2026-03-28)
- [x] StreamingParser<H> Tier 2 — O(largest block) streaming (2026-03-28)
- [ ] Parser gaps: table parsing, footnote parsing, math parsing
- [ ] Markdown family (pulldown-cmark backed; adapter hardening + fuzz)
- [x] HTML (html5ever backed) — **5-Production** (R:5†/W:5†; 85/85 COVERAGE.md items, 2026-07-26)
  - [x] `html-fmt` crate created (2026-04-11): standalone HTML5 AST, parse (html5ever RcDom), events (AST walk), batch (StreamingParser/BatchParser), emit (with pretty-print), streaming writer. `rescribe-read-html` and `rescribe-write-html` rewired as thin adapters over `html_fmt::HtmlDoc`. Note: HTML5 tree construction algorithm requires full tree for correctness (foster parenting, adoption agency), so `events()` and `StreamingParser` build the tree internally — this is a spec limitation, not a library choice, documented in `batch.rs`.
  - [x] Footnote anchor convention (2026-07-26): the reader had **no** footnote recognition at all before this (write-only, unverified). Now recognizes `<sup class="footnote-ref"><a href="#fn-{label}">…</a></sup>` and `<div id="fn-{label}" class="footnote"><sup class="footnote-label">…</sup><span class="footnote-content">…</span><a class="footnote-back">…</a></div>` and reconstructs `footnote_ref`/`footnote_def`. Marker/backlink are regenerated deterministically from the label on write (not read back), so the round-trip only needs the content span to survive — lossless without depending on fragile whitespace/id matching. Fixture: `fixtures/html/footnote/`.
  - [x] Inline MathML (2026-07-26): added `html_fmt::emit_fragment`/`emit_fragment_with_options` (general-purpose subtree serializer in `html-fmt`, usable by any consumer — not adapter-only) plus reader support for `<math>…</math>`. Full structural modeling into `math:*` node kinds was judged out of scope (MathML has its own large presentation/content vocabulary); per CLAUDE.md's raw-preservation pattern it's captured verbatim as `math_inline`/`math_display` with `math:format="mathml"` + `math:source` holding the exact MathML markup (`display="block"` → math_display). Writer now branches on `math:format`: MathML round-trips byte-for-byte via `Raw`, LaTeX `math:source` keeps the existing `\(…\)`/`\[…\]` convention. Fixture: `fixtures/html/inline-math-mathml/`.
  - [x] Megabyte pathological fixture (2026-07-26): `fixtures/html/path-large-inline-text/` — single `<p>` with a ~4.9MB text node.
  - 8 new/updated unit tests added across `rescribe-read-html`/`rescribe-write-html` covering footnote and MathML round-trips; `cargo clippy --all-targets --all-features -p html-fmt -p rescribe-read-html -p rescribe-write-html -- -D warnings` and full test suite both clean.
- [ ] DOCX, PPTX, XLSX (ooxml-* backed; same) — DOCX reader at 5-Production (2026-03-03); others at 4-Fuzz; gaps below

  **DOCX reader** (closest to production):
  - [x] Endnote content — `doc.get_endnotes()` pre-loaded; `footnote_ref` nodes with `label:"en{id}"` prefix
  - [x] Para-props raw preservation — `docx:space-before`, `docx:space-after`, `docx:line-spacing`, `docx:indent-left/right/first-line/hanging` props
  - [x] List ordering — numbering definitions consulted via `ParagraphExt::num_fmt()`; `ordered: true` for decimal
  - [x] Audit `_ => {}` at line 370 — `MoveFrom`/`MoveTo`/`SubDoc` now emit fidelity warnings
  - [x] Fixtures: all 22 fixtures have expected.json (image, hyperlink, small_caps, all_caps, hidden, highlight, ordered lists, table_header, endnote, para_spacing, para_indent)
  - [x] Roundtrip fuzz target (`fuzz_docx_roundtrip`) — 441K runs clean (2026-03-03)
  - [x] No-panic fuzz gate (`fuzz_docx_reader`) — 5.7M runs clean (2026-03-03)
  - [x] **5-Production** — all gates passed (2026-03-03)

  **DOCX writer**:
  - [x] Image embedding (resource:xxx → embedded DOCX media via pre-registration + CTDrawing clone)
  - [x] Footnote writing (`footnote_ref` → endnote API)
  - [x] Hyperlink writing (`link` URL → rel-registered hyperlink)
  - [x] Metadata writing (`doc.metadata` → `set_core_properties()`)
  - [x] Roundtrip fuzz target — clean

  **DOCX streaming writer** (`WmlWriter<W>`):
  - [x] Image support — `register_image(rel_id, data, content_type)` on `WmlWriter`;
        maps caller rel_ids to builder-assigned rel_ids; `Image { rel_id }` event
        embeds via `DocumentBuilder::add_image` + `Drawing` → `RunContent::Drawing`
  - [ ] Footnote/endnote support — add `register_footnote(id, Vec<OwnedWmlEvent>)` /
        `register_endnote(id, Vec<OwnedWmlEvent>)`; process via same stack machine into
        `FootnoteEndnote` bodies; wire `FootnoteRef`/`EndnoteRef` events to registered bodies

  **XLSX streaming writer** (`SmlWriter<W>`):
  - [x] Shared-string resolution — `set_shared_strings(Vec<String>)` on `SmlWriter`;
        `CellType::SharedString` cells now index into the table instead of emitting
        the raw index as a number

  **PPTX streaming writer** (`PmlWriter<W>`):
  - [x] Multi-slide support — `new_slide()` method records a slide-boundary position;
        `process_pml_events` slices the event buffer per slide and calls `process_slide`
        once each; no `new_slide()` call = single-slide (original behaviour preserved)
  - [x] Table content — `StartTableCell`/`EndTableCell` treated as paragraph boundaries;
        text inside cells collected into current shape's paragraph list
  - [ ] Shape geometry — **design decision required**: add EMU position/size fields to
        `StartShape` in `PmlEvent` (requires YAML + codegen regen); until then, round-trip
        fidelity for shape layout is impossible

  **XLSX reader**:
  - [x] Cell formatting fidelity warning — cells with style_index > 0 emit warning (2026-03-03)
  - [x] Charts fidelity warning — embedded charts per sheet emit warning (2026-03-03)
  - [x] Named ranges fidelity warning — workbook defined_names emit warning (2026-03-03)
  - [x] Formula fixture (xlsx/formula) — xlsx:formula property preserved (2026-03-03)
  - [x] Roundtrip fuzz target (fuzz_xlsx_roundtrip) — 157K runs clean (2026-03-03)
  - [ ] Metadata extraction (TODO stub in code — ooxml-sml doesn't expose core properties)
  - [ ] More fixtures (formatted cells, etc.)

  **PPTX reader**:
  - [x] Bullet/list detection warning — paragraphs with level() > 0 emit fidelity warning (2026-03-03)
  - [x] Speaker notes plain-text warning — notes div emitted with warning about lost rich text (2026-03-03)
  - [x] Charts/SmartArt fidelity warnings — per-slide warnings when chart_rel_ids/smartart_rel_ids non-empty (2026-03-03)
  - [x] Notes fixture (pptx/notes) — speaker notes div structure (2026-03-03)
  - [x] Fix Cargo.toml: workspace deps (was path deps) (2026-03-03)
  - [x] Bullet/list structure in IR — consecutive bullet paragraphs grouped into list/list_item nodes (2026-03-20)
  - [ ] Nested bullet levels (currently flattened to single level with fidelity warning)
  - [ ] Roundtrip fuzz target (requires PPTX writer capable of roundtrip)
- [x] EPUB — 3-Harness (30 fixtures, fuzz target compiles, 2026-03-28)
- [ ] ODT writer (no library; treat as a vertical)
- [ ] AZW3 reader/writer (boko as reference, MIT attribution)
- [ ] PDF reader (pdf-extract backed; already at 4)

### ooxml-fmt rework (major milestone — after five-crate streaming upgrade)

The ooxml-* crates are our biggest value proposition: no other Rust ecosystem library
handles DOCX/XLSX/PPTX at production quality. The rework consolidates them and adds
the full three-API streaming architecture from `docs/format-library-design.md`.

**Why streaming is non-optional for OOXML:**
DOCX/XLSX/PPTX files in legal discovery, academic corpora, and enterprise search
routinely exceed available RAM. A library that requires the full file in memory before
parsing starts is unusable for these workloads. `StreamingParser<H>` with O(nesting
depth + largest token) memory is the primary use case, not an afterthought.

**Architecture targets:**
- OPC layer: chunked ZIP entry streaming — decompress one entry at a time, never the
  full archive. The ZIP central directory is parsed first (it's at the end of the file,
  so this requires two passes or a seekable source); entries are decompressed on demand.
- XML layer: SAX-style events from `quick-xml` fed directly to the format state machine.
  No intermediate DOM allocation.
- Format layer (`wml`, `sml`, `pml`): `StreamingParser<H>` translates XML events to
  format-level events. The handler receives `Event::StartParagraph`, `Event::Text(cow)`,
  etc. — no intermediate `Block` allocation.
- `parse()`: direct tree construction from the SAX stream. No events() indirection.
- `events()`: format-level pull iterator over a fully-loaded `&[u8]`. Wraps the same
  state machine as `StreamingParser` but driven by `Iterator::next()`.

**Consolidation:**
- [ ] Merge `ooxml-wml`, `ooxml-sml`, `ooxml-pml`, `ooxml-dml`, `ooxml-omml`,
  `ooxml-opc`, `ooxml-xml` into a single `ooxml-fmt` crate with feature flags.
  Shared infrastructure (`opc`, `xml`) always compiled; `wml`/`sml`/`pml`/`dml`/`omml`
  feature-gated. `crates/tools/ooxml-codegen` stays separate (build tool).
- [ ] Implement `StreamingParser<H>` for DOCX (wml) first — largest user base.
- [ ] Implement `StreamingParser<H>` for XLSX (sml) — critical for data pipelines.
- [ ] Implement `StreamingParser<H>` for PPTX (pml).
- [ ] `parse()` as direct recursive descent (independent of events()).
- [ ] `events()` as true pull iterator (frame-stack, no block-granular buffering).
- [ ] Publish `ooxml-fmt` to crates.io.
- [ ] Deprecate individual crates — final version with deprecation notice pointing to
  `ooxml-fmt`. Keep compiling; mark `#[deprecated]` on the re-exported API surface.

### Milestone: M2.5 — Streaming IR layer

End-to-end streaming conversion with O(nesting depth + largest token) memory.
Never materializes the full document. Required for large-document workloads.
See CLAUDE.md "Streaming IR" section for architecture and rationale.

**Prerequisite:** All five hand-rolled crates at true Tier 2 `StreamingParser`
(see DEBT section above). ooxml-fmt rework also required before OOXML can stream.

**rescribe-core additions:**
- [ ] `IrEvent<'a>` — format-agnostic SAX-style open/close event type, mirroring
  rescribe-std node kinds (StartParagraph/EndParagraph, StartHeading{level}/EndHeading, Text(Cow), etc.)
- [ ] `IrHandler` trait — `fn handle(&mut self, event: IrEvent<'_>)`
- [ ] `StreamingReader` trait — `feed(&mut self, chunk: &[u8])` + `finish(self)`
  where the impl drives a format `StreamingParser` and translates format events to `IrEvent`
- [ ] `StreamingWriter` trait — `handle(&mut self, event: IrEvent<'_>)` + `finish(self) -> Vec<u8>`
- [ ] `IrTransformer` — `IrHandler` wrapper that transforms events and forwards to inner `IrHandler`
- [ ] `DocumentBuilderHandler` — `IrHandler` impl that assembles a `Document` (materialized path)

**Format adapter additions (one per format):**
- [ ] Each `rescribe-read-{fmt}` gains a `StreamingReader` impl that wraps the format
  library's `StreamingParser` and translates format events → `IrEvent`
- [ ] Each `rescribe-write-{fmt}` gains a `StreamingWriter` impl

**Pipeline:**
```
feed(chunk) → StreamingReader → IrEvent → IrTransformer → IrEvent → StreamingWriter → output chunk
```

---

### Milestone: M3 — Tier B/C verticals

Tier B formats at 3-Harness or 2-Fixtures (where harness is N/A), each with a
standalone library where the ecosystem gap justifies it.

- [ ] `t2t` vertical — **4-Fuzz** → needs re-fuzz after expansion (2026-03-29)
  - [x] Split monolith lib.rs into ast.rs / parse.rs / emit.rs
  - [x] Span on every AST node; Diagnostic type; strip_spans()
  - [x] parse() infallible → (T2tDoc, Vec<Diagnostic>)
  - [x] No-panic fuzz gate (`fuzz_t2t_reader`) — 2M runs clean; needs re-run
  - [x] Roundtrip fuzz target (`fuzz_t2t_roundtrip`) — 939K runs clean; needs re-run
  - [x] New constructs: DefinitionList block; Verbatim/Tagged inlines; document header metadata (2026-03-29)
  - [x] All API modes: ast + stream + batch + w-build + w-stream (2026-03-29)
  - [x] Oracle harness + benchmarks (2026-03-29)
  - [x] Fixtures: COVERAGE.md all boxes checked (2026-03-29)
  - [ ] Re-run fuzz clean after construct expansion
- [ ] `markua` vertical — **4-Fuzz** → needs re-fuzz after expansion (2026-03-29)
  - [x] No-panic fuzz gate + roundtrip fuzz — clean on original constructs; needs re-run
  - [x] New constructs: DefinitionList, PageBreak, Figure blocks; SpecialBlock reworked to hold Vec<Block>; Subscript/Superscript/Underline/SmallCaps/FootnoteRef/IndexTerm/MathInline inlines (2026-03-29)
  - [x] All API modes: ast + stream + batch + w-build + w-stream (2026-03-29)
  - [x] Benchmarks: markua_parse_small, markua_parse_medium, markua_emit_medium (2026-03-29)
  - [x] Fixtures: COVERAGE.md all boxes checked (2026-03-29)
  - [ ] Re-run fuzz clean after construct expansion
- [ ] MOBI reader (boko as reference)
- [ ] KFX reader/writer (Ion spec + boko)
- [ ] Remaining Tier B/C formats: audit and bring to target stage

---

## Someday/Maybe

Low priority; add if there's demand.

- [ ] Marp (CommonMark + slide separators + speaker-note comments; ~50 lines on top of GFM reader; write support is Beamer/revealjs-style)
- [ ] Gemtext (Gemini protocol markup)
- [ ] Mermaid (diagram markup)
- [ ] PlantUML (UML diagrams)
- [ ] GraphViz DOT (graph descriptions)
- [ ] PHP Markdown Extra
- [ ] Setext (original lightweight markup)
- [ ] troff/nroff variants beyond man
- [ ] DITA (technical documentation)
- [ ] Confluence wiki markup
- [ ] Notion export format
- [ ] Roam Research export
- [ ] Logseq export

### Update CLAUDE.md — corrections as documentation lag (2026-03-29)

Add to the corrections section:
> **Corrections are documentation lag, not model failure.** When the same mistake recurs, the fix is writing the invariant down — not repeating the correction. Every correction that doesn't produce a CLAUDE.md edit will happen again. Exception: during active design, corrections are the work itself — don't prematurely document a design that hasn't settled yet.

Add to the Session Handoff section:
> **Initiate a handoff after a significant mid-session correction.** When a correction happens after substantial wrong-path work, the wrong reasoning is still in context and keeps pulling. Writing down the invariant and starting fresh beats continuing with poisoned context — the next session loads the invariant from turn 1 before any wrong reasoning exists.

Conventional commit: `docs: add corrections-as-documentation-lag + context-poisoning handoff rule`

---

## Ad-hoc dispatch findings (2026-05-29)

From an ecosystem-wide investigation of ad-hoc dispatch architecture (2026-05-29). The recurring anti-pattern: N parallel dispatch tables keyed on a closed name/enum set where one registry/trait/visitor belongs — strongest tell is DRIFT (parallel tables disagreeing). Each finding names the general mechanism it should have been.

- **R1 — 3 parallel format-match arms bypass the `Parser`/`Emitter` traits.** `rescribe-cli/src/main.rs`: `parse_text` (line ~805), `parse_binary` (line ~874), `emit` (line ~900) each manually enumerate every format and call format-specific free functions; plus a 60-entry `const FORMATS` (lines ~80–676). The library's `Parser`/`Emitter` traits expose `fn formats(&self)` — the exact dispatch mechanism — but the CLI ignores it. Adding a format = 4-place edit, compiler can't enforce consistency. SHOULD BE: registry dispatch via `Parser::formats()`/`Emitter`. This is the cleanest bypassed-abstraction finding in the conversion cluster.

## JATS citation/bibliography IR vertical closed (2026-07-28)

Following DocBook's citation vertical (`8aedfb80fa`), the JATS citation/reference-list
design fork noted above (and in `fixtures/jats/COVERAGE.md`) is resolved: `<ref-list>` ->
`bibliography`, `<ref>` -> `bibliography_entry`, `<element-citation>`/`<mixed-citation>`
fields -> `bibliography_field` children, using the same node kinds added in `4e15c996`.
`jats-fmt` itself needed no changes (its AST is generic XML, like docbook-fmt's) — all the
work is in `rescribe-read-jats`/`rescribe-write-jats`.

One correction to the original task framing worth recording: the date-handling
instructions referenced `<pub-date>`'s `year`/`month`/`day` children, but the JATS 1.3 Tag
Library (fetched live, not from memory) confirms `<element-citation>`'s content model has
no `<pub-date>` child at all — the actual date-bearing elements are bare
`<year>`/`<month>`/`<day>` and/or a `<date>` wrapper, both optionally carrying an
`iso-8601-date` attribute (per the Tag Library's own tagged examples, e.g. `<year
iso-8601-date="2001-11">2001</year>`). Implemented against the schema-verified elements
instead; the attribute-preferred-over-reconstruction design intent was unaffected.

Fixtures: `fixtures/jats/citation-{simple-author,multi-author,markup-in-field,
mixed-citation,date}`, `fixtures/jats/path-many-references`. COVERAGE.md's back-matter/
integration/pathological reference-list boxes are now all checked; the two remaining
open boxes (MathML `<math>` as an alternative to `<tex-math>`, and `<alternatives>`'s
block-vs-inline non-classification) are unrelated pre-existing design forks, untouched.

Also extended `crates/rescribe-fixtures`' `check_prop_in` (and `fixtures/spec.md`) to match
JSON objects against `PropValue::Map` — needed to assert `prop::DATE` in the new `citation-
date` fixture, and a general gap: DocBook's own earlier citation fixtures never exercised
`prop::DATE` at all, for lack of this.

## TEI citation/bibliography IR vertical closed (2026-07-28)

Following DocBook's (`8aedfb80fa`) and JATS's (`060c0858d5`) citation verticals, TEI is
done using the same `bibliography`/`bibliography_entry`/`bibliography_field` node kinds
added in `4e15c996`. `tei-fmt` itself needed no changes (its AST is generic XML, like
docbook-fmt's/jats-fmt's) — all the work is in `rescribe-read-tei`/`rescribe-write-tei`.
`<listBibl>` -> `bibliography`; `<biblStruct>`/a `<bibl>` directly inside `<listBibl>` ->
`bibliography_entry`; a bare `<bibl>` used elsewhere (e.g. inline `<cit>` attribution) is
deliberately left as the pre-existing plain-`span` mapping, per the already-passing
`int-cit-bibl` fixture.

**Analytic/monogr/series fork resolution (implemented as instructed, not re-derived):**
`<biblStruct>`'s `<analytic>` level flattens directly into the entry's own
`bibliography_field` children; `<monogr>`/`<series>` each become their own nested
`bibliography_entry`, mirroring DocBook's `<biblioset>` nesting. `<imprint>` is a third
transparent wrapper (splices into whichever entry it's inside), needed because TEI's own
`monogr` content model groups `<publisher>`/`<pubPlace>`/`<date>` there.

**Date-attribute-semantics fork — resolved, not deferred:** implemented `tei:date-attr`
raw-preservation for the single-attribute case. TEI's `att.datable` class (`@when`/
`@notBefore`/`@notAfter`/`@from`/`@to`, or their `-iso`-suffixed siblings) is judged
adequately captured by parsing into the structured `prop::DATE` map plus a `tei:date-attr`
property recording which attribute was used — a reader can tell a point (`when`) apart
from a one-sided bound (`notBefore`/`notAfter`/`from`/`to`) without the distinction being
lost. **However, when `@notBefore`+`@notAfter` (or `@from`+`@to`) are present *together*,
this reader does NOT populate `prop::DATE` at all** — that pair jointly expresses a
genuine two-point RANGE (a lower bound and an upper bound), which does not fit
`prop::DATE`'s single year/month/day Map even with `tei:date-attr` attached: there is no
single "the" point to store. This is exactly the structural mismatch the original task
brief anticipated as a possible fork. Per CLAUDE.md's no-guessing rule, no new range
representation was invented for it — the range case falls back to raw-preserving
`@notBefore`+`@notAfter` (or `@from`+`@to`) verbatim on a `misc` `bibliography_field`
instead, so nothing is silently dropped; only the *modeling* of a two-point range as a
first-class `prop::DATE`-like property remains open. **Decision needed:** should
`rescribe-std` eventually gain a distinct range-shaped date property (e.g. `prop::
DATE_RANGE` as a `{from: Map, to: Map}` Map-of-Maps, or two Maps under `date:from`/
`date:to`), or is raw-preservation-only sufficient for this case indefinitely? See
`fixtures/tei/citation-date` (the R3 assertion) and `fixtures/tei/COVERAGE.md`'s
Bibliography/citation section for the concrete fixture demonstrating this.

Fixtures: `fixtures/tei/citation-{simple-author,multi-author,markup-in-field,bibl-mixed,
analytic-monogr-series,date}`. `fixtures/tei/COVERAGE.md`'s new Bibliography/citation
section is fully checked except for the range-date modeling question above, which is
tracked here rather than silently marked done.

## DOCX (OOXML `b:` bibliography namespace) citation vertical deferred (2026-07-28)

Following DocBook (`8aedfb80fa`), JATS (`060c0858d5`), and TEI (`b61994215c`), the fourth
planned citation vertical — `b:Sources`/`b:Source` -> `bibliography`/`bibliography_entry`
in `ooxml-wml`/`rescribe-read-docx`/`rescribe-write-docx` — was **not** implemented this
session, per the original brief's explicit stretch-goal clause (defer if the crate isn't
architecturally ready).

Why: DocBook/JATS/TEI all share generic-XML-AST format crates where `<bibliography>`/
`<ref-list>`/`<listBibl>` and their entry elements were already parseable-but-unhandled —
all three verticals only needed adapter-layer work. OOXML's bibliography namespace
(`http://schemas.openxmlformats.org/officeDocument/2006/bibliography`, ECMA-376 Part 4) is
architecturally different: `ooxml-wml`'s `generated.rs`/`generated_parsers.rs`/
`generated_serializers.rs` are codegenned (`build.rs`) from RELAX NG compact schemas —
currently only `wml.rnc` plus `shared-commonSimpleTypes.rnc` are wired into that pipeline.
The bibliography namespace has no schema file in the codegen input set at all (confirmed:
`grep` over `build.rs` and the crate for `b:Sources`/`CTSources`/`bibliography.rnc` finds
nothing beyond an unrelated `w:bibliography` compatibility-settings `CTEmpty` flag). Adding
real support means sourcing/vendoring the bibliography RNC/XSD schema and extending the
codegen input set — a schema-generation-pipeline change, not an adapter-layer fill-in —
before any `rescribe-read-docx`/`rescribe-write-docx` work could even begin. That's a
materially larger and differently-shaped task than the other three verticals, so per
CLAUDE.md's "work one vertical to completion, no horizontal sweeps" and the brief's own
deferral clause, it's left as a clearly-scoped follow-up rather than attempted partially.

**Follow-up vertical, when picked up:** (1) vendor the OOXML bibliography RNC/XSD schema
and wire it into `ooxml-wml/build.rs`'s codegen alongside `wml.rnc`; (2) map `b:Sources` ->
`bibliography`, `b:Source` -> `bibliography_entry` with `bibliography_field` children (all
fields are `ST_String255` — flat, no nested markup possible in this namespace, so each
field's children will just be a single `text` node, unlike DocBook/JATS/TEI); (3) raw-
preserve `b:Tag`/`b:SourceType` as `docx:tag`/`docx:source-type` (round-trip-critical:
Word keys in-text citations off `b:Tag`); (4) `b:Year`/`b:Month`/`b:Day` -> `prop::DATE`.

### Discovered gap: pre-existing bibliography readers don't use this IR shape

While adding the citation IR shape above, noticed that `rescribe-read-bibtex`,
`rescribe-read-csl-json`, `rescribe-read-ris`, and `rescribe-read-endnotexml` (all
pre-existing, not touched this session) use a completely different, ad-hoc representation:
a `definition_list` node with each entry's fields flattened into `Properties` as plain
strings (e.g. `rescribe-read-csl-json/src/lib.rs`'s `convert_item`). This predates the
`bibliography`/`bibliography_entry`/`bibliography_field` node kinds added in `4e15c996` and
was not migrated onto them — those four formats are pure-metadata bibliography formats
(BibTeX/CSL-JSON/RIS/EndNote XML) rather than markup-in-document formats like DocBook/JATS/
TEI, so the flat-string approach may or may not be an actual fidelity problem for them (CSL
fields like `title`/`container-title` are effectively always plain text in practice, unlike
DocBook's/JATS's/TEI's markup-permitting equivalents) — this needs a human call, not a
guess: (a) leave the four metadata-format readers as-is (flat properties) since their
source formats genuinely have no nested markup capability, accepting that rescribe now has
two different bibliography-entry shapes in the IR depending on which format produced them,
or (b) migrate all four onto `bibliography`/`bibliography_entry`/`bibliography_field` for
consistency across the whole bibliography surface, even though the field-children-as-
inline-nodes indirection buys nothing for these formats. Flagging rather than deciding.

**Resolved (2026-07-28): option (b), migrated.** The human approved migrating all four.
`rescribe-read-bibtex`/`rescribe-write-bibtex` (`f41fc7e5`), `rescribe-read-csl-json`/
`rescribe-write-csl-json` (`a7271632`), `rescribe-read-ris`/`rescribe-write-ris`
(`f56a8e16`), and `rescribe-read-endnotexml`/`rescribe-write-endnotexml` (`7ecb1d16`) now
all emit `bibliography`/`bibliography_entry`/`bibliography_field`, closing the two-shapes-
for-one-concept split — every bibliography-producing reader in the codebase now uses one
shape. Each format keeps its own field-name property (`bibtex:field`/`csl:field`/
`ris:field`/`endnote:field`) alongside `field:role`, since several source-format field names
share one semantic role (e.g. BibTeX's `address`/`location` both -> `publisher_location`) or
have none at all (e.g. `note`/`abstract`/`keywords` -> `misc`) — the extra property is what
lets each writer reconstruct the exact original field name/element instead of guessing.
Writers keep their old flat-shape entry kinds (`bibtex:entry`, `csl:item`, `ris:entry`,
`endnote:entry`, plus each format's generic `citation_entry`/typed-entry fallbacks) as
secondary dispatch arms for documents built by other producers — only the primary path
changed.

The rewrite also closed several previously-undetected silent drops in these four readers,
found while re-deriving each field mapping from the field-content-model check ADR 0006
requires (not because CSL-JSON/BibTeX/RIS/EndNote turned out to need child-node fields for
markup after all — per ADR 0006's own reasoning they're flat/plain-text, matching OOXML's
`b:` schema case — but writing a *complete* field-role table surfaced dead code the old
flat-property readers had accumulated): `rescribe-read-csl-json` parsed `collection-title`
into its struct but never wrote it to the output at all; `rescribe-read-bibtex` only read
~8 of BibTeX's fields (`abstract`/`keywords`/`organization`/`edition`/`address`/`series`/...
were silently dropped); `rescribe-read-ris` only read ~8 RIS tags despite the underlying
`ris` crate already exposing every tag via a generic map; `rescribe-read-endnotexml`
conflated `isbn`/`issn` into one field (second-parsed one silently overwrote the first),
never read `keywords` despite collecting them into a `Vec` first, and had no support at all
for `<style>` markup runs (now real `emphasis`/`strong`/`underline`/`superscript`/
`subscript` inline nodes — EndNote XML is the one of the four where the field-node shape
actually earns its keep over a flat string, verified via a concrete parse -> emit -> reparse
round-trip in `fixtures/endnotexml/rare-style-markup`). None of these were introduced by
this migration — the rewrite just made them visible while rebuilding each field table from
scratch, and fixed them as a natural byproduct rather than reproducing them under the new
shape.

**Pre-existing architectural violations found, not fixed (out of scope for this
migration):** `rescribe-read-bibtex`/`rescribe-write-bibtex` call the `biblatex` crate's
parser directly in production code, with no standalone `bibtex-fmt` crate — same for
`rescribe-read-csl-json`/`rescribe-write-csl-json` (`serde_json` plus the adapter-owned
`CslItem`/`CslName`/`CslDate` structs, i.e. the CSL-JSON *schema knowledge* itself lives in
the adapter, not just generic JSON parsing) and `rescribe-read-endnotexml`/
`rescribe-write-endnotexml` (`quick_xml::Reader`/`Writer` directly, no `endnotexml-fmt`
crate — the migration replaced one hand-rolled `quick_xml` state machine with a slightly
more capable one, a small generic-XML-tree walker, to support `<style>` markup correctly,
but did not move that logic out of the adapter). `rescribe-read-ris`/`rescribe-write-ris`
is the one exception: RIS already has a proper standalone `ris` crate
(`crates/formats/ris`) with a generic tag-map AST, so no violation there. Per CLAUDE.md's
"adapter layer must never contain parsing or writing logic" rule, a future session should
extract `bibtex-fmt` (wrapping or replacing `biblatex`), a `csl-json-fmt` crate for the
`CslItem` schema, and an `endnotexml-fmt` crate for the EndNote XML element vocabulary —
each becoming a general-purpose Rust library, not just an internal rescribe helper, per
CLAUDE.md's "the -fmt crates are not rescribe internals" principle. Not attempted here since
it was explicitly out of scope for this task.

**Resolved by investigation (2026-07-28): RIS's `SN`/`TY` ambiguity is genuine, not an
oversight — `field:scheme = "sn"` stays.** The open question above (whether RIS's `TY`
reference-type tag could be used to resolve `SN` to `isbn` vs. `issn`) was checked against
the available RIS documentation:

- Wikipedia's "RIS (file format)" tag table — the closest thing to a canonical current RIS
  reference, since Clarivate/EndNote does not publish a standalone current RIS spec —
  defines `SN` uniformly as **"ISSN, ISBN, or report/document/patent number"** for every
  entry type, with no per-`TY` breakdown anywhere in the article's text or type table.
- The `gris` Python RIS library's spec docs likewise define `SN` only as `"ISBN/ISSN"`,
  no type dependency.
- No current, reachable EndNote/Clarivate official RIS documentation stating a `TY`-based
  `SN` rule was found (the commonly-cited EndNote PDF URL 404s; Clarivate does not appear to
  maintain a current authoritative RIS spec page at all).
- Two tools *do* apply a `TY`-based heuristic internally: Zotero's RIS translator
  (`RIS.js`) defaults `SN` to ISBN and overrides to ISSN only for `journalArticle`/
  `magazineArticle`/`newspaperArticle` (plus patent/report special cases), and refbase maps
  `SN`→ISBN for `BOOK`/`CHAP`/`STD`/`THES` and ISSN otherwise. Both are explicitly
  self-described by their authors as heuristics/workarounds (refbase's author: "some kind of
  content-sniffing mechanism would be even better"), not citations of a spec.

Conclusion: no authoritative RIS specification defines a `TY`→`SN` disambiguation. The
splits that exist are tool-specific implementation conventions (and the two found tools
don't even agree with each other exactly). Implementing a `TY`-based mapping in
`rescribe-read-ris`/`rescribe-write-ris` would mean adopting one tool's convention (most
plausibly Zotero's, as the more widely-deployed reference manager) and presenting it as
RIS's own semantics — exactly the kind of invented-convention-as-settled-fact CLAUDE.md
prohibits. `field:scheme = "sn"` (naming the scheme after the RIS tag itself, per the
original non-guess) remains the correct, honest representation. This question is now closed
as "investigated, genuinely unresolvable from the spec" rather than left dangling; a future
session should not reopen it without new spec evidence (e.g., a rediscovered current
EndNote/Clarivate authoritative document that does state a rule).

## MathML resolved for DocBook and JATS; two boxes closed, three re-verified as genuine forks (2026-07-28)

TODO.md previously described DocBook's `equation`/`inlineequation` and JATS's MathML box
as genuinely undecided design forks ("whether to reuse `math_inline`/`math_display` with
MathML as raw content, or something else"). Re-investigated this session: **it was not
actually a fork.** A MathML convention already exists in this repo, established for HTML
(`rescribe-read-html`'s `convert_mathml`/`rescribe-write-html`'s `convert_math_inline`,
documented in this file at the "Inline MathML (2026-07-26)" entry): raw-preserve the
`<math>…</math>` subtree verbatim as `math_inline`/`math_display` with
`math:format="mathml"` + `math:source` holding the exact markup, via each format's own
`emit_fragment`. Checking DocBook 5.2 (tdg.docbook.org) and JATS 1.3
(jats.nlm.nih.gov) confirmed this precedent transfers cleanly to both.

**JATS** (`disp-formula`/`inline-formula`, commit below): the Tag Library's content model
is `label?, (tex-math | mml:math)?` — i.e. MathML and TeX are alternatives, not combined.
The reader previously matched `"tex-math" | "mml:math" => None` ("already captured by the
parent") and then built `math:source` via `extract_text(&rest)` — correct for `tex-math`
(genuinely flat text), but for `mml:math` this flattened the real `<mml:mrow>`/`<mml:mi>`/…
element structure to bare text, a live, currently-shipping loss bug (not a "waiting on a
design decision" situation — the bug existed regardless of any MathML-modeling choice).
Fixed: `convert_children` now intercepts an `<mml:math>` child of `disp-formula`/
`inline-formula` before generic conversion, raw-capturing it via `jats_fmt::emit_fragment`
into a transient sentinel (`split_mathml`, mirroring the existing `split_label` pattern).
`tex-math`'s existing behavior (implicit/no `math:format`, matching the repo's convention
that only the MathML case sets `math:format` — HTML's LaTeX case leaves it unset too) is
unchanged. Writer: `formula_children` re-splices the raw MathML via `JNode::Raw` when
`math:format == "mathml"`, otherwise keeps the pre-existing `<tex-math>` text path.
Fixtures: `fixtures/jats/math-display-mathml`, `fixtures/jats/math-inline-mathml`. Writer
round-trip verified both via the fixture's parse assertions and two new unit tests in
`rescribe-write-jats` (parse -> emit -> reparse, confirming byte-identical `math:source`
recovery). Closes `fixtures/jats/COVERAGE.md`'s "MathML math" box (now 108/109).

**DocBook** (`equation`/`informalequation`/`inlineequation`): the 5.2 reference's content
model is `title?, alt?, (mediaobject+ | mathphrase+ | mml:*+), caption?` — three *mutually
exclusive* content alternatives, not one construct with one obvious shape, so each needed
its own decision rather than a single MathML answer:
- **MathML** (`<mml:math>` or any `{prefix}:math` — DocBook, unlike JATS's Tag Library,
  doesn't mandate the `mml:` prefix specifically, so matching is by local name via
  `is_mathml_root`, not a hardcoded prefix string): same raw-capture-before-generic-
  conversion treatment as JATS, via a new `mathml-raw` sentinel / `split_mathml` in
  `rescribe-read-docbook`, `math:format="mathml"` + verbatim `math:source`.
- **`<mathphrase>`**: confirmed via the DocBook 5.2 reference this is NOT a flat-text
  format — it holds ordinary DocBook phrase-level markup (the reference's own example:
  `x<superscript>n</superscript> + y<superscript>n</superscript>`). Flattening it with
  `extract_text` (the tex-math-style treatment) would destroy that markup the same way the
  JATS MathML bug did. Resolved by keeping its already-converted children (real
  `superscript`/`emphasis`/`text` nodes) as literal children of the `math_display`/
  `math_inline` node — no `math:source`/`math:format` set — matching the repo's existing
  "nested markup survives as real child nodes, not a flat string" convention (the same one
  `bibliography_field` uses for citation markup). This is a genuine, spec-grounded modeling
  decision (not a guess): mathphrase content literally *is* markup-shaped, so it gets
  markup-shaped IR treatment.
- **`<mediaobject>`/`<inlinemediaobject>`** (an image standing in for the equation):
  already converts to a plain `image` node via the pre-existing `mediaobject` arm; kept as
  an ordinary child of the `math_display`/`math_inline` node rather than inventing a
  separate representation — it's still equation content per the spec (one of the three
  content alternatives), just encoded as an image instead of markup or MathML.

  Writer (`equation_children` in `rescribe-write-docbook`, used identically for both
  `math_display` and `math_inline` via a `write_child` fn parameter so an image child
  becomes `<mediaobject>` at block position / `<inlinemediaobject>` at inline position,
  matching each content model's own alternative): re-splices MathML via `DbNode::Raw`,
  wraps any leftover non-image children back in `<mathphrase>`, and picks `<equation>` vs
  `<informalequation>` by title presence (same convention the existing `TABLE`/
  `informaltable` arm uses). `docbook-fmt` itself needed no changes (its AST is generic
  XML, like jats-fmt's) — all the work is in the adapter layer, per CLAUDE.md.

  Fixtures: `fixtures/docbook/equation-mathml` (title + MathML, closes `equation`),
  `fixtures/docbook/inlineequation-mathml` (closes `inlineequation`),
  `fixtures/docbook/equation-mathphrase` (informalequation + mathphrase markup
  round-trip, extra coverage beyond the two closed boxes). Three new unit tests in
  `rescribe-write-docbook` verify parse -> emit -> reparse round-trips (MathML byte-
  identical recovery for both equation/inlineequation, and mathphrase markup structure
  preserved through emit). Closes `fixtures/docbook/COVERAGE.md`'s "equation (display
  math)" and "inlineequation (inline math)" boxes (now 101/105).

**Second-precedent check (per CLAUDE.md's priority hierarchy — checked whether another
format's math handling conflicts with or refines the HTML convention before applying it
here):** `ooxml-omml` (Office Math ML, `crates/formats/ooxml-omml/src/math.rs`) exists as a
standalone library crate but is **not wired into `rescribe-read-docx`/`rescribe-write-docx`
at all** (confirmed: no `math_inline`/`math_display`/`math:format`/`math:source` reference
anywhere in either adapter crate) — DOCX math isn't consumed into the rescribe IR yet in any
form. No conflicting precedent exists; nothing to reconcile. This is a separate, unstarted
gap (DOCX's own math vertical), not touched by this session.

**Not implemented — still genuine, re-verified design forks, not lookup-resolvable (the
`equation`/`inlineequation` MathML fork was the only one collapsed by the HTML precedent;
these three were re-checked against the DocBook/JATS specs and Pandoc's own DocBook reader
source this session and remain genuinely open):**

- ~~**DocBook `qandaset`/`qandaentry`**~~ — **resolved.** The prior session's open fork
  (reproduced below in the original session's own words, for the record) assumed the choice
  was "add a new `qa_list`/`qa_entry` node-kind pair to `rescribe-std`" vs. "stay
  raw-preserved via `generic_div`," without first checking two things: (1) `DIV` already
  nests arbitrarily in this IR (the same pattern `generic_div`/sectioning containers already
  use), so `qandadiv`'s recursive nesting-with-title is not actually a blocker; and (2) the
  existing `DEFINITION_LIST`/`DEFINITION_TERM`/`DEFINITION_DESC` shape — once fixed to be the
  flat, run-grouped convention `rescribe-read-markdown`/`rescribe-read-html` already use (a
  group is 1+ consecutive `DEFINITION_TERM` then 0+ consecutive `DEFINITION_DESC`, direct
  children, no wrapper node) — already models "N terms, M defs per group," which is exactly
  `qandaentry ::= question, answer*`'s shape. No new node kind was needed: `qandaset`/
  `qandadiv` map to `DIV` tagged `docbook:tag`; each `qandaset`/`qandadiv`'s directly-owned
  `qandaentry` children flatten `question`→`DEFINITION_TERM`/`answer`→`DEFINITION_DESC` into
  one synthesized `DEFINITION_LIST` tagged `docbook:list-kind = "qanda"` (see
  `wrap_qanda_entries` in `rescribe-read-docbook`, `write_definition_list` in
  `rescribe-write-docbook`); `defaultlabel` round-trips via `docbook:qanda-defaultlabel`.
  Getting there required fixing a live bug first: `rescribe-read-docbook`'s `"variablelist"`
  reader wrapped each `<varlistentry>` in a `docbook:varlistentry` node containing a
  `DEFINITION_TERM` + a plain `LIST_ITEM` (not `DEFINITION_DESC`), while
  `rescribe-write-docbook`'s `DEFINITION_LIST` write arm assumed direct children were a flat
  `[term, desc, term, desc, ...]` pairing by index and had no write arm for
  `docbook:varlistentry` at all (silently dropped by the generic catch-all) — independently
  reproduced: any `<variablelist>` with 2+ `<varlistentry>`s wrote back with entries bled
  into one merged `<varlistentry>`. Fixed both sides to the flat convention, added
  `fixtures/docbook/definition-list-multi-entry` and `-multi-term` as regression coverage,
  and closed both `qandaset` and Q&A-sub-structure boxes in `fixtures/docbook/COVERAGE.md`
  with `qandaset` and `qandaset-qandadiv` fixtures. Original open-fork text, for context on
  why it looked harder than it was: *"does `rescribe-std` gain a dedicated node-kind pair
  (e.g. `qa_list`/`qa_entry`...) — a real cross-format IR addition... or does it stay
  raw-preserved wholesale via `generic_div`... This is a genuine IR-design call... that only
  a human should make, not something a spec or existing-convention lookup resolves — no
  other currently-modeled format in this repo has an equivalent Q&A-list construct to
  generalize from."* Both premises were wrong on inspection — `DIV`-nesting and
  run-grouped-`definition_list` generalize just fine, and the fork was a spec-lookup problem
  (checking the actual node shapes already in the codebase) once the varlistentry bug was
  out of the way, not a novel IR-design decision.

- **DocBook `programlistingco`/`co`/calloutlist composition — resolved 2026-07-28**
  (`fixtures/docbook/COVERAGE.md`, 3 boxes: `programlistingco`, `co`, "callout listing +
  callout list", all now `[x]`). The prior writeup here framed this as a genuine IR-shape
  question — whether `code_block` content can ever be non-flat, to carry an embedded `<co>`
  marker — but that framing turned out to be wrong: per ADR 0006's actual test (child node
  only if the content model permits *nested markup*, not "is this positional"), neither
  `<co>` (EMPTY, no content of its own) nor `<area>`'s `coords` (plain numeric/positional
  data) ever need markup-bearing children, so `code_block`'s flat-string `content` contract
  didn't need to change at all. What actually closed the three boxes:
  - `<co/>` markers embedded in a verbatim element's mixed content (valid directly inside a
    bare `<programlisting>`, no `<programlistingco>` wrapper required — confirmed against
    the DocBook 5.2 reference's `%co.class;` content-model inclusion) are recorded as a
    `docbook:callout-markers` property on the `code_block`: a list of `{id, offset, label}`
    maps, `offset` being the marker's character position in the extracted flat text. The
    writer splices `<co/>` back into the text at those offsets on emit.
  - `<area>`/`<areaset>` (the external-coordinates alternative, inside `<areaspec>`) fold
    into a `docbook:areaspec` map property on the `code_block` — `id`/`units`/`otherunits`
    plus an `areas` list of `{kind, id, coords, units, otherunits, label}` maps (`areaset`
    entries nest their own `areas` list of plain `area` entries). `<area>`'s optional `<alt>`
    child (DocBook 5.2: permits text + `inlinemediaobject`) is flattened to plain text — the
    common case is plain text; an image nested inside an area's `<alt>` would degrade to
    flattened text rather than a full child-node representation. This one sub-case is an
    intentionally out-of-scope residual, not a silent drop (the text itself still
    round-trips), and no fixture exercises it.
  - `<programlistingco>` (content model `areaspec?, programlisting`) maps to a `div` tagged
    `docbook:tag = "programlistingco"` wrapping the (possibly `docbook:areaspec`-augmented)
    `code_block` — no new node kind.
  - `<calloutlist>`/`<callout>` map to `list`/`list_item` tagged `docbook:tag`, matching the
    existing `procedure`/`step` convention: `<callout>`'s content is ordinary block markup
    (real prose, per ADR 0006's other branch — ADR 0006's test cuts both ways in this one
    construct family, denying child-node status to `coords` while requiring it for
    `<callout>`'s body), so it stays real child nodes; `arearefs` (IDREFS) is raw-preserved
    as a `docbook:arearefs` space-joined string property.
  Implementation: `crates/readers/rescribe-read-docbook/src/lib.rs` (`"co"`/`"area"`/
  `"areaset"`/`"areaspec"`/`"programlistingco"`/`"calloutlist"`/`"callout"` arms,
  `extract_verbatim_text`/`build_area_map`); `crates/writers/rescribe-write-docbook/src/
  lib.rs` (`write_verbatim_children`/`write_areaspec`/`write_area_entry`/`write_area`, plus
  the `programlistingco` DIV arm and `arearefs` on the `callout`-tagged `LIST_ITEM`).
  Verified both content-model flavors independently: `fixtures/docbook/co-callout-inline`
  (inline `<co>` markers, no `<programlistingco>` wrapper) and `fixtures/docbook/
  programlistingco-areaspec` (external `<area>` coordinates via `<programlistingco>`/
  `<areaspec>`), each backed by a full `parse(emit(parse(input))) == parse(input)`
  round-trip unit test in `rescribe-write-docbook`'s test module (both passed on first
  implementation, no design iteration needed after the ADR 0006 analysis).

- **JATS `<alternatives>`** (`fixtures/jats/COVERAGE.md`, box now closed —
  `math-display-mathml-alternatives`, `math-inline-mathml-alternatives`,
  `figure-alternatives-graphics`). Resolved in two parts, previously conflated as one
  "genuine design fork" that turned out to be two separable, non-forked questions once
  actually worked through:
  - **Math-in-`<alternatives>` (bug fix, not a design question).** The just-shipped
    `<mml:math>` raw-capture fix (`242d7d9ecb`) only checked direct children of
    `<disp-formula>`/`<inline-formula>` — but the JATS-recommended pattern for offering
    both a MathML and a TeX rendering of the same formula wraps them in an intervening
    `<alternatives>`. Confirmed empirically (before the fix) that this silently
    *corrupted* content rather than merely dropping it: `<alternatives><tex-math>E=mc^2
    </tex-math><mml:math>...<mml:mi>E</mml:mi>...</mml:math></alternatives>` parsed to
    `math:source = "E=mc^2E"` — the TeX and (flattened) MathML text concatenated into one
    garbled string. Fixed by treating `<alternatives>` as transparent in exactly this
    context (`convert_children`'s new interception, `rescribe-read-jats/src/lib.rs`):
    `<mml:math>` is raw-captured via the same `mml-math-raw` sentinel used for the
    direct-child case (so `split_mathml`/the `disp-formula`/`inline-formula` arms needed
    no changes), and every *other* sibling inside the `<alternatives>` (the `<tex-math>`,
    or a rarer third alternative) is raw-preserved verbatim under a new
    `jats:alternatives-raw` property rather than dropped. `rescribe-write-jats`'s
    `formula_children` re-wraps in `<alternatives>` and splices the raw sibling back in
    via `JNode::Raw` when that property is present.
  - **The general (non-math) case turned out to already be lossless, not a design fork.**
    JATS 1.3's own expanded content model for `<alternatives>` is `((object-id)*, (array |
    chem-struct | code | graphic | inline-graphic | inline-media |
    inline-supplementary-material | media | preformat | private-char |
    supplementary-material | table | textual-form | tex-math | mml:math)+)` — fetched and
    verified directly from the Tag Library page (not from a remembered summary — see ADR
    0006's "check the schema, don't trust a precedent claim" methodology). The natural
    "pick the richest alternative, raw-preserve the rest" design (matching the DocBook
    `equation` precedent, `788f8a9b68`) was worked through concretely against the Tag
    Library's own tagged sample (`<fig>` with `<alternatives>` wrapping two `<graphic>`
    variants differentiated by `specific-use="print"`/`"online"`) and **rejected**: unlike
    MathML (which has no IR node kind of its own, so raw-string capture is the only
    tier-2 option), every element `<alternatives>` can otherwise contain either already has
    a dedicated IR mapping (`graphic`/`inline-graphic` → `image`, `table` → `table`) or
    converts through the existing generic fallback with full nested-markup preservation
    (`textual-form`'s phrase-level content, per its own expanded content model, already
    round-trips as real child nodes, not flattened text). Demoting a non-chosen `<graphic>`
    to an opaque raw-XML string would *regress* fidelity — it would turn a structurally
    addressable second alternative (independently queryable/re-renderable, e.g. by a
    pipeline swapping the online variant for a different URL) into inert text nobody could
    ever again treat as "an image with a URL." Empirically verified
    (`fixtures/jats/figure-alternatives-graphics`): the reader's pre-existing generic-
    wrapper fallback (`<alternatives>` unrecognized → `generic_span`, whose children
    convert through the normal per-element pipeline like any other content) already keeps
    *both* graphics as full `image` nodes with no fidelity warning needed, because nothing
    is lost. `<alternatives>` itself never becomes an IR node in either sub-case — it's
    either elided entirely (math case, replaced by the promoted MathML sentinel) or its
    children simply convert in whatever shape they'd have had without the wrapper (general
    case) — so JATS's own refusal to classify `<alternatives>` as block-or-inline
    (`%alternatives-display.class;` vs `%block-alternatives.class;`) is moot: nothing ever
    needs to classify `<alternatives>` itself.
  - **Residual, pre-existing, out-of-scope gap found along the way (not fixed here):**
    `<graphic>`/`<inline-graphic>` attributes other than `xlink:href` (e.g.
    `specific-use`, which is exactly what distinguishes the two alternatives in the Tag
    Library's own sample) are silently dropped by the reader's `"graphic" |
    "inline-graphic"` arm — it builds a bare `IMAGE` node and never calls
    `attach_all_attrs`, unlike every `generic_span`/`generic_div`. This predates and is
    unrelated to the `<alternatives>` work (it affects any `<graphic>`, alternatives or
    not); worth its own fixture-and-fix pass, not folded into this entry.

Commits (JATS, then DocBook, both after `cargo clippy --all-targets --all-features -- -D
warnings` and `cargo test -q` clean): `fix(jats): raw-preserve MathML in disp-formula/
inline-formula; close MathML fixture box`, `feat(docbook): raw-preserve MathML and model
mathphrase markup in equation/inlineequation; close 2 fixture boxes`.

## `rescribe query` shipped: IR serde + jq embedding, library + CLI (2026-07-28)

Implemented the `rescribe query` capability end to end, per a prior scope-research pass
(now superseded by the actual implementation below):

- **`rescribe-core` `serde` feature wired up** (was declared in `Cargo.toml` but
  completely dead — no derives anywhere). `Document`/`Node`/`NodeKind`/`Span`/
  `Properties`/`SourceInfo`/`ResourceId` derive `Serialize`; `NodeKind`/`ResourceId`/
  `Properties` use `#[serde(transparent)]` so they serialize as plain JSON strings/objects
  rather than wrapped newtypes (this matters for jq ergonomics — `.kind` reads as a plain
  string, `.metadata.title` works without a `.0` hop). `PropValue` and `Resource` have
  hand-written `Serialize` impls for two acknowledged compromises, each recorded as its
  own ADR: [0009](docs/adr/0009-propvalue-float-json-sentinel.md) (non-finite floats →
  string sentinel) and [0010](docs/adr/0010-resource-data-base64-json.md) (resource bytes
  → base64, unconditionally, no lazy/opt-in mode yet). `Deserialize` was deliberately not
  implemented — `PropValue::Int` vs `Float` and the float sentinel vs a genuine string are
  both ambiguous from raw JSON, so a lossless inverse isn't possible without a tagged wire
  format; Serialize-only is the honest scope for the query/export use case.
- **`rescribe::query` module** (opt-in `query` feature on the `rescribe` umbrella crate,
  not part of `all` — keeps the `jaq` engine out of the dependency graph for consumers who
  don't want it). `query(doc, expr) -> Result<Vec<serde_json::Value>, QueryError>` plus a
  reusable `CompiledQuery` for running one filter against many documents. Followed
  `normalize`'s jaq embedding precedent (`normalize-knowledge-graph::jq_compile`/
  `jq_run_all` in `crates/normalize-knowledge-graph/src/store.rs`, read directly rather
  than inferred from docs.rs) — `serde_json::to_value(doc)` → `serde_json::from_value::<Val>`
  (jaq-json's `serde` feature) → run → `serde_json::from_str(&format!("{val}"))` to convert
  results back. This resolved the scope doc's open question about jaq-json's serde-interop
  surface concretely: `Val` gets `Deserialize` (not `Serialize`) from the `serde` feature,
  and `normalize` converts `Val → serde_json::Value` via `Val`'s `Display` impl, not a
  direct serde path.
- **`rescribe query` CLI subcommand**, reusing `convert`'s format-detection/read plumbing
  verbatim. Default output: pretty-printed JSON per result (matches plain `jq`); `-c`/
  `--compact` for one-line-per-result; `-r`/`--raw-output` unquotes string results like
  `jq -r`. Fidelity warnings from the reader print to stderr exactly as `convert` does.
- **Verified concretely** (not just asserted) that `query` subsumes both use cases the
  scope doc claimed it would: `.metadata` for metadata inspection, and
  `[.. | .kind?] | map(select(. != null)) | group_by(.) | map({kind: .[0], count: length})`
  for a node-kind census — both have passing unit tests (`crates/rescribe/src/query.rs`)
  and CLI end-to-end tests (`crates/rescribe-cli/tests/query.rs`) asserting on actual
  output, not just "it compiles."

**Observed overlap not acted on** (per task scope — out of scope for this pass):
`rescribe-write-native` and `rescribe-write-pandoc-json` hand-roll their own IR-walking
JSON/pretty-printer emitters against *Pandoc's* JSON schema (not rescribe's own IR shape),
predating and unrelated to the generic `Document: Serialize` impl added here. They are a
different target schema (Pandoc's `{"t": ..., "c": ...}` AST, not a dump of rescribe's own
node/property shape) and were explicitly left alone. Worth a future look at whether either
could be rebuilt on top of `serde_json::Value` + a schema-mapping layer instead of manual
tree-walking, but that's a distinct refactor from what `query` needed.

**Not implemented (see ADR 0010's reopening condition)**: a `--resources=omit|hash|base64`
flag or lazy/on-demand resource encoding. Every `query` run currently base64-encodes every
embedded resource's bytes unconditionally, even when the filter never touches
`.resources`. Fine for the fixture-sized documents this was tested against; would need
revisiting before recommending `query` for large-corpus batch use against documents with
many/large embedded images.

## Cross-API harness: docbook/jats/tei, odt, ansi audited (2026-07-30)

Second pass on the cross-API harness (`crates/rescribe-fixtures/src/streaming_harness.rs`,
`tests/streaming_apis.rs`) begun in the prior session, scoped to document/markup/data
formats. 5 formats moved out of `NOT_YET_AUDITED`; 12 new `KnownFailure` entries. See
`docs/format-audit.md`'s "Cross-API harness inventory" for the full per-format/per-API table.
Defect classes surfacing again, by name, as the task brief predicted:

- **XML-passthrough sibling-crate bug propagation** (docbook-fmt/jats-fmt/tei-fmt): the
  three crates are byte-identical in implementation shape (confirmed via `diff` across
  `batch.rs`/`writer.rs` — only doc comments and AST/event type names differ), so the same
  three bugs, found once in docbook, apply identically to all three:
  1. `events()`/`EventIter` emits one `Text` event per resolved character/predefined XML
     entity instead of merging it into the surrounding text run the way `parse()`'s AST
     does (`parse.rs`'s `current_text` accumulator, documented as deliberate coalescing).
  2. `StreamingParser`'s `drain()` sets `check_end_names = false` and
     `allow_unmatched_ends = true` on its per-drain-call `quick_xml::Reader` — genuinely
     architecturally required, since each call only ever sees the unconsumed tail, not the
     full document (documented in `batch.rs`) — but the side effect is that a mismatched end
     tag (e.g. `<article><para>Unclosed<para>Another</article>`) is silently accepted
     instead of rejected the way `events()`'s single continuous `Reader` (default
     `check_end_names = true`) correctly rejects it. Zero diagnostics, wrong event sequence.
  3. Downstream of (1)/(2)-adjacent: `parse()` has explicit malformed-XML recovery
     (auto-closes unclosed elements with synthetic `EndElement` nodes) so `build()` always
     emits well-formed output, but `events()` has no such recovery — it just stops at the
     parse error — so the streaming `Writer` (fed by `events()`) emits truncated/unclosed
     XML for input `build()` recovers from cleanly.
  Fix belongs in `docbook-fmt`'s `events.rs`/`batch.rs` (then propagate the same diff to
  `jats-fmt`/`tei-fmt`, given the crates are kept in lockstep) — see
  `streaming_harness::KNOWN_FAILURES` for the exact fixture names (`adv-entity-references`,
  `adv-malformed-xml`).

- **Event-enum expressiveness gap** (`odf-fmt`, backing `odt`/`ods`/`odp`): `OdfEvent` had
  no variant carrying the document's `mimetype`, `meta` (title/author/date), `styles.xml`
  content, or embedded image bytes — `events()` only covered document *body* content
  (paragraphs, tables, slides). The streaming `Writer` genuinely builds its `OdfDocument`
  incrementally per event (the same sanctioned shape as `ooxml-sml`'s `SmlWriter` — real
  per-event work, ZIP byte packaging deferred to `finish()` only because ZIP's central
  directory lives at the end of the file), so it is not architecturally hollow — but the
  reconstructed document always had `mimetype: ""` and empty `meta`/`styles`/`images`
  compared to `parse()`'s AST. Same defect class as org-fmt's missing-metadata-variant gap.
  Also confirmed and corrected a prior (wrong) assessment that called `odf-fmt`'s `events()`
  a `parse()`-then-walk fake — it is a genuine independent `quick_xml` scan of
  `content.xml`, just eagerly/fully buffered rather than lazily incremental (self-documented
  in `events.rs`), and no `StreamingParser<H>` exists in the crate yet at all.

  **Fixed (2026-07-30, follow-up session):** added `OdfEvent::Mimetype(String)`,
  `Meta(OdfMeta)`, `AutomaticStyle(StyleEntry)`, `NamedStyle(StyleEntry)`,
  `ListStyle(String, bool)`, `PageLayout(PageLayout)`, and `EmbeddedImage { name, data }`
  (named `EmbeddedImage` rather than `Image` to avoid colliding with the pre-existing inline
  `Image { href }` body-content event). `events::extract_events` now reads
  `mimetype`/`meta.xml`/`styles.xml`/`content.xml`'s `<office:automatic-styles>`/
  `Pictures`+`media` via the same free functions `parser::parse` uses (`read_zip_text`,
  `parse_meta_xml`, `parse_styles_xml`, `parse_auto_styles_block` — made `pub(crate)` for
  this), and `batch::DocBuilder::process` now sets/pushes them onto the reconstructed
  `OdfDocument`. Verifying the fix against `adv-corrupt-image` (the fixture the original
  `KnownFailure` cited) surfaced a second, directly-adjacent bug in the same fixture:
  `OdfEvent::StartFrame` had no `width`/`height` fields, so `draw:frame`'s `svg:width`/
  `svg:height` were silently dropped even though `ast::Frame` has both — fixed by adding
  them to the variant. Checking `adv-empty` surfaced a third: `events.rs`'s own
  `content.xml` scan didn't recognize a self-closing `<office:text/>` (only `parser.rs`'s
  `parse_content_xml` handled that case), so the streaming path always built `OdfBody::Empty`
  instead of `OdfBody::Text(vec![])` — fixed by mirroring `parser.rs`'s handling in the
  `Event::Empty` branch.

  **Still open — much larger, separate gap surfaced by the fix above:** once the
  resource-loss and two adjacent bugs were fixed, the byte-identical-to-builder check still
  fails: 12 of 66 odt fixtures diverge (`annotation`, `bookmark`, `colspan-rowspan`,
  `endnote`, `footnote`, `footnote-formatted`, `heading`, `image-caption`,
  `non-breaking-space`, `path-deeply-nested-table`, `soft-hyphen`, `text-box`) because
  `OdfEvent` has never covered several `Inline`/structural constructs that `parser.rs`'s
  unified `parse_inlines` handles: `office:annotation`, `text:bookmark`/`bookmark-start`,
  field elements (`text:date`, `text:page-number`, `text:author-name`, etc. — 9 element
  names), `text:soft-hyphen`, `text:soft-page-break`, table cell `col-span`/`row-span`,
  footnote/endnote `<text:note-citation>` content, and `draw:text-box` inside a *text-body*
  `draw:frame` (only presentation-body frames get `StartTextBox`/`EndTextBox` today). This
  is a distinct defect class (or rather, the same class applied to a much larger surface)
  from the mimetype/meta/styles/images gap this entry originally tracked, and is a
  substantially larger body of work — closing it means extending `OdfEvent`'s inline
  vocabulary to match `ast::Inline`'s full surface, plus (for fields/citations) adding
  small amounts of buffering state to `events.rs`'s otherwise-flat SAX scan, since those
  need to capture text between a start and end tag into a single event rather than letting
  it fall through as loose `Text` events into the enclosing paragraph/span. Left as the
  `streaming_writer` `KnownFailure` for `odt` (updated description, not flipped to `Wired`)
  rather than attempted in the same pass — out of scope for a task specifically bounded to
  the resource-loss gap.

- **Logic bugs surfaced by cross-checking, not present in either implementation alone**
  (`ansi-fmt`): `events()`'s `parse_csi_event` 'm' arm emits a `ResetStyle` event whenever
  the resulting style is empty (`self.style.is_empty() && !params.is_empty()`), conflating
  "style ended up empty" with "an explicit reset code (`\x1b[0m`) was seen" — an
  unrecognized/no-op SGR group (e.g. `\x1b[999m`) that changes nothing still emits a
  spurious `ResetStyle`, which the streaming `Writer` then faithfully turns into spurious
  literal `\x1b[0m` bytes in the output (fixture `adv-unknown-sgr`: `build()` emits
  `"Text\n"`, the streaming path emits `"\x1b[0mText\x1b[0m\n"`). Separately,
  `StreamingParser::drain_complete` (`batch.rs`) constructs a **fresh** `EventIter` (with
  `style: Style::default()`) on every drain call instead of carrying running style state
  across drains, so a chunk boundary landing between an SGR sequence and the text it colors
  loses that style under fine-grained (e.g. `single_byte`) chunking. Also confirmed
  `parse()`'s `AnsiNode` has no variant for a bare SGR sequence at all (`apply_sgr` folds it
  into a running `style` var and returns no node) — the reverse of the usual "events() is
  lossier than the AST" shape, so no faithful `ast_to_events` projection exists for this
  crate; `events` left `NotYetWired` rather than wiring a shape-only/lossy comparison.

**`latex` investigated, left `NOT_YET_AUDITED`**: there is no `latex-fmt` standalone crate
at all. `rescribe-read-latex`'s adapter (`crates/readers/rescribe-read-latex`) contains an
~895-line hand-rolled LaTeX tokenizer/parser directly in adapter production code — a
CLAUDE.md "adapter must not contain parsing logic" / "`{format}-fmt` crate must already
exist before writing `rescribe-read-{format}`" violation in its own right, not merely a
cross-API-harness gap. Extracting a proper `latex-fmt` crate (parse/events/StreamingParser/
emit/Writer, per the vertical-completion checklist) is a prerequisite before this format can
get a real `CAPABILITIES` entry; until then there is no five-API contract to check against.
Tracked here rather than guessed at.

**`rtf` spot-checked, left `NOT_YET_AUDITED`**: `rtf-fmt`'s `COVERAGE.md` claims
"5-Production" in prose; this was not taken at face value. Reading `sem_events.rs` and
`batch.rs` directly: `events()` is self-documented as parsing the full input into an
`RtfDoc` before yielding the first event ("the document is fully parsed... Events are then
streamed lazily from the AST via a frame stack"), and `StreamingParser<H>`'s `feed()` only
buffers bytes, calling `sem_events::events()` inside `finish()`. Both look like the same
architecturally-hollow pattern already tracked as `KnownFailure` for texinfo/fb2/textile —
but `sem_events.rs`'s module doc claims a structural justification (RTF's font/colour
tables and group-stack property inheritance must be fully known before body content can be
interpreted), similar in *shape* to html-fmt's accepted `NotApplicable` carve-out. Whether
that justification is as airtight as HTML5's provably-order-independent-impossible tree
construction (vs. an implementation choice that a more careful streaming design could avoid,
since font/colour tables are conventionally declared before body use) needs real scrutiny
before committing to either `KnownFailure` or `NotApplicable` — left `NOT_YET_AUDITED`
rather than guess between them.

**Remaining scope for a future pass** (all still `NOT_YET_AUDITED`, 44 formats total):
`epub`, `bibtex`, `biblatex`, `csl-json`, `endnotexml`, `opml`, `ipynb`, `typst`,
`pandoc-json` have **no standalone `-fmt` crate at all** (only `rescribe-read-{format}`/
`rescribe-write-{format}` adapters) — each needs the same "does the adapter contain parsing
logic" check applied to `latex` above before a `CAPABILITIES` entry is meaningful; not
verified this session. `man`, `csv`, `tsv`, `ris`, `native` were not reached; per the task
brief's hint, `csv-fmt`/`tsv-fmt`/`ris` have no `batch.rs`/`events.rs`/`writer.rs` at all
(confirmed by directory listing only, not read in full) so they likely warrant
`NotApplicable` streaming-writer/streaming-parser entries, but this was not verified against
source the way the audited formats above were, so no entry was written rather than guess.

**Merge note (2026-07-30):** this docbook/jats/tei/odt/ansi pass and the separate
bbcode/creole/dokuwiki/jira/mediawiki/tikiwiki/twiki/vimwiki/xwiki/zimwiki/markua/muse/t2t/
pod/haddock/fountain pass (entries earlier in this file) were done in parallel worktrees
against the same 49-entry `NOT_YET_AUDITED` baseline, so the "44 formats total" figure above
and the "33 remain"/"45 formats remain" figures earlier in this file were each correct only
against their own single-branch diff. After merging both branches, the actual state of
`streaming_harness.rs` is: **35** formats in `CAPABILITIES` (14 pre-existing + 5 from this
pass + 16 from the wiki-verticals passes), **28** remaining in `NOT_YET_AUDITED`
(49 − 5 − 16), and **57** total `KNOWN_FAILURES` entries (12 from this pass + 45 from the
wiki-verticals passes). See `docs/format-audit.md`'s "Merge reconciliation" paragraph for the
same figures cross-checked against the merged table contents directly.

## docbook/jats/tei `StreamingParser` mismatched/unmatched-end-tag bug — fixed (2026-07-30)

Bug (2) from the "Cross-API harness: docbook/jats/tei, odt, ansi audited" entry above is
fixed. `StreamingParser::drain()` (`batch.rs`, all three crates) still has to build a fresh
`quick_xml::Reader` over just the unconsumed tail on every call and disable
`check_end_names`/`allow_unmatched_ends` on it — that part is architecturally required,
since that reader never saw the `Start` tag matching an `End` tag consumed by a *previous*
`drain()` call. What was missing was any *other* mechanism enforcing tag balance in its
place, so a genuinely mismatched or unmatched end tag was silently accepted with zero
diagnostics, diverging from `parse()`/`events()`'s default `check_end_names = true` /
`allow_unmatched_ends = false` convention.

Fix: `StreamingParser` now tracks open-element names itself in a `Vec<String>` field
(`open_stack`) that persists *across* `drain()` calls — the same "survives multiple
`feed()` calls" shape `entity_resolver` already had. Every `StartElement` event pushes;
every `EndElement` event pops and compares. A mismatch, or an `End` against an empty stack,
pushes a `Diagnostic` (message shape mirrors `parse()`'s own quick-xml-error wrapping,
`"XML parse error: ..."`) and sets a new `failed` field so draining stops for good — the
same "fatal diagnostic + stop" behavior `parse()` gets for free from quick-xml's own
`check_end_names = true` erroring out of `read_event_into`. Deliberately does *not*
replicate `parse()`'s further "auto-close still-open elements at EOF" recovery step (that
recovery is AST-shaped — synthetic `EndElement` nodes closing a tree — and doesn't have an
obvious event-stream analog); simplest option that satisfies "always at least one
`Diagnostic`, never silently accepted" was chosen, matching what the task's design-fork
question explicitly allowed.

Applied identically to `docbook-fmt`, `jats-fmt`, `tei-fmt` (confirmed still byte-identical
in shape via `diff` across all three `batch.rs` files before editing — only doc comments
and AST/event type names differ).

`CAPABILITIES`/`KNOWN_FAILURES` updated: `docbook`/`tei` `streaming_parser` flipped
`KnownFailure` → `Wired` (both fully pass `*_streaming_parser_matches_events_under_
adversarial_chunking`, confirmed via `cargo test`). `jats` stays `KnownFailure`, but the
description changed: the mismatch/unmatched bug itself is fixed and confirmed (new fixtures
`adv-mismatched-end-tag`/`adv-unmatched-end-tag` pass), but jats-fmt's own pre-existing
`adv-malformed-xml` fixture is a *truncated*-input case (`<article ...><body><p>Unterminated
content`, no closing tags at all, unlike docbook's/tei's mismatched-end-tag versions of the
same fixture name) that exposes an unrelated, narrower gap once unmasked: the
adversarial-chunking test's incrementality probe feeds exactly the first half of the input
bytes (40 of 81) and asserts at least one event was delivered before `finish()`; those 40
bytes land mid-attribute-value inside the still-open root `<article xmlns:xlink="...">`
start tag (the attribute value alone is longer than half the file), so zero events is
correct, spec-conforming behavior for that exact split point — not a `StreamingParser`
defect, just a fixed-50%-split probe that isn't fixture-shape-aware. Not fixed here (out of
scope for this task); would need either a smarter incrementality probe or a differently-
shaped single fixture that doesn't have its very first token straddle the midpoint.

`streaming_writer` entries for all three formats deliberately left untouched (still
`KnownFailure`) — that's bug (3) above, a separate, still-open gap (`events()` lacking
`parse()`'s malformed-XML auto-close recovery, which the streaming Writer inherits).

New fixtures: `fixtures/{docbook,tei}/adv-unmatched-end-tag`, `fixtures/jats/adv-unmatched-
end-tag`, `fixtures/jats/adv-mismatched-end-tag` (jats needed its own mismatch fixture since
its existing `adv-malformed-xml` never exercised end-tag mismatch at all). `COVERAGE.md`
updated for all three formats' Adversarial sections.

## wiki-family `parse_list()` marker-type-change defect — fixed in 5 crates, surveyed across 11 (2026-07-30)

The cross-API harness's `zimwiki`/`markua` `KNOWN_FAILURES` entries both independently
described the same shape of bug: `parse_list()`'s loop condition only checked that *some*
recognized marker matched ("is this a bullet or a numbered item?"), never that the marker
matched the *specific* list being built (its `ordered` flag, fixed once from the first
item). A blank-line-separated (or, in some crates, directly adjacent) run of the other
marker type got silently absorbed into the current list and mislabeled with the first
group's `ordered` value, instead of ending the list. Given the entries independently
described identical logic in two unrelated crates, the task was to fix both and check the
rest of the wiki/lightweight-markup family (`bbcode`, `creole`, `dokuwiki`, `jira`,
`mediawiki`, `tikiwiki`, `twiki`, `vimwiki`, `xwiki`, `muse`, `t2t`) for the same
copy-paste-lineage defect, since their fixture suites might simply lack a triggering case.

**Survey result — read every `parse_list`-equivalent function in all 11 crates:**

| Crate | Affected? | Why |
|---|---|---|
| `zimwiki` | **Yes — fixed** | `parse.rs`'s `parse_list` accepted `is_bullet \|\| is_numbered` with no check against `ordered`. |
| `markua` | **Yes — fixed** | Identical shape in `parse.rs`'s `parse_list`. |
| `vimwiki-fmt` | **Yes — fixed** | Identical shape (plus a third marker class, `#`, also treated as "ordered"); this turned out to be the *exact same root cause* as the already-tracked, separately-worded `vimwiki` `streaming_parser` `KnownFailure` ("StreamingParser and events() disagree on where a list ends") — fixing `parse_list()` here fixed both at once. |
| `twiki` | **Yes — fixed** | Two copies of the defect (`parse_list` and `parse_nested_list` both have the same same-depth item loop with no marker check) — worse than the others, since the wrong-marker branch of the `if ordered { strip_prefix(...) }` also silently failed and left the raw marker text in the extracted content, not just mislabeling `ordered`. Not previously caught by any fixture. |
| `dokuwiki` | **Yes — fixed** | `parse_list_items` never threaded `ordered` into its same-depth item loop at all (only nested nested-list recursion computed a fresh `nested_ordered` locally); same-depth items of the wrong marker were silently absorbed. Not previously caught by any fixture. |
| `tikiwiki` | No | Structurally immune: `line_depth` is counted using only the *fixed* `marker` char decided from the first line, so a differently-marked line always counts as depth 0, which is `< depth` and breaks immediately — the bug can't reach the "same depth, wrong marker" case at all. |
| `jira-fmt` | No | Explicitly checks `line_marker == marker` before accepting a same-depth item; the `else` branch already breaks on any mismatch. |
| `mediawiki-fmt` | No | Computes `marker` from `ordered` up front and checks `trimmed.starts_with(marker)` directly; a mismatched marker already breaks the loop. |
| `xwiki` | No | Same shape as mediawiki-fmt — fixed marker string, `!line.starts_with(marker)` breaks. |
| `bbcode-fmt` | No | Uses explicit `[list]`/`[/list]`/`[*]` delimiters, not marker-character sniffing — the defect class doesn't apply. |
| `muse-fmt` | No | Has *separate* `parse_unordered_list`/`parse_ordered_list` functions, each checking its own fixed marker directly with an unconditional break on mismatch — no shared "accept either marker" loop exists. |
| `t2t` | No | Computes `marker` from `ordered` up front (`"+ "` vs `"- "`) and checks `trimmed.starts_with(marker)` directly, same shape as mediawiki/xwiki. |

**Fix, applied identically to zimwiki/markua/vimwiki-fmt/twiki/dokuwiki:** after determining
a line matches *some* recognized list marker, additionally check that the marker's
ordered-ness matches the list's own `ordered` flag; if not, `break` out of the item loop
(returning control to the block-level dispatcher, which re-detects the marker type on the
next line and starts a new, correctly-typed list — verified no infinite-loop risk, since
every affected crate's outer dispatcher independently re-checks the current line's marker
before calling `parse_list` again).

**Regression-proof methodology:** for each of the 5 fixes, the fix was `git stash`ed out,
the new fixture re-run to confirm it fails without the fix (proving the fixture actually
exercises the bug, not just a coincidentally-passing shape), then the fix restored and
re-verified passing. Two fixtures (`twiki`, `dokuwiki`) needed their input changed from
blank-line-separated lists to directly-adjacent differently-marked lines after this check
revealed those two crates' loops have no blank-line-continuation logic at all (a blank line
already unconditionally ends the list, independent of this bug) — the "blank line" framing
from the original zimwiki/markua bug reports doesn't apply to every crate's loop shape.

**New fixtures** (added per-crate naming convention — `zimwiki`/`vimwiki`/`twiki` use no
composition prefix, `markua` uses `comp-`, `dokuwiki` uses `int-`):
`fixtures/zimwiki/mixed-list-markers`, `fixtures/markua/comp-mixed-list-markers`,
`fixtures/vimwiki/mixed-list-markers`, `fixtures/twiki/mixed-list-markers`,
`fixtures/dokuwiki/int-mixed-list-markers`.

**`KNOWN_FAILURES`/`CAPABILITIES` updated:** `zimwiki`/`streaming_parser`,
`markua`/`streaming_parser`, and `vimwiki`/`streaming_parser` all flip `KnownFailure` →
`Wired` (all three confirmed passing under `cargo test -p rescribe-fixtures`, including the
adversarial-chunking equivalence checks). `twiki` and `dokuwiki` had no existing
`KNOWN_FAILURES` entry for this bug — it was found via the family survey, not the harness —
so there was nothing to remove there; both were already independently fixed and are now
additionally covered by a fixture.

**Not touched:** `zimwiki`/`markua`/`vimwiki` `streaming_writer` (architecturally hollow
buffer-then-`finish()` writers — a separate, still-open, out-of-scope gap) and every other
crate's own unrelated tracked gaps.

## docbook/jats/tei entity-Text coalescing — fixed in events() and StreamingParser (2026-07-30)

`events()`/`EventIter` (all three crates, byte-identical shape) emitted one `Text` event per
resolved character/predefined/DTD entity reference (e.g. `&amp;` decoded to its own
`Text("&")` event) instead of merging it into the surrounding text run the way `parse()`'s
`current_text` accumulator does — found via this harness's `events()`-vs-`events_from_doc(&
parse())` equivalence check (fixture `adv-entity-references`).

**Fix:** `EventIter::next()` now accumulates a run of adjacent text-equivalent tokens (`Text`,
resolved char refs, resolved predefined/DTD entity refs) into a merged `String` before
dispatching, using a one-token lookahead: since `next()` inevitably has to read one token
*past* the end of the run to discover the run is over, and a `quick_xml::Reader` token can't
be "unread," that lookahead token is stashed in a new `pending: Option<Event<'a>>` field and
returned on the very next `next()` call instead of being re-read. Confirmed via a direct
check against fixture `adv-entity-references`: `a &amp; b &lt; c &gt; d &apos;e&apos;
&quot;f&quot;` now yields one `Text("a & b < c > d 'e' \"f\"")` event, not six.

Applied identically to docbook-fmt, jats-fmt, tei-fmt (re-confirmed byte-identical shape via
`diff` before splicing the fix into all three).

**Fixing `events()` alone broke `StreamingParser`↔`events()` equivalence** (caught
immediately by the harness's adversarial-chunking check, exactly as designed): `batch.rs`'s
`StreamingParser::drain()` does its own entity handling and text dispatch, independently of
`events.rs`, and hadn't been touched, so it kept emitting one `Text` per entity while
`events()` now merged them. Fixed the same way: a `pending_text: Option<String>` field on
`StreamingParser` that persists across `drain()`/`feed()` calls (same shape as
`entity_resolver`), flushed to the handler whenever a non-text event is about to be
dispatched or at a definite end of input. One follow-up bug surfaced while verifying this
against fixture `adv-abstract`-shaped inputs (trailing whitespace text after the last closing
tag): the pre-existing `if self.pending.is_empty() { return; }` early-return at the top of
`drain()`'s loop didn't flush `pending_text` on `is_final`, silently dropping the final
merged text run — fixed by flushing there too when `is_final`. Applied identically to all
three crates.

**No new fixture needed** — the existing `fixtures/{docbook,jats,tei}/adv-entity-references`
fixture already exercises this via the harness's generic `events()`-vs-`events_from_doc(&
parse())` equivalence sweep over every fixture in the directory; it now passes.

**`KNOWN_FAILURES` note (not removed, description corrected):** the `events` entries for all
three formats are still `KnownFailure` — but now for a completely different, already-tracked
reason. `parse()` auto-closes unclosed elements on malformed XML (synthetic `EndElement`
nodes) but `events()` has no such recovery, so it diverges from `events_from_doc(&parse())`
on fixture `adv-malformed-xml` specifically (which contains no entities at all — this is not
a regression of the entity-coalescing bug, just a second, unrelated cause that happens to
trip the same equivalence check). This is the same root cause already tracked by the
`streaming_writer` `KnownFailure` entry for all three formats. Descriptions updated to make
this explicit instead of citing the now-fixed entity-coalescing reason.

## fountain StreamingParser: spurious per-block StartDocument/EndDocument + title-page misread — fixed (2026-07-30)

`fountain_fmt::batch::StreamingParser::emit_block()` re-parsed each accumulated block via
`crate::events::events(&text)` and forwarded every event it yielded — including that call's
own `StartDocument`/`EndDocument` pair — straight to the handler with no filtering. Bulk
`events()` over the whole input emits exactly one `StartDocument`/`EndDocument` pair;
`StreamingParser` emitted one pair PER accumulated block, diverging on any fixture with more
than one blank-line-separated block (the dominant case, not an edge case).

A second, narrower defect shared the same re-parse-in-isolation root cause:
`parse_title_page()` ran unconditionally at the start of every `parse()` call with no "is
this really the first block of the whole document" guard, so a body block matching
`key: value` for one of the 9 recognized title-page field names (title/credit/author/
authors/source/draft date/contact/copyright/notes) got misread as metadata when re-parsed in
isolation — `parse_screenplay()` never even saw those lines, so the block's content was
silently dropped entirely (not just mislabeled).

**Fix:**
- `StreamingParser` now owns exactly one `StartDocument`/`EndDocument` pair for the whole
  stream: `StartDocument` is dispatched eagerly in `new()` (matching bulk `events()`'s
  behavior on empty input, which still emits `[StartDocument, EndDocument]`), `EndDocument`
  in `finish()`. Both are filtered out of every per-block re-parse's forwarded events.
- Only the first accumulated block is parsed via the full `crate::events::events()` (so real
  title-page metadata is still recognized when genuinely present at the start of the
  document). Every later block goes through a new `crate::events::events_body()`, backed by
  a new `crate::parse::parse_screenplay_only()` that skips `parse_title_page()` entirely —
  not just filters its output, since filtering alone can't recover content
  `parse_title_page()` already consumed into the metadata map.

New fixture: `fixtures/fountain/adv-body-line-looks-like-title-field` (a scene heading +
action block, then a later action-shaped `Source: ...` line) — verified against the ground
truth via a direct `parse()` call before writing `expected.json`, and confirmed the fixture
fails without the fix (a body-only regression check, matching the same
stash-fix-then-rerun methodology used for the wiki-family `parse_list()` fixes above).

**`KNOWN_FAILURES` entry removed:** `fountain`/`streaming_parser` — both bugs it described
are fixed; `docs/format-audit.md`'s fountain row and `CAPABILITIES`'s comment updated in
place. `fountain`/`streaming_writer` (architecturally hollow buffer-then-`finish()` writer)
is untouched — a separate, out-of-scope gap.

## ansi StreamingParser/events: spurious ResetStyle + cross-chunk style/text loss — two of three bugs fixed, one left open (2026-07-30)

Two bugs were tracked: (1) `events()` emitting a spurious `ResetStyle` event whenever an
unrecognized/no-op SGR group (e.g. `\x1b[999m`) happened to leave `style` empty, conflating
"style ended up empty" with "an explicit reset code was seen"; (2) `StreamingParser::
drain_complete` losing running SGR style state across chunk boundaries, since it built a
brand-new `EventIter` (`style: Style::default()`) on every drain call.

**Fix (1):** `apply_sgr_event()` now returns whether it actually applied an explicit reset
code (`0` or an empty code); only that return value triggers `ResetStyle`. A no-op/
unrecognized SGR group now emits `SetStyle(unchanged style)` instead of a spurious reset —
still an event (not silently dropped, since parse()'s `AnsiNode` has no representation of a
no-op SGR group either way — see the `NotYetWired` `events` entry), just no longer
mislabeled as an explicit reset.

**Fix (2):** `EventIter` gained `new_with_style()`/`current_style()` so `StreamingParser` can
carry its running `style` field forward across `drain_complete()`/`finish()` calls by hand
(same shape as docbook-fmt's `entity_resolver` persistence pattern) instead of resetting to
default every call.

**Fixing (2) uncovered a third, previously-masked bug** (not in the original two-item brief,
but the same "genuine cross-chunk state" class): since `drain_complete()` built a fresh
`EventIter` per call regardless, adjacent `Text` events from separate drain calls were never
merged — fine-grained (e.g. single-byte) chunking fragmented one text run into one `Text`
event per call, reproducing even with an *unchanging* style throughout (so unrelated to bug
(2)'s SGR-state-loss mechanism specifically). Fixed via a `pending_text: Option<(String,
Style)>` accumulator on `StreamingParser`, flushed whenever a non-`Text` event is dispatched,
the style changes, or at end of input — same shape as the docbook-fmt/jats-fmt/tei-fmt
entity-coalescing fix.

**A fourth, distinct bug was found and left open** (not attempted — real architecture, not a
small fix, per the task's own "leave it as a KnownFailure with a sharpened reason" guidance):
`EventIter::next()` treats an OSC 8 hyperlink as one atomic token, scanning forward within a
single `next()` call all the way to its *matching closing* OSC 8 sequence
(`\x1b]8;;\x07`). `find_safe_boundary()` has no concept of this open/close pairing — it only
asks "is this one escape sequence, by itself, syntactically complete" — so it happily calls a
complete *opening* OSC 8 sequence a safe boundary on its own. Under fine-grained chunking
(fixtures `hyperlink`, `rare-hyperlink-uri`; reproduces under `single_byte` and
`chunks_of_3`, not `whole`), `drain_complete()` then parses just the opening sequence in
isolation, finds no closer within that truncated slice, and emits a `Hyperlink` event with
empty text immediately — the link text and the closing sequence then get parsed separately
(as plain `Text` and a stray `RawEscape`) on a later call. Properly fixing this means teaching
`find_safe_boundary()` to recognize an opening OSC 8 sequence and hold everything up to its
matching closer as one unsplittable unit — the same class of "buffer until a semantic close
is seen" logic `html-fmt`'s `StreamingParser` already needs for HTML5 tree construction, not
a one-line patch. Left as a `KnownFailure` with a corrected, narrowed description.

Verified no other divergences exist beyond the hyperlink fixtures: wrote a temporary
scratch test iterating every ansi fixture under `whole`/`single_byte`/`chunks_of_3` chunking
and diffing bulk `events()` against `StreamingParser` output directly (the harness's own
equivalence check only records the *first* divergence per run, via `result.is_ok()` gating,
so it can't by itself rule out additional masked bugs) — only `hyperlink` and
`rare-hyperlink-uri` diverged, both only under fine-grained chunking, confirming the two
originally-scoped bugs (and the incidentally-discovered text-coalescing one) are the only
things actually fixed here, with no further surprises hiding behind them.

**No new fixtures needed** — the existing `fixtures/ansi/adv-unknown-sgr` fixture already
exercises both fixed bugs via the harness's existing cross-API equivalence checks (it's the
exact fixture the original bug reports cited).

**`KNOWN_FAILURES` entries not removed, both corrected:** `ansi`/`streaming_parser`'s
description is narrowed to describe only the remaining hyperlink-span-atomicity gap (the two
originally-described bugs are noted fixed in a comment above the entry). `ansi`/
`streaming_writer`'s description is corrected too: it still fails on `adv-unknown-sgr`, but
for what is now a *different*, pre-existing, genuine reason (`parse()`'s AST drops a real
trailing `\x1b[0m` that `events()`/the streaming Writer faithfully preserve — a `parse()`/
`build()` fidelity gap, not a streaming-API defect) rather than the fixed spurious-ResetStyle
cause it originally cited.

## Merge note: second bugfix-pass reconciliation (2026-07-30)

The docbook/jats/tei end-tag/entity-coalescing/wiki-`parse_list()`/fountain/ansi pass above
and the separate org/texinfo/djot/odf/t2t `Event`-vocabulary pass (see the `Event::Metadata`,
`Event::Title`, `Event::LinkDef`, `OdfEvent`, and `Event::Header` entries earlier in this
file) were done in parallel worktrees against the same merged-baseline `streaming_harness.rs`
(the one already reconciled in the "Merge note (2026-07-30)" entry above, 35/28/57). Neither
branch changed which formats are in `CAPABILITIES` vs. `NOT_YET_AUDITED`, so those two counts
are unchanged (**35** / **28**). `KNOWN_FAILURES` shrank from 57 to **50**: seven entries were
deleted because the bug is fixed and the format+API is now `Wired` (docbook/tei/vimwiki/
zimwiki/markua/fountain `streaming_parser`, org `streaming_writer`); three were narrowed in
place rather than deleted, because part of what they tracked is fixed but a distinct,
still-open issue remains (texinfo `streaming_writer`: title-drop fixed, hollow buffer-then-emit
remains; djot `streaming_writer`: `LinkDef`-drop fixed, hollow buffer-then-emit remains; t2t
`streaming_parser`/`streaming_writer`: header-misread/title-author-date-drop fixed,
per-block-reparse/hollow-writer issues remain); odf `streaming_writer` was also narrowed
(mimetype/meta/styles/images fixed, a much larger body-content `OdfEvent`-vocabulary gap over
12/66 odt fixtures remains); and ansi's two entries were corrected in place (two of three
original bugs fixed, the remaining issue in each is a distinct bug, not a subset of the
original description). See `docs/format-audit.md`'s "Second merge reconciliation" paragraph
for the same figures cross-checked directly against the merged table contents, and for the
full per-entry breakdown.

## Event-vocabulary expressiveness gaps, continued: commonmark-fmt + asciidoc fixed, markua verified clean, ooxml-wml/ooxml-sml fenced (2026-07-30)

Continuation of the survey that produced the five `Event`-vocabulary fixes above (org-fmt
`Metadata`, texinfo `Title`, djot `LinkDef`, odf seven-variant, t2t `Header`). Five more
candidates were assigned; two were fixed the same way, one was verified to not be a real gap,
and two (both OOXML) turned out to need a different, larger kind of fix than "add an `Event`
variant" — fenced here rather than half-done.

**`commonmark-fmt` — fixed.** `CmDoc.link_defs: Vec<LinkDef>` (reference-style `[label]: url`
definitions) had no `Event` variant; pulldown-cmark's own event stream never surfaces these at
all (it silently resolves references to their target inline), so `parse()`'s `collect_link_defs`
helper (a second, separate `Parser::reference_definitions()` scan) was the only place they
existed. Direct template: `b745efa497`'s djot-fmt `LinkDef` fix, same shape. Added
`Event::LinkDef { label, url, title }` (mirrors `ast::LinkDef` field-for-field, same as djot's).
`collect_link_defs` promoted to `pub(crate)` so `events.rs` can reuse it rather than
duplicating pulldown-cmark's `reference_definitions()` extraction. `EventIter` computes
`link_defs` eagerly at construction (same pattern `parse()` already used — a second `Parser`
instance read via `&self` before `into_offset_iter()` consumes the first one) and drains them
as `LinkDef` events once the pulldown stream is exhausted, right before `EndDocument` (no
footnote-defs section to place them before, unlike djot). `StreamingParser` gets this for free
(finish() calls `events()` on the whole buffered input). `writer.rs`'s `DocBuilder` gained a
`link_defs: Vec<LinkDef>` field, pushed to by a new `LinkDef` event arm, threaded into
`CmDoc::link_defs` at `finish()` (previously hardcoded `vec![]`). `gfm`/`markdown` share this
crate, so this closes all three harness entries' `link_defs`-shaped loss simultaneously. The
`commonmark`/`events` and `commonmark`/`streaming_writer` `KNOWN_FAILURES` entries were **not**
touched — both already tracked separate, still-open, unrelated bugs (image alt-text
Text/StartImage ordering; unmerged-Text-events; the writer's buffer-then-emit hollowness) that
remain exactly as documented; `link_defs` was never mentioned in either entry's text, so there
was nothing to narrow. Confirmed via `cargo test -p commonmark-fmt --all-features` (82 tests)
and `cargo test -p rescribe-fixtures --test streaming_apis` (full 90-test suite, including the
two commonmark checks) — both still resolve to the same `KnownFailure` states, i.e. no
regression and no silent flip. Fixture coverage: `fixtures/commonmark/path-many-link-defs` and
`rare-link-ref-def` already exercised this construct; no new fixture needed.

**`asciidoc` — fixed.** `AsciiDoc.attributes: HashMap<String, String>` (document `:key: value`
declarations) had no `Event` variant; `StartDocument` carries no payload, and the aggregate map
was reachable only via `EventIter::take_attributes()`, a `pub(crate)` method called solely by
`collect_doc_from_iter` (the `parse()`-internal helper) — a caller driving public `events()` as
an iterator never saw attributes at all. This is the "genuine key/value bag →
`Event::Metadata{key,value}`" case (org-fmt's precedent), but with a wrinkle org's `Vec<(String,
String)>`-backed metadata didn't have: AsciiDoc attributes can be declared anywhere in the
document (not just a header), and `AsciiDoc.attributes` is an unordered `HashMap` — the AST
itself already discards declaration order/position and keeps only the final value per key.
Chose position-faithful emission over front-loading: added a parallel `attribute_log:
Vec<(String, String)>` field to `EventIter`, appended at the exact same `try_parse_block()` call
site that inserts into `self.attributes`, and the `Iterator::next()`'s block-pulling arm
snapshots `attribute_log.len()` before/after each `try_parse_block()` call (identical
before/after-diff shape to org-fmt's `metadata_before`/`self.metadata[metadata_before..]`
pattern in `parse.rs`), pushing one `Event::Metadata` frame per newly-logged declaration
immediately before the block it precedes — including the case where only attribute lines were
consumed with no following block (e.g. trailing declarations at EOF). `StreamingParser` gets
this for free (its `emit_block()` re-parses each accumulated block via `events()`).
`writer.rs`'s `DocBuilder` gained an `attributes: HashMap<String, String>` field, updated via
`insert` on each `Metadata` event (same last-write-wins semantics as `parse()`), threaded into
`AsciiDoc::attributes` at `finish()` (previously hardcoded `Default::default()`).
`events::handle_event` (the `collect_doc_from_iter` helper) got a no-op `Metadata` arm since
that path still reaches into `EventIter`'s private `attributes` field directly rather than
rebuilding it from events.

The AST's information loss (no position, no duplicates) meant the existing strict
`asciidoc_events_equals_ast_projection_over_all_fixtures` `assert!` needed a real design
decision, not just an update: a hand-written AST→events projection *cannot* reproduce
source-accurate `Metadata` positions from a `HashMap`, because the AST was never given that
information to begin with. Fix: `ad_ast_to_events` emits one canonical `Metadata` event per
`doc.attributes` entry, sorted by key, right after `StartDocument`; the test then splits
`Metadata` events out of both the expected and actual sequences before comparing — non-Metadata
events still compared with strict positional equality (unchanged rigor), `Metadata` events
compared as an order-independent, final-value-per-key set (actual-side duplicates from
`attribute_log` collapsed via `HashMap::insert` in event order, matching `DocBuilder`'s own
semantics). This is a case where `events()` is now **more** faithful to the source than the AST
projection can express, not less — documented at length in `ad_ast_to_events`'s doc comment so
the asymmetry isn't mistaken for a bug later. Existing fixtures (`attribute-def`, `doc-metadata`,
`path-many-attrs`) already exercise this construct; no new fixture needed. Confirmed via
`cargo test -p asciidoc --all-features` (40 tests) and
`cargo test -p rescribe-fixtures --test streaming_apis asciidoc` (3 tests, including the
strengthened projection check).

**Separate, pre-existing gap found while verifying asciidoc (not fixed, not in scope of this
pass): `asciidoc::emit::build()` never serializes `AsciiDoc.attributes` back into `:key: value`
lines at all** — `build()` calls only `build_blocks(&doc.blocks, ...)`; `doc.attributes` is
never read anywhere in `emit.rs`. This means `parse(emit(parse(input)))` already silently drops
every document attribute, independent of the streaming layer entirely (both the plain builder
`build()` and the streaming `Writer` — which also calls `emit::build` internally — lose
attributes equally, so this pass's fix doesn't regress or improve that specific behavior either
way). This is a CLAUDE.md losslessness violation in the AST-level emitter, not the event
vocabulary; needs its own fix (attributes need a defined serialization position in
`emit::build`'s output — most likely as leading `:key: value` lines before the first block,
matching where `parse()` accepts them) and its own fixture-driven round-trip check. Tracked
here, not fixed.

**`markua` — verified, not a real gap.** The task's shallow spot-check flagged
`MarkuaDoc.title`/`author`/`description` as inaccessible via `StartDocument`. Verified against
`crates/formats/markua/src/parse.rs:13`, the crate's only production `MarkuaDoc` construction
site: `title`/`author`/`description` are hardcoded `None` unconditionally, and no other code
path in the crate ever assigns them (confirmed via `grep -rn "title\|author\|description"` over
`parse.rs`, `lib.rs`, `writer.rs` — the only other hits are `writer.rs`'s own hardcoded `None`s
and an unrelated in-heading `title` local variable at `parse.rs:163`, which is Markdown heading
text, not document metadata). There is no data for `events()`/`StreamingParser`/the streaming
writer to lose, because `parse()` never produces any to begin with. The existing
`markua`/`streaming_writer` `KNOWN_FAILURES` entry already states this precisely ("MarkuaDoc::
title/author/description are permanently None because parse() never populates them from any
Markua syntax... unreachable via fixtures") — left unchanged, since it was already correct. No
code change made.

**`ooxml-wml` (docx) and `ooxml-sml` (xlsx) — fenced, not fixed; this is a different, larger
class of gap than the other four in this pass.** The task named `core_properties`/
`app_properties`/`gen_styles` (wml) and `styles: Stylesheet` (sml). Verified both are real: on
`ooxml-wml`, `Document<R>` (`document.rs`) — the package-level struct that resolves the zip's
relationships and reads `docProps/core.xml`/`docProps/app.xml`/`word/styles.xml` as separate
parts — carries `core_properties: Option<CoreProperties>`, `app_properties:
Option<AppProperties>`, `gen_styles: types::Styles`, none of which `WmlEvent` has a variant for.
Same shape on `ooxml-sml`: `Workbook<R>` (`workbook.rs`) carries `styles: crate::types::
Stylesheet`, resolved from `xl/styles.xml`, with no `SmlEvent` variant.

The reason this isn't a same-shaped fix as commonmark/asciidoc: **`events(bytes: &[u8])` on
both crates is explicitly scoped to a single already-extracted XML part, not the zip package.**
`ooxml-wml/src/events.rs`'s own doc comment: "`bytes` should be the raw content of
`word/document.xml` extracted from the DOCX zip." `ooxml-sml/src/events.rs`: "`bytes` should be
the raw content of `xl/worksheets/sheet1.xml` (or similar)." Neither `WmlEventIter` nor
`SmlEventIter` ever opens a zip, resolves a relationship, or sees any other part — there is
structurally no code path today that could ever produce a `CoreProperties`/`AppProperties`/
`Styles` event, because the reader never touches those parts. This is confirmed independently
on the writer side, and self-documented already: `ooxml-sml/src/streaming.rs`'s own module doc
states outright that "styles, charts, comments, pivot tables, and merged cells are
`WorkbookBuilder`-only features with no `SmlEvent` representation, so they are out of scope for
this writer, not deferred by it" — `SmlWriter` has a `set_shared_strings()` side-channel setter
(same pattern as `register_image` in wml) but no equivalent for styles. `WmlWriter` has no
side-channel for `core_properties`/`app_properties`/`gen_styles` either, and — unlike
`register_image`, which the crate did think to add a side-channel for — nothing analogous
exists for these three fields at all.

Closing this gap for real needs a **new, package-level streaming reader entry point** — something
that opens the zip, walks `_rels`, and emits `CoreProperties`/`AppProperties`/`Styles` (wml) or
`Styles` (sml) as their own leaf events before/around the existing per-part `document.xml`/
`sheetN.xml` event stream — not an additional `Event` variant bolted onto the current
part-scoped `events()`. That is squarely the "ooxml-fmt rework... most important streaming work
in the queue" milestone CLAUDE.md already names as the priority OOXML target, and is
substantially bigger than this pass's other four items. Not attempted here.

No code changes made to `ooxml-wml`/`ooxml-sml`. No `CAPABILITIES`/`KNOWN_FAILURES` entries
were touched for `docx`/`xlsx`: neither crate's harness checks currently exercise
`core_properties`/`app_properties`/`gen_styles`/`styles` at all (`docx`'s `events` `KnownFailure`
is the pre-existing, unrelated Text-drop/End-tag-reversal bug; `docx` `streaming_parser`/
`streaming_writer` and `xlsx` `streaming_parser` are `NotYetWired`; `xlsx` `events`/
`streaming_writer` are `Wired` for what they do check, which doesn't include styles) — so there
was nothing stale to narrow or remove. This finding is new information, not a correction of an
existing claim.

## Streaming-writer incrementality sweep: 16 wiki/lightweight-markup formats — fixed (2026-07-31)

Per CLAUDE.md's "-fmt crates are not rescribe internals" section, all 16 wiki/lightweight-markup
crates' streaming `Writer`s — `bbcode-fmt`, `creole`, `dokuwiki`, `jira-fmt`, `mediawiki-fmt`,
`tikiwiki`, `twiki`, `vimwiki-fmt`, `xwiki`, `zimwiki`, `markua`, `muse-fmt`, `textile-fmt`,
`pod-fmt`, `haddock-fmt`, `fountain-fmt` — were confirmed architecturally hollow across the
2026-07-30 audit passes above (`ObservableSink` incrementality probe: zero bytes reach the sink
before `finish()`). All 16 are now rewritten to genuinely incremental per-event writers,
following the `rst-fmt`/`ooxml-wml`/`ooxml-sml` template: a single shared output buffer plus
offset marks instead of per-frame buffers, a compact O(nesting-depth) frame stack, deferral only
for constructs whose prefix depends on content not yet seen. Work was split across three
parallel worktrees (roughly 5/5/6 crates each), then merged and reconciled centrally.

**Classification was close to uniform across the family**, confirming the task's prediction
that wiki markup would need less deferral than RST: the large majority of every format's
constructs are write-straight-through. The genuine deferrals found, all small and local:
- O(1) bool/`Option` flags: jira's table-row header-ness, mediawiki's link-text accumulation,
  haddock's lazy description separator, muse's paragraph terminator.
- O(1) parent-frame lookups: textile's blockquote/list-item paragraph attribute suppression.
- O(depth) counters: list/blockquote nesting depth (several crates).
- One genuine *reordering* case: xwiki's `[[label>>url]]` syntax needs the label written before
  the url, but the url is known first in event order — solved by holding `url` on the frame.
- O(field-count) buffering: fountain's title-page fields, muse's new `Metadata` event.
- One in-place insert: markua's figure captions (same technique as rst-fmt's headings).

No format in this family needed true unbounded buffering.

**Five previously-unknown, independent content bugs were found and fixed** by the
byte-identical-to-builder checks each rewrite required (the same category of finding
CLAUDE.md's vertical-completion checklist calls out: "this check is what caught ooxml-sml
silently dropping cell style indices"):
- **markua**: `EndFigure` always built `caption: vec![]`, silently dropping figure captions on
  every round-trip through the old writer.
- **twiki**: `build_list_items` wrote an item's own newline *before* recursing into nested
  lists instead of after — an ordering bug independent of the incrementality rewrite, caught
  because the new writer's byte-identical test was the first-ever exercise of this code path.
- **haddock-fmt**: `events()` emits a redundant `Text` child inside `Link` that the AST builder
  never reads; fixed in the new writer by suppressing it via a dedicated frame (an `events()`
  bug, not touched at the source — tracked here since fixing `events()` itself was out of scope
  for a writer-focused pass).
- **fountain-fmt**: `flush_metadata_if_pending` fired on `StartDocument` itself, permanently
  discarding title-page metadata (`Title:`/`Author:`/etc.) before it ever arrived.
- **pod-fmt**: `emit.rs`'s `build_inline` escaped `<`/`>` via two sequential `String::replace`
  calls — `s.replace('<', "E<lt>").replace('>', "E<gt>")` — which corrupts its own output: the
  first pass's replacement text `"E<lt>"` contains a literal `>`, which the second,
  unconditional pass then rewrites again, turning a lone `<` into `"E<ltE<gt>"` instead of
  `"E<lt>"` (fixture `adv-unclosed-format`). This is a bug in the pre-existing AST builder path
  (`emit.rs`), not something the rewrite introduced — the streaming writer's own char-by-char
  escaping never had it, which is what exposed the divergence. Fixed by rewriting the escape to
  be char-by-char in `emit.rs` too.

**A separate, real test-infrastructure bug was found and fixed alongside this work**: 12 of the
16 crates' new peak-memory regression tests (added per this pass's requirements) tracked
current/peak allocated bytes via a `static AtomicUsize` pair inside a test-local
`#[global_allocator]`. Since `cargo test` runs a crate's tests on multiple threads sharing one
process-wide allocator, an unrelated test running concurrently on another thread during the
measured window added its own allocations into the same counters. This is not hypothetical —
`pod-fmt`'s peak-memory test failed with a spurious 407x ratio under full-workspace
`cargo test -q`, but passed cleanly under `--test-threads=1`. Fixed in all 12 affected crates
(`pod-fmt`, `jira-fmt`, `creole`, `mediawiki-fmt`, `twiki`, `xwiki`, `zimwiki`, `vimwiki-fmt`,
`fountain-fmt`, `muse-fmt`, `haddock-fmt`, `textile-fmt`) by converting to
`thread_local! { static ...: Cell<usize> }`, which also let several crates drop a
`Mutex`-based `TEST_LOCK`/`ALLOC_TEST_GUARD`/`PROBE_LOCK` that only ever serialized the two
memory-guard tests against each other and never protected against the crate's *other* tests.
`bbcode-fmt`, `dokuwiki`, `tikiwiki`, and `markua` already used a different (non-flaky)
measurement technique (a counting `Write` sink) and needed no change.

Measured before/after throughput gains: roughly 1.3x-7x depending on format (fountain-fmt was
the outlier, at ~1.33x and still slower than its own plain builder — its per-block metadata/
uppercase-transform buffering has more inherent overhead than the other formats' writers; not
chased further here since it's still a real incrementality win, just a smaller speed one). Peak
memory reductions ranged roughly 1,000x-40,000x on large synthetic documents (50,000-section or
20,000-paragraph documents, format-dependent) where a peak-memory regression test was practical
to write against a discarding/counting sink.

**Central bookkeeping**: `crates/rescribe-fixtures/src/streaming_harness.rs`'s `CAPABILITIES`
table flips `streaming_writer` from `ApiState::KnownFailure` to `ApiState::Wired` for all 16
formats; the matching `KNOWN_FAILURES` entries were removed (not narrowed — all 16 are fully
fixed), taking the table from 50 to 34 entries. `crates/rescribe-fixtures/tests/
streaming_apis.rs`'s hand-rolled `muse_ast_to_events()` parity helper was updated to emit the
new `MuseEvent::Metadata` event (added alongside the writer rewrite, since `parse()` already
populated title/author/date/desc/keywords but no event carried them), keeping
`muse_events_equals_ast_projection_over_all_fixtures` accurate. `docs/format-audit.md`'s
per-format streaming-writer column and session-tally narrative were updated to match.

**Explicitly not touched** (out of scope for this writer-focused pass, all pre-existing and
independently tracked): `xwiki`'s and `muse-fmt`'s `streaming_parser` `KnownFailure` entries
(buffer-then-finish batch parsers); `pod-fmt`'s `streaming_parser` `KnownFailure` (explicitly
self-documented buffer-then-finish); `textile-fmt`'s `streaming_parser` `KnownFailure`; and
every other format's `events()`/`streaming_parser` gaps tracked in the sections above. Also not
touched, per the task's explicit fence: `texinfo`, `djot-fmt`, `t2t`, `org-fmt`,
`NOT_YET_AUDITED` formats, and the OOXML package-level streaming reader work.

Verification: `cargo clippy --all-targets --all-features -- -D warnings`,
`cargo fmt --check`, and `cargo test -q --workspace --exclude ooxml-codegen` (the exclusion is
for a pre-existing, unrelated failure — `ooxml-codegen`'s `test_generate_wml`/
`test_eg_definitions` read an external `spec/OfficeOpenXML-RELAXNG-Transitional/wml.rnc` file
not present in this environment, confirmed pre-existing via `git log` on that test file) all
pass clean: 418 test binaries, 0 failures.

## muse's incrementality probe repaired + man-fmt wired + no-fmt-crate claim verified (2026-07-31)

**muse's probe fixed.** `muse_streaming_parser_matches_events_and_is_incremental` was the one
call site of `assert_streaming_parser_is_incremental` still using the original fixed-50%-byte
fixture split (inside the per-fixture loop) instead of a hand-built synthetic sample — skipped
during the earlier fb2/texinfo/xwiki/textile/jats/pod repair pass because muse's
`streaming_parser` was already `Wired`, so the repair wasn't required for correctness at the
time. Replaced with a hand-built sample (`"* Heading\n\nUnterminated paragraph text..."`, run
once outside the fixture loop, `finish()` never called), matching the other six call sites.

**man-fmt wired into `CAPABILITIES`** (`events: Wired`, `streaming_parser: KnownFailure`,
`streaming_writer: KnownFailure`), chosen for its 29-fixture corpus (the largest among
`NOT_YET_AUDITED` entries with a real crate, ahead of `native`'s 25 and `rtf`'s 38 — `rtf` was
not picked up this pass; see below). `events()` follows the same "parse() then walk the AST"
shape as t2t/pod/haddock/fountain (`EventIter::new(&doc)` eagerly collected), so per that
precedent it is `Wired`.

**A real, previously-unknown bug in `man-fmt`'s own `events.rs` was found and fixed** while
building the events()-vs-AST-projection check (`fixtures/man/bold` failed immediately on first
exercise): every inline container (Bold/Italic/Superscript/Subscript/Link) pushed a synthetic
children-walking `Frame::Inlines` with `close: CloseKind::Paragraph` as a "dummy" value — but
the dummy was never actually inert. When that children frame ran out of items it unconditionally
emitted a real, spurious `EndParagraph` event, landing between the container's content and its
real close event (`Text, EndParagraph(spurious), EndBold, EndParagraph(real)` instead of `Text,
EndBold, EndParagraph`). man-fmt's own pre-existing `events.rs` tests only asserted `.any(...)`
membership on the event stream, never exact ordering, so this shipped undetected. This is a
small, well-scoped bug (not an architectural rewrite), so it was fixed in-session: added a real
`CloseKind::None` variant (crates/formats/man-fmt/src/events.rs) that pops its frame without
emitting any event, replacing the dummy `CloseKind::Paragraph` at all five inline-container
sites. Confirmed via `cargo test -p man-fmt` (34+ tests, all passing) and the
events()-vs-AST-projection check (now passing over all man fixtures).

**Two genuine, remaining `man-fmt` defects tracked as `KnownFailure`** (both directly verified
by running real code, not inferred from reading):
- `streaming_parser`: `StreamingParser::emit_block()` re-parses each accumulated block in
  isolation via `events()`, which always wraps its output in its own `StartDocument`/
  `EndDocument` pair — so `StreamingParser` emits one such pair per block instead of one for the
  whole document (same root-cause class as t2t-fmt/fountain-fmt's pre-fix bug, but reproducing
  on every multi-block fixture, not needing a specific trigger). Verified: `events()` on a
  2-heading input yields 1 `StartDocument`/14 events; `StreamingParser` on the same input yields
  2 `StartDocument`/16 events.
- `streaming_writer`: two stacked defects — (1) `Writer` buffers all events into a
  `Vec<OwnedManEvent>` and only reconstructs the AST + calls `emit::build()` inside `finish()`
  (self-admitted in its own module doc, the same fake-streaming-writer pattern as
  t2t/pod/haddock/fountain/commonmark); (2) independently, `ManEvent` has no variant carrying
  document metadata (`ManDoc::title`/`section`/`date`/`source`/`manual`), so a `.TH` line's
  title/section/date/source is always dropped once fed through `events()`, even after (1) is
  fixed — `collect_doc_from_events` always builds `ManDoc { title: None, .. }`. Verified: on a
  `.TH TEST 1 "2024-01-01" "Version 1.0"` input, `build()` emits `.TH TEST 1 "2024-01-01`
  `"Version 1.0" ""`, the events()-fed streaming Writer emits `.TH UNTITLED 1 "" "" ""`.

**The "no `-fmt` crate" claim for 10 formats was verified, not assumed.** Confirmed by listing
`crates/formats/` (35 crates, alphabetically) and cross-checking against `epub`, `bibtex`,
`biblatex`, `csl-json`, `endnotexml`, `opml`, `ipynb`, `typst`, `pandoc-json`, `latex` — none of
the ten has a matching `{format}-fmt` entry; only `rescribe-read-{format}`/`rescribe-write-
{format}` adapter crates exist. For `latex` specifically: `crates/readers/rescribe-read-latex/
src/handwritten.rs` is a 895-line hand-rolled parser and `src/treesitter.rs` is a separate
662-line tree-sitter-backed parser, both directly in the adapter crate (confirmed by reading
both files in full) — a CLAUDE.md "adapter layer must never contain parsing or writing logic"
violation. Fixing it (extracting a real `latex-fmt` crate) is out of scope for this pass per the
task's explicit fence; left in `NOT_YET_AUDITED` with the violation documented inline as a code
comment. Separately, while investigating `multimarkdown` for the same question,
`rescribe-read-multimarkdown`'s `Cargo.toml` was found to depend on `pulldown-cmark` directly
(not on `commonmark-fmt`) — a further, previously-undocumented "parsing logic in the adapter"
candidate. Not investigated further (out of scope for this pass); noted inline in
`NOT_YET_AUDITED` and here for a future audit pass.

**`NOT_YET_AUDITED` restructured with per-entry reasons** (was a flat list of 28 names with no
individual justification): grouped into "no `-fmt` crate exists" (10, the list above), "has a
real crate but not yet individually audited this pass" (`native`, `csv`, `tsv`, `ris`, `rtf`,
`multimarkdown`, `pdf`), and "zero-fixture pandoc output-format variants, not prioritized per
CLAUDE.md's fixture-suite-first ordering" (`beamer`, `revealjs`, `slidy`, `s5`, `dzslides`,
`slideous`, `context`, `ms`, `icml`, `chunkedhtml`). `man` was removed from the list (now in
`CAPABILITIES`).

**Not attempted this pass**: wiring additional `NOT_YET_AUDITED` formats beyond `man` (`native`,
`rtf`, `csv`, `tsv`, `ris`, `multimarkdown` remain honest placeholders) — reading each crate's
`events.rs`/`batch.rs`/`writer.rs` in full and hand-building an `ast_to_events` projection plus
three test functions per format (the `man-fmt` pattern above) is substantial, non-parallelizable
per-format work; `rtf-fmt` (38 fixtures, the largest remaining corpus) is the natural next pick.

Verification: `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`,
and `cargo test -q` all pass (see individual commits for exact scopes exercised).

## native/csv-fmt/tsv-fmt/ris audited into the cross-API harness — all NotYetWired, no code exists (2026-08-01)

Fourth pass on `crates/rescribe-fixtures/src/streaming_harness.rs`, picking up the four
`NOT_YET_AUDITED` entries flagged in the prior (man-fmt/rtf-fmt) session as having a real
`{format}-fmt`/standalone crate but no individual audit yet: `native`, `csv`, `tsv`, `ris`.

**Verified, not assumed, before writing anything**: `crates/formats/native/src/lib.rs` (the
crate's only source file, 638 lines) and `crates/formats/{csv-fmt,tsv-fmt,ris}/src/{ast,parse,
emit}.rs` (each crate's complete source) were read in full, and each crate's directory and
`Cargo.toml` were checked. Result: all four crates implement **only** `parse()` (an eager,
whole-input AST reader) and `emit()`/`build()` (an eager AST-to-string builder) — no
`events()`/`EventIter`, no `StreamingParser<H>`/`batch` module, no event-driven streaming
`Writer`, anywhere in any of the four. Confirmed by grepping every file in each crate for
`StreamingParser`/`EventIter`/`mod events`/`mod batch`/`mod writer`/`impl Iterator`/
`fn next(&mut self)` — zero matches across all four — and by reading each `Cargo.toml`: all four
have **zero dependencies**, so there is no wrapped library that could be hiding an unexploited
streaming mode (unlike, say, an XML- or ZIP-backed crate).

**Classified as `ApiState::NotYetWired`, not `NotApplicable`, for all twelve cells** (3 APIs ×
4 formats). `NotApplicable` requires a genuine structural barrier the harness's existing
precedents (html-fmt: the HTML5 spec mandates full tree construction before any event can be
correct; commonmark-fmt: pulldown-cmark requires the complete input as one `&str`, the sole
CLAUDE.md-sanctioned exemption) — csv/tsv are flat, row-delimited formats with no cross-row
parser state (`parse.rs` never looks past the current row); RIS entries are self-contained
between `TY` and `ER  -` lines with no cross-entry state; `native` is a small recursive
tree-of-nodes debug format with a straightforward recursive-descent grammar. None of these has
anything analogous to HTML5's tree-construction algorithm or pulldown-cmark's API contract
forcing whole-input buffering — a chunk-driven reader yielding one row/entry/node at a time, and
a writer streaming rows/entries straight to a sink, are both plausible additions; nobody has
built them. Building three new APIs from scratch, times four crates, is a substantial body of
work (12 new implementations), explicitly out of scope for a harness-wiring/small-defect pass
per this task's own fence — so left as an honest, specific `NotYetWired` gap, following the
`odf-fmt` `streaming_parser` precedent ("no `StreamingParser<H>` type exists... at all yet")
for how to phrase "confirmed absent, not merely unaudited."

**A stale doc comment was corrected.** `ApiState::NotApplicable`'s own doc comment in
`streaming_harness.rs` cited "csv/tsv/ris/native have no meaningful streaming writer" as an
example of a legitimate `NotApplicable` — written speculatively, before this pass (or any prior
one) had actually read any of the four crates' source. Since the actual finding is "no
structural barrier, just not built yet," that example was wrong and has been replaced with a
pointer to html-fmt's genuinely-structural precedent, plus an explicit note that a crate with no
code and no structural barrier is `NotYetWired`, never `NotApplicable`.

**No defects found or fixed.** No streaming code existed in any of the four crates to contain a
bug, and `parse()`/`emit()` in all four are already tracked elsewhere in `docs/format-audit.md`
as production/fuzz-clean (807K-1.1M fuzz runs each); this pass did not re-audit that code, since
the task's scope was specifically the three streaming APIs.

**Tally**: `streaming_harness::CAPABILITIES` grew from 37 to **41** rows (the 4 new formats),
`NOT_YET_AUDITED` shrank from 26 to **22** entries, `KNOWN_FAILURES` is unchanged at **28**
entries (no new failing check — every one of the 12 new cells is `NotYetWired`, not a check that
runs and fails). Verified programmatically (`awk`-scoped `grep -c` over each `pub const` array
in `streaming_harness.rs`, not by hand-counting the source). No test functions were added to
`tests/streaming_apis.rs`: per the `docx`/`pptx`/`odf-fmt` precedent, a `NotYetWired` cell has no
accompanying check to wire (there is no API to check yet) — only the `CAPABILITIES` table entry
itself, plus `docs/format-audit.md`'s "Cross-API harness inventory" table and this writeup, are
the artifacts of an honest "looked, found nothing" audit.

**In scope for a future pass, not attempted here**: `rtf` (38 fixtures, wired in the immediately
preceding session but never got a corresponding `docs/format-audit.md` narrative paragraph until
this pass added one after the fact — see the "Cross-API harness inventory" section);
`multimarkdown`/`pdf` (no standalone `-fmt` crate at all, a separate, larger architectural gap);
building any of the twelve `NotYetWired` APIs found here from scratch for native/csv/tsv/ris.

Verification: `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`,
and `cargo test -q` all pass (0 test failures across the full workspace).

## man-fmt's streaming_writer fixed: genuine incremental writer + ManEvent::Metadata (2026-08-03)

**Both `man-fmt` `KnownFailure` defects tracked for `streaming_writer` are fixed.** Confirmed the
original bugs first, directly, before touching code (`cargo test -p rescribe-fixtures --test
streaming_apis -- man --nocapture`, and a scratch example printing `parse()`+`build()` on a
`.TH TEST 1 "2024-01-01" "Version 1.0"` input): `build()` from the real AST correctly emits
`.TH TEST 1 "2024-01-01" "Version 1.0" ""`, while the old events()-fed streaming `Writer` emitted
`.TH UNTITLED 1 "" "" ""` — `ManEvent` had no metadata-carrying variant, so `collect_doc_from_
events` always built `ManDoc { title: None, .. }`. Separately, `writer.rs`'s own module doc
admitted "This implementation buffers all events, reconstructs the AST, then emits" — the same
fake-streaming-writer pattern already fixed this session in t2t-fmt/pod-fmt/haddock-fmt/
fountain-fmt/bbcode/creole/dokuwiki/mediawiki/tikiwiki/twiki/vimwiki/zimwiki/markua/muse/xwiki.

**Fix 1 — `ManEvent::Metadata`.** Added a variant carrying `title`/`section`/`date`/`source`/
`manual` (all `Option<Cow<'a, str>>`), mirroring t2t-fmt's `Event::Header` (a fixed-field atomic
unit, not a generic metadata bag). `EventIter::next()` emits it exactly once, immediately after
`StartDocument` (a new `metadata_emitted: bool` field gates this). Unlike t2t's `Header` (only
emitted when at least one field is set), `Metadata` is always emitted — `build()`'s own `.TH`
line is unconditional, so events()-fed input needs the same unconditional signal.
`collect_doc_from_events` now intercepts `Metadata` before it reaches `handle_event` (which keeps
a no-op arm only for match-exhaustiveness) and threads the five fields into the returned
`ManDoc` instead of hardcoding `None` for all of them.

**Fix 2 — `Writer` rewritten as a genuine incremental writer** (`crates/formats/man-fmt/src/
writer.rs`), following the `rst-fmt`/`t2t-fmt` shape: a single shared `out: String` buffer plus a
frame stack of marks/scalars (never accumulated subtree content), flushing each completed
top-level block to the sink as soon as it closes. Reading `emit.rs` end to end found almost every
construct write-straight-through, with exactly two bounded (not O(document)) exceptions:
- **The `.TH` line** needs its five fields before it can be written, and must be first in the
  output — O(field count) buffering via `ManEvent::Metadata` above, written by a dedicated
  `write_th` method called from `write_event` before any block event reaches `process()`.
- **Heading text.** `emit.rs`'s `Block::Heading` arm uses `extract_text()`, not
  `build_inlines()` — *all* inline markup (bold/italic/superscript/subscript/link wrappers, and
  escaping) is dropped from a heading's title, only raw flattened text survives, uppercased. A
  heading's text must therefore be assembled before the `.SH`/`.SS` line can be written —
  O(heading text length), one nesting frame, not O(document size). Implemented by *not* pushing
  an `Inline`/`Link` frame for a heading's nested inline containers at all (checked via
  `in_heading()`, true whenever the stack top is a `Heading` frame) — `Text`/`Code` events append
  raw content straight into the `Heading` frame's `text: String` field, and `Start`/`EndBold`,
  `Italic`, `Superscript`, `Subscript`, `Link` become no-ops while it's on top (matching
  `extract_text`'s recursive-into-children, drop-the-wrapper behavior, including dropping a
  `Link`'s URL entirely).

`emit.rs`'s `Block::List`/`Block::DefinitionList` arms give a `Paragraph` child different framing
by *parent type*, not position — bare (no `.PP\n` marker) directly inside a `ListItem` or
`DefinitionDesc`, full `.PP\n` form everywhere else — decided at `StartParagraph`/`EndParagraph`
purely by inspecting the parent frame already on the stack, the same "known at open, applied at
close" shape t2t-fmt's writer uses for its own parent-dependent framing.

**A real, subtle bug was found and fixed while wiring this**, not present in `emit.rs` itself:
the first draft's `newline()` helper (mirroring `emit.rs`'s `BuildContext::newline`, "write `\n`
only if the buffer doesn't already end with one") checked `self.out.ends_with('\n')` directly —
correct for the tree-based builder's single never-cleared buffer, but wrong here, since `out` is
cleared after every top-level block flushes. Right after a flush, `out.is_empty()` is true, and
an empty string's `ends_with('\n')` is `false`, so `newline()` spuriously inserted a blank line
before every second-and-later top-level block (verified directly: `.TH ...\n\n.SH TEST\n\n.SH
NAME\n\n.PP\ntest\n` instead of `.TH ...\n.SH TEST\n.SH NAME\n.PP\ntest\n`). Fixed by proving (by
reading every `EndX` arm in `process()`) that every top-level block's own close logic always
writes a trailing `\n` before the flush that empties the buffer, so an empty buffer is always
logically preceded by a newline — `newline()` now treats "empty" the same as "already ends with
`\n`" (`if !self.out.is_empty() && !self.out.ends_with('\n')`).

**A second, independent, pre-existing bug was found (not fixed — out of scope for this task)**:
`parse.rs`'s `.TH` handling *also* synthesizes a `Block::Heading { level: 1, .. }` from the
title, in addition to setting `ManDoc.title` — so `build()` on real `.TH` input duplicates the
title as a spurious `.SH TITLE` line in the body (verified: `.TH TEST 1 ...\n.SH NAME\n` parses
to blocks `[Heading{level:1,"TEST"}, Heading{level:2,"NAME"}, ...]`, and `build()` on that doc
emits `.SH TEST` right after the `.TH` line, before the real `.SH NAME`). This is a `parse()`/
`build()`-level bug affecting *both* the tree builder and the (now-fixed) streaming writer
identically, since both walk the same `ManDoc.blocks` — not a streaming-API-specific defect, so
the byte-identical-to-`build()` test correctly does not flag it (both sides reproduce it the same
way). Left as a documented gap here rather than fixed, since fixing `parse()`'s title/heading
duplication is outside this task's fence (`man-fmt`'s streaming *writer*, not its reader).

**Two new tests** (`crates/formats/man-fmt/src/writer.rs`): `test_writer_byte_identical_to_
builder` (12 hand-built inputs spanning `.TH`, headings with markup/links, all inline kinds,
code/example blocks, ordered/unordered lists, definition lists, `.sp`, comments, `.IP`) and
`test_writer_th_metadata_via_events`, pinning the `.TH` fix directly. Both pass, plus the
existing `rescribe-fixtures` byte-identical sweep now passes over the full `fixtures/man/`
corpus (previously `KnownFailure`).

**Allocator instrumentation consolidated.** `man-fmt`'s `events.rs` already had its own
`#[global_allocator]`-declaring test (`test_events_no_per_call_leak`, tracking net bytes via a
crate-local `CountingAlloc`) from an earlier pass; Rust permits only one `#[global_allocator]`
per test binary, and the new writer tests need peak-memory tracking too. Extracted a shared
`crate::test_alloc` module (`#[cfg(test)]`-gated, included from `lib.rs`) with a single
`TrackingAlloc` + thread-local `CURRENT`/`PEAK` cells (thread-local, not a shared `AtomicUsize` —
the shared-counter design caused a real cross-thread flake in 12+ crates this session under
`cargo test`'s default concurrent-test execution), and rewired `events.rs`'s existing leak-guard
test to read `CURRENT` instead of declaring its own allocator.

**Peak memory measured directly** (`test_writer_peak_memory_and_throughput_report`, `#[ignore]`d,
run with `--release --ignored --nocapture`): on a synthetic 929,494-byte / 5000-section document,
the streaming `Writer` peaks at **4,235 bytes** versus `parse()`+`build()`'s **2,162,703 bytes**
— a **~511x** reduction. Throughput is ~0.83x of the (very lightweight) builder — noted honestly
per CLAUDE.md rather than chased further, the same finer-event-dispatch-granularity tradeoff
already documented for t2t-fmt. A separate `test_writer_no_subtree_reconstruction_blowup` guards
near-linear allocation count in event count (200 vs 2000 sections).

**`KNOWN_FAILURES` diff**: the `man`/`streaming_writer` entry removed entirely (was two
paragraphs describing the stacked defects above). The paired `man`/`streaming_parser`
`KnownFailure` (the `StreamingParser::emit_block()` isolated-block-reparse issue, a distinct
architectural gap) is untouched — confirmed still failing and still correctly acknowledged after
this change (`man_streaming_parser_matches_events_under_adversarial_chunking` still prints
`ACKNOWLEDGED KNOWN FAILURE [man/streaming_parser]`). `CAPABILITIES`'s `man` row's
`streaming_writer` field promoted `KnownFailure(..)` → `Wired`; the `events`/`streaming_parser`
fields and their explanatory comment block updated to describe the `Metadata` variant and point
at the still-open `streaming_parser` gap rather than re-describing the now-fixed writer.
`docs/format-audit.md`'s `man` row updated to match.

Verification: `cargo clippy -p man-fmt -p rescribe-fixtures --all-targets --all-features -- -D
warnings`, `cargo test -q` (full workspace; the only 2 failures, `ooxml-codegen`'s
`test_generate_wml`/`test_eg_definitions`, are a pre-existing missing-fixture-file issue
confirmed via `git stash` to reproduce identically on `master`, unrelated to this change), and
`cargo fmt --check` all pass clean.
