# The sieve, and what the motive looks like so far

All four candidates against [00](00-the-motive-question.md)'s three tests, then the assembled picture and the ranked
next probes.

---

## 1. The three tests

|  | T-1 factorization | T-2 no side conditions | T-3 tight fit |
| --- | --- | --- | --- |
| **P** polarity | **passes** — locates every realization on one side or the other, and gives sound its factorization through `U` | passes — the adjunction has no provisos | n/a — P types the boundary, it carries no content |
| **G** graded | n/a | **fails** — needs an annihilator music does not have (§2) | **too coarse** — silent on 9 of 10 residue rows |
| **F** fibred | n/a | **passes, and is the only candidate that removes one** — L18 becomes unconditional | untested; F-strong stands or falls with Q-B |
| **T** torsor | **passes for pitch** — every realization is a quotient map (Prop 5) | passes | **too coarse for form, timbre, rhythm**; exactly right for pitch |
| **E** enriched | n/a — E is a criterion for the term language, not a description of the object | passes — Prop 7 *replaces* a side condition with a test ("name the consumer") | n/a; it fits the compiler, not the music |

No candidate passes all three alone. That is the finding, not a failure: the tests are separating them by *which part of
the object* each describes, which is what a good sieve does.

## 2. Coverage against the residue

From [01](01-realizations-and-residue.md)'s assembled table:

| Component | Covered by | Status |
| --- | --- | --- |
| Exact rational span | Atom 1 | settled, never challenged |
| Payload point + quotient tower | **T** | leading; D-2 refuted, T corroborated by convergence (P-2) |
| Voice membership | payload tag | settled by Gates 0 and 2 |
| Metrical layers | **F** | live; blocked on Q-B |
| Shape on a span | L24 | settled |
| Correspondences between passages | **nobody** — T §4 is a conjecture | the gap |
| Branching | nobody | parked (Atom 3 refuted as proposed) |
| Patch / bindings | **P** places it outside the motive; **E** says where its *description* lives | dissolved — see §4 and [08](08-candidate-enriched.md) |

## 3. The assembled picture

Putting the surviving pieces together, stated as precisely as the evidence currently allows:

> **The motive of a piece, provisional.** An object of the base — a finite set of pulse layers `(phase, period)` — over
> which sits a finite multiset of occurrences on exact rational half-open spans, each carrying a payload that is a
> *point in a torsor* under a group `G`, together with the tower of quotients `A/G′` for the subgroups `G′` each
> consumer is invariant under. Realizations are the quotient maps. The morphisms are `G`-correspondences, and the sound
> realization escapes the finite world through the polarity adjunction, its instrument bindings being coefficient data
> rather than part of the piece.

Three of those four clauses are supported. The morphisms clause is a slogan with one worked example. That is the honest
state, and it is why this is `docs/scratch/` and not a proposal.

**What is notable is how little of this is new machinery.** Atoms 1, 5, and 6 need no kernel change at all; F changes
what an object is rather than what terms exist; P is a way of reading a boundary the repo already has. The candidate
that would have required the most new machinery — a single unifying algebra containing both the temporal and the signal
core — is the one the work dissolved rather than built.

## 4. What fell out

Per the working rules: what emerged that was not put in.

1. **The studio question dissolved.** The directory opened because `docs/core-boundary.md` §6.7 says the studio gets no
   calculus, which does not follow from §5. The answer is not "give the studio a calculus" and not "unify the two
   cores." It is that a patch is *not part of the piece at all* — five of six realizations extract nothing from it
   ([01](01-realizations-and-residue.md) E8) — and is coefficient data for one realization, which R1's own shape
   `prepare(M, B, s)` already said. §6.7 is defensible as "no second temporal calculus" and false as "no structure."
   **[08](08-candidate-enriched.md) then supplied the criterion that makes this actionable**: SPJ keeps `let` in the
   core because type-checking cannot be stated after it is erased, and musa's render cache cannot be stated after
   `StudioGraphSpec` is erased. Same argument, same remedy — the patch's *description* stays in the term language as far
   as its last consumer, while the patch itself stays out of the motive. Neither of the two options the question was
   originally framed around was the answer.
2. **L18's proviso is evidence, not noise.** It is the kernel reporting that it has unnamed objects, and the objects it
   wants are the ones Atom 4 wants for unrelated reasons ([04](04-candidate-fibred.md) §2). Two independent arguments
   converging on one atom is the strongest structural signal in this directory.
3. **The kernel's own counterexample test is a hemiola.** `interchange_fails_without_synchronization` is a piece of
   music with a name in OMT `118`. The algebra's exception class is a musical category.
4. **Durations are not a semiring, for a musical reason.** No annihilator, because no music swallows its continuation
   ([03](03-candidate-graded.md) §2).
5. **Harmonic function survived a test it was expected to fail** — scale degree is the orbit under translation-by-tonic,
   so tonal function sits in the same tower as set class ([05](05-candidate-torsor.md) §5).
6. **The pitch layer had already built T's tower, and nobody had said so.** `scale.rs` has the sections, the quotient
   map, a doc comment reaching for "canonical representative", and a test asserting a commuting square — written with no
   category theory in view ([07-probe-log.md](07-probe-log.md) P-2). This arrived by *refuting* the prediction that was
   supposed to support T, which is the most useful thing that happened in this directory: the probe designed to find a
   deficiency found a structure instead.

## 5. What required side conditions

- **G** needs an annihilator to be a resource algebra. It does not get one. Wrong level.
- **F-weak** needs padding reintroduced, which needs a caller under course-correction §34 — and `extend` was struck once
  already for lacking one.
- **F-strong** needs Q-B answered: are layers denotation or analysis?
- **T** needs the correspondence category to say anything about form, and does not have it.

## 6. Ranked next probes

1. ~~**Probe T/D-2**~~ — **run; refuted. See [07-probe-log.md](07-probe-log.md) P-1 and P-2.** D-2 predicted the
   quotient tower would be missing and re-derived by hand. It is already built: `scale.rs` has sections both ways, the
   quotient map `Frame::locate` documented as returning "the canonical representative", and a test asserting a square
   between two levels commutes. T is *corroborated* by the convergence and *not* advanced by it — the probe tested the
   repo, not the music. The one residue is a cleanup: `voice_leading.rs:445` open-codes what `Frame::locate` provides.
2. **Answer Q-B** (`docs/kernel-hypothesis/05-open-questions.md`) — decides whether F-strong is a kernel change or an
   analysis feature, and therefore whether the base is in the motive.
3. **Constrain `B`'s equality when `B` is introduced** — *restated after [07](07-probe-log.md) P-3 corrected it.* The
   earlier wording claimed a present defect and named the wrong object. `B` is R1's *instrument bindings*, not
   `StudioGraphSpec`, and `B` does not exist yet: prompts 130, 131, and 132 are all `pending`. So this is not a repair,
   it is a design constraint on 131/132 — whatever `B` becomes must be comparable and hashable, since R1's cache key
   `semantic_hash(M) ⊕ B ⊕ s` is only well-defined if it is. Cheap at introduction, expensive to retrofit. If `B` ends
   up carrying anything graph-shaped, note the ordering trap: identity that is positional rather than structural makes
   two identical bindings built in different orders miss the cache.
4. **Attack the correspondences** — the hottest and least supported idea in the directory ([05](05-candidate-torsor.md)
   §4). What is composition of two correspondences, and do idempotents split? Until this has an answer, form has no
   account and the motive is incomplete in a way the other gaps are not.
5. **Settle `StudioSpec` versus `StudioGraphSpec`** — the question P-3 opened, and the only one in this list that is
   both fully posed and independent of the motive. Two representations of one graph, in `musa-compiler` and
   `musa-audio`. Prop 7's test decides it: name the pass that needs the second, or let the first survive to that pass.

## 7. Did we find the motive?

No — and the shape of the "no" is worth keeping. We found that the object has at least three separable parts (base,
payload, boundary), that two of them have good candidates needing no new machinery, and that the part which would tie
them into a single thing — the morphisms — is the part nobody has written down. That is a better position than the
directory started in, where the question was "what algebra goes under the studio" and the answer turned out to be that
the studio was in the wrong place.

What the directory *did* settle is smaller and immediately useful: the question it opened with. The studio needs no
calculus of its own and no merger with the temporal one; it needs its description to survive with an equality as far as
the consumer that reads it ([08](08-candidate-enriched.md) Prop 7). That is one prompt's worth of work and it does not
wait on the motive. Everything else here — the base, the payload, the morphisms — remains research, and the morphisms
remain the part where there is nothing but a slogan.
