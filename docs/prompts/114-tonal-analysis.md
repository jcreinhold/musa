---
id: 114
slug: tonal-analysis
status: pending
depends_on: [107, 113]
phase: 3
---

# Evidence-Based Tonal and Cadential Analysis

## Task

Implement opt-in tonal analysis over closed facts: chord-segment candidates, Roman-numeral labels relative to explicit
or candidate keys, cadence evidence, and tonicization/modulation candidates. Return alternatives and evidence when the
music is ambiguous; verify explicit annotations when asked; never turn an analysis into the notes' ontology.

## Read

- OMT `020-roman-numerals.md`, `036-introduction-to-harmony-cadences-and-phrase-endings.md`, `037`–`048` on
  phrase-model harmonic functions, `050-tonicization.md`,
  `051-extended-tonicization-and-modulation-to-closely-related-keys.md`,
  `052`–`056` on phrase/form context, and `061-modal-mixture.md`.
- OMT 051 §“Tonicization versus modulation”: duration/formal emphasis make this a continuum, not a syntax-level
  bit.
- Prompts 35, 65, 107, and 110; key/harmony/phrase annotations are evidence, not unquestionable truth.

## Design

Segment simultaneities by an explicit request policy (attacks, sustained coverage, or harmony-lane windows). Roman
analysis takes a key candidate and uses prompt 107's constructive spelling as a reference; it reports exact, incomplete,
non-chord-tone, and conflicting fits separately. Cadence classification requires the OMT-defined harmonic/melodic/formal
evidence and states which evidence is missing rather than labeling any V–I a perfect authentic cadence.

Key-region analysis returns candidates supported by signatures, scale membership, emphasized sonorities, phrase
boundaries, and duration. A written key fact is strong evidence but may coexist with tonicization/mixture. Tonicization
and modulation findings state the OMT 050/051 criteria they satisfy; no fixed bar-count threshold is invented. When
criteria underdetermine the distinction, return both candidates and the evidence a human must judge.

Algorithms are deterministic and benchmarked on committed excerpts/constructed fixtures whose intended readings are
documented. Tests include chromatic non-chord tones, mixture, pivot-chord ambiguity, direct modulation, tonicized
dominant, deceptive cadence, half cadence, imperfect/perfect authentic cadence, and a passage for which “unknown” is
the only honest answer.

## Target

- Tonal/chord/cadence analysis kinds in the prompt 113 service and CLI renderers.
- `examples/analysis/` fixtures with source notes documenting OMT chapter/section and accepted candidate sets.
- `crates/musa-compiler/tests/tonal_analysis_validation.rs`: evidence-level assertions, ambiguity/unknown cases,
  annotation comparison, and no semantic mutation.
- `docs/language/07-analysis.md`: algorithms, assumptions, OMT citations, known limits, and false-positive corpus.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-cli
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-cli -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- analyze examples/analysis/pivot-ambiguity.musa --kind tonal --format text
cargo bench -p musa-compiler
```

Commit as `Add evidence-based tonal analysis`.

## Stop

- No single mandatory key/Roman/cadence label when multiple candidates survive.
- No score rewriting, auto-correction, reharmonization, or global compile warning.
- No machine-learning score, opaque confidence percentage, or unstated corpus prior.
- No claim that tonicization versus modulation is decided by duration alone.
