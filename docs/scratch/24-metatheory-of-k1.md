# Metatheory of K₁

**Status: fixed proof draft for external attack; governs nothing.** This note proves only the calculus and algebras
actually defined in [20](20-candidate-staged-algebras.md) and [21](21-semantics-of-k1.md). It does not prove that the
named music-domain sketches in [22](22-encodings-and-sieve.md) are culturally adequate, that every Rust primitive
satisfies its contract, or that Candidate E₃ should be admitted.

The proof is intentionally conditional at implementation boundaries. Naming an assumption is preferable to hiding a
compiler obligation inside “standard.”

---

## 1. Scope and assumptions

Let `K₁⁰` be the monomorphic core after rank-1 instantiation, module elaboration, and erasure. It is call-by-value STLC
with unit, booleans, naturals, exact rationals, products, sums, finite vectors, strictly positive algebraic data,
structural folds, non-recursive `let`, and opaque constants for validated timelines, warps, process descriptions, and
domain data.

Assume:

- **A1 — finite declarations.** The import, structure-functor, and value-definition graph is finite and acyclic.
- **A2 — explicit checking.** Every lambda parameter, exported member, abstract type, clock, tick, port row, and
  ambiguous literal has an annotation. Rank-1 parameters occur only in prenex schemes on declarations.
- **A3 — total primitives.** Each source primitive maps every closed value in its declared argument domain to exactly
  one closed value in its declared result domain. Partiality is represented by `Option` or `Result`.
- **A4 — primitive purity.** Primitive results depend only on their explicit arguments and have no source-evaluation
  side effect.
- **A5 — finite folds.** Every fold consumes an already constructed finite strictly positive value and recursively calls
  itself only on immediate structural subdata.
- **A6 — canonical data.** Every admitted `Data` type has a deterministic finite encoder which is complete for its
  declared semantic equality.
- **A7 — solver.** Natural index constraints are formulas of quantifier-free Presburger arithmetic. Port rows have
  distinct labels and finite structurally checkable member types.
- **A8 — process primitives.** Each process primitive supplies deterministic initial state and transition, and every
  primitive marked as a delay is strictly causal by at least one tick on the marked ports.

A1–A5 and A7 are statically enforceable. A6 and A8 are ownership-boundary contracts: the generic proof transports them
but cannot establish arbitrary foreign implementations.

## 2. Static decidability

### Lemma 2.1 — index equality is decidable

For any well-formed `Δ` and natural index expressions `i,j`, whether `Δ ⊨ i=j` is decidable.

*Proof.* By A7, `Δ` is a finite conjunction of Presburger formulas and `i=j` is a Presburger formula. The judgment holds
iff `Δ ∧ i≠j` is unsatisfiable. Presburger satisfiability is decidable. ∎

### Lemma 2.2 — type conversion is decidable

For well-kinded types `A,B`, the judgment `Δ ⊢ A ≡ B` is decidable.

*Proof.* Normalize finite port rows by label. Compare the outer constructors of `A` and `B` structurally. Nominal types,
clocks, ticks, and rates compare by stable identity. Natural indices invoke Lemma 2.1. Every recursive comparison is on
a strict syntactic subterm, and there is no value-term reduction or user function in a type, so the procedure
terminates. It accepts exactly the congruence generators in [21](21-semantics-of-k1.md) §2. ∎

### Lemma 2.3 — monomorphic term checking is decidable

Given well-formed `Δ,Γ`, a term `e`, and expected type `A`, whether `Δ;Γ ⊢ e:A` is decidable.

*Proof.* Induct on the finite syntax of `e`. Each rule has finitely many premises on strict subterms. Variable lookup,
constructor lookup, exhaustiveness of finite constructor matches, and primitive-signature lookup are decidable. Every
comparison of inferred and expected types uses Lemma 2.2. Folds are checked from their explicit carrier and step types;
the checker does not evaluate them. ∎

### Theorem 2.4 — project checking and rank-1 elaboration are decidable

For a finite project satisfying A1–A2, checking either rejects with a finite diagnostic or produces one finite
monomorphic `K₁⁰` program.

*Proof.* Topologically sort the finite declaration graph; reject if the sort exposes a cycle. Process declarations in
that order. At each direct use of a prenex declaration, first-order unification over structural types produces a finite
set of type and index equations. Structural equations are solved syntactically; natural equations are decided by Lemma
2.1. An unsolved or non-unique parameter is rejected rather than guessed. Because the graph and every body have finitely
many call sites, only finitely many monomorphic instances are generated. Each body instance is checked by Lemma 2.3.
Static structure functors elaborate once per finite named application site, so A1 makes that stage finite too. ∎

**Corollary 2.5.** K₁ does not need full dependent-type inference. Its checking claim would cease to follow if timeline
plans, arbitrary rationals with nonlinear multiplication, set union, or user predicates were admitted to type
conversion.

## 3. Type safety of the source core

Write `e → e'` for deterministic left-to-right call-by-value reduction. A value is a constant, closure, fully evaluated
constructor, pair, injection, vector, or validated opaque description.

### Lemma 3.1 — weakening

If `Γ ⊢ e:A` and `x∉dom(Γ)`, then `Γ,x:B ⊢ e:A`.

*Proof.* Induction on the typing derivation. The variable case uses the original membership; every other rule applies
the induction hypotheses to its premises. ∎

### Lemma 3.2 — substitution

If `Γ,x:A ⊢ e:B` and `Γ ⊢ v:A`, then `Γ ⊢ e[v/x]:B`.

*Proof.* Induction on the derivation of `Γ,x:A ⊢ e:B`. In the variable case, use `v` when the variable is `x` and the
original variable rule otherwise. The lambda and match cases α-rename binders to avoid capture and apply the induction
hypothesis under the extended context. Products, sums, application, let, constructors, and folds follow by congruence. ∎

### Lemma 3.3 — canonical forms

If `v` is a closed value of:

- arrow type, then it is a closure;
- product type, then it is a pair;
- sum type, then it is the corresponding injection;
- a strictly positive data type, then it is one of that type's constructors;
- `Timeline`, `Warp`, or `Process` type, then it is a validated opaque value of that exact nominal/indexed type.

*Proof.* Inspect the value grammar and invert the final typing rule. Nominal and indexed opaque values can only be
introduced by a primitive whose result signature fixes the type; there is no cast or representation eliminator. ∎

### Theorem 3.4 — preservation

If `Γ ⊢ e:A` and `e→e'`, then `Γ ⊢ e':A`.

*Proof.* Induct on the reduction derivation. Congruence cases use the induction hypothesis and rebuild the typing rule.
β-reduction uses Lemma 3.2. Let-reduction is substitution. Match reduction selects an arm whose constructor bindings
have the types obtained by inversion and then applies substitution. A fold step substitutes the constructor fields and
recursive fold results into its well-typed algebra. A primitive δ-step preserves the declared result type by A3. Opaque
timeline, warp, and process combinators are δ-primitives at this level, so the same case applies. ∎

### Theorem 3.5 — progress

If `∅ ⊢ e:A`, then either `e` is a value or there exists `e'` with `e→e'`.

*Proof.* Induct on the typing derivation. For application, either a subterm steps or both are values; Lemma 3.3 makes
the function a closure, enabling β-reduction. Products, injections, constructors, and primitive arguments step left to
right until values. A closed match scrutinee is either reducible or, by Lemma 3.3, a constructor selected by an
exhaustive arm. A fold scrutinee is either reducible or a finite constructor, enabling its fold equation. When all
primitive arguments are values, A3 supplies a δ-step. `Result` carries domain failure as an ordinary value, so no case
is stuck on a failed bounds or validation check. ∎

### Theorem 3.6 — one-step determinism

If `e→e₁` and `e→e₂`, then `e₁=e₂`.

*Proof.* Every non-value term has a unique decomposition `E[r]` into the specified left-to-right evaluation context and
one redex. β-, let-, match-, and fold-redex forms are disjoint. A3–A4 make a primitive δ-redex a function of its value
arguments. Hence the redex has one contractum and both derivations coincide. ∎

## 4. Strong normalization and total evaluation

Let `SN` be the set of terms admitting no infinite reduction sequence. Define reducibility predicates by induction on
types:

```text
R_b(t)       iff t is well typed at base/opaque type b and t∈SN
R_(A×B)(t)   iff t∈SN and whenever t→*(v₁,v₂), R_A(v₁) and R_B(v₂)
R_(A+B)(t)   iff t∈SN and whenever t→*inl(v), R_A(v), and similarly for inr
R_(A→B)(t)   iff t∈SN and for every u with R_A(u), R_B(t u)
```

For a strictly positive data type, require `t∈SN` and reducibility of every recursive and parameter field in every
constructor normal form.

### Lemma 4.1 — candidate properties

For every type `A`:

1. `R_A(t)` implies `t∈SN`;
2. if `R_A(t)` and `t→t'`, then `R_A(t')`;
3. a neutral term belongs to `R_A` when all its reducts do.

*Proof.* Simultaneous induction on `A`. Base and opaque types are immediate. Product, sum, and data cases use closure of
their reducible fields under reduction. The arrow case applies the induction hypothesis after application to an
arbitrary reducible argument. ∎

### Lemma 4.2 — folds preserve reducibility

If a finite data value and every algebra branch are reducible at their declared types, applying its structural fold
produces a reducible result.

*Proof.* Well-founded induction on the number of constructors in the folded value, secondarily on reduction heights of
the algebra arguments. A fold equation replaces the outer constructor by the corresponding algebra branch and folds only
immediate recursive fields. Each recursive field has strictly fewer constructors by strict positivity and A5, so the
induction hypothesis makes every recursive result reducible. Reducibility of the algebra branch then gives a reducible
result. ∎

### Lemma 4.3 — fundamental lemma

If `Γ ⊢ e:A` and a substitution `θ` maps every `x:B∈Γ` to a term in `R_B`, then `R_A(eθ)`.

*Proof.* Induct on the typing derivation. Variables use the hypothesis on `θ`; introduction and elimination forms use
the corresponding definition of `R`; β-compatible application uses the arrow clause. Data constructors use reducible
fields. Fold cases use Lemma 4.2. A primitive applied to reducible arguments first normalizes those arguments; A3 then
contracts the primitive in one step to a closed value, which is reducible at its declared type. ∎

### Theorem 4.4 — strong normalization

Every well-typed `K₁⁰` term is strongly normalizing.

*Proof.* Apply Lemma 4.3 to the identity substitution and then Lemma 4.1(1). ∎

### Corollary 4.5 — deterministic total source evaluation

Every accepted closed source expression evaluates to exactly one value of its declared type.

*Proof.* Theorem 4.4 and progress give existence of a value normal form. Preservation gives its type. Determinism gives
uniqueness. The finite elaboration theorem transfers the result from the monomorphic core to accepted source
expressions. ∎

This is source totality only. It does not say a causal process has a last output frame.

## 5. Timeline algebra

Let `M=(d,E)`, `N=(e,F)`, and `P=(f,G)` be well-formed timelines.

### Theorem 5.1 — timeline closure

`M;N` and `M⊕N` are well formed.

*Proof.* For a shifted occurrence `(e_s,e_t,a)∈F`,

```text
0 ≤ d+e_s ≤ d+e_t ≤ d+e,
```

so every occurrence in `M;N` lies in its extent. Occurrences from `E` remain bounded by `d≤d+e`. For overlay, every
occurrence is bounded by its original extent, which is at most `max(d,e)`. Both multisets remain finite. ∎

### Theorem 5.2 — sequence monoid

`(M;N);P = M;(N;P)` and `(0,∅)` is a two-sided unit.

*Proof.* Both associations have extent `d+e+f`. Their occurrence multisets are respectively

```text
E ⊎ τ_d(F) ⊎ τ_(d+e)(G)
```

after associativity of multiset union and `τ_d∘τ_e=τ_(d+e)`. The unit equations set the shift to zero and union with the
empty multiset. ∎

### Theorem 5.3 — overlay commutative semigroup and fixed-extent monoid

Overlay is associative and commutative. At fixed extent `d`, `(d,∅)` is its unit.

*Proof.* `max` and multiset union are associative and commutative. At fixed extent, `max(d,d)=d` and union with `∅` is
identity. ∎

### Theorem 5.4 — synchronized interchange

If `extent(M)=extent(N)=d` and `extent(P)=extent(Q)=e`, then

```text
(M⊕N);(P⊕Q) = (M;P)⊕(N;Q).
```

*Proof.* Both sides have extent `d+e`. On the left, both `P` and `Q` are shifted by `d`. On the right, the second member
of each sequence is shifted by the corresponding first extent, both equal to `d`. Both multisets are therefore
`E_M ⊎ E_N ⊎ τ_d(E_P) ⊎ τ_d(E_Q)` with the same multiplicities. ∎

**Counterexample 5.5 — unequal extents cannot be repaired by typing overlay.** Let `M` and `N` have extents `1` and `2`,
and let `P,Q` each contain a point at local time zero. On the left of interchange, both later points shift by `2`; on
the right they shift by `1` and `2`. The results differ. Requiring equal extent at `M⊕N` would make this equation type
correct by rejecting the musical input, not make the ordinary unequal overlay invalid. The side condition belongs to the
theorem using interchange.

## 6. Warp theorems

Let warps be continuous, strictly increasing, finite piecewise-affine rational maps sending zero to zero.

### Lemma 6.1 — closure under composition and tail

If `w:a→b` and `v:b→c` are warps, then `v∘w` is a warp. For every rational `d` in the domain of `w`,
`tail(w,d)(t)=w(d+t)-w(d)` is a warp on the remaining domain.

*Proof.* Composition and translation preserve continuity and strict increase. Slopes of a composite segment are products
of positive rationals. Its breakpoints are the breakpoints of `w` together with preimages under `w` of the finitely many
breakpoints of `v`; strict piecewise-affine rational `w` has at most one rational preimage per segment, so the set is
finite and rational. Tail translates the finite breakpoint set by `-d`, subtracts a rational constant from the range,
retains positive rational slopes, and sends zero to zero. ∎

### Theorem 6.2 — warp functoriality

`W_id(M)=M` and `W_v(W_w(M))=W_(v∘w)(M)`.

*Proof.* Identity fixes the ambient endpoint and every occurrence endpoint. Composition maps each coordinate `t` first
to `w(t)` and then `v(w(t))`, exactly the composite definition. Payloads and multiplicities are untouched. ∎

### Theorem 6.3 — warp preserves overlay

If one warp covers both extents, `W_w(M⊕N)=W_w(M)⊕W_w(N)`.

*Proof.* Strict increase gives

```text
w(max(d,e)) = max(w(d),w(e)).
```

Both sides map the same occurrence multiset endpointwise and preserve multiplicity. ∎

### Theorem 6.4 — rebased sequence law

If `w` covers `d+e`, then

```text
W_w(M;N) = W_w(M) ; W_tail(w,d)(N).
```

*Proof.* Both sides have ambient extent `w(d+e)`, because the right extent is

```text
w(d) + (w(d+e)-w(d)).
```

An occurrence `(s,t,a)` from `M` maps identically. An occurrence from `N` maps on the left, after source shifting, to
`(w(d+s),w(d+t),a)`. On the right it first maps to

```text
(w(d+s)-w(d), w(d+t)-w(d), a)
```

and sequence shifts by `w(d)`, giving the same endpoints. ∎

### Theorem 6.5 — warp commutes with payload mapping

`W_w(T(f)(M))=T(f)(W_w(M))`.

*Proof.* Warp changes only endpoints; payload map changes only labels. The operations commute pointwise. ∎

## 7. Causal-process theorems

For histories `x,y`, write `x=_n y` when their values agree through tick `n`. A function is causal when it preserves
every prefix equivalence.

### Lemma 7.1 — identities, composition, products, and permutations are causal

*Proof.* Identity and permutation preserve equality pointwise. If `x=_n y`, causality of `F` gives `F(x)=_nF(y)`, then
causality of `G` gives `G(F(x))=_nG(F(y))`. Product processes apply the same argument componentwise. ∎

### Theorem 7.2 — process combinators are sound

The denotations of `wire`, `serial`, `parallel`, and `permute` are causal and satisfy the symmetric monoidal category
laws extensionally.

*Proof.* Causality is Lemma 7.1. The laws reduce to associativity and identity of function composition, associativity
and unit of finite products, the permutation group laws, and pointwise interchange

```text
(G₁×G₂)∘(F₁×F₂) = (G₁∘F₁)×(G₂∘F₂).
```

∎

### Theorem 7.3 — guarded feedback exists and is unique

Let `F : Hist(I×X) → Hist(O×X)` be causal and strictly causal in its `X` input: its `X` output through tick `n+1`
depends on that input only through tick `n`. Fix an initial feedback value. For every input history `i`, there exists a
unique pair `(o,x)` satisfying

```text
(o,shift(x)) = F(i,x),
```

where `x(0)` is the fixed initial value. The induced map `i↦o` is causal.

*Proof.* Construct `x` by induction on ticks. The initial value fixes `x(0)`. Suppose `x` is uniquely fixed through tick
`n`. Strict causality makes the feedback output at tick `n+1` depend only on the already fixed prefix through `n`, so it
uniquely determines `x(n+1)`. Causality of `F` then uniquely determines `o(n)` from the input and feedback prefixes
through `n`. This constructs a solution and proves uniqueness by the same induction.

For causality, if `i=_n i'`, simultaneous induction gives equal feedback prefixes through `n` and therefore equal output
prefixes through `n`. ∎

### Theorem 7.4 — guarded graph execution is deterministic and causal

If every directed cycle in a finite process graph crosses a declared one-tick delay, the graph has a deterministic
per-tick schedule and denotes a causal process.

*Proof.* Cut each delay edge at its state output. Any cycle in the remaining graph would be a directed cycle containing
no delay, contradicting the hypothesis. The cut graph is therefore a finite DAG and has a topological order. At each
tick, read delay outputs from prior state, evaluate every primitive once in topological order, then commit delay inputs
as next state. A8 makes each transition deterministic; induction on topological order and then on ticks makes the whole
execution deterministic. Each output at tick `n` depends only on external inputs through `n` and delay states computed
from earlier ticks, hence the denotation is causal. ∎

**Implementation qualification.** This theorem is parametric in the chosen tick. Musa's current audio graph scheduler
cuts a feedback edge at block granularity, while processor state advances within a block at frame granularity. The
theorem proves causality for either model after it is stated consistently; it does not prove that the present block
scheduler implements a proposed frame-tick denotation exactly.

## 8. Preparation coherence

Let `nf_T` be timeline normal form, `nf_P` process structural normal form, and `canon_B` canonical binding data. Let
`prepare_spec` be a deterministic total-or-`Result` function of those values, seed, and explicit options.

### Theorem 8.1 — R1-K₁

If `M≡T N`, `canon_B(B)=canon_B(B')`, `nf_P(P)=nf_P(P')`, and seeds/options are equal, then

```text
prepare(M,B,P,s,o) = prepare(N,B',P',s,o).
```

*Proof.* Timeline semantic equality is defined by `nf_T(M)=nf_T(N)`. Expanding the definition of `prepare` gives two
applications of `prepare_spec` to componentwise equal arguments. Determinism makes their `Result PreparedSpec` values
equal. ∎

### Corollary 8.2 — cache correctness

A collision-checked cache keyed by canonical encodings of `nf_T(M)`, `canon_B(B)`, `nf_P(P)`, `s`, and `o` returns the
same `PreparedSpec` as recomputation.

*Proof.* A key hit means equality of every canonical argument by A6 and the encoders for the normal forms. Apply Theorem
8.1. Collision checking is necessary because a fixed-width digest alone is not injective. ∎

### Non-theorem 8.3 — frame equality

Byte-identical rendered audio does **not** follow from Theorem 8.1 alone. It additionally assumes identical device/input
histories, deterministic allocation, deterministic primitive implementations, fixed floating-point semantics, and no
host-dependent nondeterminism. Those are runtime conformance obligations.

## 9. Optional E₃ facts

For a finite event presentation, well-formedness and configuration enumeration are decidable: finite graph algorithms
decide acyclicity and transitive closure; finite pair checks decide conflict axioms; at most `2^|E|` subsets can be
tested for down-closure and conflict. The exponential bound makes enumeration resource-limited but not semantically
partial.

The schedule-realization propositions in [23](23-candidate-event-presentations.md) follow directly from the definitions.
No theorem above depends on E₃.

## 10. Dependency ledger and remaining proof debt

```text
Presburger decidability
  └─ index equality
      └─ type conversion
          └─ term checking
              └─ finite project elaboration

weakening + substitution + canonical forms
  ├─ preservation
  └─ progress

reducibility candidates + fold lemma
  └─ strong normalization
      └─ deterministic total source evaluation

timeline definitions ── timeline laws
warp definitions ────── W1–W5
strict causality ────── guarded feedback ── guarded graph execution
canonical factorization ─────────────────── R1-K₁
```

Still unproved or intentionally external:

1. correctness of a concrete Presburger/row solver;
2. soundness and completeness of the actual source-to-`K₁⁰` elaborator;
3. A6 for every abstract data package;
4. denotational soundness of process graph normalization beyond structural diagram laws;
5. deterministic floating-point behavior of every DSP primitive;
6. the cross-clock event adapter from interaction to frames;
7. cultural adequacy of every theory package;
8. any claim that all desired musical artifacts fit the finite-data, rational-timeline, or discrete-causal-process
   assumptions.

The proof-review task should attack the theorem statements as well as the arguments. In particular it should look for a
hidden value dependency in type conversion, an invalid reducibility step for opaque primitives or folds, an incorrect
strict-causality index in Theorem 7.3, and a stronger R1 conclusion than the assumptions support.
