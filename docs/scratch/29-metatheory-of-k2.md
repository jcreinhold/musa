# Metatheory of K₂

**Status: fixed proof draft for independent review; governs nothing.** This note proves the exact core and algebras in
[28](28-candidate-k2.md). The failed K₁ proof and its review remain in [24](24-metatheory-of-k1.md) and
[25](25-proof-review.md). No repair below is retroactively credited to K₁.

---

## 1. Definitions and assumptions

Fix a satisfiable static context `Δ`. Let `K₂⁰` be the explicitly typed monomorphic term grammar of [28 §3], including
its administrative `timeline_build` form. Polynomial data declarations are finite and acyclic as a declaration graph;
recursive references inside one declaration are allowed only in the polynomial positions of [28 §2.2].

Assume:

- **B1 — decidable declarations.** Constructor, nominal identity, port-row, and primitive-signature lookup are finite
  and decidable.
- **B2 — satisfiable indices.** `Δ` is a finite satisfiable Presburger context. Types contain no value terms.
- **B3 — first-order foreign boundary.** Every saturated `op` accepts only `Data` arguments, has a `Data` result, and
  `δ(op,v̄)` is exactly one closed canonical result of the declared type.
- **B4 — pure foreign boundary.** `δ` depends only on the operation identity and explicit canonical arguments.
- **B5 — finite polynomial data.** Every recursive data value has a finite constructor tree; a fold call is generated
  only for strict immediate subtrees.
- **B6 — canonical data.** Each nominal `Data` equality has one deterministic finite complete encoder.
- **B7 — process contracts.** Every primitive process transition is total and deterministic; state is node-local; its
  declared instantaneous port-dependency relation conservatively contains every current-tick dependency.

B1, B2, and the syntactic portions of B3/B5 are checker obligations. Semantic totality and purity of foreign Rust code
in B3, B4, B6, and B7 are trusted ownership-boundary contracts.

## 2. Static decidability

### Lemma 2.1 — static-context validity is decidable

Whether `⊢ Δ ok` is decidable.

*Proof.* Finiteness and correct kinding are structural checks by B1. The remaining condition is satisfiability of a
finite Presburger formula, which is decidable. ∎

### Lemma 2.2 — conversion is decidable

For well-kinded given types `A,B`, whether `Δ ⊢ A ≡ B` is decidable.

*Proof.* Compare constructors recursively. Nominal types, clocks, ticks, and rates compare by identity. Normalize each
finite row by label and compare corresponding member types. For natural indices `i,j`, decide whether `Δ ∧ i≠j` is
unsatisfiable in full Presburger arithmetic. Every recursive call is on strict type subsyntax and no value term or user
function reduces during conversion. ∎

### Theorem 2.3 — monomorphic checking is decidable

Given `Δ,Γ,e,A`, whether `Δ;Γ ⊢ e:A` is decidable.

*Proof.* Induct on the finite syntax of `e`. Lookup and constructor/fold branch validation are decidable by B1 and B5.
Exhaustiveness ranges over a finite constructor set. Every comparison of a synthesized type with an expected type uses
Lemma 2.2. `timeline_map` checks its two premises directly. Administrative forms have syntax-directed rules and cannot
occur in source input. The algorithm checks an annotation; it performs no existential index inference. ∎

**Non-theorem 2.4.** This does not prove the present Musa parser, signatures, structure templates, or imports elaborate
soundly to `K₂⁰`. Such a theorem requires an explicit source syntax, bidirectional judgments, generativity/sealing
semantics, translation, and preservation proof. K₂ intentionally makes no source-totality claim before those exist.

## 3. Operational safety

Write `e ↦ e'` for the compatible closure of [28 §3.2]'s principal reductions under its evaluation contexts.

### Lemma 3.1 — substitution

If `Δ;Γ,x:A ⊢ e:B` and `Δ;Γ ⊢ v:A`, then `Δ;Γ ⊢ e[v/x]:B`.

*Proof.* By induction on the typing derivation of `e`, renaming bound variables to avoid capture. `T-Conv` follows by
the induction hypothesis and the unchanged conversion premise. The opaque operations add no binders. ∎

### Lemma 3.2 — canonical forms

For a closed value `v`:

- if `⊢ v:A→B`, then `v` is a lambda;
- if `⊢ v:A×B`, then `v` is a pair;
- if `⊢ v:A+B`, then `v` is the corresponding injection; and
- if `⊢ v:D Ā`, then `v` has a constructor of `D`; an opaque `Data` type has its corresponding canonical `u` form.

*Proof.* Inspect the value grammar and invert its typing derivation. A foreign operation is saturated syntax and never a
value, so there is no missing primitive-arrow case. Nominal opacity prevents a canonical `u` from acquiring an unrelated
type. ∎

### Theorem 3.3 — preservation

If `Δ;∅ ⊢ e:A` and `e↦e'`, then `Δ;∅ ⊢ e':A`.

*Proof.* Induct on the reduction derivation. Context closure uses the typing rule for the selected context and the
induction hypothesis. β and `let` use Lemma 3.1. Projections and cases follow by inversion and substitution. A
polynomial fold branch has exactly the declared carrier type, with recursive calls placed only at recursive fields. B3
gives the declared type for a `δ` result. For `timeline_map`, inversion gives `f:A→B`, every stored payload `aᵢ:A`, and
preserved spans; hence every administrative expression `f aᵢ:B`, `timeline_build` has type `Timeline c B`, and its final
canonical timeline has that type. `T-Conv` is stable because reduction does not change `Δ`. ∎

### Theorem 3.4 — progress

If `Δ;∅ ⊢ e:A`, then `e` is a value or there exists `e'` with `e↦e'`.

*Proof.* Induct on the typing derivation, using canonical forms when all premises are values. Application, projections,
case, `let`, and fold expose their unique principal redex. A saturated foreign operation on canonical arguments reduces
by B3. `timeline_map` either reduces a premise or exposes the finite `timeline_build` form; the build either reduces its
leftmost unfinished payload or returns the canonical timeline. `Result` represents domain errors, so no primitive
application is stuck on a well-typed canonical argument. Conversion adds no runtime form. ∎

### Lemma 3.5 — deterministic decomposition

Every non-value term has at most one decomposition `E[r]` into an evaluation context and principal redex, and every
principal redex has exactly one reduct.

*Proof.* Structural induction on the term grammar. Left-to-right contexts are disjoint by the first non-value child.
Principal forms are syntactically disjoint. β, eliminators, fold, and timeline administrative rules have one reduct; B3
makes `δ` single-valued. ∎

### Corollary 3.6 — deterministic reduction

If `e↦e₁` and `e↦e₂`, then `e₁=e₂`.

*Proof.* Lemma 3.5. ∎

## 4. Strong normalization

The proof must not assume that higher-order callback evaluation happens inside a total foreign step.

Define reducibility `R_A(e)` for well-typed closed terms:

```text
R_A(e)                    iff e is strongly normalizing,                       A : Data or scalar
R_(A×B)(e)                iff e is SN and, if e↦* (v₁,v₂), R_A(v₁) and R_B(v₂)
R_(A+B)(e)                iff e is SN and its eventual injection payload is reducible
R_(A→B)(e)                iff e is SN and for every R_A(a), R_B(e a)
```

Products and sums whose components are `Data` satisfy both clauses; the displayed separation only makes the higher-order
cases explicit. Extend the definition structurally to `Result` and polynomial data, requiring reducibility of every
non-recursive field and every finite recursive subtree.

### Lemma 4.1 — candidate closure

Reducibility is closed under reduction and under expansion by a neutral term whose immediate reducts are reducible.
Every reducible term is strongly normalizing.

*Proof.* Simultaneous induction on the structure of `A`. The arrow expansion case uses well-founded induction on the
maximum reduction height of the reducible argument: a reduct of `e a` either reduces `e`, reduces `a`, or fires β after
both are values. The first two decrease the respective reduction heights; the β result is an assumed immediate reduct.
The product, sum, and polynomial cases use their eliminations and finite subcomponents. This is the standard
Tait-reducibility closure argument, with `Data` as the base strongly-normalizing interpretation. ∎

### Lemma 4.2 — polynomial folds preserve reducibility

If a polynomial data argument and every fold branch are reducible at their declared types, then the fold application is
reducible at its carrier type.

*Proof.* Induct on the finite constructor-tree size of the already normalized data value. Every recursive fold in the
selected branch consumes a strict immediate subtree by B5 and therefore has smaller size. The branch is reducible when
applied to reducible fields and recursive results. For a non-value argument, candidate expansion from Lemma 4.1 handles
its finitely descending reductions. ∎

### Lemma 4.3 — foreign operations preserve reducibility

If `op : A₁×⋯×Aₙ⇒B` is foreign and each argument is reducible, then `op(ē)` is reducible at `B`.

*Proof.* Every `Aᵢ` and `B` is `Data`, never an arrow and never contains one. Reduce arguments left to right; their
strong normalization makes this finite. On canonical arguments, B3 reduces the saturated operation once to a closed
canonical `Data` value. That value has no internal term reduction and is reducible at `B`. Candidate expansion gives
reducibility of the original application. ∎

### Lemma 4.4 — timeline mapping preserves reducibility

If `R_(A→B)(f)` and `R_(Timeline c A)(M)`, then `R_(Timeline c B)(timeline_map(f,M))`.

*Proof.* Reduce `M` to its unique canonical finite timeline, using candidate expansion for earlier reductions. Suppose
it has `n` occurrences. The administrative build contains exactly `n` applications `f aᵢ`. Each stored payload is a
canonical `Data` value and hence reducible at `A`; arrow reducibility gives `R_B(f aᵢ)`. Left-to-right evaluation of a
finite list of strongly normalizing expressions terminates, after which the build returns one canonical timeline.
Candidate expansion proves the original map reducible. ∎

### Theorem 4.5 — fundamental lemma

If `Δ;Γ ⊢ e:A` and a closing substitution maps every `x:B∈Γ` to a term reducible at `B`, then the substituted term is
reducible at `A`.

*Proof.* Induct on the typing derivation. Variables use the substitution hypothesis. Introduction and ordinary
elimination rules use Lemma 4.1 and the induction hypotheses. A lambda maps every reducible argument to the reducible
substituted body by extending the closing substitution. Folds use Lemma 4.2, foreign applications use Lemma 4.3, and
timeline mapping uses Lemma 4.4. Conversion is harmless because definitionally equal types have identical reducibility
interpretations. ∎

### Corollary 4.6 — core totality

Every closed well-typed K₂ core term reduces in finitely many deterministic steps to exactly one value of its declared
type.

*Proof.* Apply Theorem 4.5 to the empty substitution. Reducibility implies strong normalization; progress excludes a
stuck normal form; preservation gives the value's type; Corollary 3.6 gives uniqueness. ∎

This theorem is conditional on the foreign ownership contracts and does not transfer to source modules until a source
elaboration theorem exists. A process value is finite and terminates as a source computation; running it can produce an
unbounded history.

## 5. Timeline laws

Let `M=(d,E)`, `N=(e,F)`, and `P=(f,G)` be well-formed timelines.

### Theorem 5.1 — closure

`M;N` and `M⊕N` are well formed.

*Proof.* Every shifted occurrence from `F` satisfies `0≤d+s≤d+t≤d+e`; occurrences from `E` remain within `d≤d+e`. Under
overlay, each occurrence remains bounded by its old extent, which is at most `max(d,e)`. Both multisets remain finite. ∎

### Theorem 5.2 — sequence and overlay laws

Sequence is a monoid with unit `(0,∅)`. Overlay is associative and commutative; at each fixed extent `d`, `(d,∅)` is its
unit.

*Proof.* Sequence associativity follows from associativity of rational addition and multiset union together with
`τ_d∘τ_e=τ_(d+e)`. The zero shift and empty multiset give its unit. Overlay follows from associativity and commutativity
of `max` and multiset union. At fixed extent, `max(d,d)=d`. ∎

### Theorem 5.3 — map laws

Timeline map preserves identity, composition, sequence, and overlay.

*Proof.* It changes only payloads pointwise, while sequence and overlay change only coordinates and multiset placement.
Function identity and composition hold on every occurrence with multiplicity retained. ∎

### Theorem 5.4 — synchronized interchange

If `extent(M)=extent(N)=d` and `extent(P)=extent(Q)=e`, then

```text
(M⊕N);(P⊕Q) = (M;P)⊕(N;Q).
```

*Proof.* Both sides have extent `d+e` and multiset `E_M⊎E_N⊎τ_d(E_P)⊎τ_d(E_Q)`. ∎

Equal extent is a premise of this equation, not of overlay formation. The unequal-extent counterexample in [24 §5.5]
still proves that adding an equality premise to `over` would reject ordinary temporal material without validating a more
general interchange law.

## 6. Total-warp laws

Let a K₂ warp be a total eventually-affine rational map of [28 §5].

### Lemma 6.1 — closure under composition and tail

If `w:Warp a b` and `v:Warp b c`, then `v∘w:Warp a c`; for every `d≥0`, `tail(w,d):Warp a b`.

*Proof.* Composition and translation preserve continuity, strict increase, zero at the appropriate local origin, and
positive rational slopes. A composite breakpoint is either a breakpoint of `w` or the unique rational preimage, within
one affine segment of `w`, of a breakpoint of `v`; there are finitely many. After the final breakpoint of `w`, `w` is a
positive affine ray and eventually crosses the final breakpoint of `v`; the composite is then affine forever. Tail
translates and truncates the finite breakpoint list, retains the final ray, subtracts `w(d)`, and sends zero to zero. ∎

### Theorem 6.2 — warp action laws

For every finite timeline:

```text
W_id(M)                         = M
W_v(W_w(M))                     = W_(v∘w)(M)
W_w(M⊕N)                        = W_w(M)⊕W_w(N)
W_w(M;N)                        = W_w(M);W_tail(w,extent(M))(N)
W_w(timeline_map(f,M))          = timeline_map(f,W_w(M)).
```

*Proof.* Totality removes every coverage premise. Identity and composition act endpointwise. Strict increase gives
`w(max(d,e))=max(w(d),w(e))` for overlay. For sequence, a second-operand point `t` maps on the left to `w(d+t)` and on
the right to `w(d)+(w(d+t)-w(d))`. The last equation holds because warp changes only coordinates and map changes only
payloads. ∎

## 7. Process-graph soundness

For a fixed tick `k`, a history over row `I` is `ℕ→Frame(I)`. A history function is causal when equal input prefixes
through `n` give equal output prefixes through `n`.

### Lemma 7.1 — unguarded combinators are causal

Identity, serial composition, tagged parallel product, renaming, and permutation preserve causality and satisfy the
symmetric-monoidal laws extensionally.

*Proof.* Identity and permutations act pointwise. Causal functions are closed under composition and finite product.
Tagged tensor is an actual disjoint product, so the usual product associators, unitors, symmetry, and interchange are
well formed; explicit renaming witnesses any flattened presentation. ∎

### Theorem 7.2 — validated graph execution is total, deterministic, and causal

If `close(G)=Ok(P)`, then `P` has one total deterministic transition per tick and denotes a causal history function.

*Proof.* `close` has constructed the finite instantaneous port-dependency graph and accepted only if it is acyclic. A
finite DAG has a topological order. At tick `n`, old local states and current external inputs are fixed. Induction over
the stored order determines every current port exactly once: each instantaneous predecessor is earlier in the order;
every delayed contribution is read from old local state. B7 makes each primitive computation and next-state value total
and deterministic. After all current outputs are fixed, next states are committed, so induction on `n` gives one
unbounded execution.

If two external histories agree through `n`, simultaneous induction on ticks and, within a tick, on the topological
order gives equal port values and states through `n`; outputs therefore agree through `n`. Thus the denotation is
causal. ∎

### Corollary 7.3 — no accepted algebraic loop

Every directed cycle of current-tick influence is rejected, including a pass-through path inside a multiport primitive
which delays a different path.

*Proof.* Current-tick influence edges are exactly the conservative `dep_p` edges plus graph wires. Any such cycle is a
cycle in the validated instantaneous graph, contradicting acyclicity. Delayed relations are state transitions between
ticks and are not removed merely because some other port of the same node is delayed. ∎

The theorem is relative to the named tick. It does not identify block-causal and frame-causal execution.

## 8. Lineage laws

Assume lineage edges are finite, every target has a source or explicit generation witness, and composed reason paths are
normalized as lists.

### Proposition 8.1 — lineage composition is a category

Typed lineage multirelations have associative composition and identity lineages as two-sided units.

*Proof.* The underlying operation is finite relational composition, which is associative. Reason paths compose by list
concatenation, also associative. Role summarization is a fold over the complete path, so it is independent of grouping.
Relational identity adds an empty reason path and preserves every edge. ∎

### Proposition 8.2 — pass composition preserves lineage totality

If every successful pass gives each target anchor a source edge or explicit `Generated` edge, then the composition of
two successful passes has the same property.

*Proof.* Take a final target anchor. Its second-pass edge either names an intermediate anchor or is generated. In the
first case, totality of the first lineage supplies at least one predecessor or generation witness, and relational path
composition supplies the final edge. In the second case, composition retains the second pass's generation reason and its
source rule/site rather than inventing an intermediate predecessor. Finiteness is preserved by finite union and finite
products of edge sets. ∎

## 9. Preparation and cache theorems

Let

```text
args(M,B,P,s,o) = (nf_T(M),canon_B(B),nf_P(P),s,o)
prepare(M,B,P,s,o) = prepare_spec(args(M,B,P,s,o)).
```

### Theorem 9.1 — identity-sensitive R1

If `args(M,B,P,s,o)=args(N,B',P',s,o)`, then the two `prepare` results are equal.

*Proof.* Expand `prepare`. Both sides are applications of the same deterministic function to equal arguments. ∎

This is a factorization theorem by definition. Termination of `prepare_spec` is an additional premise for evaluation,
not a proof of factorization. Current `nf_T` includes `Origin`, so this theorem is identity-sensitive.

### Theorem 9.2 — versioned cache correctness

Suppose `CacheInvariant(C,v)` from [28 §9] holds. If lookup of `key_v(args)` confirms both version `v` and the complete
canonical `args` and returns `r`, then `r=prepare_spec_v(args)`.

*Proof.* This is exactly the per-entry clause of `CacheInvariant`, applied after collision-checking the complete
arguments and version. The conclusion covers both `Ok PreparedSpec` and `Err PrepareError`. ∎

### Non-theorem 9.3 — content-only frame equality

No preceding result proves that content-equal but identity-distinct scores render the same frames. Such a theorem needs
a declared content quotient, factorization of every identity-sensitive preparation decision through that quotient, and
fixed deterministic runtime inputs and floating-point semantics.

## 10. What has and has not been proved

Relative to B1–B7, the fixed K₂ core has decidable checking, preservation, progress, deterministic reduction, and strong
normalization. Timelines admit unequal-extent overlay with the stated algebra. Total eventually-affine warps act on
every finite timeline and satisfy the rebased sequence law. Accepted process graphs have deterministic causal execution
at their named tick. Lineage and identity-sensitive R1 compose as claimed.

This proof does **not** establish:

- source-module elaboration or type inference;
- any Rust foreign operation's ownership contract;
- completeness of declared process dependency relations;
- cultural adequacy of a music-theory package;
- equivalence of block and frame process semantics;
- denotational equality of arbitrary processes; or
- the content-only rendering law.

Those are separate proof, validation, or design obligations. The independent reviewer should reject this draft if any
theorem above implicitly relies on one of them.
