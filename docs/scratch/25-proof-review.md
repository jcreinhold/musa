# Proof review: K₁ metatheory

**Status: independent review of the fixed draft; governs nothing.** I reviewed [24](24-metatheory-of-k1.md) against the
definitions in [20](20-candidate-staged-algebras.md) and [21](21-semantics-of-k1.md), with the examples in
[22](22-encodings-and-sieve.md) as intended-use pressure and the optional event-plan candidate in
[23](23-candidate-event-presentations.md) where it makes mathematical claims. The standard is cross-document consistency
and a full local check of every displayed theorem, not implementation conformance or cultural adequacy.

The review fixes the draft as written. Suggested repairs below are not silently imported into the verdict.

## Findings

### High

1. **A3 does not imply the reducibility property used to prove strong normalization.**
   - **Location**: [24 §1 A3](24-metatheory-of-k1.md), Lemma 4.3, Theorem 4.4, and Corollary 4.5.
   - **Type**: false statement / missing hypothesis.
   - **Problem**: A3 says that a saturated primitive application on closed value arguments takes one step to a closed
     value of the declared result type. Lemma 4.3 then asserts that this result is reducible. That implication is false
     at higher-order result types. For a hostile primitive

     ```text
     p : 1 → (1 → 1)
     p(⋆) → fn (x:1) { p(⋆)(x) },
     ```

     A3 and A4 hold literally: `p(⋆)` deterministically reduces to one closed, well-typed value and is pure. But

     ```text
     p(⋆)(⋆) → (fn x { p(⋆)(x) })(⋆) → p(⋆)(⋆) → ⋯ .
     ```

     Thus Theorem 4.4 and Corollary 4.5 are false under the stated assumptions. The same issue is less artificial for
     a higher-order primitive such as timeline `map`: treating its callback execution as one total δ-step assumes the
     very termination property the logical-relation argument is meant to establish.
   - **Why it matters**: total source evaluation is the principal stage-separation claim of K₁. This is fatal to the
     current proof, though not to a suitably restricted calculus.
   - **Suggested repair**: replace A3, for the normalization proof, with the logical-relation contract “reducible
     arguments imply a reducible primitive application.” A simpler implementable alternative is to require every foreign
     primitive to be first-order and `Data`-returning, define higher-order combinators such as `map` inside the total
     calculus, and give primitive constants no first-class arrow-valued result. A3 remains enough for progress and
     preservation, but not for strong normalization.

2. **`warp` is typed as total although its fixed semantics makes it partial, and composition is not closed as stated.**
   - **Location**: [20 §8](20-candidate-staged-algebras.md), [21 §5](21-semantics-of-k1.md), and [24 Lemma 6.1 and
     Theorem 6.2](24-metatheory-of-k1.md).
   - **Type**: cross-document inconsistency / false statement.
   - **Problem**: `warp` has result type `Timeline d A`, but [21] gives each warp a finite domain endpoint and says that
     application beyond it is an error. A timeline whose extent exceeds that endpoint is nevertheless well typed. Its
     `warp(w,M)` is therefore either stuck, contradicting progress and A3, or has an unmentioned error result.

     The same missing domain data invalidates unconditional composition. Let `w` have domain `[0,1]` with `w(t)=2t`,
     and let non-extensible `v` have domain `[0,1]`. The composite is undefined on the latter half of `w`'s domain.
     It is not a warp on the stated domain merely because slopes and breakpoints compose. Theorem 6.2 also needs both
     applications and the composite to cover the relevant timeline extent. Extension policy, which [21] correctly
     calls semantic, makes this impossible to leave implicit.
   - **Why it matters**: this is a well-typed stuck-term counterexample and a false closure theorem, not only an omitted
     side calculation. It is fatal to the current Warp API and proof.
   - **Suggested repair**: choose one coherent design:

     - admit only total, eventually affine warps on all of `ℚ≥0`; or
     - make `warp` and incompatible `compose` return `Result`; or
     - define composition to restrict to a computed maximal compatible prefix and expose that domain in the result.

     Then state W2 with exact coverage premises. `quantize` needs the same treatment: [20] types it as returning a
     timeline while saying it returns evidence or diagnostics.

3. **The guarded-feedback theorem proves a shifted recurrence, not the guarded trace described by the API.**
   - **Location**: [21 §6.2](21-semantics-of-k1.md) and [24 Theorem 7.3](24-metatheory-of-k1.md).
   - **Type**: wrong relation or object / cross-document inconsistency.
   - **Problem**: `shift` is undefined. If it means the tail `shift(x)(n)=x(n+1)`, the displayed equation says

     ```text
     x(n+1) = F_X(i,x)(n).
     ```

     This recurrence has a unique solution for every merely causal `F` once `x(0)` is fixed. The proof in fact uses
     ordinary causality at output tick `n`; it neither uses nor proves the stated strict-causality index at output tick
     `n+1`. The operator has inserted a feedback register externally. In contrast, [20] and [21] say the
     `GuardedProcess` witness certifies a delay already lying on every internal feedback path. Applying both readings
     either double-counts the delay or proves soundness for a different operator. If `shift` has its conventional
     stream-delay meaning instead, the displayed equation does not yield the recurrence used in the proof.
     The newly recorded implementation fact makes a second distinction explicit: Musa currently cuts feedback at
     **block** boundaries, whereas the displayed recurrence and `Frames r` examples use a **frame** tick. Block-causal
     execution is still causal, but it is not an operational implementation of the one-frame recurrence.
   - **Why it matters**: existence and uniqueness are plausible, but the theorem does not validate the actual graph
     acceptance rule. The guard appears load-bearing in the API and irrelevant in the proof.
   - **Suggested repair**: pick one of two definitions. For an **externally delayed feedback** operator, define
     `next(x)(n)=x(n+1)`, retain a fixed initial value, and assume only causality. For an **instantaneous trace guarded
     by an internal delay**, state `(o,x)=F(i,x)` and define strict causality with an explicit tick-zero clause; do not
     add a second shift. Relate the selected theorem to the graph cut construction by a separate denotational-soundness
     lemma.

4. **Theorem 2.4 has no defined elaboration relation, and Lemma 2.1 is a decision procedure, not an inference solver.**
   - **Location**: [20 §§4–6](20-candidate-staged-algebras.md), [21 §§1–2 and 7.1](21-semantics-of-k1.md), and
     [24 Theorem 2.4](24-metatheory-of-k1.md).
   - **Type**: proof gap / unverified dependency.
   - **Problem**: equality checking for two already known index expressions is decidable, as Lemma 2.1 says. Rank-1
     instantiation instead requires synthesizing substitutions for metavariables. For example, using
     `f : ∀n. Vec (2·n) A → B` at `Vec 6 A` requires finding `n=3`; contextual variables require a specified class of
     representable substitutions. Saying that the generated equations are “decided by Lemma 2.1” does not give that
     algorithm. Presburger arithmetic can support a decidable design, but the draft must say whether it performs
     existential solving and uniqueness checks, requires explicit static arguments, or restricts inferred occurrences to
     a syntactic pattern fragment.

     More seriously, no syntax, judgments, or translation are given for signature matching, abstract-type sealing,
     generative structure functors, or monomorphization. Acyclicity proves that a defined finite-site elaborator would
     terminate; it does not define one or prove type preservation. Item 2 in §10's debt ledger acknowledges the actual
     elaborator remains unproved, but Corollary 4.5 nevertheless uses Theorem 2.4 to transfer totality to all accepted
     source expressions.
   - **Why it matters**: the monomorphic core may be decidable, but project checking and source-to-core soundness are
     load-bearing and currently only asserted. This is fatal to the claimed source-level theorem, not evidence that
     rank-1 polymorphism itself is undecidable.
   - **Suggested repair**: first state a bidirectional core checker. Then define scheme instantiation and its index-meta
     fragment, a small static module elaboration judgment, and an elaboration-preservation theorem. With A1, a separate
     finite-specialization lemma should then be straightforward.

5. **The optional E₃ constructor `before` is not closed on prime event structures.**
   - **Location**: [23 §§1–2](23-candidate-event-presentations.md); [24 §9](24-metatheory-of-k1.md) does not use this
     constructor, but calls E₃'s formation checks decidable.
   - **Type**: false statement.
   - **Problem**: let `P` contain conflicting events `p # q`, and let `Q` contain one event `r`. The proposed
     `before(P,Q)` adds both `p ≤ r` and `q ≤ r`. Heredity gives `p # r` from `p # q` and `q ≤ r`; symmetry gives
     `r # p`; then heredity with `p ≤ r` gives `r # r`, contradicting irreflexivity. Thus the claim that construction
     cannot produce an ill-formed plan is false. Conceptually, a prime event cannot have the disjunctive enabling
     “whichever conflicting branch occurred” as a shared future.
   - **Why it matters**: this is fatal to E₃'s total `before` operation and several advertised examples, but no K₁
     theorem depends on E₃. Decidability of validating a finite candidate and Propositions E8–E9 for an already valid
     schedule remain correct.
   - **Suggested repair**: either restrict `before(P,Q)` to conflict-free `P`, duplicate `Q` separately under each
     maximal alternative, or move to an event-structure presentation with disjunctive/general enabling. Do not hide this
     under “hereditary closure”: closure is exactly what exposes the contradiction.

### Medium

1. **R1-K₁ is a correct identity-sensitive factorization lemma, but cache correctness needs another invariant.**
   - **Location**: [21 §9](21-semantics-of-k1.md), [24 Theorem 8.1 and Corollary 8.2](24-metatheory-of-k1.md), and
     [27 §7](27-lineage-is-the-link.md).
   - **Type**: missing hypothesis / intended-theorem boundary.
   - **Problem**: Theorem 8.1 is valid by definition: a function explicitly defined through `nf_T`, `canon_B`, and
     `nf_P` is invariant when those arguments are equal. The implementation check recorded in [27] matters here: current
     kernel equality includes `ScoreFact`'s `Origin` in its canonical payload key. The current law is therefore
     identity/provenance-sensitive; this review found no evidence that its full normal form presently forgets Origin.
     The theorem does not prove the proposed stronger content-only render law. That law needs a separate quotient and a
     proof that lineage differences cannot affect frames.

     Corollary 8.2 also lacks the cache invariant. Equality of keys proves equality of arguments, not that a stored
     value was produced by the current `prepare_spec`. A stale entry, corrupt entry, changed primitive schema, or
     changed preparation implementation is an immediate counterexample. The statement also says the cache returns a
     `PreparedSpec`, although recomputation can return `Err`.
   - **Why it matters**: the factorization theorem and current Origin-sensitive R1 survive. The stronger content law and
     cache correctness do not yet follow.
   - **Suggested repair**: keep `R1-identity` and any future `R1-content` as separate theorems. State the cache
     invariant `entry[key(args)] = prepare_spec(args)`, key the compiler/preparation and primitive-schema versions,
     compare the full canonical key after digest lookup, and conclude equality of `Result PreparedSpec PrepareError`.

2. **“Strictly positive” is too broad for the constructor-count induction in Lemma 4.2.**
   - **Location**: [20 §4](20-candidate-staged-algebras.md), [24 A5 and Lemma 4.2](24-metatheory-of-k1.md).
   - **Type**: proof gap / missing definition.
   - **Problem**: customary strict positivity permits a recursive occurrence in a positive function codomain, such as
     `data D = C (Nat → D)`. Such children are not a finite list of already constructed immediate subdata, so “number of
     constructors in the folded value” does not justify the recursive calls claimed in the proof. Standard strong
     normalization for richer strictly positive recursors is possible, but needs a type-indexed reducibility argument,
     not this measure.
   - **Why it matters**: after repairing primitives, this remains a real gap in the normalization scaffold.
   - **Suggested repair**: define K₁ data declarations initially as first-order polynomial strictly positive types whose
     recursive fields occur in finitely traversable containers. If higher-order strictly positive functors are desired,
     state their map action and prove recursor reducibility separately.

3. **The small-step safety proof and the environment semantics do not yet name the same core.**
   - **Location**: [21 §7](21-semantics-of-k1.md) and [24 §§3–4](24-metatheory-of-k1.md).
   - **Type**: undefined term / proof gap.
   - **Problem**: [21] gives representative big-step closure rules; [24] uses substitution-based small-step β-reduction
     while declaring closures to be term values. `constant`, `closure`, `neutral`, `shift`, evaluation contexts,
     primitive arities, conversion, vector eliminators, and the fold reduction rule are not defined. If a primitive
     constant itself has arrow type, the arrow canonical-forms claim is already false; if primitives are saturated
     syntax, that restriction needs to be stated. Preservation also omits the type-conversion case.
   - **Why it matters**: the familiar STLC proof is probably recoverable, but there is no exact derivation to invert
     yet.
   - **Suggested repair**: choose substitution semantics or closures, publish the full value/redex/evaluation-context
     grammars, make primitives saturated first-order forms (or add their canonical form), and include conversion and
     vector/fold cases explicitly.

4. **Process tensor and delay cutting need formation at port-dependency level.**
   - **Location**: [20 §9](20-candidate-staged-algebras.md), [21 §§6 and 8.3](21-semantics-of-k1.md), and [24 Theorems
     7.2 and 7.4](24-metatheory-of-k1.md).
   - **Type**: missing hypothesis / undefined operation.
   - **Problem**: port rows require distinct labels, but `parallel` has no disjoint-label premise and `I ⊗ J` is not
     defined to tag or freshen colliding labels. Thus a well-formed pair of inputs need not have a well-formed tensor.
     Separately, “every cycle crosses a delay” must be checked on instantaneous **port dependencies**, not merely on
     processor nodes. A multiport stateful primitive may delay one path and pass another through instantaneously.
     Cutting the whole node or an unspecified “delay edge” can then accept an algebraic loop. A8 also needs `step_p` to
     be total and each primitive's state to be unshared except through explicit graph edges. The new block-granularity
     qualification in [21] and [24] correctly prevents claiming that today's scheduler is a frame-tick implementation;
     it does not supply the missing port-dependency definition.
   - **Why it matters**: the extensional SMC laws and DAG scheduling argument are correct only after these formation
     conditions are made precise.
   - **Suggested repair**: define row tensor as tagged disjoint union or require/provide explicit renaming. Have each
     primitive export an instantaneous input–output dependency relation and designated delayed state edges;
     topologically sort that dependency graph, not the coarse node graph.

5. **A3 and A4 are implementation contracts, despite being called statically enforceable.**
   - **Location**: [24 §1](24-metatheory-of-k1.md).
   - **Type**: cross-document inconsistency / trust-boundary error.
   - **Problem**: totality and purity of arbitrary compiler or Rust primitives cannot be established by checking their
     source signatures. The paragraph classifies A1–A5 as statically enforceable and only A6/A8 as ownership-boundary
     contracts. That understates the trusted base.
   - **Why it matters**: an explicit trust ledger is one of the draft's strengths; this classification defeats it at the
     most important assumption.
   - **Suggested repair**: make A3–A4 ownership-boundary contracts too, unless all primitives are defined in a
     separately total checked metalanguage. Add conformance tests, but do not call tests proofs of totality.

6. **The index theory is decidable, but satisfiability and inference boundaries need to be stated.**
   - **Location**: [21 §§1–2](21-semantics-of-k1.md) and [24 Lemmas 2.1–2.3](24-metatheory-of-k1.md).
   - **Type**: missing hypothesis.
   - **Problem**: conversion under an inconsistent `Δ` identifies every index. A well-formed static context therefore
     needs a satisfiability condition, especially before proof erasure. Also, negating `i=j` takes the proof outside the
     displayed positive grammar for `φ`; that is harmless if the solver language is full Presburger logic but should be
     said. None of this reintroduces value-level dependency.
   - **Why it matters**: these are small but real obligations behind index erasure and vector safety.
   - **Suggested repair**: define `⊢ Δ ok` to include Presburger satisfiability and distinguish the user constraint
     grammar from the solver's closed logical queries.

7. **E₃'s nominal schedule witness is not expressible in the stated K₁ type grammar.**
   - **Location**: [23 §8](23-candidate-event-presentations.md) versus [20 §4](20-candidate-staged-algebras.md).
   - **Type**: encoding / intended-theorem mismatch.
   - **Problem**: `exists PId. Scheduled { ... }` requires first-class existential or generative dependent packaging,
     neither of which appears in K₁'s types. Static generative modules do not create a fresh type identity for each
     runtime call to `schedule`.
   - **Why it matters**: the proposed escape from value-dependent type conversion currently adds an unadmitted type
     former.
   - **Suggested repair**: make `Scheduled c A` one opaque data type whose private representation contains the plan,
     checked schedule, and runtime identity together. Clients need not see a type-level `PId`. Add first-class
     existentials only if another caller needs them.

### Low

1. **Several theorem statements rely on implicit domains or overloaded equality.**
   - **Location**: throughout [24 §§3–8](24-metatheory-of-k1.md).
   - **Type**: exposition issue only.
   - **Problem**: the safety lemmas drop `Δ`; timeline theorems use set-level equality while R1 uses normal-form
     equality; process laws use extensional equality; and warp arrows use clock letters without displaying finite
     domains. The intended comparisons can be reconstructed, but not locally in every statement.
   - **Suggested repair**: annotate theorem statements with `=`, `≡T`, structural normal-form equality, or extensional
     process equality as appropriate, and state the ambient static context once for the core safety section.

2. **The reducibility-candidate lemmas are sketches, not complete proofs.**
   - **Location**: [24 Lemmas 4.1–4.3](24-metatheory-of-k1.md).
   - **Type**: exposition issue only after the substantive primitive and fold repairs.
   - **Problem**: the neutral-arrow case needs induction on reduction height of the argument; data reducibility needs an
     explicit interpretation for parameters and recursive positions; and applying the fundamental lemma to the identity
     substitution requires variables to be reducible neutrals. These are standard details but do not follow from
     “simultaneous induction on `A`” alone.
   - **Suggested repair**: state the candidate closure lemmas in the conventional expansion/reduction form and prove
     them before the fundamental lemma.

## Verdict

- **Decision**: **Incorrect**.
- **Basis**: I reconstructed the stated dependency chain, checked the timeline, warp, process, normalization, and R1
  arguments against the fixed denotations, and tested hostile small examples. A3 admits an explicit non-normalizing
  primitive, a finite-domain warp gives a well-typed stuck term and breaks unconditional composition, and E₃'s `before`
  constructor gives an explicit self-conflict. The feedback and elaboration arguments prove less or something different
  from their intended claims.
- **Limits**: this is a paper-proof review. I did not check a mechanized formalization, a concrete Presburger solver,
  the Rust compiler/DSP implementations, or the cultural adequacy of the packages in [22]. No external theorem is needed
  for the counterexamples above. The verdict applies to the fixed proof draft, not to the strategic K₁ design, which
  appears repairable without adding full dependent types, CBPV, worlds, or theta-links.

## Open questions and required assumptions

The shortest honest repair order is:

1. choose the exact monomorphic term syntax and the admissible primitive contract;
2. choose total warps or expose domain failure in the types;
3. choose external-delay feedback or internal-delay guarded trace, then prove that exact operator;
4. define rank-1/index/module elaboration and prove preservation before transferring core totality;
5. prove that the chosen preparation quotient retains every datum the real compiler needs, then state a versioned cache
   invariant; and
6. either restrict E₃'s sequencing or replace prime causality with a presentation supporting disjunctive enabling.

The following assumptions should be visible in the next draft: satisfiable static contexts; collision-free row tensor;
total primitive transitions; port-level delay dependencies; canonical-normalizer termination and soundness; and an
explicit trust classification for every foreign primitive.

## Clean passes

1. **Fixed-expression index equality and type conversion survive.** With a satisfiable `Δ`, no value terms in types,
   finite rows, nominal clocks/types, and Presburger natural arithmetic, Lemmas 2.1–2.2 are sound and give the intended
   decidable conversion boundary. The proposed index language does not recreate [09]'s plan/set-concatenation problem.
2. **The timeline algebra survives completely.** Closure, the sequence monoid, commutative overlay, the fixed-extent
   overlay unit, synchronized interchange, and Counterexample 5.5 are correct with multiplicities retained. In
   particular, the proof supports the design conclusion that overlay should accept unequal extents; equality belongs as
   a premise of interchange, not as an overlay typing rule.
3. **The substantive warp laws survive after domain repair.** For continuous strictly increasing warps covering all
   displayed coordinates, W1–W5 are correct. The rebased sequence law W4 is especially useful and correctly rejects the
   false claim that a non-additive warp preserves sequence using the same local origin.
4. **The unguarded process combinators survive extensionally.** Identity, serial composition, pointwise parallel, and
   permutations preserve causality and satisfy the symmetric-monoidal equations once port tensor is well formed.
5. **DAG execution survives under a precise delay graph.** Cutting genuinely delayed port dependencies makes the
   remaining finite graph acyclic; a fixed topological scheduler over total deterministic state transitions is causal
   and deterministic.
6. **R1's central correction survives.** Structural recursion proves termination, not quotient invariance. Explicit
   factorization through canonical normal forms proves the narrow Theorem 8.1. Frame equality correctly remains a
   separate runtime obligation.
7. **The scope discipline survives domain pressure.** [24] does not pretend its calculus proves that Western or
   non-Western packages are culturally adequate, and it does not force a pitch, key, meter, notation, or work ontology
   into the metatheory. The cross-tradition examples therefore reveal representational obligations without becoming
   false premises of the mathematical proofs.
8. **The valid fragment of E₃ remains decidable.** Well-formedness and configuration enumeration are decidable for a
   finite event presentation already satisfying the axioms, and E8–E9 follow from the schedule definition.
