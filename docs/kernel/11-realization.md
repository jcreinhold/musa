# 11 — Realization

**Status: governing** (written at prompt 66, implemented at prompt 67, given its surface at prompt 68).

This document answers **Q2** — aleatory semantics — by the trigger Q2 itself named: the first aleatory surface feature
is now being designed. The answer is the working stance Q2 already held, confirmed by design rather than reversed:
**each realized performance of an aleatory construct produces an ordinary finite kernel timeline, and the choice
mechanism lives above the kernel.**

The kernel gains no operation, no term form, and no constructor. `10-term-calculus.md`'s T1–T5 and
`05-normalization.md`'s N1–N6 are untouched, and that is not a happy accident — it is the criterion the design was
chosen by.

## The music this is for

Not a gesture at "experimental music". Six concrete pieces, and every one of them defeats the obvious design:

| Work | The freedom | What a fixed alternative set would have to be |
| --- | --- | --- |
| Riley, *In C* | repeat each of 53 figures "as many times as you like" | unbounded |
| Stockhausen, Klavierstück XI | play the 19 fragments in any order you look at them in | 19! ≈ 1.2 × 10¹⁷ |
| Cage, *Music of Changes* | durations drawn from a continuum by chance operation | uncountable |
| Feldman, *Projection 1* | durations proportional to notation, fixed only in relation | uncountable |
| A jazz chart | "solo — 4 choruses, open" | not enumerable at all |
| A DJ edit or a dance cue | loop this bar until the cue lands | unbounded |

Two more the same mechanism covers without being about indeterminacy at all: a folk tune whose verse count depends on
how many verses there are, and a game or installation whose length is set at run time.

## The candidate that is refused, and why

The obvious design is a term form — `choose { a | b | c }` — argued for exactly as `Progress` was argued for at prompt
45: *an interchange file should carry the work, not one performance of it*. That argument is good. It still fails, and
the four reasons are recorded here because this is a design that will be proposed again.

1. **It cannot express the repertoire it is proposed for.** Read the table above. A finite alternative set covers the
   narrowest subcase — "one of these three endings" — and leaves out every piece that motivated the feature. A form
   that fails its own examples is not a kernel form.

2. **It breaks T2 (`let` transparency).** In `let x = choose { a | b } in over x x`, does sharing share the
   *decision*? Both readings are musically real: one performer's choice heard twice, or two performers choosing
   independently. Neither is canonical, which is §16's "no canonical `join`" wearing a new costume. T2 is not a
   decoration — it is what prompt 49's measured −36% elaboration time and −60% allocations rest on, because it is what
   makes it safe to evaluate a shared body once.

3. **It breaks T3, T4 and N6 together.** Evaluation stops being deterministic and stops being unique, so there is no
   normal form, so there is no semantic hash. Prompt 43 keyed the session's recompilation on that hash: editing an
   unchosen branch would change the work without changing the hash, and the session would not recompile. One term form
   would silently break a feature four prompts away.

4. **It destroys the artifact that justified it.** A `.musa.kernel` file containing `choose` cannot be normalized or hashed
   without a choice environment, so the environment must ship alongside the file — which makes the file a *realization*
   corpus after all, at the cost of every theorem above. The argument for `choose` is self-defeating: it buys nothing
   the refused design does not already give, and it pays for it in four places.

Applying §34's rule literally: removing `choose` makes nothing impossible, and adding it makes two consumers — the
engraver and the interchange format — strictly worse. It stays out. **§35 item 11 is upheld, not amended**: it forbids
aleatory choice *in the finite kernel*, which is exactly what this document does.

## What goes in instead

**A realization is a compile parameter. The freedom is a payload value. The kernel does not change.**

```text
source ──elaborate(realization)──▶ Term<ScoreFact> ──evaluate──▶ Timeline ──▶ page, performance, .musa.kernel
  │                                                                  ▲
  └── states the freedom as ordinary facts ──────────────────────────┘
      (a fragment, an open repeat, a free duration, an improvised region:
       printed by the page, resolved by the realization)
```

Two halves, and the split is the whole design:

- **The freedom is written in the source and survives into the timeline as ordinary occurrences.** The page can
  therefore print *ad lib.*, "repeat as many times as you like", a boxed fragment, or a proportional duration —
  because the instruction is a fact like any other fact, not a hole where a fact would be. This is prompt 58's rule
  exactly: *the timeline holds every pass; the page prints the instruction once.*

- **The decision is a `Realization`: a seed plus a set of explicit overrides.** It is an input to elaboration, not a
  thing the kernel knows about. By the time a `Term` exists, every choice is made — so evaluation is still total, still
  deterministic, still confluent, and the normal form still has a hash.

The determinism law, stated so an implementation can be tested against it:

> **R1.** Same source **and same realization** ⇒ same term, same normal form, same semantic hash, byte-identical
> exports.

## Identity: a choice must be nameable across an edit

The hard part is not making a choice. It is making the *same* choice again after the composer edits an unrelated bar —
otherwise every keystroke re-rolls the performance and the score view flickers with music nobody wrote.

That needs a stable name per decision site. Two obvious names are wrong:

- **Not a source span.** Reformatting the file would re-roll the performance. So would inserting a comment.
- **Not a `DeclarationId`.** Inserting a declaration renumbers everything after it, so adding one motif at the top
  re-rolls every decision below.

**A `ChoicePath` is a structural path of names**: the motif or bar a site sits in, and an ordinal within the innermost
*named* thing. Insert a bar at the top of a voice and the paths below it are unchanged, because names do not shift.
Rename a motif and its decisions move with it, which is the right answer — the composer renamed the thing the choice
belongs to.

**A site written among a voice's own items has no name above it.** Its path is its ordinal alone, counted from zero in
each voice, so the k-th such site in every voice is one site with one decision. This was written "part, voice, motif,
bar" when the document was drafted, and prompt 67 found the reading wrong: prompt 57's rule is that a repeat barline
crosses the system, so a repeat the page can *draw* is one repeat of the whole piece, written once in each voice that
sounds under it. A per-voice path would decide it several times over and the voices would come apart — and the
engraver, which already refuses to draw repeats whose counts disagree, would silently write the passage out. It is the
same argument prompt 64 made for `meter`, reaching the same answer: what is written at a place in the piece belongs to
the piece.

A freedom that genuinely *is* one player's — *In C*, where each performer repeats independently and no barline could
span them — is a different construct with a per-voice path, and belongs with the rest of open form (prompt 68).

Randomness is derived **per path**, not drawn from a sequential stream:

```text
site_seed = fnv1a_128(seed ‖ path)
```

using the workspace's one stable digest (`musa-kernel/src/hash.rs`). A stream would re-roll every later decision when a
site is inserted — the same failure the span identity has, arriving later and much less visibly, because the first
decisions would still look right.

An **override** is a `ChoicePath` mapped to a decision. It wins over the derived value, which is what makes a
realization editable: a composer who likes the fourth pass but not the sixth pins the sixth and leaves the rest to the
seed.

## The cost, conceded up front

**The score view stops being a function of the source alone.** Every law of the form "same source, same picture"
becomes "same source *and the same realization*, same picture". That is a real weakening of a real guarantee, and the
two consequences must be paid rather than hidden:

1. **Fixtures pin a seed.** A regression fixture whose realization is unpinned is not a fixture. Every golden that
   touches an indeterminate construct states its realization in the file.
2. **The interface must be able to show the realization.** A composer who cannot see which decisions produced this
   page cannot tell why the page changed — and would rightly conclude the editor is unreliable. That is prompt 76's
   job, and it is a condition of this design rather than a nicety on top of it.

This is the honest price. It is smaller than the price of `choose`, and it is paid in one place instead of in four
theorems.

## What a conforming consumer owes

Adding to `07-backend-contract.md`'s list, and changing none of it:

1. **A `.musa.kernel` file is the projection of one realization.** It is not the work; it is one reading of the work. Its
   header says which realization produced it, and a consumer that reproduces the file must be given the same one.
2. **A consumer never chooses.** Choosing happens once, above the kernel, before a term exists. A consumer that draws
   a random number has produced a different piece and the semantic hash will say so.
3. **An unknown realization header is a refusal, not a default.** Reading a file whose realization you cannot
   reproduce and pretending otherwise is the one failure mode this whole design exists to prevent.

## What this does not settle

- **Q1 (infinite/live patterns)** and **Q5 (recursion)** stay open. An unbounded repeat count resembles both and is
  neither: the count is chosen at compile time and the result is an ordinary finite timeline.
- **No probability distributions, no weighted choice, no Markov models.** A seed and an override set is the whole
  mechanism. Generative composition is a different product, and one that would want the kernel to be a different
  thing.
- **No surface syntax.** Prompt 68 writes it, against this document.
