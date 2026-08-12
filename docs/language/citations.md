# Where the theory comes from

Musa implements a bounded set of theoretical constructions. Every one of them either comes from a named chapter of
[Open Music Theory](https://viva.pressbooks.pub/openmusictheory/) or is proved locally, and this chapter says which, for
each.

The reason to keep the map is not scholarly manners. A convention that is built into a compiler stops looking like a
convention: a musician who disagrees with it has nothing to disagree *with* unless the software can say where it got the
idea. Naming the chapter makes the decision arguable again.

**How the citations are checked.** OMT is cited by filename — `` `017-triads.md` `` — and
`scripts/check-language-docs.sh` verifies that every filename cited anywhere in `docs/language/` exists in the local OMT
checkout (`$OMT_ROOT`, default `~/Code/papers/music-theory/open-music-theory`). A chapter that is renamed upstream
breaks the build rather than becoming a dead reference. The check skips with a notice when the checkout is absent, so
the repository does not require it to build.

## 1. Notation

| What Musa implements | Where it comes from |
| --- | --- |
| staff position, clefs, ledger lines | OMT `002-notation-of-notes-clefs-and-ledger-lines.md`, `003-reading-clefs.md` |
| octave numbering (`c4` is middle C) | OMT `006-american-standard-pitch-notation-aspn.md` |
| accidentals and half/whole steps | OMT `005-half-steps-whole-steps-and-accidentals.md` |
| note values, dots, ties, tuplets | OMT `009-notating-rhythm.md`, `012-other-rhythmic-essentials.md` |
| simple and compound time signatures | OMT `010-simple-meter-and-time-signatures.md`, `011-compound-meter-and-time-signatures.md` |
| dynamics, articulation, tempo as *notation* | OMT `007-other-aspects-of-notation.md` |
| voices and parts as texture, not as tracks | OMT `008-texture.md` |
| polymeter as two parts each in its own meter | OMT `098-twentieth-century-rhythmic-techniques.md` §Polymeter |
| repeats with first and second endings | OMT `057-binary-form.md` |

`meter none` — an unmeasured passage — is not from a chapter. It is a notational fact Musa needs because chant and
cadenzas exist, and it is specified in [`01-surface.md`](01-surface.md).

## 2. Pitch, interval, and pitch class

| What Musa implements | Where it comes from |
| --- | --- |
| `Pitch` and `Interval` as staff-and-chromatic pairs | OMT `005-half-steps-whole-steps-and-accidentals.md`, `016-intervals.md` |
| interval names (`M3`, `d5`, `P8`) and their qualities | OMT `016-intervals.md` |
| `NoteName`: a spelled pitch class, octave forgotten | Musa definition — the quotient lemma, [`03-musical-domains.md`](03-musical-domains.md) §1 |
| `Pc12`: `ℤ/12ℤ`, where `B♯ = C` | OMT `099-pitch-and-pitch-class.md`, `100-intervals-in-integer-notation.md` |

Musa proves two things here rather than citing them, because they are what make spelling survive arithmetic: that
interval addition is a **faithful action** on `ℤ²`, and that the action **descends** to the octave quotient. Both are in
[`03-musical-domains.md`](03-musical-domains.md) §1, and the consequence a composer sees is
[What musa refuses to blur §1](../book/src/concepts/distinctions.md#1-written-pitch-is-not-sounding-pitch).

## 3. Scales, keys, modes, and collections

| What Musa implements | Where it comes from |
| --- | --- |
| major scales, degrees, key signatures | OMT `013-major-scales-scale-degrees-and-key-signatures.md` |
| the three minors as three collections under one key | OMT `014-minor-scales-scale-degrees-and-key-signatures.md` |
| diatonic modes | OMT `015-introduction-to-diatonic-modes-and-the-chromatic-scale.md`, `105-diatonic-modes.md` |
| octatonic, whole-tone, hexatonic, pentatonic | OMT `106-collections.md` |
| reading a passage against a collection | OMT `107-analyzing-with-modes-scales-and-collections.md` |
| `Degree` and `Frame`: an ordinal plus its register | Musa definition — the round-trip lemma, `../03-musical-domains.md` §2 |

The separation of `Key` from `Scale` is Musa's, forced by minor: OMT `014` describes three collections that share one
key signature, and no single collection can stand for the key. See
[What musa refuses to blur §3](../book/src/concepts/distinctions.md#3-a-key-is-not-a-scale).

## 4. Harmony

| What Musa implements | Where it comes from |
| --- | --- |
| triads and their qualities | OMT `017-triads.md` |
| seventh chords | OMT `018-seventh-chords.md` |
| inversion and bass position | OMT `019-inversion.md` |
| Roman numerals | OMT `020-roman-numerals.md` |
| figured-bass figures on numerals | OMT `021-figured-bass-and-roman-numerals-with-figures.md` |
| chord symbols | OMT `075-chord-symbols.md` |
| applied chords and tonicization | OMT `050-tonicization.md` |
| modulation to closely related keys | OMT `051-extended-tonicization-and-modulation-to-closely-related-keys.md` |
| modal mixture | OMT `061-modal-mixture.md` |
| Neapolitan sixth | OMT `062-neapolitan-sixth-chords-ii6.md` |
| Italian, French, and German augmented sixths | OMT `063-augmented-sixth-chords.md` |
| altered dominants | OMT `071-altered-and-extended-chords.md` |
| embellishing tones, as a reason a slice may fit no chord | OMT `039-embellishing-tones.md` |

That a numeral carries **no quality** is a Musa result, not an OMT one: the quality-is-the-collection's lemma
(`../03-musical-domains.md` §3) shows that `ii` is minor in major and `II` is major in Dorian as a *consequence* of the
collection, so storing a quality on the numeral would let it contradict the notes.

## 5. Voicing

| What Musa implements | Where it comes from |
| --- | --- |
| SATB spacing, doubling, ranges, crossing | OMT `022-chords-in-satb-style.md` |
| close position, drop voicings, rootless voicings | OMT `076-jazz-voicings.md` |
| omitting the fifth of a dominant seventh | OMT `076-jazz-voicings.md` §Omitting Notes |
| a voicing forgets down to its chord class, not up | Musa definition — the forgetfulness lemma, `../03-musical-domains.md` §3 |

## 6. Sequences, schemas, and transformations

| What Musa implements | Where it comes from |
| --- | --- |
| diatonic sequences: falling fifths, thirds, seconds, parallel sixths | OMT `049-diatonic-sequences-in-middles.md` |
| galant schemas (Prinner, Fonte, Monte, Fenaroli, Quiescenza, and the rest) | OMT `033-galant-schemas.md`, `034-galant-schemas-summary.md` |
| the rule of the octave | OMT `035-galant-schemas-the-rule-of-the-octave-and-harmonizing-the-scale-with-sequences.md` |
| neo-Riemannian `P`, `L`, `R` and their cycles | OMT `072-neo-riemannian-triadic-progressions.md` |

`std::transformational` implements the three parsimonious transformations and says in the module what it does not
attempt: it is not a general `GIS`, and nothing in it decides whether a progression *is* neo-Riemannian.

## 7. Post-tonal

| What Musa implements | Where it comes from |
| --- | --- |
| pitch-class sets, normal order, `Tn`/`TnI` | OMT `101-pitch-class-sets-normal-order-and-transformations.md` |
| set class and prime form | OMT `102-set-class-and-prime-form.md` |
| interval-class vectors | OMT `103-interval-class-vectors.md` |
| twelve-tone rows and their `P`/`I`/`R`/`RI` forms | OMT `108-basics-of-twelve-tone-theory.md` |
| row naming conventions, stated rather than assumed | OMT `109-naming-conventions-for-rows.md` |
| symmetric rows with fewer than 48 distinct forms | OMT `110-row-properties.md` |

The finite-closure lemma (`../03-musical-domains.md` §4) is Musa's: composing a row with any index transformation yields
a bijection, which is why row operations are total once construction has succeeded — and why construction is the partial
step, returning what repeated and what never arrived.

## 8. Counterpoint and voice leading

Every style rule Musa checks is in the registry at `crates/musa-compiler/src/analysis/rules.rs`, and **every row of that
registry carries a citation** — a rule whose source is "everyone knows" is exactly what the registry exists to refuse.
The full table, with each rule's strength and the section it cites, is
[`07-analysis.md`](07-analysis.md#voice-leading-and-counterpoint).

| Profile | Where it comes from |
| --- | --- |
| `satb_common_practice` | OMT `022-chords-in-satb-style.md` |
| `species_1` … `species_4` | OMT `023-introduction-to-species-counterpoint.md` through `027-fourth-species-counterpoint.md` |
| `species_5` | OMT `028-fifth-species-counterpoint.md` — computed as the union of the first four rule lists, because that is what the chapter says florid counterpoint is |
| `jazz_voice_leading` | OMT `076-jazz-voicings.md` |

The profiles are named in the request precisely because they disagree. OMT `022` and OMT `076` take opposite views of
parallel fifths, and they are both right about the music they describe; a reading that averaged them would describe
none. See
[What musa refuses to blur §8](../book/src/concepts/distinctions.md#8-a-failed-claim-is-not-a-style-violation).

## 9. Cadences and analysis

| What Musa implements | Where it comes from |
| --- | --- |
| cadence types and the criteria each one needs | OMT `036-introduction-to-harmony-cadences-and-phrase-endings.md` |
| tonicization read apart from modulation | OMT `050-tonicization.md`, `051-extended-tonicization-and-modulation-to-closely-related-keys.md` |
| a minor key read through natural *and* harmonic collections | OMT `014-minor-scales-scale-degrees-and-key-signatures.md` |
| the standing of a reading that has alternatives | Musa definition — the admission rule, [`07-analysis.md`](07-analysis.md) §2 |

Each cadence finding names the criteria it met and the ones it did not, taken from OMT `036` — which is what lets a
report say "authentic, but the soprano does not arrive on the tonic" instead of picking a label.

## 10. Performance and sound

| What Musa implements | Where it comes from |
| --- | --- |
| swing as a performed ratio over straight notation | OMT `074-swing-rhythms.md` |
| drum patterns as both performed and programmed practice | OMT `084-drumbeats.md` |
| dynamics and tempo as marks whose realization is a separate choice | OMT `007-other-aspects-of-notation.md` |

OMT `074` is explicit that the swing ratio varies and is not what the page says, which is the whole reason notated
duration and performed duration are different types
([What musa refuses to blur §10](../book/src/concepts/distinctions.md#10-notated-time-is-not-performed-time)).

Instruments, the studio, and orchestration are specified in [`08-performance-and-sound.md`](08-performance-and-sound.md)
against OMT `114-core-principles-of-orchestration.md` and `116-transcription-from-piano.md`, and are built by prompts
130–142. Their citations belong to that specification until then; prompt 143 adds their half of this handbook.

## 11. What Musa proves for itself

Six results are not cited because they are not OMT's; they are stated and proved in
[`03-musical-domains.md`](03-musical-domains.md), and each exists to license an operation that would otherwise be a
convention.

| Result | What it licenses |
| --- | --- |
| faithful action | transposition preserves spelling by construction, not by a repair pass |
| the action descends | interval arithmetic on `NoteName` is well defined |
| round trip | a degree plus a frame determines exactly one pitch, and back |
| the quality is the collection's | a Roman numeral needs no quality field |
| voicing forgetfulness | a voicing has one chord class; a chord class has many voicings |
| finite closure | row transformations are total after construction |

Everything below the musical layer — the total core calculus, the temporal kernel, elaboration, and normalization — is
Musa's own and is specified in [`02-core-calculus.md`](02-core-calculus.md) and `../kernel/`. No music theory is cited
there, because none is used: the kernel knows about exact time and typed occurrences and nothing about notes.

## 12. Cited, and deliberately not implemented

OMT covers a great deal that Musa does not, and the gap is a design decision rather than a backlog. Musa builds the
constructions that are **decidable from the notated score** and refuses the ones that require a reading it cannot
defend. It has no opinion on Schenkerian reduction, no formal-function analyzer, no style classifier, and no way to
decide whether a passage *is* in a style — `../07-analysis.md` §8 lists the limits this produces and the false positives
each one causes, stated up front rather than discovered.

The larger omission is the one worth saying plainly: the theory implemented here is one pedagogical tradition, mostly
Western and mostly common-practice. That a convention is built into this compiler is a fact about this software, not a
fact about music.
