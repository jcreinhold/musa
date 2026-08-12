# Realizations and residue

The method of `~/Code/papers/category-theory/50-examples-for-the-motive/text.md`, applied to music. Ten examples, each
chosen because some realization sees something the others do not. For each: what every realization extracts, then the
residue — the smallest description they all agree on.

Realizations abbreviated **EN** (engraving), **PF** (performance), **SD** (sound), **AN** (analysis), **MI** (MIDI),
**EQ** (equational reasoning / semantic hash).

---

## E1. One note — `c#4/4`

- **EN** a notehead on the C space with a sharp, stem up, one beat of a bar.
- **PF** an onset at a rational offset, a gate fraction, an amplitude from the prevailing dynamic.
- **SD** a fundamental near 277.18 Hz for some physical duration.
- **AN** scale degree, or pc 1, depending on which analysis.
- **MI** note number 61, a velocity, a tick range.
- **EQ** one occurrence, `note c#4 1/4`.

**Residue.** An exact rational half-open span, and a pitch *point* that each realization reads at a different
granularity. Nobody except EN needs the spelling; nobody except SD needs the frequency; and both are recoverable from
the same point. Already visible: the realizations differ by **how much they forget**, not by what they are looking at.

## E2. The same note respelled — `db4/4`

- **EN** different: a D notehead with a flat. **AN** same pc, possibly different function. **MI** same 61. **SD** same
  frequency in equal temperament, *different* in any historical temperament. **EQ** must say these are different pieces.

**Residue.** The pitch point is finer than pc and finer than MIDI number, and coarser than nothing. This is the first
evidence for a **tower of quotients** rather than one payload: EN reads the top, AN and MI read quotients of it, and SD
reads a quotient that depends on a temperament parameter it supplies itself.

## E3. A triplet against a duple division

- **EN** a bracketed 3 over three eighths; **PF/SD/MI** onsets at exact thirds; **AN** notes the division; **EQ** must
  make `1/3 + 1/3 + 1/3 = 1`.

**Residue.** Exact rational time, unanimously. This is `docs/kernel-hypothesis/01-atoms.md` Atom 1 and no realization
disputes it. *Nothing here is contested;* it is recorded because a motive must contain it and because it is the one atom
that has never been challenged.

## E4. Parallel motion versus voice exchange

Four occurrences: C4 and A3 on `[0,1)`, D4 and B3 on `[1,2)`, read either as two parallel lines or as a crossing.

- **EN** different stems and beams. **AN** first-species returns different verdicts. **PF/SD/MI** identical. **EQ** —
  the kernel in isolation identifies them; the pipeline does not, because `VoiceId` rides in the payload.

**Residue.** Voice membership is read by exactly two realizations (EN, AN) and by no others. It is therefore *in* the
motive — but the evidence log's G0.2 and G2.1 establish that a per-note tag carries it adequately and that no consumer
wanted a line relation. **Recorded verdict: in the motive, as payload, not as structure.**

## E5. A hemiola — a 3-layer against the notated 4-layer

The tresillo of OMT `118-metrical-dissonance.md`, Example 6.

- **EN** must choose: dotted rhythms, or ties, or a written polymeter. The choice is *not determined* by the sounding
  content. **PF/SD/MI** onsets, with no notion of which layer they belong to. **AN** the whole point: a 3-layer against
  a 4-layer, realigning after 12 pulses. **EQ** today: two barrings of the same onsets hash equal.

**Residue.** Two pieces of data that no other example separates: the onsets, and **the layers in force**. OMT is
unambiguous that the layers are musical content and not presentation — grouping dissonance *is* the coexistence of
unaligned layers. But EN's freedom to re-bar says the *barline* is presentation. So the motive carries layers and does
not carry barlines. This is `docs/kernel-hypothesis/01-atoms.md` Atom 4, and it is the example that most sharply
separates content from notation.

## E6. A motif and its transposition — `cell()` and `T₄ cell()`

- **EN** two passages, no relation marked. **PF/SD/MI** two passages. **AN** *the relation is the content* — OMT `101`
  treats Tₙ as the thing being analysed. **EQ** must say these are not equal, and must also be able to say they are
  related.

**Residue.** Something no realization but AN reads, and which is not a property of either passage: **the correspondence
between them.** Every other row of this table is about what a piece *is*; this row is the first about what a map between
pieces is, and it is the row that says the motive needs morphisms and not only a carrier. See
[00-the-motive-question.md](00-the-motive-question.md) §4.2.

## E7. A crescendo over four bars

- **EN** a hairpin. **PF** a curve `E(b)` over the span. **SD** a gain envelope in dB. **AN** a dynamic process. **MI**
  a stream of CC7 or velocity scaling. **EQ** must not confuse the marking with any of its numeric readings.

**Residue.** A *shape on a span*, with each realization supplying its own numeric interpretation. Roadmap §2's "dynamic
marking ≠ decibels" is this row. `docs/kernel/04-algebraic-laws.md` L24 already says the right thing — a curve-bearing
occurrence transforms by its span alone — which means continuous shape is payload and costs the kernel no operation.
**Recorded verdict: in the motive, as an opaque payload value with a span.**

## E8. A patch — `oscillator(sine) |> lowpass(cutoff: 1800 Hz) |> output`

This is the row that started the directory, and it behaves unlike every other row.

- **EN** *nothing.* **AN** *nothing.* **MI** at most a program-change number. **PF** nothing. **EQ** — unclear, and
  today literally nothing: `StudioGraphSpec` has no `PartialEq`. **SD** everything.

**Residue: empty.** Five of six realizations extract nothing at all.

> **Proposition 1 (derived).** A patch is not part of the motive of a piece. It is part of the *coefficient data of one
> realization*.

This is the analogy doing real work for once. Betti cohomology takes a coefficient ring; ℓ-adic cohomology takes a
prime. Neither coefficient choice is part of the variety. R1 already has this shape and nobody noticed:

```text
prepare(M, B, s)      -- M the gesture timeline, B the instrument bindings, s the seed
```

`M` is the motive; `B` and `s` are coefficients. So the answer to "what calculus goes under the studio?" is **not** "the
same one as the score, or a second one beside it." It is: *the studio is the coefficient system of the sound
realization, and its structure is whatever makes that realization functorial in it.* Which is exactly what R1 says.

Two consequences follow immediately, and both are checkable:

1. **`B` must have an equality**, because a functor's argument needs one for the functor to be well defined on
   isomorphism classes. Today `StudioGraphSpec` derives `Clone, Debug, Default` and nothing else, so R1 is stated over
   an object half of which has no equality, and the cache key `semantic_hash(M) ⊕ B ⊕ s` is not computable. This is a
   defect R1 *entails*, not a defect one has to argue for separately.
2. **`docs/core-boundary.md` §6.7 is answering the wrong question.** "No calculus under the studio" is defensible if it
   means "the studio is not a second temporal calculus." It is indefensible as "the studio has no structure." The
   structure it needs is the structure of a coefficient system: identity, equality, and enough functoriality for R1.

## E9. First and second endings

- Every realization: one hearing, both endings sounding at different times. **EQ** the expansion is deterministic.

**Residue.** Nothing new. Recorded because `docs/kernel-hypothesis/06-evidence-log.md` G0.3 established that this
example was *mis-cited* as evidence for a conflict primitive, and a table that omitted the refuted row would be hiding
its own correction.

## E10. An ossia, or a mobile

- **EN** both alternatives are engraved. **PF/SD/MI** one of them happens. **AN** the piece *is* the set of
  alternatives. **EQ** — no answer today.

**Residue.** A genuine branching, extracted differently by EN (both) and SD (one). This is the only row where two
realizations disagree about *how many pieces there are*. Atom 3 was refuted as proposed; this row records that the
underlying phenomenon is real and unmodelled, and that the disagreement is between realizations rather than within one.

---

## The residue, assembled

Reading down the "residue" lines, and keeping only what at least two realizations need:

| Component | Read by | Contested? |
| --- | --- | --- |
| Exact rational span (position + extent) | all six | no — Atom 1, never challenged |
| A payload **point**, read at different granularities per realization | all six | no, but the *tower* is new here — see E1, E2 |
| Voice membership | EN, AN | settled: payload tag suffices (G0.2, G2.1) |
| Metrical layers in force | EN, AN | **live** — Atom 4, and E5 is its cleanest statement |
| Shape on a span (curves) | PF, SD, AN | settled: opaque payload, L24 |
| Correspondences between passages | AN, EQ | **live and barely modelled** — E6 |
| Branching | EN vs SD disagree | parked — E10 |
| Patch / instrument binding | SD only | **not in the motive at all** — E8, Proposition 1 |

Two things fell out that were not put in:

- **E8 dissolves the question the directory was opened to answer.** The studio does not need a calculus *of the piece*
  because it is not part of the piece. That is a dissolution in the Grothendieck sense — the problem disappears at the
  right level rather than being solved.
- **E1 and E2 together say the payload is a tower of quotients, not a type.** Each realization reads at its own
  granularity, and the granularities are ordered. That is [05-candidate-torsor.md](05-candidate-torsor.md), and it is
  why that candidate is the leading one.

One thing required a side condition and is flagged for [04](04-candidate-fibred.md): E5's layers cannot be added to the
present kernel without deciding whether layers are denotation or analysis, which `docs/kernel-hypothesis/01-atoms.md` §4
already names as the sharpest unresolved question and does not settle.
