---
id: 127e
slug: source-language-clean-break
status: pending
depends_on: [127da]
phase: 3
---

# Delete Contextual Music and Cut the Surface Over

## Task

Move all current notation and theory programs onto ordinary inferred values, `EventTrack`, and `Machine`. Delete the
universal contextual `Music` value and every compiler path whose only purpose is closing it later.

## Read

- The prompt-127a language specification, prompt 127da's proved adapter contract, research `05-selected-calculus.md`
  §§7–10, and `17-final-review.md`.
- Current contextual `Music`, closure, binding-table, controlled-operation, note/block elaboration, quotation, template,
  stdlib, example, and handbook code.
- Open Music Theory files cited by each retained notation or theory adapter.

## Design

Keep the fixed reader, indentation-based file structure, compiler order, syntax data, path-aware fold, source maps, and
adapter contracts proved by prompt 127da. The notation surface is an adapter into ordinary inferred expressions, not a
second semantic core.

Keep the common staff spelling already selected: `c4/4`, `c4/4.`, `[c4 e4 g4]/2`, and `rest/8`. Use `c4(3/8)` when the
source gives an exact duration without claiming a conventional written note value. Written rhythm and exact temporal
length remain different data even when they cover the same span. Do not add sticky duration, relative octave, or
type-directed note literals.

Written pitch is explicit data. Meter, key, clef, and other contextual facts are explicit inputs or track payloads.
Functions that need a context take it as an argument and return an ordinary value or `Result`; no closure reads hidden
placement or mutable musical context. Named bars, motifs, repeats, transforms, templates, and quotation become ordinary
data/functions or event-track operations.

Delete the `Music` type, contextual instantiation judgment, partial controlled built-ins, old source spellings that
require them, and their caches. Rewrite stdlib, examples, book code, tests, LSP facts, and desktop fixtures. Do not keep
an accepted alias or hidden compatibility evaluator. A rejected old form may receive a diagnostic and mechanical fix,
but it never compiles.

Preserve source spans and derivation records through adapter expansion and track construction. Notation and audio-first
programs are equal citizens; a project need not export a score.

Use prompt 127da's complete staff and studio trials as migration tests. Neither adapter receives compiler privilege.

## Target

- One surface-to-resolved-to-inferred-to-value path with notation adapters before inference.
- Complete deletion of contextual `Music` and old controlled-call machinery.
- One public adapter contract, the two complete adapter trials, and source-map/hygiene/edit round-trip tests.
- Migrated stdlib, corpus, tools, handbook, and snapshots; exact expected-change ledger for intentional clean breaks.
- Complete notation-led and audio-led source fixtures with inferred types and origin paths.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
cd editors/tree-sitter-musa && tree-sitter test
! rg -n 'ContextualMusic|contextual `?Music|close_music|BuiltinValue' crates docs/rules docs/book examples stdlib
```

Commit as `Cut the source language over to tracks and machines`.

## Stop

- No compatibility evaluator, accepted old alias, type-directed macro, general syntax macro system, dependent type, or
  new theory domain.
- No audio step execution or scheduler.
- No claim that one notation adapter covers every musical practice.
