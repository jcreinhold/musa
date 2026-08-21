# Candidate P — polarity (call-by-push-value)

**Source.**
`~/Code/papers/logic-and-computation/type-theory/call-by-push-value-decomposing-call-by-value-and-call-by-name/text.md`
(Levy). Supporting: `normalization-by-evaluation-for-call-by-push-value-and-polarized-lambda-calculus`.

**One-line claim.** The score/signal boundary that `docs/core-boundary.md` §5 erects as a fence is the CBPV polarity
boundary, and stating it that way replaces the fence with an adjunction.

---

## 1. The subsystem, written out

CBPV has two disjoint classes of term and two of type. Levy §3:

```text
value types        A ::= U B | Σᵢ Aᵢ | 1 | A × A
computation types  B ::= F A | Πᵢ Bᵢ | A → B

Γ ⊢ᵥ V : A      "V is a value of type A"
Γ ⊢ᶜ M : B      "M is a computation of type B"
```

The operational summary, which is the part that matters here (Levy §5.2):

```text
A value of type U B        is a thunk of a computation of type B.
A computation of type F A  returns a value of type A.
A computation of type A → B pops a value of type A, then behaves as B.
```

and the slogan the whole calculus is organized around:

> **a value *is*, whereas a computation *does*.**

An identifier may be bound only to a value, so contexts hold value types only. `U` and `F` are adjoint: `F ⊣ U`.

## 2. The reading

| CBPV | Musa |
| --- | --- |
| value type | notation, score, anything finite and inductive — `Timeline[ScoreFact]`, `Gesture`, `Pitch` |
| computation type | a signal: an unbounded sample stream, defined by what it produces when observed |
| `F A` | a computation that yields a score value — the compiler's own pipeline stage |
| `U B` | **the prepared render plan**: a *value* that suspends a signal computation |
| `force` | what `musa-playback` does in the audio callback |
| `thunk` | what `prepare(M, B, s)` returns |

The claim is narrow and checkable: `docs/core-boundary.md` §5 says signals are coinductive, the kernel is inductive, and
"the object that crosses is the prepared render plan." CBPV says the same three things, and says them as one thing:
values are the positive/inductive pole, computations are the negative/coinductive pole, and `U` is the unique way a
computation becomes something a value context may hold.

## 3. Derivations

**D-1. "Signals stay out of the core" falls out.** A context binds only value types (Levy §3). A signal is a computation
type. Therefore no signal can be bound to an identifier or placed in a payload — not by fiat, but because payloads live
in value position. *What does the work:* the restriction of contexts to value types.

**D-2. "The prepared render plan is the object that crosses" falls out.** The only value built from a computation is
`U B`. So if anything crosses from the signal world to the score world, it is a thunk. *What does the work:* the type
grammar; `U B` is the only constructor mentioning `B` in `A`'s grammar.

**D-3. R1 becomes a typing statement rather than an axiom.** R1 says semantically equal gesture timelines prepare
identically. Under P, `prepare : Timeline[Gesture] → Bindings → Seed → U(Signal)` is a function *in value position*, so
it is a function of its arguments' values, so it respects any equality those values are quotiented by. R1 stops being a
law one must impose and becomes the statement that `prepare` is a function at all. *What does the work:* `U(Signal)` is
a value type, so `prepare` is an ordinary value-level function.

**D-4. Sharing and T2 fall out of complex values.** Levy §8.2 adds *complex values* — values formed with `let` and
pattern matching — and proves (Prop 14) definability and conservativity: complex values do not affect computations, and
an equation between computations is provable with them iff without them. That is exactly T2 (`a let bound once and used
twice is the same term as the same body written twice`) and exactly the justification for prompt 127's sharing table:
the `let shared` bindings the elaborator introduces are complex values, and Prop 14 is the theorem that says introducing
them changes nothing. *What does the work:* Prop 14's conservativity.

**D-5. The studio's location, cross-checked.** [01](01-realizations-and-residue.md) E8 concluded a patch is coefficient
data for one realization. Under P a patch is a *value* — it is finite, first-order, and inspectable — of type
`U(Signal)` or of a value type that `prepare` consumes to produce one. Either way it sits in value position, which means
**the studio graph is governed by the value calculus** (`docs/rules/language/02-core-calculus.md`), and
`docs/core-boundary.md` §6.7's "no calculus under the studio" is false as stated while its intent — no *second temporal*
calculus — is preserved exactly. *What does the work:* patches are finite and inspectable, hence positive.

## 4. Gaps — stated, not patched

**G-1. CBPV as published has no recursive types.** Levy: *"We omit recursive types, as these are beyond the scope of
this paper."* A sample stream needs a recursive computation type (`νX. A → X`, or a resumption). So P gives the right
*shape* for the boundary and does not, on its own, supply the object on the far side. This is the largest gap and it is
not fatal — the far side is `musa-dsp`, which is not asking the kernel for a type — but it must not be papered over.

**G-2. Negative ≠ coinductive, exactly.** CBPV computation types are algebras for a strong monad (Levy §3); "codata
defined by its observations" is the polarized reading, which is compatible but not identical. `A → B` pops, `Πᵢ Bᵢ` pops
a tag — those are observations, so the reading is well supported — but `F A` is not an observation, and the
identification of the negative pole with coinduction is *judged*, not derived.

**G-3. P says nothing about time.** Nothing in this candidate mentions rational extents, meters, or occurrences. P
locates the boundary; it does not describe either side. It composes with [03](03-candidate-graded.md) and
[04](04-candidate-fibred.md) rather than competing with them, and it does not compete with [05](05-candidate-torsor.md)
either.

## 5. Five-example sieve

| # | Case | Verdict |
| --- | --- | --- |
| 1 | trivial: a single note | passes — a value, nothing to say |
| 2 | one effect: a note reaching sound | passes — `force` of a thunk; D-2 |
| 3 | composition: two patches in series | passes — value-level composition, D-5 |
| 4 | dependency across a morphism: a gesture curve read by a filter | passes — the curve is a value payload (L24), read at force time |
| 5 | hardest: a live-coded pattern that both sounds continuously and has notated extent | **fails** — this is precisely `docs/rules/kernel/08-open-questions.md` Q1, and P has no answer: the object is negative (it sounds forever) and positive (it has an extent), and CBPV forbids that. |

Case 5 failing is the informative one and it is *the same* failure `docs/core-boundary.md` §5 names as the trigger that
reopens the signal question. Two independent framings agreeing on where the boundary breaks is evidence the boundary is
in the right place.

## 6. Verdict

**Keep, as the frame for the boundary; it is not the motive.** P answers "what kind of thing is on each side and how do
they connect" better than anything else read, and it answers the studio question that opened this directory. It does not
answer "what is a piece of music," because it says nothing about time, pitch, or meter. Its value is that it makes three
separate stipulations in `docs/core-boundary.md` — signals out, plan crosses, R1 — into one adjunction with the first
two as consequences.
