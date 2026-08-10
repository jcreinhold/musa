---
id: 102
slug: chord-classes-and-voicings
status: done
depends_on: [35, 101]
phase: 3
---

# Chord Classes, Triads, Voicings, and Sounded Chords

## Task

Add typed generative harmony without making chord symbols the ontology of notes. Separate chord annotations, rooted
spelled chord-class content, the major/minor-triad refinement, and absolute voicings; then let a composer realize a
chosen voicing as `music` with register, spacing, doubling, and bass made explicit.

## Read

- `docs/language/03-musical-domains.md`; prompt 35's existing `ChordSymbol` and harmony lane.
- OMT `017-triads.md`, `018-seventh-chords.md`, `019-inversion.md`, `022-chords-in-satb-style.md`,
  `075-chord-symbols.md`, and `076-jazz-voicings.md`. Note OMT 075's distinction between chord symbols and Roman
  numerals and OMT 019's distinction between root and bass.
- Every renderer/project caller of the current public chord-symbol structs before changing their boundary.

## Design

Use distinct types and checked constructors:

- `chord_symbol` — existing written annotation and its exact text/structure;
- `chord_class` — rooted spelled pitch classes, quality/extensions, optional designated bass pitch class;
- `triad` — a refinement proving major/minor triadic content, the domain prompt 106 needs;
- `voicing` — finite ordered absolute written pitches with bass, spacing, and doublings fixed.

Building a chord class chooses no register. Inversion chooses/designates a bass and validates member requirements for a
true inversion; slash bass outside the class remains explicitly a slash-bass construction. Voicing policies are named
functions returning `option voicing` or diagnostics when their range/spacing preconditions fail. `play(voicing,duration)
-> music` sounds exactly those pitches. Musician sugar such as `stack c4 maj7/2` must desugar to one documented close-
position policy with the absolute root fixing register; `stack c maj7/2` is incomplete and rejected.

Chord spelling follows the diatonic letter stack and interval quality from OMT 017–018, not pitch-class arithmetic
that can spell a third as a second. Property tests compare a small reference formula across roots/qualities and cover
inversions, extensions, omissions, and backend round trips. No constructor asserts that separately authored notes match
an annotation; prompt 114 adds that explicit claim.

## Target

- Typed chord/voicing values, constructors, source syntax/desugaring, diagnostics, and `std::harmony`/`std::voicing`.
- Migration of current `ChordSymbol` callers without exposing evaluator values or adding a theory crate.
- `examples/chord-voicings.musa`: one chord class in two inversions and three voicings, including a jazz rootless
  policy with its omitted root explicit.
- `crates/musa-compiler/tests/chord_construction_laws.rs`: spelling formula, refinement, inversion/root/bass,
  voicing-policy preconditions, and annotation independence.
- MEI/LilyPond/MusicXML/MIDI tests proving written spelling and sounded notes survive separately.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-render -p musa-project
cargo clippy --all-targets -p musa-compiler -p musa-render -p musa-project -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/chord-voicings.musa
cargo run -p musa-cli -- render examples/chord-voicings.musa --to musicxml -o /tmp/chord-voicings.musicxml
cargo bench -p musa-compiler
```

Commit as `Separate chord classes from voicings`.

## Stop

- No chord-symbol-to-notes implication, automatic voicing, Roman-numeral analysis, or global chord correctness check.
- No root-equals-bass assumption.
- No PLR/SNH — prompt 106 — and no tonal harmony constructors — prompt 107.
- No public fields exposing chord storage; callers use domain operations.
