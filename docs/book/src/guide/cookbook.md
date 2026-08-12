# Cookbook

Each recipe names the fixture that does it in full. Open the file; every one is a complete piece that checks and
renders, and they are run as regression tests, so what you read is what compiles today.

```sh
musa check examples/<name>.musa
musa render examples/<name>.musa --to lilypond
```

## Notation

**Change key partway through.** Write `key` where the music reaches it. It must land on a barline, because a key
signature is printed at one. From `examples/modulation.musa`:

```musa
| f4/4 a4/4 c5/4 a4/4
| bb4/4 a4/4 g4/4 f4/4

key d minor;
| d4/4 f4/4 a4/4 f4/4
```

**Change clef partway through.** `examples/clef-change.musa`. A clef change is the one thing that may land mid-bar,
because a clef is printed where the notes need it.

**Change meter partway through.** `examples/changing-meter.musa` alternates a 7/8 phrase with a refrain.
`examples/bulgarian.musa` runs a 7/8 melody against a 4/4 drum, and `examples/hemiola.musa` puts 6/8 against 3/4 —
polymeter is two parts each in their own meter, not one part fighting a barline.

**Stop barring altogether.** `meter none;` — `examples/chant.musa` for a whole unmeasured piece, `examples/cadenza.musa`
for an unmeasured passage inside a measured movement.

**Repeats and endings.** `examples/repeats.musa`. One statement, two projections: the page prints the body once between
repeat barlines and the performance plays it through twice.

```musa
repeat 2 {
    | d5/4 g5/4 f#5/4 g5/4
    | a5/2 b5/2
    ending 1 {
        | a5/2 d5/2
    }
    ending 2 {
        | g5/1
    }
}
```

**Tuplets, slurs, ties, and the rest of the expressive layer.** `examples/tuplet-fixture.musa`.

**Ornaments and grace notes.** `examples/ornaments.musa` for the marks a Baroque page carries; `examples/graces.musa`
and `examples/graces-reordered.musa` for grace notes and the question a page refuses to answer about them.

**Tempo changes and ramps.** `examples/tempo-changes.musa`, `examples/rubato.musa` for a ritardando and an a tempo, and
`examples/riser.musa` for an eight-bar accelerando.

**Named phrases and annotations.** `examples/annotated.musa`.

**Text that is not ASCII.** `examples/unicode-fixture.musa` puts characters above U+007F in every place a piece can
carry text.

## Construction

**Write a phrase once and use it in several keys.** Make it a `music` value and place it inside `in scale`;
`examples/scale-context.musa`. Music values are contextual, so the same phrase elaborates differently at two sites.

**Answer a subject through a transformation the caller picks.** `examples/canon-functions.musa`. Also
`examples/canon.musa` and `examples/canon-x.musa` for two parts at their own speeds.

**Harmonize a line.** `examples/harmonize-function.musa` maps each sounding pitch to another voice's pitch, without ever
seeing a barline.

**Vary a theme.** `examples/variation.musa`: one theme, then the same theme transformed, with every note's origin still
pointing back at the theme.

**Build tonal harmony from numerals rather than typing the notes.** `examples/tonal-construction.musa`, using
`std::tonal::harmony` and `std::voicing`. Diatonic chords, applied dominants, and the two minor dominants side by side
rather than one chosen silently.

**Harmonize a bass scale.** `examples/rule-of-the-octave.musa` asks `std::tonal::schemas` for the chords and then voices
them twice, under two policies that share no decision — because a harmonic skeleton is not music until somebody voices
it.

**Write a sequence.** `examples/diatonic-sequences.musa`: a pattern, a count, and nothing else, from
`std::tonal::sequences`.

**Move between triads by common tone.** `examples/neo-riemannian.musa` runs the hexatonic cycle `P L P L P L` with
`std::transformational`, and says what it does not do.

**Work with rows and pitch-class sets.** `examples/serial-forms.musa`, using `std::post_tonal::serial` and
`std::post_tonal::pcset`. Row construction can fail, and the reasons — which position repeated, which class never
arrived — are exact values rather than a message.

**Voice one chord several ways.** `examples/chord-voicings.musa`: close, drop-2, hand-written, rootless, each a policy
applied to one class.

**Use a standard-library function at all.** `examples/stdlib-basics.musa` is the smallest piece that imports
`std::core`, `std::list`, and `std::option` and uses each.

## Structure

**Make the same piece in several keys.** `examples/template-study.musa`: a `template piece` whose key, collection, and
subject arrive as arguments.

**Pass a bundle of facts that belong together.** `examples/module-functor-study.musa`: a `signature`, two structures
from `std::context`, and a `template structure` over them.

## Checking and reading

**Claim something about a passage.** `examples/theory-assertions.musa` writes out all five assertion kinds.

**See what a failed claim looks like.** Anything in `examples/broken/`. Every file there has a checked-in golden of its
rendered diagnostic.

**Ask for a reading rather than a claim.** `examples/analysis/` — `pivot-ambiguity.musa` for a modulation nobody can
date to one chord, `cadence-evidence.musa` for four phrase endings and the evidence each has, `equivocal-sonority.musa`
for chords that fit more than one label, `unknown-passage.musa` for a passage that fits none, and `species-1.musa`
through `species-5.musa` and `satb.musa` for style readings.

**Write a kernel term by hand.** `examples/kernel-splice.musa`, and read the bottom of that file before you do.

## Open form

**Let the performer choose.** `examples/in-c.musa` is Terry Riley's *In C*; `examples/mobile.musa` is Stockhausen's
Klavierstück XI; `examples/loop-lengths.musa` runs fills "a few times" rather than a fixed number. A piece that leaves
something open compiles to a *realization*, and `--seed` chooses which one:

```sh
musa render examples/in-c.musa --seed 7 --to midi
```

A piece that leaves nothing open compiles to the same bytes under every seed.

## Idiom

**A jazz chart.** `examples/changes.musa` — head, solos, head out.

**A drum chart.** `examples/drum-chart.musa`, which is mostly words.

**Grooves and swing.** `examples/shuffle.musa` for a swung twelve-bar blues, `examples/house.musa` for four-on-the-floor
with a pushed bass. Swing is an interpretation profile, not a notation: the page still says straight eighths
(`examples/profile-fixture.musa`).
