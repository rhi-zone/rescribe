---
name: mine-session-transcripts-for-decisions
description: "INCOMPLETE / experimental — validated exactly once. Mine prior Claude Code session transcripts for design-decision moments (the reasoning behind a convention, not just its existence) using normalize sessions messages --grep. Use when backfilling an ADR log, recovering the rationale behind an existing convention, or answering \"why is it like this\" when code and comments don't say. Central lesson: grep reasoning language (rejected-alternative phrasing, tradeoff words), not domain/IR vocabulary — vocabulary greps mostly return code dumps."
---

# Mine session transcripts for decisions

**Status: incomplete / experimental.** This codifies a technique used exactly once
in this repo — one topic (rescribe IR design decisions), one session, to populate
`docs/adr/`. It is not a proven workflow. Known gaps are listed at the bottom;
read them before trusting this beyond its one validated use.

## When to reach for this

- Backfilling or populating an ADR (architecture decision record) log for a project
  that has none.
- Recovering *why* an existing convention exists when the code and its comments are
  silent on rationale.
- Answering "why is it like this" when the only record of the reasoning, if it
  exists at all, is buried in a past agent session rather than in any committed doc.

This is not for finding *what* was built (read the code/git log for that) — it's
for finding the *argument* that led to a design choice.

## The tool

```bash
normalize sessions messages --limit 0 --role all --grep '<pattern>' [--json] [--context N]
```

Confirmed present at `~/git/rhizone/normalize/target/debug/normalize` and confirmed
working via `normalize sessions messages --help` as of this writing. Useful flags
beyond `--grep`: `--role {user,assistant,tool,system,all}`, `--context N` (lines of
context around a hit), `--since`/`--until`/`--days` to bound the window, `--session`
to pin one transcript, `--json`/`--jsonl` for machine parsing. Re-check `--help`
before relying on exact flag names — this skill has not been re-validated against
tool changes.

## The central finding: pattern selection dominates results

Two grep strategies were tried; only one worked.

**Domain/IR vocabulary greps failed.** Searching for terms like `footnote_ref`,
`NodeKind`, `namespace` returned mostly raw code dumps and near-zero signal —
these words appear constantly in ordinary implementation chatter (writing code,
reading code, error messages), not just at the moment a decision was made.

**Reasoning-language greps worked.** Searching for the *linguistic markers of a
decision being weighed* surfaced the genuine decision moments:

- A specific rejected-alternative phrasing tied to the actual thing that was
  rejected, e.g. `events().collect` (found the decision to reject
  `parse() = events().collect()` as an implementation strategy).
- A specific claim under dispute, e.g. `span.*includ.*delimiter` (found the span
  boundary semantics decision).
- General tradeoff/rejection phrasing: `instead of`, `rather than`.

Even with good patterns, hit rate was roughly **one third genuine decision content,
two thirds noise** — this is not a high-precision tool, budget triage time
accordingly.

**A negative result is itself information.** Grepping for the `style:`/`layout:`/
`{format}:` property-namespace convention found *no* decision moment anywhere in
session history. That doesn't mean the search failed — it means the decision either
predates the available transcript window or was adopted without ever being
explicitly argued out loud. Record "no decision moment found" as a real finding,
not a gap to keep digging at indefinitely.

## Practical approach

1. **Start from the artifact, not the tool.** Identify the specific convention or
   decision whose rationale you want (a property namespace, a rejected API shape,
   a span-semantics rule) before writing any grep pattern.
2. **Write patterns as reasoning language, not vocabulary.** Prefer a distinctive
   phrase tied to the specific rejected alternative or disputed claim over a
   general domain term. If you only have a domain term, pair it with a tradeoff
   word (`instead of`, `rather than`, `because`, `the problem with`) to cut noise.
3. **Search broadly first, deep-read second.** Run the grep with `--role all` and
   a wide time window, then deep-read only the highest-signal hits — don't try to
   read every hit at full context.
4. **Triage: a genuine decision moment shows a tradeoff being weighed and
   resolved** — an alternative named, a reason given for rejecting or accepting it,
   and a resolution. A bare mention of the term, a code dump containing it, or an
   agent narrating what it's about to do is not a decision moment — discard it.
5. **Cross-check every mined "decision" against the actual committed code before
   writing it up.** This step is mandatory, not optional. A transcript alone
   cannot distinguish a proposal that was later rejected from one that shipped —
   an agent's speculation ("we could do X instead") can read exactly like a
   settled decision out of context. Verify the claimed outcome is what's actually
   in the codebase before recording it as an ADR.
6. **Absence is a valid outcome.** If no decision moment turns up after reasonable
   pattern variation, write that down (as here, for the property-namespace
   convention) rather than treating it as an unsolved search problem.

## Known gaps (read before trusting this beyond its one use)

- **Validated on exactly one repo, one topic.** Only exercised for rescribe IR
  design decisions. Unknown how well it generalizes to other kinds of decisions
  (build tooling, process, naming) or other projects.
- **No cross-project search guidance.** Everything above assumes searching one
  project's transcript history. Searching across multiple projects'
  histories (`--all-projects`, if supported) hasn't been tried for this purpose.
- **Mediocre signal/noise, no pattern library yet.** ~1-in-3 hit rate even with
  good patterns; no systematic collection of "patterns that reliably find
  decisions" has been built up beyond the handful of examples above.
- **Transcript retention window is unknown.** It isn't known how far back
  transcripts actually go, or whether old sessions get pruned — a negative
  result could mean "no decision was ever argued" or "the session that argued
  it is no longer in the corpus." These are indistinguishable without external
  evidence.
- **Attribution is unverified without the code cross-check.** A transcript alone
  does not prove a mined "decision" shipped — see step 5 above. Treat that
  cross-check as load-bearing, not a nice-to-have.
