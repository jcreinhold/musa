---
id: 107
slug: tonal-harmony-construction
status: in-progress
depends_on: [99, 101, 102]
phase: 3
---

# Tonal Harmony as Typed Construction

## Task

Add typed tonal-harmony values and standard-library constructors for diatonic Roman-numeral chords, inversions,
secondary/applied chords, modal mixture, predominant chromatic chords, and altered/extended dominants. These functions
construct explicit chord classes from an explicit key/scale; they neither infer an analysis nor sound notes until a
separate voicing policy is chosen.

## Read

- OMT `020-roman-numerals.md`, `021-figured-bass-and-roman-numerals-with-figures.md`,
  `036-introduction-to-harmony-cadences-and-phrase-endings.md`, `037`–`048` on phrase-model functions and common
  diatonic/chromatic sonorities, `050-tonicization.md`, `061-modal-mixture.md`, `062-neapolitan-sixth-chords-ii6.md`,
  `063-augmented-sixth-chords.md`, and `071-altered-and-extended-chords.md`.
- OMT 075's chord-symbol versus Roman-numeral distinction; prompts 101–102.

## Design

`roman` is a typed scale-degree/function/quality/inversion description relative to a declared tonal context, not a
string and not a pitch collection by itself. `realize_roman(key, roman) -> chord_class` spells members diatonically and
applies explicit alterations. Secondary dominants/leading-tone chords carry their tonicized target. Mixture carries the
borrowed parallel collection. Neapolitan and augmented-sixth constructors name their altered degrees and context;
dominant extensions/alterations name which members are present rather than assuming a universal jazz/classical set.

Writing those constructors needs one operation the scale domain does not have yet: the pitch class a scale degree
names, without a register. A Roman numeral is register-free by definition, and the members it stacks are spelled from
the collection rather than from a fixed interval pattern — `vii°` in C major is `b–d–f` because those are the seventh,
second, and fourth degrees, not because a diminished triad was transposed there. Today every route from a degree to a
sounding name goes through a `frame`, which is built by `pitch_frame(scale, pitch)` and so demands an absolute tonic
pitch; and no primitive turns a `pitchclass` back into a `pitch`, so a library function could only supply that pitch as
a hardcoded octave literal. That literal would be wrong for a scale rooted on `eb` and would be exactly the implicit
register the Stop list forbids. `scale_class(scale, ordinal) -> option[pitchclass]` therefore joins the scale
primitives beside `scale_pitch`: the same ordered-offset table answering the register-free question, absent when the
ordinal is outside the collection's period. It reads private scale state, which is why it is a primitive and not
`.musa`.

Spelling the members is a second operation, and it is not the same one. A diatonic chord's quality is not chosen and
then transposed; it *falls out* of the collection, which is why `ii` is minor in major and `II` is major in Dorian
without either being stipulated. Stacking degrees `n`, `n+2`, `n+4` gives three spelled classes, but nothing turns
three classes back into a `chord_class`: there is no interval between two pitch classes, no equality on `pitchclass`
or `interval` with which to test a candidate template, and `chord_on` re-roots a template rather than building one.
`scale_chord(scale, degree, members) -> option[chord_class]` is therefore the second primitive — the diatonic stack of
`members` thirds from `degree`, named as the chord type it turns out to be, and absent when the collection stacks to
no nameable type. It hides the scale's offsets and the chord-type table at once, which is exactly why neither half can
be written in source. Chords that are *not* diatonic — Neapolitan, augmented sixth, mixture, altered dominants — name
their altered degrees explicitly and build on `scale_class` plus `chord_on`, so both primitives are load-bearing and
neither subsumes the other.

The `roman` refinement's own constructor and accessors are not part of that budget and are not theory. They are the
wiring every refinement in this crate already has — `chord_triad`/`triad_chord`/`triad_major` for the triad,
`voicing_of` and its readers for the voicing — because a refinement's invariant is the one thing source cannot check
for itself. A numeral carries an ordinal, a member count, and an inversion, and no quality: in a tonal context the
quality is the collection's, so storing one would let a caller write a numeral that disagreed with the collection it
is realized against, with no honest answer to which wins. Realization is therefore ordinary `.musa` — the diatonic
stack, then the inversion — and not a third primitive.

Two chromatic sonorities the Target names cannot be built from `scale_class` and `chord_on` as the paragraph above
assumes, because `chord_on` re-roots a chord type and prompt 102's vocabulary has no type for them. The augmented
sixths are the clearer case: an Italian sixth is a root, a major third, and an *augmented sixth*, and a German sixth
spells that augmented sixth where a dominant seventh spells a minor seventh — the same twelve-tone content, a
different chord, and re-rooting `dominant7` produces the wrong letters. Altered dominants are the same problem one
step further: a seventh with a lowered fifth or a raised ninth is a distinct stack, and "name which members are
present" has no way to say so if the vocabulary cannot name them. So this prompt adds seven entries to the chord-type
table — the three augmented sixths and the four common alterations of the dominant seventh — and nothing else about
the chord domain changes. The augmented sixths are also the table's first types that are not stacks of thirds, which
the table's own invariant test has to record by name, exactly as it already records the suspensions and the added
sixths.

Every constructor cites the OMT chapter/table it implements and has a formula test over several keys, including minor.
Where OMT presents stylistic tendencies or multiple spellings rather than a definition, expose a named policy or return
several candidates—never bake one interpretation into `chord_class`. Tonicization is a local harmonic relationship;
modulation remains a claim about a passage and belongs to analysis.

## Target

- The register-free degree lookup `scale_class` and the diatonic stack `scale_chord`, checked and evaluated beside
  `scale_pitch`, and read by `std::scale`. These two are the whole *theory* budget: every chromatic constructor below
  is `scale_class`, `scale_on`, and `chord_on` in ordinary `.musa`.
- The three augmented sixths and the four altered dominant sevenths in the chord-type table, spelled by letter like
  every other entry, with the stacked-thirds invariant naming the augmented sixths as it already names the
  suspensions.
- `stdlib/tonal-harmony.musa` and source docs for typed Roman values and constructors.
- Small core refinements only where invalid states cannot be represented in source; no public Rust theory surface.
  A `roman` is such a case: a bare product of degree, quality, and inversion is constructible with a degree of nine or
  an inversion of five, so the checked constructor and its `option` are what make the invalid states unsayable. Its
  constructor and three accessors are the refinement wiring the triad and the voicing already have.
- `examples/tonal-construction.musa`: major/minor diatonic harmonies, secondary dominant, mixture, Neapolitan,
  augmented sixth, and altered dominant, each voiced by an explicit policy.
- `crates/musa-compiler/tests/tonal_harmony_construction_laws.rs`: OMT formula tables across keys, inversion spelling,
  minor variants, applied targets, and negative domain cases.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-render
cargo clippy --all-targets -p musa-compiler -p musa-render -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/tonal-construction.musa
cargo run -p musa-cli -- render examples/tonal-construction.musa --to musicxml -o /tmp/tonal-construction.musicxml
```

Commit as `Add typed tonal harmony constructors`.

## Stop

- No inferred Roman numerals, cadence labels, tonicization/modulation decision, or note-to-chord matching.
- No implicit key, minor-scale choice, inversion, register, doubling, omission, or voicing policy.
- No compiler keyword for a source-library function.
- Do not describe a stylistic guideline as a type invariant.
