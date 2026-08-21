# 52. The musical algebra the language has to be able to say

A decision record. Governs nothing. Note 51 argues that three decisions of the course correction work against terseness;
this one says what terseness is *for* in this domain — which musical structures a Musa author should be able to name,
and what each one asks of the language.

The method is the audit rule of note 51 §2, run forward instead of backward: every structure below is exhibited in the
committed standard library or in a chapter of *Open Music Theory*, together with the workaround it is written as today.

## 1. The domain is more general than the standard library admits

`std::post_tonal` is written for one modulus, `std::pitch` for one tuning, and `std::tonal` for one metrical system. The
corpus the language is built for is not:

- **Not twelve.** `068-equal-divisions-of-the-octave.md` is about dividing the octave into n parts and enumerating which
  n admit which symmetries. Quarter-tone practice needs 24; Turkish and Arabic theory are usually modelled at 24 or 53;
  a 19- or 31-tone meantone keyboard is an ordinary object.
- **Not the octave.** Bohlen–Pierce divides the *tritave* (3:1) into 13. The period is a parameter, not a constant.
- **Not equal.** Gamelan slendro and pelog are unequal and instrument-specific; a scale is an interval pattern that sums
  to the period, and equal division is the special case where every step is the same.
- **Not metrical in the European sense.** A Carnatic tala and a West African bell pattern are cycles of n pulses with an
  onset set — and `098-twentieth-century-rhythmic-techniques.md`'s additive meters are the same object with the cycle
  written as its step pattern.

That last point is the one with the most leverage, so it is worth stating on its own.

> **A pitch-class set in Z/12 and a bell pattern in a 12-pulse cycle are the same mathematical object.** Transposition
> is rotation; inversion is reflection; the interval-class vector is the pattern's autocorrelation. `102-set-class-and-
> prime-form.md`'s prime form and a rhythm's canonical rotation are one algorithm.

A language that names the structure once serves both, and one that names `pc12` twelve times serves neither. PoSD ch. 6:
general-purpose modules are deeper, and the test is whether the same interface covers several current needs.

## 2. The five structures, and what the library writes instead

### 2.1 Torsor: a point moved by a vector

`Pitch` and `Interval`; `Position` and `Duration`; `Pc(n)` and `Ic(n)`. Two pitches subtract to an interval, an interval
moves a pitch, and **two pitches do not add**. The same for time. Roadmap §2's layer separation already asserts this in
prose — "written pitch ≠ MIDI number", "notated duration ≠ performed duration" — and a torsor is that assertion with an
algebra attached.

*Written today as:* `interval_add`, `interval_inverse`, and a `Transposable<A>` trait with a single `transposed` method,
in `stdlib/src/pitch.musa`. Three unrelated names for the group operation, the inverse, and the action.

*Asks of the language:* a two-parameter trait relating a point type to its vector type, with `-` and `+` at the two
signatures. Musa's trait system already supports this shape (`Iterable<List<A>, A>` is head-determined lookup with the
second parameter derived).

### 2.2 Group action: the transformation is the object

The T/I group acting on `Pc(n)` is the dihedral group of order 2n. The PLR group acting on consonant triads is dihedral
of order 24, and — Lewin's observation, and the reason both exist — the two are each other's centralizer. Serial
operations act on two sides at once: T and I on pitch classes, R on order positions.

`stdlib/src/post_tonal/serial.musa` knows this and says so in a comment it cannot turn into code:

> The four labels P, I, R, and RI are 24 affine pitch-class operations times the reversal R, which is a separate
> involution acting on order positions. That is `D12 x C2`, so there are at most 48 labelled forms.

*Written today as:* one top-level function per operation per carrier — `transposed`, `inverted`, `retrograde_of`,
`retrograde_inversion_of` in `serial.musa`; `transposed_by`, `inverted_about` in `pcset.musa`; `parallel`, `relative`,
`leading_tone_exchange`, `slide`, `nebenverwandt`, `hexpole` in `transformational.musa`. The word `transposed` names two
unrelated functions in two modules because a row and a pitch cannot be two carriers of one action.

*Asks of the language:* an `Action<G, X>` trait with `act`, the identity and composition laws, and — from note 51 §6 —
**sections**, so that `T₃` is a value of `G` and not a call site with a missing argument. Without sections there is no
group element to compose, and the entire transformational apparatus stays a list of functions.

### 2.3 Orbit and stabilizer: two OMT chapters are this and nothing else

- A **set class** is an orbit of pc sets under the T/I group; **prime form** is its canonical representative
  (`102-set-class-and-prime-form.md`).
- **Messiaen's modes of limited transposition** are exactly the collections with a nontrivial stabilizer under T
  (`106-collections.md`).
- A row's **symmetries** — the transformations that fix it — are its stabilizer, and derived rows are those with a large
  one (`110-row-properties.md`).

*Written today as:* the builtins `row12_symmetries`, `row12_forms`, and `row12_matrix`, one per question, each at the
one modulus.

*Asks of the language:* nothing new beyond §2.2 and an index — **orbit and stabilizer are ordinary functions of a finite
action**, computable once the carrier's size is known. `Pc(n)` gives the size. This is the clearest case where an index
deletes builtins rather than adding machinery.

### 2.4 Quotient, and the section that undoes it

`forget_spelling : NoteName -> Pc(12)` is total, not injective, and has no inverse without a policy;
`spelled_in(member, collection)` is the policy-chosen section. `stdlib/src/post_tonal/pcset.musa` states both carefully,
and `stdlib/src/transformational.musa` states the consequence:

> Finite-group statements are true only after the spelling is explicitly forgotten … it is a separate function precisely
> so that the projection is something an author performs rather than something that happens to them.

That is exactly right and should be preserved as a design commitment, not smoothed away: **the PLR cycle does not close
in the spelled domain.** `P L P L P L` returns a triad that sounds like its origin and is spelled `dbb`. The group laws
hold downstream of the quotient and nowhere else.

*Asks of the language:* nothing structural — but it does ask that law checking (§3) be stated at the carrier where the
law actually holds, and refuse to certify it at the one where it does not. A law system that "helpfully" proved
associativity for spelled triads would be wrong.

### 2.5 The free structure and the fold

A motif is a free construction; its expansion is the unique homomorphism out of it. `Iterable`/`Buildable` are the
container instance of this and already exist.

*Asks of the language:* nothing new. Noted so the list is complete, and because it is the one structure the current
design already serves well.

## 3. How laws get checked without a proof assistant

Note 51 §7 keeps `Id`, `refl`, and `J` deleted. Nothing here reopens that, and the reason is that **this domain does not
need proofs — it needs exhaustion.**

An index gives the checker the carrier's size. `Pc(n)` has exactly n inhabitants; the T/I group on it has exactly 2n; a
12-pulse cycle has 4096 subsets. So a trait's laws over a finite indexed carrier are decidable **by enumeration**,
inside the existing budget, with no proof term, no identity type, and no termination argument:

```
trait Action<G, X> {
    fn act(operation: G, subject: X) -> X;
    law identity:    act(unit, x) == x;
    law composition: act(compose(g, h), x) == act(g, act(h, x));
}
```

For `Action<Ti(12), Pc(12)>` that is 24 × 24 × 12 = 6,912 checks — an ordinary compile-time computation. For an
unbounded carrier the laws are *asserted with finite evidence* through the existing `assert` and `05-verification.md`
machinery, exactly as they are today, and the trait says which of the two it got.

This is the whole answer to "principled and rigorously correct without proof machinery": indices make carriers finite
and known, finiteness makes laws decidable, and decidable laws need no proofs. It is also why §2.4 matters — the checker
will *refuse* to certify associativity for spelled triads, because enumeration will find the counterexample the
library's comment describes.

## 4. What this asks of the language, collected

Nothing below is new machinery invented here; each is note 51's proposal or a mechanism Musa already has.

| Need | Mechanism | Status |
| --- | --- | --- |
| one type per structure, not one per modulus | index over ℕ / exact ℚ (note 51 §4) | to admit |
| `T₃` as a value | sections (note 51 §6) | to admit |
| `f . g` for transformations | composition operator over the section form | to admit |
| a point/vector pair | two-parameter trait, head-determined lookup | **already have** |
| an action of a group on a carrier | same | **already have** |
| laws on a finite carrier | enumeration under the existing budget | small addition |
| laws on an unbounded carrier | `assert` and finite evidence | **already have** |
| orbit, stabilizer, prime form | ordinary functions of a finite action | falls out |
| notation built from all of it | quotation and adapters | **already have, untouched** |

The last row is the one to hold onto. Macros stay the primary extensibility mechanism, exactly as directed: none of this
touches `quote`, splices, categories, anchors, or the expansion phase. The index stratum is orthogonal to quotation by
construction — indices are erased before evaluation, and a syntax category was already a literal index.

## 5. The program this note is answerable to

Note 51 §9 keeps the staff adapter as the falsifier for the language change. This note adds a second, which is a better
test of *musical* expressiveness because the staff adapter is a notation program and this is a theory one:

> **Rewrite `stdlib/src/post_tonal/` for arbitrary n**, and let `examples/` contain a piece that uses a 24-EDO row and a
> West African bell pattern analysed with the same operations as a pitch-class set.

If that rewrite is not substantially shorter than today's 213 lines plus 17 builtins, and if the bell pattern still
needs its own vocabulary, then the structures named here are not the right ones and this note is wrong.
