---
id: 98
slug: higher-order-music
status: complete
depends_on: [97]
phase: 3
---

# Higher-Order Music and Controlled Traversals

## Task

Make functions over `music` genuinely higher order and demonstrate the capability with two different musical clients:
a delayed canon taking a `music -> music` answer function, and a harmonizer taking a `pitch -> pitch` mapping. Add the
small controlled traversal needed for user-defined pitch work without exposing kernel facts, source ASTs, or the
representation of contextual music.

## Read

- `docs/language/00-semantics.md` §§ on opacity, equality, provenance, and cache soundness; `01-surface.md` examples.
- Roadmap §5.4 and prompt 34's transform laws; prompt 49's sharing and marks.
- OMT `016-intervals.md` only for the musical interpretation of the canon's intervallic answer. The higher-order and
  traversal laws are Musa definitions and must be proved from their specified semantics.

## Design

Functions remain ordinary closures: arguments/results may be functions or `music`, and partial application is a value.
Provide source-visible curried/partial `transpose(interval) : music -> music` and the other existing transforms without
duplicating block semantics.

The one new primitive is:

```text
map_note_pitches : (pitch -> pitch) -> music -> music
```

It visits written pitches in note and sounded-chord events, preserves every onset/span/extent and non-pitch field,
retains the original Origin path plus one named transform step, and deliberately leaves key signatures, harmony
annotations, text, and non-sounding marks unchanged. It is not `map ScoreFact` and not a way to inspect occurrence
order. A diatonic caller must handle `option` explicitly once prompt 101 adds scale location.

Prove/test function identity/composition, canon extent, map identity/composition, exact temporal support preservation,
and agreement between block and function spellings under contextual musical equality plus their separate provenance
theorem. Conservative instantiation identity includes definition revision, typed arguments, every observable environment
field, placement, and realization; no field may be removed without a dependency-tracking proof.

## Target

- Higher-order application over `music`; partial transform functions; private primitive registration.
- `map_note_pitches` with one centralized fact-coverage table and exhaustive tests for every `FactKind`.
- `examples/{canon-functions,harmonize-function}.musa`, written first for musicians and commented only where the type
  distinction is not audible from the source.
- `crates/musa-compiler/tests/higher_order_music_laws.rs`: reference examples, laws, provenance, negative type cases,
  and cache-key separation cases.
- Keyword/LSP hover documentation for the new source-visible constructs, sourced from `musa-language`.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-language -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-language -p musa-lsp -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/canon-functions.musa
cargo run -p musa-cli -- check examples/harmonize-function.musa
cargo bench -p musa-compiler
```

Commit as `Add higher-order functions over music`.

## Stop

- No general occurrence iterator, `ScoreFact` callback, mutable event object, source-AST macro, or piece/voice value.
- No automatic chord choice or key inference.
- Do not rewrite harmony annotations during note mapping; comparison belongs to assertions/analysis.
- No cache optimization justified only by the new key's size.
