---
id: 115
slug: schemas-and-harmonization
status: done
depends_on: [98, 107, 110, 114]
phase: 3
---

# Finite Schemas, Sequences, and Harmonization Functions

## Task

Use the total functional language to implement musically named, finite generative algorithms: diatonic sequences,
selected galant schemas, the Rule of the Octave, and scale harmonization patterns. This is the proof that Musa's
expressivity lives in ordinary functions and finite folds rather than an expanding catalog of compiler constructs.

## Read

- OMT `033-galant-schemas.md`, `034-galant-schemas-summary.md`,
  `035-galant-schemas-the-rule-of-the-octave-and-harmonizing-the-scale-with-sequences.md`,
  `049-diatonic-sequences-in-middles.md`, and `069-chromatic-sequences.md`.
- OMT `039-embellishing-tones.md` where a schema needs a passing/suspension distinction.
- Prompts 98, 101–102, and 107; every output remains explicit chord class/voicing/music and reads a named scale/key.
- Prompt 110 for the module tree these files live in, and prompt 108 for the discipline any new primitive must discharge
  — which is the sharper form of this prompt's existing "stop and demonstrate" rule.

## Design

Implement each schema as a source function over explicit inputs and a finite count/range. Separate harmonic skeleton
from voicing and rhythm: a descending-fifths sequence returns chord classes/degrees; a caller supplies voicing and
duration functions to produce `music`. Rule-of-the-Octave ascent and descent are different named functions because OMT
35 gives direction-dependent harmonizations. Chromatic variants carry their alterations explicitly.

For every exported function, source docs cite the exact OMT section and state: input domain, output shape, direction,
scale/key assumption, inversion/voice-leading policy if any, and what is not guaranteed. Encode finite index formulas in
a small reference table and prove with property tests that generated roots/degrees follow the pattern modulo the scale
cycle, counts determine finite extent, and zero/one counts have normal meanings.

The musician-facing fixtures must read as named musical intentions, while the library source remains legible to a
developer learning Musa's functions, folds, options, and higher-order parameters. If an algorithm requires a hidden
primitive, stop and demonstrate what information source code cannot express before adding one.

## Target

- `stdlib/src/tonal/schemas.musa` and `stdlib/src/tonal/sequences.musa`, declared by `tonal/mod.musa`, with source
  documentation and explicit exports. `sequences.musa` already exists and was committed *orphaned* — reachable from no
  import — which is the defect prompt 110 exists to make impossible; this prompt is where it becomes real library.
- `examples/{diatonic-sequences,rule-of-the-octave}.musa`, rendered with at least two voicing/rhythm policies.
- `crates/musa-compiler/tests/suite/schema_generation_laws.rs`: OMT examples, reference index patterns, direction,
  extent, finite boundaries, and source-library-versus-reference agreement.
- A primitive-ownership test proving this prompt adds no compiler primitive unless a documented repair was necessary. If
  one is necessary, it discharges prompt 108's D1–D4 and appears in that registry; there is no longer a route by which a
  primitive enters without one.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-render
cargo clippy --all-targets -p musa-compiler -p musa-render -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/diatonic-sequences.musa
cargo run -p musa -- check examples/rule-of-the-octave.musa
cargo bench -p musa-compiler
```

Commit as `Add finite schema and harmonization libraries`.

## Stop

- No schema keyword, kernel node, hidden recursion, unbounded generator, or inferred harmonic analysis.
- No one “best” voicing or rhythm attached to a harmonic skeleton.
- No claim that a historical schema exhausts how a passage may be heard.
- Do not add all named OMT schemas; implement the materially different representatives named in Target, then extend by
  ordinary library changes when a concrete composition needs another.
