# Proof review: K₂ candidate and metatheory

**Status: independent review of the frozen K₂ draft; governs nothing.** I reviewed [28](28-candidate-k2.md) and
[29](29-metatheory-of-k2.md) as fixed documents. I also checked every finding in the K₁ review [25](25-proof-review.md)
against its claimed K₂ repair. For the identity-sensitive preparation claim I inspected the governing normalization
specification and the current `ScoreFact::canonical_key` implementation; I did not assume that “includes Origin” meant
“includes every field of Origin.”

The standard is full local proof and cross-document consistency. A theorem conditional on an ownership contract may
survive, but the contract must imply the exact fact used. A plausible implementation not supplied by the interface is
not silently added to the proof.

## Findings

### Fatal to the fixed proof

1. **The port-level DAG is not an executable schedule for the primitive interface.**
   - **Location**: [28 §6](28-candidate-k2.md), B7, and [29 Theorem 7.2](29-metatheory-of-k2.md).
   - **Type**: false statement / encoding–intended-theorem mismatch.
   - **Problem**: a primitive exposes only one operation

     ```text
     step_p : State_p × Frame(I) → State_p × Frame(O).
     ```

     It does not expose one evaluator per output port or a separate next-state evaluator. The validator nevertheless
     topologically sorts individual ports using `dep_p` and the proof claims to determine each current port in that
     order. These interfaces do not match.

     A two-node example is enough. Let `A` have input `x`, output `y`, and `dep_A=∅` because its current `y` is
     independent of `x`; let `B` have input `u`, output `v`, and `dep_B={(u,v)}`. Wire

     ```text
     A.y → B.u
     B.v → A.x.
     ```

     The instantaneous port graph is the DAG

     ```text
     A.y → B.u → B.v → A.x,
     ```

     so `close` accepts it. But the stored API cannot evaluate `A.y` first: calling `step_A` requires the whole input
     frame, including `A.x`, which is available only after `B.v`; calling `step_B` first requires `B.u`. Soundness of
     the extensional dependency declaration says `A.y` is independent of `A.x`; it does not give the runtime a function
     which computes that projection, an inhabitant to use for unavailable inputs, or a way to compute next state later
     without invoking `step_A` again. The induction in Theorem 7.2 therefore applies an operation the candidate did not
     define.
   - **Why it matters**: Theorem 7.2 claims a stored, total transition and a deterministic execution schedule, not only
     the classical existence of some factorization of an output function. That load-bearing process theorem is false for
     the fixed interface, so the overall metatheory is incorrect.
   - **Suggested repair**: either make scheduling node-granular and conservatively add every input-to-output edge needed
     before one whole-node `step`, or change the primitive contract to supply executable

     ```text
     output_p,o : State_p × Frame(deps(o)) → Frame(o)
     next_p     : State_p × Frame(I) → State_p
     ```

     operations, with a coherence law relating them to the primitive denotation. The latter supports the intended
     port-sensitive acceptance rule. Then prove graph execution for that exact two-phase interface.

### High

1. **Generated lineage is not an inhabitant of the lineage type used by the category proof.**
   - **Location**: [27 §§2–4](27-lineage-is-the-link.md), adopted by [28 §7](28-candidate-k2.md), and
     [29 Propositions 8.1–8.2](29-metatheory-of-k2.md).
   - **Type**: undefined term / wrong object.
   - **Problem**: the fixed definition says every `DerivationEdge S T` contains

     ```text
     source : AnchorId_S.
     ```

     Yet formation and Proposition 8.2 distinguish a source edge from an explicit `Generated` edge, and the proof says
     a second-pass generated edge does not name an intermediate anchor. Those are different data types. If a source
     presentation has no anchors and a pass generates one target by an explicit default, the formation prose permits
     the pass, but no `DerivationEdge S T` can be constructed because `AnchorId_S` has no inhabitant.

     If instead every `Generated` edge still names an intermediate source anchor, it is an ordinary relation edge and
     the “generated” case in Proposition 8.2 is not a second case. If it is source-less, ordinary relational
     composition does not compose it. K₂ needs a specified sum such as `From AnchorId_S | Generated GenerationSite`
     and composition rules for all combinations before associativity or preservation of totality can be checked.

     There is further type drift: a primitive edge stores one `OriginStep`, while a composed edge is said to store a
     normalized list; `RegionMap` composition is not defined. These are not supplied by “underlying relational
     composition.”
   - **Why it matters**: lineage is K₂'s proposed coherence structure. The ordinary-category proof currently concerns
     plain relations, while the intended compiler lineage includes source-less generation and region evidence.
   - **Suggested repair**: define the exact edge sum, path type, generation-site typing, region-map composition, and
     canonical equality first. Then re-prove identity, associativity, and totality by cases. A simpler alternative is to
     require every generated value to point to an explicit rule anchor in the source anchor view; if chosen, remove the
     source-less generated case everywhere.

2. **The stated R1 theorem does not establish governing semantic R1 and lineage preservation simultaneously.**
   - **Location**: [28 §§4, 7, and 9](28-candidate-k2.md), [29 Theorem 9.1](29-metatheory-of-k2.md),
     `docs/rules/kernel/05-normalization.md` N3–N6, and `crates/musa-compiler/src/elaborate.rs`.
   - **Type**: wrong relation or object / cross-document inconsistency.
   - **Problem**: current timeline normalization preserves full payload values, while governing semantic equality uses
     each payload's `canonical_key`. The current `ScoreFact` key includes `source_span` and `expansion_path` but omits
     `Origin.definition_span` and `Origin.declaration`; N3 explicitly says those two fields are quotiented away. Take
     one-occurrence timelines whose facts differ only in either omitted field. They are semantically equal under N4, but
     their normalized Rust values remain structurally different because `ScoreFact` derives full `PartialEq`.

     Theorem 9.1 assumes equality of `args`, not governing `M ≡ N`. There are therefore two readings, and neither proves
     the advertised combination:

     - if equality of `nf_T` is structural equality of full normalized values, governing semantic equality does not
       imply the theorem's premise, so this is not current R1;
     - if equality of `nf_T` is the canonical-key quotient, a lineage-bearing `prepare_spec` may distinguish the omitted
       definition span or declaration, so substitution of “equal” arguments is not valid for that operation.

     The phrase “current `nf_T` includes Origin” hides this distinction. It includes some Origin fields in semantic
     equality and preserves the full payload in the normalized representation; those are not the same claim.
     Moreover, governing R1 in `core-boundary.md` includes frame-for-frame rendered-stream equality after semantically
     equal gesture timelines. Theorem 9.1 concludes only equality of preparation results, and K₂ supplies no runtime
     determinism/congruence theorem which derives the second half. Calling content-only frame equality a non-theorem is
     prudent, but it does not discharge governing R1 even on N4-equal inputs.
   - **Why it matters**: this is the exact-object failure K₂'s three-equality discipline was meant to prevent. The
     literal factorization theorem is tautologically correct for whichever `args` equality is chosen, but it does not
     prove that the governing semantic quotient is adequate for a preparation pass that returns derivation lineage or
     the governing rendered-stream conclusion.
   - **Suggested repair**: use two explicit inputs and laws. Keep content/semantic R1 over the governing semantic normal
     form for the portion of preparation proved insensitive to omitted provenance. Pass a separate canonical
     presentation/derivation value containing every anchor and Origin field needed for lineage, with its own equality.
     State whether `PreparedSpec` equality includes lineage. Do not call structural normalized-value equality,
     canonical-key equality, and derivation equality all `nf_T` equality.

### Medium

1. **The “exact monomorphic grammar” still omits required static applications.**
   - **Location**: [28 §§2–4](28-candidate-k2.md) and [29 Theorem 2.3](29-metatheory-of-k2.md).
   - **Type**: proof gap / undefined syntax.
   - **Problem**: operations such as

     ```text
     empty : Rat≥0 ⇒ Timeline c A
     ```

     are schematic in `c` and `A`, but term syntax contains only `op(ē)`, with no explicit static arguments. The term

     ```text
     let x = empty(0) in ()
     ```

     does not constrain either `c` or `A`. K₂ says the checker performs no existential inference and that ambiguous
     static arguments are explicit, but the exact grammar has nowhere to write them. Parameterized nullary data
     constructors have the same problem. A finite declaration table does not turn this into a syntax-directed check.
   - **Why it matters**: the intended no-inference boundary is sensible, but Theorem 2.3 does not yet describe an
     algorithm for the displayed terms.
   - **Suggested repair**: add explicit `op[ῑ](ē)` and `C[ῑ](ē)` forms, or give a complete bidirectional grammar in
     which every non-synthesizing occurrence is checked against an expected type. Annotate `let` when its right-hand
     side cannot synthesize. Define `Rat≥0` as an opaque validated data type or remove it from signatures; it is not in
     the displayed type grammar.

2. **The administrative timeline and polynomial-fold reductions remain schemas rather than exact rules.**
   - **Location**: [28 §3](28-candidate-k2.md) and
     [29 Theorems 2.3, 3.3–3.5, and Lemmas 4.2–4.4](29-metatheory-of-k2.md).
   - **Type**: proof gap / undefined notation.
   - **Problem**: `timeline_build` is absent from the displayed term grammar even though the metatheory says it is in
     `K₂⁰`. No typing rule or displayed all-payloads-finished reduction is given for it. Likewise,
     `branch_C(v̄, recursive folds)` does not define how a fold traverses recursive positions beneath a sum, product, or
     opaque finite vector. The proofs repeatedly invert or reduce these administrative forms as if those judgments had
     already been stated.
   - **Why it matters**: the repairs are routine, but preservation, progress, deterministic decomposition, and fold
     normalization are not full proofs of one fixed reduction system until the forms exist.
   - **Suggested repair**: publish the complete administrative grammar, typing rules, and terminal reductions. For
     polynomial data, define the functorial map generated from each declaration, including a finite vector-build form,
     and define the catamorphism through it.

3. **The reducibility proof is plausible but still omits two active substitutions.**
   - **Location**: [29 Lemmas 4.1–4.4 and Theorem 4.5](29-metatheory-of-k2.md).
   - **Type**: proof gap.
   - **Problem**: Lemma 3.1 proves substitution only for a value, while the fundamental lemma substitutes arbitrary
     reducible terms. The lambda case says to extend the closing substitution by an arbitrary reducible argument, which
     therefore uses a stronger substitution lemma. Also Lemma 4.4 starts by reducing `M`, although the fixed
     left-to-right semantics reduces `f` before `M`; candidate expansion must account for reductions of both operands in
     the actual order. The conversion case needs the reducibility interpretation of Presburger-equal vector lengths to
     be explicitly identical, not merely syntactically asserted.
   - **Why it matters**: unlike K₁'s primitive counterexample, I found no non-normalizing K₂ term. These are genuine
     proof omissions, not evidence that the first-order boundary failed.
   - **Suggested repair**: generalize typing substitution to any well-typed term; state the value-substitution corollary
     separately. Prove a two-argument expansion lemma for `timeline_map` in evaluation order, and define reducibility on
     the semantic normal form of index expressions under `Δ`.

4. **The category of process descriptions is asserted only after passing silently to extensional denotations.**
   - **Location**: [28 §§2.1 and 6](28-candidate-k2.md) and [29 Lemma 7.1](29-metatheory-of-k2.md).
   - **Type**: missing hypothesis / exposition issue with mathematical content.
   - **Problem**: tagged tensor is not strictly associative: `(I⊗J)⊗K` and `I⊗(J⊗K)` have different nested labels. Lemma
     7.1 correctly mentions associators and explicit renaming, but no denotation or coherence laws for `PortIso` and
     `rename` are given. Thus the extensional causal functions form the expected symmetric monoidal category up to
     canonical port isomorphism; the displayed process descriptions have not yet been proved to do so.
   - **Why it matters**: this does not affect serial/parallel causality, but it limits exactly what “satisfy the SMC
     laws” has established.
   - **Suggested repair**: define the canonical associator, unitors, symmetry, and rename denotation, and state the
     pentagon, triangle, and symmetry coherence equalities extensionally. Alternatively normalize nested port tags to a
     flat canonical row before typing process tensor.

5. **Preparation purity and determinism are used but not placed explicitly in the trust ledger.**
   - **Location**: [28 §§9–10](28-candidate-k2.md) and [29 Theorems 9.1–9.2](29-metatheory-of-k2.md).
   - **Type**: missing hypothesis.
   - **Problem**: Theorem 9.1 calls `prepare_spec` the same deterministic function. If it is a checked K₂ term, core
     totality and purity provide that fact; if it is Rust at the compiler boundary, B3/B4 must cover its complete
     signature. The trust ledger names preparation normalizers but not `prepare_spec` itself. The cache invariant is
     also an invariant to be preserved by insert, invalidation, and version migration; Theorem 9.2 merely eliminates its
     already-assumed per-entry clause.
   - **Why it matters**: the conditional cache theorem is correct, but implementation cache correctness needs an
     invariant-preservation proof and a visible owner for preparation determinism.
   - **Suggested repair**: classify `prepare_spec` explicitly as checked core or trusted foreign code. Prove
     empty-cache, insertion, lookup, invalidation, and version-change preservation of `CacheInvariant` for the concrete
     cache.

### Low

1. **Several nominal kinds and equality signs remain implicit.**
   - **Location**: throughout [28–29](28-candidate-k2.md).
   - **Type**: exposition issue only.
   - **Problem**: `Rat≥0`, `Frame(I)`, `OpenGraph`, `PortIso`, canonical timeline equality, structural normalized-value
     equality, and extensional process equality are used without one consolidated formation/equality table. The local
     intention is usually recoverable, but the R1 error demonstrates that equality overloading is not harmless here.
   - **Suggested repair**: add a table giving each construct's kind, introduction boundary, eliminators, and equality.

2. **The total-warp representation needs a breakpoint convention, though its laws do not.**
   - **Location**: [28 §5](28-candidate-k2.md) and [29 §6](29-metatheory-of-k2.md).
   - **Type**: exposition issue only.
   - **Problem**: continuity makes endpoint values unambiguous, but a canonical finite encoding still needs to say which
     adjacent segment owns a breakpoint and how redundant collinear points normalize.
   - **Suggested repair**: choose half-open segment storage plus a final closed ray, reduce rationals, and merge
     adjacent collinear pieces before encoding.

## Verdict

- **Decision**: **Incorrect**.
- **Basis**: the accepted port-DAG example above has no execution in the only primitive interface K₂ supplies, so
  Theorem 7.2 is false as an operational theorem. Generated lineage is not represented by the relation type used in
  Propositions 8.1–8.2, and Theorem 9.1 does not bridge governing semantic equality to lineage-sensitive preparation. I
  also checked the static, reduction, normalization, timeline, total-warp, cache, and K₁-repair claims directly.
- **Limits**: this is a paper-proof review, not a mechanized check. The ownership contracts B3–B7, concrete dependency
  declarations, warp validators, encoders, and cache operations were not implementation-verified. I inspected the
  current canonical-key implementation only to test the draft's specific “includes Origin” reading. The verdict is on
  the fixed proof, not the K₂ strategy; the strategy remains repairable without full dependent types, CBPV, worlds, or
  theta-links.

## K₁ repair audit

| K₁ finding | K₂ result |
| --- | --- |
| Higher-order foreign primitive defeats normalization | **Repaired.** Foreign operations are saturated, first-order, and transitively `Data`-only; `timeline_map` exposes callbacks. |
| Finite-domain `Warp` typed as total | **Repaired.** K₂ warps are total, eventually affine, and closed under composition and tail. |
| Ambiguous shifted feedback theorem | **Repaired by removal.** Block and frame ticks are distinct. The replacement validator has the new executable-interface failure above. |
| Rank-1/module elaboration asserted without a judgment | **Repaired by withdrawal.** Non-theorem 2.4 states the missing work and core totality is not transferred. |
| E₃ `before` not closed | **Repaired by removal.** E₃ is not primitive in K₂. |
| Broad strict positivity invalidates constructor-size proof | **Repaired.** First-order polynomial data gives finite constructor trees, subject to spelling out nested fold administration. |
| Big-step/closure and small-step/substitution cores disagreed | **Mostly repaired.** K₂ chooses substitution semantics, but administrative forms remain incomplete. |
| Port-row collision and coarse delay graph | **Partly repaired.** Tagged rows and port dependencies are right; `step_p` does not implement the fine schedule. |
| Primitive trust boundary understated | **Repaired for B3–B7.** `prepare_spec` still needs explicit ownership. |
| Static-context satisfiability omitted | **Repaired.** `Δ-Ok` includes Presburger satisfiability and solver negation is explicit. |
| E₃ existential witness absent from the type grammar | **Repaired by removal.** |
| Cache theorem lacked an invariant | **Repaired conditionally.** Theorem 9.2 assumes the exact versioned per-entry invariant; preservation remains implementation work. |
| Current R1 and content-only rendering conflated | **Partly repaired.** Content-only rendering is correctly a non-theorem, but semantic, structural, and derivation equality remain conflated at `nf_T`. |

## Clean passes

1. **The hostile K₁ primitive no longer exists.** The first-order, arrow-free foreign boundary is strong enough for
   Lemma 4.3. Callback evaluation in `timeline_map` is finite and visible. Subject to completing the administrative
   rules, I found no well-typed non-normalizing K₂ term.
2. **Fixed-expression conversion is decidable.** A satisfiable finite Presburger context, explicit monomorphic indices,
   nominal identity, finite rows, and no value terms in types give the claimed conversion boundary. K₂ does not claim
   existential index inference or source elaboration.
3. **The timeline algebra survives unchanged.** Closure, the sequence monoid, commutative unequal-extent overlay,
   pointwise map laws, synchronized interchange, multiplicity, and the absence of inserted rests are correct. Equal
   extent remains a theorem premise, not an overlay typing rule.
4. **The total-warp repair works.** A positive eventually affine rational ray is unbounded, so composition eventually
   reaches the second warp's final ray. Rational breakpoints have rational preimages under strictly increasing rational
   affine pieces. Composition, tail, overlay preservation, rebased sequence, and payload-map commutation are correct
   without coverage premises.
5. **Process causality survives denotationally after the interface repair.** Given executable per-output dependency
   functions and total next-state transitions, a finite acyclic instantaneous graph has one deterministic causal run at
   its named tick. K₂ correctly refuses to identify block-causal and frame-causal execution.
6. **The literal factorization and cache implications are honest conditionals.** Equal complete arguments to one pure
   deterministic function give equal results. Under the stated versioned `CacheInvariant` and complete collision check,
   a lookup result equals recomputation, including `Err`. Neither theorem proves content-only frame equality.
7. **The musical scope remains disciplined.** No proof assumes a global pitch, chord, key, meter, notation, work, or
   Western ontology. Domain theories remain optional libraries, and the finite/rational/causal scope is not presented as
   a definition of all music.

## Required repair order

1. Repair the processor primitive interface and re-prove Theorem 7.2 on an executable graph.
2. Define generated lineage and region/path composition, then re-prove Propositions 8.1–8.2.
3. Split semantic timeline normal form from the presentation/derivation value used by lineage-bearing preparation.
4. Complete static applications, administrative timeline/fold syntax, and the reducibility proof.
5. Assign `prepare_spec` to the checked or trusted boundary and prove concrete cache-invariant preservation.
