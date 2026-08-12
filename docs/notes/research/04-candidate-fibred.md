# Candidate F — fibred over metrical context

**Source.** `~/Code/papers/category-theory/grothendieck-method/process.md` for the test;
`docs/rules/kernel/04-algebraic-laws.md` L18 and X-laws for the symptom; OMT `117-hypermeter.md`,
`118-metrical-dissonance.md`, `098-twentieth-century-rhythmic-techniques.md` §Polymeter for the content;
`docs/notes/research/kernel-hypothesis/01-atoms.md` §4 for the atom this argues for.

**One-line claim.** L18's side condition is not a wart. It is the kernel telling us that it has objects it has not
named, and naming them is the same move as admitting metrical layers.

---

## 1. The test, and the law that fails it

> If your proof requires a side condition that does not appear in the theorem statement, your definitions are at the
> wrong level.
>
> — `~/Code/papers/category-theory/grothendieck-method/process.md` §3

`docs/rules/kernel/04-algebraic-laws.md` L18:

```text
If duration(M) = duration(N) and duration(P) = duration(Q), then
    (M ⊕ N) ; (P ⊕ Q)  =  (M ; P) ⊕ (N ; Q)
```

The interchange law is the axiom that makes a two-composition structure coherent. Here it carries a proviso about the
arguments' extents which does not appear anywhere in the equation. By the test, the definitions are at the wrong level.

The kernel already knows the proviso is load-bearing — it keeps a counterexample test,
`interchange_fails_without_synchronization`. Worth noting what that test *is*, musically: unequal section durations, one
voice's material starting under another's still-sounding section on one side and after it on the other. That is a
hemiola. **The kernel's counterexample to its own interchange law is a piece of music with a name** (OMT
`118-metrical-dissonance.md`).

## 2. Diagnosis

In a category, `g ∘ f` is not "composition, subject to the side condition `cod(f) = dom(g)`." It is simply not defined
otherwise. The condition is discharged by *typing* and never appears in a law. So a side condition of exactly this shape
— "the operation behaves well when two arguments agree about something" — is the signature of a structure whose objects
have not been written down.

> **Proposition 4 (derived).** L18's proviso is an object-matching condition. Verify by extent alone: the left side has
> extent `max(dM,dN) + max(dP,dQ)`, the right `max(dM+dP, dN+dQ)`. With `dM = dN` and `dP = dQ` both are `dM + dP`.
> Without it they differ — take `dM=0, dP=1, dN=1, dQ=0`, giving `2` against `1`. So the two sides are not even equal
> *as extents* off the diagonal; L18 is not a law with an exception but a law about a substructure.

## 3. Where the side condition came from

It was bought, deliberately, and the receipt is in the repo. `docs/rules/kernel/03-denotational-semantics.md`:

> …of unequal extents takes the maximum without padding the shorter argument. What went is the operation that only ever
> [padded]

`extend` was struck at prompt 37 (L7 and L8 went with it) for want of a caller. That made `⊕` **total** across extents.
Totality across extents is exactly what costs unconditional interchange:

|  | `⊕` | interchange |
| --- | --- | --- |
| **Today** — max, no padding | total on all pairs | conditional (L18) |
| **Fibred** — `⊕` within one extent, padding explicit | partial; padding is a coercion | unconditional |

This is the "do not break symmetry for convenience" rule from
`~/Code/proofs/.claude/skills/rethink-math/references/working-rules/core-rules.md`: one representative (the max) was
chosen to shorten notation, and the cost was paid in a proviso. Neither column is obviously right — but the choice
should be made knowing it is a choice, and today it is not recorded as one.

## 4. What the base should be

Two bases, at two strengths. The weaker one fixes L18; the stronger one is Atom 4.

**F-weak — fibre over `ℚ≥0`.** Objects are extents; a timeline of extent `d` is a morphism in the fibre over `d`; `;`
moves between fibres by adding, `⊕` acts within one. L18 becomes unconditional. This is [03](03-candidate-graded.md)'s
grade algebra used as a base rather than as bookkeeping. It buys the law and nothing musical.

**F-strong — fibre over metrical contexts.** An object is a finite set of pulse layers `(phase, period)`, which is
`docs/notes/research/kernel-hypothesis/01-atoms.md` Atom 4 exactly. Then:

- Re-barring is a **change of base** that leaves the fibre alone — which is Atom 4 §4.2's "the barline is derived",
  arrived at from the other direction.
- Krebs's taxonomy is the classification of *maps between base objects*: equal period and unequal phase is displacement
  dissonance; incommensurable periods is grouping dissonance. Atom 4 §4.1 already observes the taxonomy falls out; the
  fibred reading adds that it falls out as a classification of base morphisms, which is where a classification of that
  shape belongs.
- OMT `117`'s remark that renotating the same sounding content in a different time signature "makes a big difference" is
  the statement that the fibres over different bases are genuinely different objects, not presentations of one.

## 5. Derivations

**D-1. L18 unconditional.** Immediate from Proposition 4 once `⊕` is fibre-local. *What does the work:* both arguments
of a `⊕` live over the same object, so `max` is idempotent on their common extent.

**D-2. The barline is not in the motive; the layers are.** [01](01-realizations-and-residue.md) E5 reached this from the
realization table (engraving may re-bar freely; analysis reads layers). F reaches it structurally: base data is carried,
presentation of the base is not. Two independent routes to one verdict.

**D-3. Polymeter becomes representable without a new operation.** A polymetric passage is one fibre whose base carries
two incommensurable layers. No constructor is added, so `docs/rules/kernel/10-term-calculus.md`'s scope rule is not
touched and T4's totality argument is untouched — the change is to what an object is, not to what terms exist.

## 6. Gaps — stated, not patched

**G-1. Where do layers come from?** `docs/notes/research/kernel-hypothesis/01-atoms.md` §4 names this as the sharpest
unresolved question in that directory and does not settle it: notated meter supplies one layer, but hypermeter and
implicit polymeter are *heard*. If layers are denotation, an analysis that hears a hypermeter is changing the piece. If
they are analysis, the base is not part of the motive and F-strong collapses to F-weak. **F does not resolve this and
must not pretend to.** It is Q-B there.

**G-2. Reintroducing padding needs a caller.** Course-correction §34 is the acceptance test, and `extend` was struck
once already for want of one. F-weak's coercion is `extend` under a new name; proposing it without a consumer would
repeat prompt 37's mistake in reverse.

**G-3. F says nothing about pitch.** Like [03](03-candidate-graded.md), it addresses time only. It is orthogonal to
[05](05-candidate-torsor.md) rather than competing with it.

## 7. Five-example sieve

| # | Case | Verdict |
| --- | --- | --- |
| 1 | trivial: one note in 4/4 | passes |
| 2 | two voices, equal length | passes; L18 applies with no proviso |
| 3 | composition: two sections sequenced | passes; base changes by addition |
| 4 | a voice that rests while another continues | **passes only with explicit padding** — the case that costs F its totality, and the honest price of D-1 |
| 5 | hardest: an explicit polymetric passage, 3/4 against 4/4 written simultaneously | passes under F-strong, fails under F-weak and fails today — this is Atom 4's falsifier and the reason F-strong is worth the trouble |

## 8. Verdict

**The strongest structural candidate, and the one to test next.** It is the only candidate here that is argued from a
defect *internal to the existing kernel* — L18's proviso — rather than from an outside framework, and it lands on the
one atom that survived both gates in `docs/notes/research/kernel-hypothesis/`. It is not the motive either: it describes
the base, and [05](05-candidate-torsor.md) describes the fibre's payload. If both are right, the motive is a pair, and
§6 G-1 is the question that decides whether the base half is real.
