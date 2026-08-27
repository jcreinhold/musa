---
id: 205ca
slug: rhythmic-source-spelling
status: done
depends_on: [205c]
phase: 2
---

# Spell Rhythmic Durations in Source Previews

## Task

Complete the written-source spelling of a proposal's written ends: turn every measured duration into its exact written
form — a binary division, a dotted note, a tuplet, a tied chain, or a rest — so the source preview stays canonical and
still parses and compiles. This is the rhythmic half of prompt 205c's source preview, which only emitted binary
subdivisions.

## Read

- Prompt 205c's proposal facade and source-preview generator.
- Prompt 203's measured durations and the corpus fixtures (`triplet-known`, `syncopated-known`, the pedal fixture).
- Open Music Theory `118` (hypermetre and how ternary division is written) and the corpus in
  `docs/notes/research/90-midi-transcription-trial.md`.

## Design

- A written end becomes: the largest binary division it equals (24 ticks = `/4`), a dotted division when the duration is
  3/2 of a binary one (`/8.`, `/4.`, `/2.`), a `tuplet n/d { … }` group for ternary divisions (8 ticks inside a 3/2
  group at 24 ticks per quarter), a tied chain when no single form spells it, and a `rest/…` for silence between written
  ends in a voice.

- Every spelling this prompt emits is verified the way 205c verifies the whole preview: parsed and compiled under the
  destination context, so an unspellable duration is a declared loss or alternative, never an uncheckable source.

- The tie/tuplet/dot decision is deterministic — one duration, one spelling, independent of voice order or grouping.

## Target

- Exact spellings for the corpus's non-binary durations in `triplet-known`, `syncopated-known`, and the pedal fixture.
- Source previews for tuplets, syncopation, grace-like gestures, and mixed rests that parse and compile.
- Determinism and compile-verification laws for each scenario; the corpus triplet/syncopation fixtures render with their
  expected notated durations and never a pedal-extended written end.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-project --bench transcription_trial
```

Commit as `Spell rhythmic durations in source previews`.

## Stop

- No Review UI, acceptance/source mutation, batch editing, or step-entry deletion (207–208).
- No new stage outputs (voice, grouping, and pitch spelling are 205–205c's); this only spells what those stages already
  measured.
- No mutable proposal AST; no learned model or frontend musical inference.
