# What Musa refuses to blur

Ordinary musical talk identifies things that are not the same. That is fine in conversation and disastrous in a
compiler: once two concepts share a representation, every later feature that needs them apart has to invent a
convention, and the conventions disagree.

Each section below names a distinction Musa keeps, the two types it keeps it with, and a **falsifier** — a concrete case
where identifying them would produce a wrong answer. A distinction with no falsifier would have been ceremony.

The full table of separations the architecture requires is in `../../../plan/roadmap.md` §"Things that must remain
separate"; the mathematical definitions and their proofs are in
[`docs/rules/language/03-musical-domains.md`](../../../rules/language/03-musical-domains.md).

## 1. Written pitch is not sounding pitch

`Pitch` is a pair of integers: where the note sits on the staff, and how many half steps it is from a reference. Both
coordinates are kept, so a spelling survives every operation. A frequency, a MIDI number, or a pitch-class integer is a
different thing, reached at a different layer.

**Falsifier.** `c#4` transposed up a diminished second must spell `d♭4`, not `c#4`. Both sound the same. A
representation that stored only the chromatic coordinate could not tell them apart, and a violinist reading the second
would play it differently from the first.

The interval names carry the same two coordinates: `M2` and `m3` and `P8` are pairs, not semitone counts, so
transposition is spelling-preserving by construction rather than by a repair pass. See `examples/pitch-arithmetic.musa`.

## 2. A pitch class is not a residue mod 12

`NoteName` is a written pitch with the octave forgotten and the spelling kept: `C♯` and `D♭` are different values.
`Pc12` is `ℤ/12ℤ`, where they are the same value. The map from one to the other exists, is named, and goes one way.

**Falsifier.** `B♯ = C` is false as `NoteName` and true as `Pc12`. Pitch-class-set theory needs the second; notation
needs the first. Coming back from `Pc12` into notation is `spelled_in`, which takes the collection that decides the
spelling and answers nothing where that collection has no such note — the loss shows up in the value rather than being
papered over.

## 3. A key is not a scale

A `Key` is a notational context: a spelled tonic, a mode family, a signature. A `Scale` is an ordered collection of
spelled pitch classes that stepwise motion walks through. `key` prints; `in scale` generates.

**Falsifier.** `key c minor` cannot determine a single collection, because C minor is natural, harmonic, and ascending
melodic minor, and a piece uses more than one of them. Identifying key with scale would force the compiler to pick, and
whichever it picked would be wrong somewhere in every minor-key piece.

A second falsifier runs the other way: a Dorian passage inside a C minor piece is a Dorian passage. If `in scale`
emitted a key fact, that passage would falsely declare a modulation.

## 4. A degree is not a pitch

A `Degree` is an ordinal in a collection and has no octave. Turning one into a `Pitch` requires a `Frame` — the
collection plus the absolute pitch that registers it.

**Falsifier.** `degree(scale c major, 1)` cannot have type `Pitch`: it is C, but which C? The same `scale c major`
framed at `c4` and at `c5` are two different frames, and degree 1 realizes to one pitch only once one has been chosen.

## 5. A chord class is not a voicing

A `ChordClass` is a root and its spelled members. A `Voicing` is exact written pitches in an order, with a bass, a
spacing, and whatever doubling and omission were chosen. Going from the first to the second takes a policy, and the
policy may decline.

**Falsifier.** Drop-2 and close position of the same C major seventh must not compare equal, because they are different
music. And a `Cmaj7` chord symbol chooses neither a C3 bass nor a C4 bass, so a symbol cannot sound on its own.

## 6. A Roman numeral is not a chord

The constructive `Roman` is an ordinal, a member count, and an inversion. It carries no quality and no collection.

**Falsifier.** `V` alone names no pitch class. Give it a collection and the quality follows: `ii` is minor in major and
`II` is major in Dorian, as a consequence of the notes rather than as a stipulation. A numeral that stored a quality
could be realized against a collection that contradicted it. (A triad also has no third inversion, which is why
construction returns an `Option`.)

## 7. Construction is not analysis

Building a chord from a collection and a numeral is construction: total, decidable, and yours. Reading a numeral off a
passage is analysis: interpretive, possibly ambiguous, and the software's opinion.

**Falsifier.** In `examples/analysis/pivot-ambiguity.musa` an A minor triad is `vi` in C major and `ii` in G major. Both
readings are reported. If one Roman-numeral reading were allowed to become a fact about the score, the other would have
been silently discarded — and a musician who disagreed would have nothing to disagree with.

## 8. A failed claim is not a style violation

An `assert` is a claim you wrote about your own passage; failing it is an error, because you said something untrue about
your music. A counterpoint or voice-leading rule is a **style** rule, indexed by the style it belongs to, and a
departure from it is evidence, not a defect.

**Falsifier.** Species counterpoint rules describe a particular pedagogical practice. Applying them as universal
well-formedness would make most of the repertoire invalid. This is why the rule set is named in the assertion —
`follows(satb_parallel_perfects)` — and why an analysis reports departures without painting the score red.

## 9. A declaration is not its occurrences

A `motif`, a function over music, or a music value is written once. Each `use` is a separate occurrence with its own
place in time, its own context, and its own identity.

**Falsifier.** `use sigh();` twice in one voice is two placements of one phrase. If they were the same object, Origin
could not tell you which note came from which, an edit to the second would change the first, and one function called
twice with equal arguments would collapse into one voice instead of two.

## 10. Notated time is not performed time

A written quarter note is a notated duration. What the performance schedules is a performed duration, and swing, rubato,
articulation, and grace-note placement all move the second without touching the first.

**Falsifier.** A shuffle is straight eighths on the page. If notation and performance shared a representation, applying
swing would rewrite the notation, and the page would stop saying what the composer wrote (`examples/shuffle.musa`,
`examples/profile-fixture.musa`).

The written side has its own data, and `std::notation::staff` is where it lives: a `WrittenDuration` is a division of
the whole note and its dots, `realize` turns one into the exact span it means, and `engrave` chooses the value that
prints a span back. What a grace note *takes* is not among the answers — `realize` refuses it rather than guessing,
because a grace's timing belongs to whoever plays it.

## 11. A voice is not a track, and a part is not an instrument

A voice is a line of music; a part is a performer. Neither is a mixer channel or a synthesizer instance. Sound is
assigned to them, and the assignment is a separate statement that can change without touching a note.

**Falsifier.** Two voices in one part share a staff and may be routed to different sounds; two parts may be routed to
the same one. Identifying either pair would make a notational decision change what you hear, or the reverse.

## 12. Source is the document

There is no second editable model. The score view, the studio view, and the outline are all projections of `.musa` text,
and every edit any of them makes is a text edit. A "dynamic marking" is `mf`, not a decibel value; the decibels are
chosen later, by a profile, and changing them does not change the page.

**Falsifier.** If the workbench held a mutable expanded score, an edit made on the page and an edit made in the text
would be two sources of truth, and the first conflict between them would have no right answer.

## 13. What the refusals look like

Seven of these distinctions have a fixture under `examples/broken/` that crosses one and is refused. Each is a whole
piece, and each one's rendered diagnostic — including the help line naming the operation that *does* cross the gap — is
compared against a checked-in golden, so the advice cannot quietly stop matching the library.

| Fixture | Distinction |
| --- | --- |
| `chord-class-is-not-music.musa` | §5 — a class has no register and no duration |
| `a-key-is-not-a-scale.musa` | §3 — C minor is three collections |
| `a-numeral-has-no-collection.musa` | §6 — a numeral carries no quality |
| `a-degree-is-not-a-pitch.musa` | §4 — an ordinal needs a frame |
| `a-note-name-has-no-octave.musa` | §1 — a written class is not a written pitch |
| `pc12-has-forgotten-the-spelling.musa` | §2 — the map into `Pc12` is one-way |
| `a-function-is-not-its-result.musa` | §9 — a nullary `fn` is still a function |

```sh
musa check examples/broken/chord-class-is-not-music.musa
```
