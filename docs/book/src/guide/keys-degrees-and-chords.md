# Keys, degrees, and chords

Everything in [First pieces](first-pieces.md) was a note you typed. This chapter is about writing the *reason* for a
note instead — a degree of a collection, a member of a chord — and letting the compiler work out which note that is.

Nothing here is automatic. Musa will not pick a minor collection for you, will not voice a chord for you, and will not
decide that your passage has modulated. Each of those is a decision, and the language makes you write it down.

## 1. A key is not a scale

They are different questions and Musa gives them different words.

`key c minor;` is a **notational** fact: three flats in the signature, from that bar to the next `key` statement. It
appears on the page. It does not decide which notes a stepwise line walks through, because C minor is not one collection
— natural, harmonic, and ascending melodic minor are three, and only you know which one this phrase means.

`in scale c dorian { ... }` is a **generative** coordinate system: it decides what `step` means for the music inside it,
and it emits nothing on the page. A Dorian passage inside a C minor piece is a Dorian passage, not a modulation.

From `examples/scale-context.musa`:

```musa
// Read as values, the domains stay separate: a key is a signature and a
// mode, and the collection it suggests is a default a passage may refuse.
let home: Key = key c minor;
let default_collection: Scale = key_scale(home);
let raised_seventh: Scale = harmonic_minor_on(scale_root(default_collection));
let anchored: Option<Frame> = frame_on(default_collection, c5);
```

`key_scale` gives the collection a key *suggests*. It is a default a passage may refuse, not a fact the key contains.
`harmonic_minor_on` and its neighbours come from `std::collections`, which names every collection Musa ships: diatonic
modes, the three minors, both octatonics, whole-tone, hexatonic, acoustic, and the two pentatonics.

## 2. Steps and degrees

Inside a scale, `step` counts through the collection rather than through semitones:

```musa
fn figure() -> EventTrack<WrittenTime> {
    music {
        c5/8
        (c5 step 1)/8
        (c5 step 2)/4
    }
}
```

That figure means something different under each collection it is placed in, which is the point:

```musa
// One phrase, two coordinate systems. Under C major the thirds
// are `e5` and `g5`; under C dorian the third is `eb5`.
in scale c major {
    use subject;
}
in scale c dorian {
    use subject;
}
```

The two minors are two collections and stepping through them differs, which is exactly how the leading tone shows up:

```musa
in scale c natural_minor {
    c5/4
    (c5 step 6)/4
}
in scale c harmonic_minor {
    c5/4
    (c5 step 6)/4
}
```

A **degree** is an ordinal in a collection — "the fifth degree" — and it does not name a pitch, because a degree has no
octave. Turning one into a pitch takes a **frame**: a collection plus the absolute pitch that registers it. That is what
`frame_on(default_collection, c5)` above is, and why it returns an `Option`: not every collection can be framed at every
pitch. `std::scale` holds the degree and frame operations.

## 3. A chord class is not a voicing

A **chord class** is content: a root and its spelled members. It chooses no register, no spacing, no doubling, no
octave, and no bass. From `examples/chord-voicings.musa`:

```musa
// The content. Root C, major seventh: c, e, g, b — spelled, so the third
// is a third and the seventh is a seventh.
let sonority: ChordClass = chord c major7;
```

A **voicing** is a realization of a class: actual written pitches, in a chosen order, from a chosen bass. One class has
as many voicings as you like, and they are not equal to each other:

```musa
// Close position stacks the members upward from an absolute bass.
let close: Option<Voicing> = close_position(sonority, c4);
// Drop 2 lowers the second note from the top by an octave. It is a
// different voicing of the same class, not a different chord.
let drop_two: Option<Voicing> = drop_position(sonority, c3, 2);
// The pitches written out by hand: the same four notes, spread wide, with
// the third on top.
let spread: Option<Voicing> = voiced_as(sonority, [c3, g3, b3, e4]);
```

The `Option` is load-bearing. `close_position` is absent when the bass you asked for is not a member of the class *as
spelled*, and that absence is an answer rather than a failure. It is why a chord symbol cannot sound on its own: to get
from a class to notes you must supply a policy, and the policy can decline.

Sounding one takes `play`, and an absent voicing becomes silence rather than a guess:

```musa
// A voicing sounds only through `play`. A policy whose preconditions fail
// sounds a rest, so an absent answer is silence rather than a guess.
fn held(chosen: Voicing) -> EventTrack<WrittenTime> { play(chosen, duration_of(1/1)) }
fn sounded(chosen: Option<Voicing>) -> EventTrack<WrittenTime> {
    option_fold(music {
        rest/1
    }, held, chosen)
}
```

`option_fold` is `std::option`: it takes what to do when there is nothing, what to do when there is something, and the
option itself. A total language has no way to "just unwrap" — you name the silence.

For the common case there is sugar. In a voice, `stack c4 major7/1` is close position with the written root fixing the
register, which is the same thing `close` names above:

```musa
// The sugar: close position with the written root fixing
// register, which is exactly what `close` names above.
stack c4 major7/1
```

Chord-class construction and inversion live in `std::harmony`; the voicing policies — close, drop-*n*, rootless,
hand-written — live in `std::voicing`.

## 4. Numerals build chords; they do not describe passages

A Roman numeral in Musa is a *constructive* value: an ordinal, a member count, and an inversion. It carries no quality
and no collection, because the quality is the collection's. From `examples/tonal-construction.musa`:

```musa
// The numerals, as values. A numeral is absent when it cannot be written:
// `triad_numeral(8)` has no answer, and neither does a triad in third
// inversion.
let one: Option<Roman> = triad_numeral(1);
let two: Option<Roman> = triad_numeral(2);
```

Give a numeral a collection and it stacks:

```musa
// The diatonic harmonies of C major. `ii` is minor and `V` is major
// because those are the notes in the collection, not because either was
// asked for.
let tonic: Option<ChordClass> = in_major(one);
let supertonic: Option<ChordClass> = in_major(two);
```

`ii` comes out minor and `V` comes out major because of the notes in the collection, not because anyone asked for a
quality. Stack the same numerals on Dorian and `II` is major, for the same reason. This is a small theorem rather than a
table (`../03-musical-domains.md` §3), and it is why a numeral that stored a quality would be able to contradict the
collection it was realized against.

`std::tonal::harmony` holds the diatonic and applied constructions, `std::tonal::sequences` the sequence patterns, and
`std::tonal::schemas` the galant schemas and the Rule of the Octave. `std::transformational` holds the Neo-Riemannian
operations.

Reading a numeral *off* a passage is a different activity with a different answer, and it is in
[Claims and readings](claims-and-readings.md).

## 5. When the spelling stops mattering

Twelve-tone and pitch-class-set work is an algebra over `pc12 = ℤ/12ℤ`, and `pc12` is not a spelling: it has forgotten
which letter the note was written with. `std::post_tonal::pcset` holds sets, normal order, prime form, and
interval-class vectors; `std::post_tonal::serial` holds rows, their forms, and their matrices. Going back into notation
is `spelled_in`, which takes the collection that decides the spelling and answers nothing where that collection has no
such note — see `examples/serial-forms.musa`, and
[What musa refuses to blur §2](../concepts/distinctions.md#2-a-pitch-class-is-not-a-residue-mod-12).

The remaining bundled modules are the plumbing: `std::core` for exact rationals and the small total operations,
`std::list` for finite lists, and `std::pitch` for the named written intervals.
