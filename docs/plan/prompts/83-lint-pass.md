---
id: 83
slug: lint-pass
status: done
depends_on: [56, 73, 78]
phase: 3
---

# The Lint Pass

## Task

Teach the compiler to warn about notation that is spelled correctly and still misleads: names nobody speaks, markings
that change nothing, gradual changes that arrive nowhere, and repetition written as copies instead of as a motif. The
rules are the machine-checkable subset of `docs/rules/style-guide.md`, and they travel as ordinary `Warning` diagnostics
with ordinary `Fix` values, so every surface that already shows diagnostics — `musa check`, the language server, both
editors, the desktop — shows lints with no new plumbing.

## Read

- `docs/rules/style-guide.md` — the standards, in prose. Each rule below cites its section; the guide is the authority
  when a rule is too coarse.
- Prompt 56 — the diagnostic document: `Code`, `Label`, `Fix`, and the rule that a fix is offered only when it is
  certain. Codes are a shipped interface; `musa explain <code>` must answer for each new one, and the deliberately
  exhaustive match in `musa_project::explain` is what keeps that honest.
- Prompt 78 — the reference index: declarations and resolved uses, by `NameKind`. "Nobody speaks this name" is a
  question it already answers.
- Prompt 73 — the gradual tempo change: `Ramp { to: Option<u32>, over }`, and why a worded ramp (`tempo "rit." over
  2/1;`) is a designed spelling, not a missing arrival.
- `crates/musa-language/src/formatter.rs` — the comment-attachment convention a suppression directive leans on, and the
  law that formatting rewrites whitespace only. Lint fixes are not formatting; they delete or they do nothing.

## Design

### One pass, in the compiler

`crates/musa-compiler/src/lint.rs`, `pub(crate)`, called from `elaborate_parsed` after the studio lowers and only when
no error-severity diagnostic has been reported — advice about a piece that does not compile is advice about a piece that
does not exist (the same doctrine `check_measure_sanity` follows). The pass reads the parse tree, the reference index,
the snapshot, and the studio — all in scope at that point — and appends warnings.

One engine, not one per layer. A CST-only rule and a semantic rule share the registry, the suppression mechanism, and
the emission path; splitting them to match where their *evidence* lives would duplicate all three for no caller.
musa-language stays silent: it does not know what a name is for.

### The rules

Four, one new `Code` variant each — the code *is* the rule's name, so suppression, `musa explain`, and the style guide
cite one vocabulary. Each is silent on every file in `examples/`, which is a law, not a hope (see Check).

Guide §3 (a change arrives somewhere) has no rule here: its dishonest spelling — a metronome'd ramp with no `to` — is
the compiler's own error (prompt 73: `this gradual tempo change goes nowhere`), and lints warn on what compiles. One
test pins that boundary so a later reader does not add the rule twice.

- `unused-material` — a `motif` or `fragment` whose reference-index entry has a declaration in this document and no uses
  (guide §1). A named `bar` is deliberately *not* checked: it plays where it stands, so its name is an address for edit
  sites and provenance, not a promise of reuse — prompt 76's realization tests insert exactly such bars, and the rule's
  first draft learned from them. Fix: delete the declaration.
- `unassigned-patch` — a `patch` declared in this document that no `assign` speaks (guide §1). The declaration span
  comes from the reference index (`NameKind::Patch`); the assignments from the `StudioSpec`. Imported patches are not
  this document's business. Fix: delete the patch.
- `redundant-marking` — a `tempo`, `meter`, or `key` statement whose written value equals the value already in force in
  its scope (guide §2). Driven from the AST in source order, tracking the in-force value per (scope, kind); a scope is
  the piece or one part. A statement carrying a ramp is exempt (whether it is a no-op depends on the clock, not the
  text), and a `senza` block resets the in-force meter — it changes the meter implicitly and restores it without a
  statement, which text cannot see. Motif bodies are templates, not sequences: they are not walked. Fix: delete the
  statement.
- `copied-bars` — three or more identical bars among the direct children of one voice block (guide §4). Identity is the
  bar's tokens ignoring whitespace and comments. Bars nested in `repeat`, hairpin, and `senza` blocks are not compared —
  a `repeat` is honest repetition, and a nested bar is someone's argument. No fix: naming the motif is the composer's;
  `help` points at `use` and the secondary labels point at every copy.

Deletion fixes delete the statement's whole line — leading indent and trailing newline included — so applying one leaves
the file formatted.

### Suppression

`// musa:allow(code, …)` in the leading comment trivia of the statement or declaration the lint would fire on — the same
attachment the formatter keeps (guide §5). One helper, consulted at emission, so no rule can forget it. No file-level
waiver, no severity knobs, no configuration file.

### Surfaces

- `musa check` shows the warnings as it shows every diagnostic. `musa check --fix` applies the certain fixes of
  *warnings* in place (errors are reported, never rewritten — a file that does not compile is a conversation, not a
  draft), then saves; it is `ProjectCommand::ApplyEdits` plus `Save`, which is why the session needs nothing new.
- The language server needs nothing: warnings publish, and a diagnostic's fixes already arrive as quick fixes (prompt
  77). One law test proves a lint warning and its quick fix make the round trip.

## Target

- `crates/musa-compiler`: `src/lint.rs` (the pass, the suppression helper, the deletion-fix builder); the four `Code`
  variants in `diagnose.rs`; the call in `elaborate.rs`.
- `crates/musa-project`: explanations for the four codes (the exhaustive match requires them).
- `crates/musa`: `--fix` on `check`, one line of usage.
- `crates/musa-lsp`: one law test — a lint warning publishes with its quick fix.
- Tests: `crates/musa-compiler/tests/suite/lint_laws.rs` — one firing fixture per rule, the suppression law, the
  fix-application law (apply the fix, recompile, silence), and the corpus law: every valid file in `examples/` compiles
  with zero warnings.
- `docs/rules/style-guide.md` and this prompt.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa -p musa-lsp -- -D warnings
cargo fmt --check
musa check examples/*.musa examples/album/pieces/*.musa   # no warnings
```

## Stop

- No lint configuration: no file, no severity knobs, no file-level suppression.
- No new rules beyond the four. In particular no unused-part or unused-voice rule: voices are declaration-only in the
  reference index, and "empty" is a different claim than "unused" (guide §1 names names, not bodies).
- No linting a piece with errors, and no rewriting one either: `--fix` touches warnings only.
- No formatter changes, and no lint that rewrites rather than deletes: restructuring source is a structured edit's job,
  with provenance.
- No `musa lint` subcommand: `check` is where diagnostics are read; a second spelling of the same list is a second
  vocabulary.
- No grammar, LSP-protocol, or editor changes: the surfaces already carry this.
