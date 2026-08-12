# Candidate G — duration as a grade

**Source.**
`~/Code/papers/logic-and-computation/type-theory/quantitative-program-reasoning-with-graded-modal-types/text.md`
(Granule), for grading by a resource algebra; `docs/rules/kernel/03-denotational-semantics.md` and
`04-algebraic-laws.md` for what is being graded.

**One-line claim.** A timeline's extent is a *grade*, the two temporal operations are the two operations of the grade
algebra, and `duration` is a homomorphism whose kernel is exactly the multiplicity information the non-laws X1 and X2
are about.

---

## 1. The observation

From `docs/rules/kernel/03-denotational-semantics.md` line 52 and `04-algebraic-laws.md` L3:

```text
M ; N = (d + e, ...)         duration(M ; N)  = duration(M) + duration(N)      (L3)
M ⊕ N = (max(d, e), E ⊎ F)   duration(M ⊕ N) = max(duration(M), duration(N))
```

Sequence adds extents; overlay takes their maximum. That pair — `(max, +)` — is the max-plus, or tropical, structure.

> **Proposition 2 (derived).** `duration : Timelines → (ℚ≥0, max, +)` is a homomorphism of both operations, by L3 and by
> the denotation of `⊕`.

## 2. Where it is not a semiring, and why that is the interesting part

The tropical semiring proper is `(ℝ ∪ {−∞}, max, +)` with additive identity `−∞` and multiplicative identity `0`. Check
the axioms against `ℚ≥0`:

| Axiom | Holds? |
| --- | --- |
| `(ℚ≥0, max, 0)` commutative monoid | yes |
| `(ℚ≥0, +, 0)` monoid | yes |
| `+` distributes over `max`: `a + max(b,c) = max(a+b, a+c)` | yes |
| **additive identity annihilates**: `0 + a = 0` | **no** — `0 + a = a` |

So durations satisfy every tropical-semiring axiom **except zero-annihilation**, and they fail it because the additive
and multiplicative identities coincide at `0`. This matters twice over:

1. **Granule-style grading needs a genuine semiring.** A graded modality is indexed by a resource algebra that is at
   least a pre-ordered semiring. Candidate G therefore cannot be imported wholesale; it needs either an adjoined bottom
   or a weaker indexing structure. *This is a side condition, and per
   `~/Code/papers/category-theory/grothendieck-method/process.md`'s test it is evidence the level is not yet right.*
2. **The missing element is musically meaningful, not a technicality.** An annihilator would be a timeline `⊥` with
   `⊥ ; M = ⊥` — music after which nothing can follow. `docs/rules/kernel/03-denotational-semantics.md` is explicit that
   `(0, ∅)` is *not* this: "empty timelines have extent, and extent is real." There is no musical object that swallows
   its continuation. So the structure is not a semiring **because music has no annihilator**, which is a better reason
   than a missing axiom.

## 3. What the homomorphism forgets

The non-laws are where this candidate earns its place.

- **X2 — no distributivity.** `M ; (N ⊕ P) ≠ (M ; N) ⊕ (M ; P)`: the left has one copy of `M`, the right has two.
- But apply `duration` to both sides: `d_M + max(d_N, d_P)` versus `max(d_M + d_N, d_M + d_P)`. **These are equal**, by
  the distributivity that does hold in the grade algebra.

> **Proposition 3 (derived).** The timeline algebra does not distribute; its image under `duration` does. Therefore
> `duration` is not injective, and what it forgets is exactly the occurrence multiplicity that makes X2 and X1 fail.

That is a precise statement of what a grade can and cannot see, and it predicts the limit of the candidate before any
implementation: **a duration grade can typecheck alignment and can never see doubling.** X1 (overlay is not idempotent —
two performers playing the same note must not collapse) is invisible to it.

## 4. Derivations

**D-1. L6 is the graded unit law.** L6 says overlay at fixed extent `d` has identity `(d, ∅)`. Under G this is "the unit
at grade `d`", and the reason there is a family of units rather than one is that `⊕` is only unital *within* a grade.
*What does the work:* `max(d, d) = d`.

**D-2. L18's side condition is a grade equation.** L18 requires `duration(M) = duration(N)` and
`duration(P) = duration(Q)`. Under G that is: interchange holds when the grades agree. This is the observation
[04](04-candidate-fibred.md) is built on, and G supplies its arithmetic form.

**D-3. Nothing musical falls out.** Running the ten examples of [01](01-realizations-and-residue.md) past G: it says
something about E3 (exact rational time) and E5 (alignment, via D-2) and *nothing whatever* about E1, E2, E4, E6, E7,
E8, E9, E10. A grade is bookkeeping. It is not a motive.

## 5. Five-example sieve

| # | Case | Verdict |
| --- | --- | --- |
| 1 | trivial: `(0, ∅)` | passes — grade 0, unit of both |
| 2 | one note | passes |
| 3 | composition: `M ; N` | passes — grades add |
| 4 | dependency: a motif whose length depends on an argument | passes, and is the one place G is *useful* — the grade is a computed extent, which is what a length-indexed type would give |
| 5 | hardest: `M ⊕ M` versus `M`, and `M ; (N ⊕ P)` | **fails informatively** — both sides agree in grade and differ as music (X1, X2). G cannot see the difference it most needs to see. |

## 6. Verdict

**Keep as a component, reject as the motive.** Proposition 3 is worth having written down because it fixes exactly what
duration bookkeeping can decide, and D-2 hands [04](04-candidate-fibred.md) its arithmetic. But a candidate that is
silent on nine of ten rows of the residue table is not the object every realization factors through. The annihilator
finding in §2 is the durable part: it is a small, exact, musically-grounded fact about why the obvious algebraic import
does not fit.
