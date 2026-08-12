# The proposal — indexed call-by-push-value over two index domains

**Status: proposal. Governs nothing.** This is the directory's synthesis: what the four surviving candidates look like
when assembled into one calculus, what its typing rules and operational semantics are, where dependency is genuinely
required, and in what order the work should be done.

**One-line claim.** The audio and musical domains do not need one algebra containing both. They need **one type
discipline instantiated at two index domains**, joined at a single polarity boundary. Everything the directory found
falls into place under that frame, and the one law with a side condition loses it.

---

## 1. The diagnosis: three orthogonal axes, conflated

Every candidate in this directory turned out to describe a *different part* of the object, which is why none passed all
three tests of [00](00-the-motive-question.md) alone. They are not competitors. They are axes:

| Axis | What it separates | Candidate | Status |
| --- | --- | --- | --- |
| **Polarity** | the finite score from the unbounded signal | [P](02-candidate-polarity.md) | sound; already implicit in `docs/core-boundary.md` §5 |
| **Index** | one extent/metrical context from another | [F](04-candidate-fibred.md), [G](03-candidate-graded.md) | argued from L18's proviso |
| **Payload** | how much a realization forgets about pitch | [T](05-candidate-torsor.md) | already built in `scale.rs` ([07](07-probe-log.md) P-2) |

Plus one criterion that is not an axis at all — [E](08-candidate-enriched.md)'s Proposition 7, which says *when* a
construct may be erased.

The conflation is the whole problem. "Unify the two cores" sounded like a question about the **payload** axis (is a
signal a payload?) when it is a question about the **polarity** axis (is a signal a value?). §5 answered the second
correctly and §6.7 then over-answered the first. Separating the axes dissolves it.

## 2. The frame

**Call-by-push-value, indexed.** CBPV because Levy's slogan is exactly musa's boundary — *a value is, a computation
does* — and a score **is** while a signal **does**. Indexed because L18's proviso is a typing condition wearing a
proviso's clothes.

Two index domains, one discipline:

| Domain | Index | Musical/engineering meaning |
| --- | --- | --- |
| Musical | `(M, d)` — a metrical plan `M` of extent `d ∈ ℚ≥0` | what metre is in force, and how long |
| Audio | `(r, n)` — sample rate, channel count | the DSP graph's shape |

They meet at exactly one type, `U(Sig)` — the thunked signal, which is the prepared render plan `docs/core-boundary.md`
§5 already names as the thing that crosses.

### 2.1 The base is not a constant

One correction to [04](04-candidate-fibred.md), which assumed a fixed metrical context per piece. Metre changes. So the
base object is a **metrical plan of extent `d`**: a finite set of pulse layers `(phase, period)`, each piecewise
constant with finitely many change points in `[0, d)`. This stays finite and first-order, so it stays decidable.

Two consequences fall out rather than being stipulated, which is the test [00](00-the-motive-question.md) §3 sets:

- **Sequencing concatenates plans.** A 4/4 passage followed by a 3/4 passage is one plan with a change point. Metre
  change needs no construct.
- **Overlaying unions layer sets.** Two pieces with incommensurable periods overlay to a plan carrying both — which is
  *polymeter*, represented with no new operation. [04](04-candidate-fibred.md) D-3 predicted exactly this and needed
  F-strong to get it; here it is a consequence of `⊕`'s typing rule.

## 3. Typing rules

Value types (positive) and computation types (negative), in the CBPV manner. `A` ranges over payloads, which are
`G`-sets per [T](05-candidate-torsor.md).

```text
A ::= ScoreFact | Gesture | Pitch_G | ℚ | …                    payloads (G-sets)
P ::= Timeline[M; d](A)   |   U(C)   |   P × P | P + P | 1     values
C ::= F(P)   |   P → C   |   Sig[r; n]   |   C & C            computations
```

The temporal rules. `Γ ⊢ t : P` is the value judgment; `M ⌢_d N` is concatenation of plans with `N` shifted by `d`.

```text
        Γ ⊢ t : Timeline[M; d](A)        Γ ⊢ u : Timeline[N; e](A)
T-Seq   ───────────────────────────────────────────────────────────
        Γ ⊢ seq t u : Timeline[M ⌢_d N; d + e](A)


        Γ ⊢ t : Timeline[M; d](A)        Γ ⊢ u : Timeline[N; d](A)
T-Over  ───────────────────────────────────────────────────────────
        Γ ⊢ over t u : Timeline[M ∪ N; d](A)


        Γ ⊢ t : Timeline[M; d](A)        d ≤ e
T-Pad   ───────────────────────────────────────────────────
        Γ ⊢ pad_e t : Timeline[M ⌢_d ∅; e](A)


        Γ ⊢ t : Timeline[M; d](A)        r ∈ ℚ>0
T-Scale ───────────────────────────────────────────────
        Γ ⊢ scale r t : Timeline[r·M; r·d](A)


        Γ ⊢ t : Timeline[M; d](A)        0 ≤ i ≤ j ≤ d
T-Restr ─────────────────────────────────────────────────────
        Γ ⊢ restrict [i,j) t : Timeline[M|[i,j); j − i](A)
```

**T-Over is the whole point.** It demands the *same* extent. That single change is what the rest of this document is
buying, and §4 is why it is worth the price.

The polarity boundary, which is where audio enters:

```text
          Γ ⊢ g : Timeline[M; d](Gesture)     Γ ⊢ B : Bindings     Γ ⊢ s : Seed
T-Prep    ────────────────────────────────────────────────────────────────────────
          Γ ⊢ prepare g B s : U(Sig[r; n])


          Γ ⊢ v : U(C)                          Γ ⊢ node : Sig[r; m] → Sig[r; n]
T-Force   ──────────────────                    (audio graph typing, same discipline,
          Γ ⊢ force v : C                        different index domain)
```

`U(Sig)` is the only value type mentioning `Sig`. That is [02](02-candidate-polarity.md) D-2 restated as a syntactic
fact about the grammar, and it is what makes "signals stay out of the core" a *theorem about the type system* rather
than a policy in a document.

## 4. What this buys, stated as derivations

**D-1. L18 loses its side condition, and the counterexample becomes a type error.** Under T-Over, `(M ⊕ N) ; (P ⊕ Q)`
only typechecks when `dur M = dur N` and `dur P = dur Q`; the right-hand side `(M ; P) ⊕ (N ; Q)` then has matching
extents automatically. So interchange holds unconditionally *on well-typed terms*. The kernel's
`interchange_fails_without_synchronization` stops being a semantic counterexample and becomes a rejected program — which
is where a hemiola-shaped failure belongs. This is the Grothendieck test of [04](04-candidate-fibred.md) §1 satisfied
rather than merely diagnosed.

**D-2. Polymeter, metre change, and hypermeter need no new constructors.** §2.1. This matters under
`docs/core-boundary.md` §6.1's "no fourth combinator" forbid: the proposal adds *indices*, not forms.

**D-3. R1 stops being an axiom and becomes a theorem.** `docs/core-boundary.md` states R1 — `M ≡ N` implies
`prepare(M,B,s) = prepare(N,B,s)` — as a law to be *measured* at prompt 144. But the kernel already proves confluence
and strong normalization for its own calculus, and `docs/language/02-core-calculus.md` §5.5 proves it for the
elaboration core. If `prepare` is *defined by structural recursion on the kernel normal form*, R1 holds by construction:
equal normal forms give equal results. The measurement at 144 becomes a regression check on the definition rather than
an empirical test of a hoped-for property.

**D-4. The audio domain gets a real type system for free.** `Sig[r; n]` catches the DSP bug class that positional
`Vec`-of-nodes cannot: mono into a stereo input, control rate into an audio-rate port, a graph whose output arity does
not match the bus. Same index machinery, different domain — which is the coherence the question asks for, without
merging the two algebras that [01](01-realizations-and-residue.md) E8 showed have nothing to merge.

**D-5. `⊕`'s totality is the thing being sold, and the price is explicit rests.** Struck honestly: T-Over makes `over`
partial where today it is total, so `pad` returns — the `extend` that prompt 37 removed for want of a caller. The caller
is now the type system. And the musical argument is better than the type-theoretic one: *"this voice is silent for the
last two bars"* is a compositional statement, not bookkeeping. Making it explicit in the term is correct notation-first
design, not ceremony.

## 5. Operational semantics

The repo's existing two-stage evaluation is preserved exactly; nothing here proposes a third stage.
`docs/language/02-core-calculus.md` §6.1 is explicit that musa is deliberately **not** SPJ's enriched-calculus
arrangement — the elaboration core has lambdas and the kernel has none, so there is no simplifying transformation
between them, and what connects them is evaluation applied twice:

```text
source ──elaborate──▶ core term ──evaluate──▶ Term[ScoreFact] ──evaluate──▶ Timeline[ScoreFact] ──prepare──▶ U(Sig)
                                  (functions gone)              (sharing gone)                    (polarity boundary)
```

This proposal adds indices to the *static* semantics of those stages and leaves the dynamic semantics alone. Concretely:

- **Value side stays big-step, total, deterministic.** The kernel's `ρ ⊢ t ⇓ (d, E)` rules are unchanged. Indices are
  erased before evaluation — they are checked, not computed with. This is the standard DML property and it is what keeps
  the runtime cost at zero.
- **Signal side is coinductive and demand-driven.** `force` on a `U(Sig)` is what the audio callback does, one block at
  a time. Productivity replaces termination as the correctness property, which is precisely why `docs/core-boundary.md`
  §5's argument was right and must not be reopened: an inductive, total, extent-carrying value calculus and a
  coinductive, productive, extentless computation calculus are different things, and CBPV's `U`/`F` is the standard way
  to have both without confusing them.
- **Type erasure is the erasure Proposition 7 licenses.** Indices survive to the type checker, which is their last
  consumer, and are erased after it — exactly SPJ §6.5's "essential for type-checking, though it can be transformed …
  after that" ([08](08-candidate-enriched.md)).

## 6. How dependent, exactly

This is where a proposal like this usually overreaches, so it is worth being precise about what is *not* being asked
for.

**Not full dependent type theory.** No universes, no identity types, no proof terms, no tactic language, no
proof-irrelevance question, no need for a proof assistant in the build.

**Dependent ML–style indexed types** (Xi & Pfenning): types are indexed by terms drawn from a *separate, restricted*
index language, and type checking reduces to constraint solving in that language. The index languages here are:

| Index | Theory | Decidable? |
| --- | --- | --- |
| extent `d` | linear rational arithmetic | yes |
| metrical plan `M` | finite sets of `(phase, period)` over ℚ, finitely many change points | yes |
| rate `r`, channels `n` | linear integer arithmetic | yes |

So type checking is LRA/LIA constraint solving. That is implementable without a solver dependency for the common cases
(most extents are literal), and with a small one if general index variables are admitted.

**One critical implementation note.** This lives in **musa's** type system, not Rust's. Rust has no dependent types, and
nothing here asks it to. `Timeline<A>` in `musa-kernel` keeps its duration as an ordinary field at runtime; the index is
a static artifact of the object language, checked by `musa-compiler` and erased. Any version of this proposal that
requires const-generic gymnastics in Rust has misunderstood it and should be rejected on sight.

## 7. The motive, stated as sharply as the evidence allows

With the frame in place, the motive question becomes askable rather than merely gesturable.

**Objects.** A piece is a metrical plan `M` of extent `d` together with a `Timeline[M; d](A)` whose payload `A` is a
`G`-set — the torsor of [05](05-candidate-torsor.md).

**Realizations factor.** Each realization is a pair *(what it does to the base, what it forgets in the payload)*:
engraving presents the base and keeps the finest payload level; MIDI forgets the base's presentation and quotients by
enharmonic identification; set-class analysis forgets the base entirely and quotients by `T/I`; sound leaves the value
world through `U`. All are forgetful; that is what makes this a factorization and not a coincidence.

**Morphisms — and here the ground gets soft.** Grothendieck does not build motives from maps; he replaces maps with
**correspondences** (formal ℚ-combinations of cycles on `X × Y`) and then splits idempotents. The musical analogue is a
span `P ← R → Q`: *this passage of P corresponds to that passage of Q, under a group element and a time translation*.
Formal combinations let a variation resemble its theme partially, which a function cannot express. Composition is
pullback. And then:

> **Conjecture (judged, unproved).** The category of musical motives is the **Karoubi envelope** — the idempotent
> completion — of pieces-with-`G`-correspondences. A `motif` in the surface language is an object `(P, e)` with
> `e ∘ e = e`: a piece together with the projector that cuts it out.

**Why this is conjecture and not result**, stated plainly because [05](05-candidate-torsor.md) §4 has been carrying this
idea as a slogan and a slogan should not be promoted by being restated in better vocabulary:

1. Composition by pullback requires the category to *have* pullbacks. Unchecked.
2. The Karoubi envelope splits *every* idempotent, and most will be musically meaningless — the T-3 tight-fit failure
   risk from [00](00-the-motive-question.md) §3. The plausible discipline is to restrict correspondences to those
   generated by `G`, time translation, and time scaling, which is roughly OMT's actual repertoire of relations
   (transposition, inversion, retrograde, augmentation, sequence). Untested.
3. Nothing here yet gives *form* (AABA, sonata) an account, which was [05](05-candidate-torsor.md) §5's third failure
   and remains the gap that makes the motive incomplete.

So: §§2–6 are a design proposal with derivations. §7 is a research programme with one plausible shape.

## 8. Work order

Ordered so that each step is independently valuable and none depends on §7 being right.

| # | Step | Depends on | Character |
| --- | --- | --- | --- |
| 1 | Index the kernel by extent; `⊕` fibre-local; `pad` explicit | nothing | provable, testable, self-contained; delivers D-1 |
| 2 | Index the audio graph by `(rate, channels)` | nothing | catches a real bug class; delivers D-4 |
| 3 | Define `prepare` by recursion on normal forms | 1 | turns R1 from axiom to theorem (D-3) |
| 4 | Settle `StudioSpec` vs `StudioGraphSpec` under Prop 7 | nothing | the question [07](07-probe-log.md) P-3 opened |
| 5 | Add the metrical plan as the base | 1, and Q-B answered | the real design decision; delivers D-2 |
| 6 | Correspondences and the Karoubi envelope | 5 | research; may not land |

Steps 1–4 are engineering with proofs attached and touch no governing document's forbids — they add indices and a
coercion, not combinators. Step 5 amends `docs/core-boundary.md` and `docs/course-correction.md` §36 together and should
not be started until `docs/scratch/kernel-hypothesis/` Q-B ("are metrical layers denotation or analysis?") has an answer,
because the answer decides whether the base belongs in the object at all.

## 9. What would falsify this

Kept because the directory's standard requires it, and because a proposal that cannot be wrong is not a proposal.

1. **A musical operation that must overlay unequal extents without an explicit rest.** If one exists, T-Over is wrong
   and prompt 37's totality was right. The burden is on this proposal to survive the search, not on the reader to supply
   one.
2. **`pad` acquiring no callers in real scores.** Then §4 D-5's trade was bad and the extent index is ceremony.
3. **R1 failing after step 3.** If `prepare` cannot be written by structural recursion on normal forms without
   consulting something normalization forgets, then the boundary is in the wrong place — which is
   `docs/core-boundary.md` §5's own stated reopening trigger, arrived at independently.
4. **Index inference proving undecidable in practice** — if real scores need index *variables* often enough that
   constraint solving becomes a build-time cost, the DML claim in §6 is wrong and the honest fallback is checking
   literal extents only.
