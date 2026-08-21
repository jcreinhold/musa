# Denotational semantics

What a kernel object denotes under the hypothesis, what the constructors do to it, which existing laws survive, and the
theorem that relates all of this back to `docs/rules/events/03-denotational-semantics.md`.

The claim to check first is §4: **the existing term language already writes down voice identity, and the existing
denotation throws it away.** If that is right, this proposal costs no new syntax.

---

## 1. E0 — the denotation

For a payload domain `A`, a finite kernel composition denotes a pair `(𝔈, d)`.

`𝔈 = (E, ≤, #, λ)` is a **finite labelled prime event structure**:

| Component | What it is | Condition |
| --- | --- | --- |
| `E` | a finite set of event occurrences | — |
| `≤` | succession: `e ≤ e′` means `e′` continues `e` | partial order |
| `#` | conflict: `e # e′` means no hearing contains both | irreflexive, symmetric, and **hereditary**: `e # e′ ≤ e″ ⟹ e # e″` |
| `λ` | `E → ℚ × ℚ × A`, giving each event its start, end, and payload | `0 ≤ s(e) ≤ t(e)` |

subject to one law linking `≤` to time (`01-atoms.md` §2.2):

```text
e < e′   ⟹   s(e) ≤ s(e′)
```

`d : 𝒞(𝔈) → ℚ≥0` is the **extent**, a monotone function on configurations with `d(x) ≥ max{ t(e) | e ∈ x }`. Extent is
per-hearing because a first ending and a second ending have different lengths, and a piece that has two of them does not
have *an* extent. For a conflict-free structure `𝒞` has a greatest element and `d` collapses to the single rational the
existing kernel calls `d`.

A **configuration** — one coherent hearing — is a subset `x ⊆ E` that is conflict-free and downward-closed under `≤`.
`𝒞(𝔈)` is the set of them, ordered by inclusion. By Winskel's theorem this is a coherent prime algebraic domain; the
only fact used below is that a conflict-free `𝔈` has `E` itself as its unique maximal configuration.

### 1.1 Why succession and not just "voice"

A tempting cheaper design is a partition of events into named voices. It fails on three counts. It cannot express
partial ordering (`01-atoms.md` §2.3, cited to OMT `110`). It cannot express a line that splits or merges — *divisi* and
its convergence — which a partial order handles with no extra machinery. And it reintroduces names, which Amendment V of
the constitution forbids: names are a second identity for events, and the whole point is that the structure is the
identity.

---

## 2. Constructors

Throughout, `⟦M⟧ = (𝔈, d)` and `⟦N⟧ = (𝔉, e)` with `𝔈 = (E, ≤_E, #_E, λ_E)` and similarly for `𝔉`.

### E1 — `timeline`

A flat timeline of `n` occurrences denotes the **discrete** structure: `E = {e₁ … eₙ}`, `≤` the identity, `#` empty, `λ`
the given spans and payloads, `d(x) = d` constant. Nothing continues anything; nothing excludes anything.

### E2 — `sequence`

For conflict-free `𝔈` — the ordinary case — `M ; N` is:

```text
E    = E ⊎ F
≤    = ≤_E ∪ ≤_F ∪ (E × F)
#    = #_F
λ    = λ_E on E,  τ_{d(E)} ∘ λ_F on F
d(x) = d(x ∩ E) + e(τ⁻¹(x ∩ F))
```

Everything in `M` precedes everything in `N`. Monotonicity holds because `τ_{d}` shifts every start in `F` past every
end in `E`.

**When `𝔈` has conflict, the continuation is duplicated.** For each maximal configuration `x` of `𝔈` there is a copy
`F_x` of `F`, translated by `d(x)`, with `F_x # F_y` for `x ≠ y` and `e ≤ f` for `e ∈ x, f ∈ F_x`. This is forced, and
the forcing is worth stating because it sharpens `01-atoms.md` §2.4: hereditary conflict means an event above two
conflicting events can never occur, so a single shared continuation after a branch is not expressible in a prime event
structure. The alternatives are duplication or general event structures with disjunctive enabling. **Judged:**
duplication, because it keeps the configuration domain well-behaved and because the cost is proportional to actual
branching — a piece with no alternatives pays nothing. An implementation must of course share rather than copy;
`04-operational-semantics.md` §4 says how.

### E3 — `overlay`

```text
E = E ⊎ F     ≤ = ≤_E ∪ ≤_F     # = #_E ∪ #_F     λ = λ_E ⊎ λ_F
d(x) = max( d(x ∩ E), e(x ∩ F) )
```

Nothing crosses. This is the whole content of the proposal in one line: `;` relates, `⊕` does not, and a piece written
as an overlay of sequences carries its voices in `≤` for free.

Overlay remains non-idempotent, for the reason `docs/rules/events/03-denotational-semantics.md` D3 already gives — two
performers playing the same note are two events — and now for a second reason: they are two *distinct* events, so a set
is the right structure and the multiset was a way of keeping distinctness without keeping identity.

### E4 — `alternative` (new)

```text
M ⊻ N :   E = E ⊎ F     ≤ = ≤_E ∪ ≤_F     # = #_E ∪ #_F ∪ (E × F)     λ = λ_E ⊎ λ_F
          d(x) = d(x) if x ⊆ E, else e(x)
```

The one new constructor. Every event of `M` conflicts with every event of `N`, so `𝒞(M ⊻ N) = 𝒞(M) ⊎ 𝒞(N)`: the hearings
of the whole are the hearings of one or the hearings of the other. First and second endings are
`body ; (ending₁ ⊻ ending₂)`; an ossia is `passage ⊻ alternate`; a mobile is an `⊻` of orderings.

`⊻` is associative and commutative, and — unlike `⊕` — **idempotent**: `M ⊻ M = M` up to isomorphism, because a choice
between two identical alternatives is not a choice. That `⊕` and `⊻` differ exactly on idempotence is the sharpest
available statement of what they respectively mean.

### E5 — `act` (new)

For `g` in the payload group `G`, `act g M` applies `g` to every payload and leaves `E`, `≤`, `#`, timings, and `d`
alone. For `g` in the time group, it applies to timings and leaves payloads alone; `scale_r` and `delay` are the
elements of the time group the kernel already has, and retrograde is the one it does not yet.

**Retrograde is the interesting case and it is a genuine finding.** Reversing time must reverse `≤` to preserve
monotonicity, so retrograde is not a payload operation and not a pure timing operation but an automorphism of the whole
structure. Under the multiset denotation retrograde is a coordinate flip on spans; under this one it also exchanges
"continues" with "is continued by," which is what a retrograde of a *line* actually is. A retrograde that reversed spans
and left voice metadata alone would produce a piece whose lines run backwards through their own material, and no test in
the current kernel would catch it.

### E6 — observation

`restrict`, `covering`, and `prevailing` (`docs/rules/events/03-denotational-semantics.md` D6, D10, D11) are unchanged
in substance. They are observations: they remove nothing from `E`, so downward-closure cannot break. They now take a
configuration argument, defaulting to the greatest configuration when `𝔈` is conflict-free — which is every existing
call site.

---

## 3. Theorem R — recovery

Let `U` forget structure:

```text
U(𝔈, d)  =  { ( d(x), ⦃ λ(e) | e ∈ x ⦄ )  |  x maximal in 𝒞(𝔈) }
```

each element a pair of an extent and a **multiset** of `(s, t, a)` triples — exactly a `D0` denotation.

> **Theorem R.** For every term `M` in the existing kernel language (E1, E2, E3, E5, E6 — that is, every term not using
> `⊻`), `U(⟦M⟧) = { ⟦M⟧_old }`, a singleton, and its element is the existing denotation.

*Proof sketch.* By induction on `M`. E1 gives an empty conflict relation. E2 and E3 form unions of conflict relations,
so conflict stays empty. A conflict-free finite `𝔈` has `E` as its unique maximal configuration, so `U` returns one
element, whose multiset is `λ[E]` and whose extent is `d(E)`. Comparing with D1, D2, D3 clause by clause: E1's `λ` image
is D1's multiset; E2's is `E ⊎ τ_d(F)` with extent `d + e`, which is D2; E3's is `E ⊎ F` with extent `max(d, e)`, which
is D3. ∎

Three corollaries, and they are the reason the hypothesis is cheap to try:

1. **Nothing is lost.** The existing denotation is recoverable from the proposed one by a total function.
2. **Every existing law that only mentions the old denotation still holds after `U`.** In particular L1–L17 and L19–L24
   survive verbatim under `U`, and most of them survive *unchanged* at the finer level — see §5.
3. **Realization is `U`.** `docs/rules/events/11-realization.md` had to define a separate mechanism to turn open form
   into performances. Under the hypothesis, realization is the forgetful map already needed for the recovery theorem,
   and the set it returns is the set of performances. One construct, not two.

---

## 4. Proposition B — the syntax already knew

> **Proposition B.** Take `M₁ = (c4 ; d4) ⊕ (a3 ; b3)` and `M₂ = (c4 ; b3) ⊕ (a3 ; d4)`, all four notes of unit extent.
> Then `⟦M₁⟧_old = ⟦M₂⟧_old` and `⟦M₁⟧ ≇ ⟦M₂⟧`.

*Proof.* Both old denotations are `(2, ⦃(0,1,C4), (1,2,D4), (0,1,A3), (1,2,B3)⦄)`: the multiset union is insensitive to
which sequence contributed which element. Under E2 and E3, `⟦M₁⟧` has `C4 < D4` and `A3 < B3` with no other strict
relations, while `⟦M₂⟧` has `C4 < B3` and `A3 < D4`. No isomorphism of labelled structures carries one to the other,
since an isomorphism must preserve `λ` and hence fix each of the four events. ∎

These are the two readings of `01-atoms.md` Proposition A: parallel motion and a voice exchange. The musician wrote them
differently; the parser preserved the difference; the elaborator preserved it; and the denotation discarded it, after
which every consumer had to recover it from payload metadata. This is the strongest single argument in the directory,
and it is an argument about *deletion*, not about expressiveness.

### 4.1 The corresponding cost

The converse over-generates. `seq(phrase₁, phrase₂)` where both phrases are four-part chords makes every note of the
first precede every note of the second, so the maximal chains include soprano-to-bass continuations that no musician
would name. The response is that this is *correct*: a term built out of block chords genuinely does not say which line
continues which, and a term built out of parts does. OMT `022-chords-in-satb-style.md` writes SATB in parts for
precisely this reason. **A voice is a maximal chain, and voices are determinate exactly when the music was written in
parts.** That is a real limitation, stated rather than hidden, and it is the honest content of the model.

---

## 5. What happens to the algebraic laws

This is the part of the proposal that breaks something, so it is stated in full.

### 5.1 Survive unchanged

L1 (`;` associative), L2 (`0` identity), L3 (duration additive, now per configuration), L4, L5, L6 (overlay monoid),
L9–L12 as `act` laws, L13–L15 (time action laws, since a positive rational scaling preserves `≤` and `#`), L16, L17
(restriction), L20, L21, L22, L23, L24, L19 (congruence). Each is proved the same way, with the added obligation that
the constructor's action on `≤` and `#` is respected, which E1–E5 make immediate.

X1 (overlay not idempotent) and X2 (no distributivity) survive and strengthen: `M ; (N ⊕ P)` and `(M ; N) ⊕ (M ; P)` now
differ in structure as well as multiplicity.

### 5.2 L18 fails, and its failure is the point

> **Corrected by Gate 0 (`06-evidence-log.md` G0.2).** This holds of the kernel with payload-blind values. It does not
> hold of the pipeline: voice tags travel in the payload and move with the notes, so both sides of L18 agree and no
> voice exchange is erased. The two tests named in §5.3 are not asserting anything false today. §5.3's migration claim
> is therefore about a change that has no current motivation.

`docs/rules/events/04-algebraic-laws.md` L18 — synchronized interchange — says that for equal durations,

```text
(M ⊕ N) ; (P ⊕ Q)  =  (M ; P) ⊕ (N ; Q)
```

and glosses it: "two voices across two synchronized sections can be built section-wise then sequenced, or voice-wise
then overlaid; the temporal facts are identical." Under E2 and E3 the left side has every event of `M` and `N` preceding
every event of `P` and `Q`; the right side has only `M < P` and `N < Q`. The two are not isomorphic whenever all four
are non-empty. **L18 is false.**

It should be. L18 is the law that says building music section-wise and building it voice-wise produce the same object,
and Proposition B is the observation that they do not — L18 with `M = c4, N = a3, P = d4, Q = b3` is exactly the
identification that erases the voice exchange. The gloss's own words give it away: the *temporal facts* are identical,
and the proposal's claim is that temporal facts are not all the facts.

What survives is the observational form:

> **L18′.** If `duration(M) = duration(N)` and `duration(P) = duration(Q)` then
> `U((M ⊕ N) ; (P ⊕ Q)) = U((M ; P) ⊕ (N ; Q))`.

Immediate from Theorem R, since both sides have the same `λ`-image and the same extent. This is the familiar shape from
concurrency theory: an interleaving semantics validates equations that a true-concurrency semantics refutes, and the
refuted equations are the ones that confuse "same observable behaviour" with "same process." That the same distinction
shows up in counterpoint is, on this hypothesis, not a coincidence — a fugue is a concurrent system whose processes are
lines.

### 5.3 The migration this implies

Two existing tests assert a false equation under the hypothesis: the property test `laws::synchronized_interchange` in
`crates/musa-kernel/tests/laws.rs:226`, and `terms::synchronized_interchange_holds_of_terms` in
`crates/musa-kernel/tests/terms.rs:205`. If the hypothesis is adopted, that test becomes
`synchronized_interchange_holds_up_to_observation` and a new counterexample test
`interchange_distinguishes_parts_from_blocks` records the voice exchange. This is a small, well-localized change, and
its size is evidence that the hypothesis is a refinement rather than a rewrite.

---

## 6. The interface consumers see

The event structure is the *implementation* of the denotation. What crosses the crate boundary is deliberately narrower,
because an atom whose interface is as wide as its implementation is APOSD's shallow module and would sink the proposal.

| Observation | Type | Who needs it |
| --- | --- | --- |
| `hearings` | `Music → List<Hearing>` | `musa-notation` for realization, `musa-playback` for playback |
| `lines` | `Hearing → List<Line>` | `musa-notation` for beaming and stems; the counterpoint style for its rules |
| `occurrences` | `Hearing → List<Occurrence>` | everything that works today, unchanged |
| `restrict`, `covering`, `prevailing` | as today, on a `Hearing` | unchanged |

`≤` and `#` themselves do not cross. A consumer never asks "does `e` precede `e′`"; it asks for the lines or for the
hearings. A default `Hearing` is supplied for conflict-free music, so every existing call site compiles with no change —
which is the concrete claim that this is additive at the interface even though it is a refinement at the denotation.
