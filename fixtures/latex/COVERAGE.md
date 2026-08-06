# LaTeX Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

## Scope of this suite (read before adding to or auditing it)

This suite covers **`latex-fmt`'s actual resolution model** (see
`crates/formats/latex-fmt/src/parse.rs`'s module docs for the full
rationale), not a semantic-modeling checklist of "which named LaTeX
commands are recognized." That distinction matters here more than for most
other formats in this repo:

- The **only** source of "this control sequence has resolved structural
  meaning" is an in-document `\def`/`\edef`/`\gdef`/`\xdef`/`\newcommand`/
  `\renewcommand`/`\providecommand`/`\newenvironment`/`\renewenvironment`
  definition, tracked with real TeX grouping-scope semantics. There is
  **no built-in table of "standard LaTeX commands."** `\section`,
  `\textbf`, `\cite`, `\item`, and everything else the LaTeX kernel,
  document class, or a package would normally define is raw-preserved
  (`ControlSymbol`/`RawEnvironment` → IR `raw_block`) with an `Info`
  fidelity warning, *unless the document itself locally defines it* —
  which is virtually never the case in real documents. This is a
  deliberate, explicit design resolution (see the session transcript this
  crate was built in / `TODO.md`), not a gap being tracked toward 100%
  named-command coverage.
- Resolving `.cls`/`.sty` package content (so `\section` etc. would
  resolve the way a real LaTeX engine sees it) is priority-#2/#3 of a
  larger, explicitly deferred design — not attempted by this crate at all
  yet. A fixture suite entry for "`\section` becomes a `heading` node" is
  **not a valid target for this suite** until that work lands; adding one
  now would assert behavior this crate deliberately does not have.
- **This suite therefore tests the resolution *mechanism*** (scope-stack
  push/pop at group/environment/mandatory-arg-group boundaries, `\gdef`/
  `\xdef` global escape, shadowing, redefinition) **and the raw-preserve
  fallback** (including for extremely common commands like `\textbf`/
  `\section`, deliberately included so the fallback path is never
  under-tested just because the construct is common) — not a catalog of
  modeled LaTeX vocabulary.
- The rescribe IR mapping (`crates/formats/latex-fmt/src/rescribe.rs`) is
  itself narrow by design (see that file's docs): plain text →
  `paragraph`/`text`, everything else → `raw_block` carrying re-emittable
  source in a `latex:source` property, plus a `FidelityWarning` when the
  diagnostic is `latex::unresolved-*`. `expected.json`'s assertion schema
  has no way to assert on warnings (only on the `Document` tree) —
  warning-emission itself is verified by `latex-fmt`'s own unit tests
  (`crates/formats/latex-fmt/src/parse.rs`, `src/rescribe.rs`), not by
  this fixture suite.

## Resolution mechanism

- [x] plain text → `paragraph`/`text` — `plain-text`
- [x] `%`-comment → its own `raw_block` — `comment`
- [x] unresolved (no in-document definition) command raw-preserves — `undefined-command-raw-preserve`
- [x] unresolved command that happens to be an extremely common one (`\section`) still raw-preserves — `undefined-section-raw-preserve`
- [x] unresolved environment raw-preserves its full `\begin`...`\end` span — `undefined-environment-raw-preserve`
- [x] `\newcommand` definition + a resolved use — `newcommand-definition-and-use`
- [x] `\newenvironment` definition + a resolved use — `newenvironment-definition-and-use`
- [x] inline `$...$` math raw-captured — `math-inline-raw-preserve`
- [x] display `$$...$$` math raw-captured — `math-display-raw-preserve`
- [x] `\verb|...|` raw-captured (tokenizer-level closed set) — `verb-raw-preserve`
- [x] `verbatim` environment body raw-captured (tokenizer-level closed set) — `verbatim-environment-raw-preserve`
- [ ] `\[...\]` display math — (missing; not currently recognized at all — no built-in command-name table means `\[`/`\]` are just unresolved control symbols, see `crates/formats/latex-fmt/src/lib.rs`'s "Known limitations")
- [ ] `lstlisting` environment (tokenizer-level, same closed set as `verbatim`) — (missing, mechanically identical to `verbatim-environment-raw-preserve`, not yet added as its own fixture)

## Scope-stack semantics (the actual differentiator of this design)

- [x] resolved macro argument containing a nested `{...}` group — `int-newcommand-nested-group-arg`
- [x] `%`-comment immediately adjacent to an unresolved command — `int-comment-adjacent-to-command`
- [x] `\gdef` inside a `{...}` group escapes to the global scope frame — `rare-gdef-global-escape`
- [x] nested `\def` shadows an outer definition of the same name, restored on scope exit — `rare-nested-shadowed-def`
- [x] `\newcommand` then `\renewcommand` of the same name — `rare-renewcommand-redefinition`
- [x] `\newcommand` with a declared optional leading argument — `rare-optional-arg-default`
- [ ] `\def` arity inferred from `#<digit>` count, then used — (missing; covered by `parse.rs`'s `def_infers_arity_from_param_count` unit test but not yet as a fixture)
- [ ] optional-argument content that itself references an enclosing-scope definition — (missing; covered by `parse.rs`'s `optional_arg_sees_enclosing_scope` unit test but not yet as a fixture)

## Adversarial

- [x] empty document — `adv-empty`
- [x] unresolved environment with no matching `\end` (truncated input) — `adv-unknown-environment`
- [x] unresolved command with an unterminated trailing group — `adv-unterminated-group`
- [x] `\begin` with no following `{name}` at all — `adv-malformed-begin`
- [ ] mismatched `\end{other}` for a `\begin{name}` — (missing; covered by `parse.rs`'s diagnostic-emission logic, `latex::mismatched-end`, but not yet as a fixture)
- [ ] a document that redefines one of the nine reserved definer names itself (e.g. `\newcommand{\def}...`) — (missing as a fixture; the *parser's* handling — unconditional definer-name dispatch always wins — was exercised and found consistent via the `fuzz_latex_fmt_roundtrip` generator's `definable_name` exclusion, see that file's docs, but not captured as a standalone fixture)

## Pathological

- [x] 60 levels of nested `{...}` groups — `path-deep-nested-groups`
- [x] a long run of plain words stays one merged text run (tokenizer maximal-munch) — `path-many-top-level-nodes`
- [ ] very deep in-document scope nesting (100+ levels) exercising `command_scopes`/`env_scopes` stack growth — (missing)
- [ ] a document defining hundreds of distinct macros (scope-frame HashMap growth) — (missing)

## Not applicable to this suite

Everything in `fixtures/spec.md`'s dimension list not listed above as
"missing" is out of scope for the reasons in the "Scope of this suite"
section — not silently dropped, deliberately excluded because this
crate's actual behavior doesn't have anything for those assertions to
check yet (they'd all currently assert `raw_block`, which the already-
present `undefined-*` fixtures already establish as the fallback
behavior; adding one per common LaTeX command would not exercise anything
new).
