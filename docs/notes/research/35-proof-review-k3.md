# Proof review: K₃ stratified kernels

**Status: independent review of the frozen K₃ draft; governs nothing.** I reviewed
[34](34-candidate-k3-stratified-kernels.md) as fixed, with the K₁ and K₂ findings in [25](25-proof-review.md) and
[30](30-proof-review-k2.md) as regression obligations. I checked the transported elaboration claim against
`docs/rules/language/02-core-calculus.md`, the temporal claims against the governing kernel specification and
implementation, the R1 split against `docs/core-boundary.md` and the current `ScoreFact` key, and the implementation
qualification against the current audio scheduler and renderer.

The review standard is deliberately literal. A familiar theorem is not proved for syntax that the cited calculus does
not contain. A finite hash is not an equality decision procedure. Conversely, a conditional ownership theorem is not
rejected merely because its implementation has not yet been written, provided the stated contract really implies the
conclusion.

## Verdict

- **Decision**: **Incomplete, but no longer incorrect at the architectural center.**
- **Basis**: K₃ repairs all three load-bearing K₂ failures. Its conservative whole-node DAG actually executes the
  primitive interface; root/rule anchors make every lineage edge inhabited; and separating semantic execution from
  presentation lineage makes R1 substitution valid. I found no counterexample to T1, the structural part of T2, P1–P3,
  L2, R1-K₃, or C1 under their stated contracts.
- **Why not correct**: E1 is advertised for a K₃ᴱ grammar strictly larger than the governing calculus whose proof it
  transports. The displayed arbitrary sums and `Result` have no governing typing, reduction, or reducibility cases.
  Several smaller definitions also remain too ambiguous for a full local proof: exact equality versus hash equality,
  graph boundary ports, and set versus multiset lineage.
- **Severity**: no fatal error; one high proof-scope gap; four medium definition or specification gaps; two low
  qualifications.

## Findings

### Fatal

**None found.** In particular, the K₂ port-DAG counterexample is rejected by K₃'s node DAG and cannot be replayed
against P1.

### High

1. **E1 is not transported for the displayed K₃ᴱ language.**
   - **Location**: [34 §1](34-candidate-k3-stratified-kernels.md), Theorem E1, and
     `docs/rules/language/02-core-calculus.md` §§1 and 5.5–5.8.
   - **Type**: theorem-scope mismatch / missing compatibility cases.
   - **Problem**: K₃ says that K₃ᴱ *is* the existing governing calculus, but displays

     ```text
     A ::= scalar | A×B | A+B | Option A | List A | A→B | Music
     ```

     and says partial operations may return `Result`. The governing type grammar is

     ```text
     τ ::= b | unit | bool | nat | ratio | τ×τ | option τ | list τ | τ→τ | music.
     ```

     It has no arbitrary sum type and no `Result` type. Its reducibility proof has product, arrow, option, list,
     structural-eliminator, contextual-music, and inert-base cases, but no arbitrary-sum or `Result` case. The sentence
     that “standard rules for ... sums ... apply” does not supply syntax, values, evaluation contexts, reductions,
     preservation cases, or a reducibility interpretation.

     The exact witness to the scope failure is

     ```text
     case (inl true : bool + unit) of
       inl x => x
       inr _ => false.
     ```

     It is a term of the displayed K₃ᴱ grammar, but it is not a term quantified over by the cited governing totality
     theorem. Standard sums are strongly normalizing, so this is not a counterexample to the *intended* theorem; it is
     a counterexample to the claim that E1 is merely the already-proved theorem transported unchanged.
   - **Why it matters**: E1 is K₃'s total elaboration boundary. The hostile K₁ primitive has genuinely been removed, but
     the replacement proof must quantify over exactly one grammar.
   - **Repair**: the smallest repair is to delete `A+B` and `Result` from K₃ and use the governing `option` plus
     compiler diagnostics. If general finite sums are wanted, add their complete typing and CBV reduction rules, the sum
     reducibility candidate, and the cases for preservation, progress, determinism, and the fundamental lemma; define
     `Result A E` as that sum or prove it separately. Then E1 may explicitly cite that conservative extension.

### Medium

1. **Ordinary evaluation of `music` does not produce a temporal term.**
   - **Location**: [34 opening diagram and §1](34-candidate-k3-stratified-kernels.md) versus
     `docs/rules/language/02-core-calculus.md` §§3 and 5.7.
   - **Type**: stage-interface mismatch.
   - **Problem**: K₃ says that evaluating accepted source yields a finite checked temporal term. The governing dynamics
     explicitly say that music constructors evaluate to contextual `Music` values and do **not** call `instantiate`
     during ordinary evaluation. Only the separately premised `instantiate` and closure operation, supplied with a
     checked environment, placement, and an accepted resource preflight, returns a closed kernel term.

     A closed accepted note/music expression is therefore the direct witness: ordinary K₃ᴱ evaluation yields an opaque
     contextual recipe, not a `Term[ScoreFact]`. An accepted scalar such as `true` is an even simpler witness to the
     unqualified “accepted source” wording.
   - **Why it matters**: E1 itself concludes only “one finite value” and survives this issue, but the artifact diagram
     elides the actual typed bridge between K₃ᴱ and K₃ᵀ.
   - **Repair**: draw and type the existing bridge explicitly:

     ```text
     eval : ClosedExpr A → Value A
     instantiate_close : Music × Context × Placement × AcceptedBudget
                       → ClosedTerm ScoreFact.
     ```

     The second operation, not source evaluation alone, is the K₃ᴱ→K₃ᵀ pass.

2. **Hash equality cannot decide the admitted semantic equality.**
   - **Location**: [34 Theorem T2](34-candidate-k3-stratified-kernels.md), `docs/rules/events/05-normalization.md`
     N4–N6, and `crates/musa-kernel/src/hash.rs`.
   - **Type**: false converse if read as a decision procedure / equality conflation.
   - **Problem**: the flat canonical form decides N4 equality exactly. The 128-bit FNV-1a semantic hash does not. There
     are infinitely many canonical timelines—for example, empty timelines at different rational extents—and only `2^128`
     hash values. Hence, by the pigeonhole principle, there exist distinct canonical forms `M≢N` with `hash(M)=hash(N)`.
     The collision probability is an engineering bound, not a logical qualification to a decision theorem.
   - **Why it matters**: T2's structural termination and uniqueness are sound. Only the final equality claim is
     overstated, but it is exactly the distinction C1 later gets right by collision-checking complete arguments.
   - **Repair**: state

     ```text
     M ≡ N  iff  canonical(M) = canonical(N),
     M ≡ N  implies  hash(M) = hash(N).
     ```

     Use the hash only as a rejection/filter key; confirm a hit by canonical bytes or complete semantic arguments.

3. **P1 uses an open-graph interface that K₃ᴾ never defines.**
   - **Location**: [34 §4.1 and Theorems P1–P2](34-candidate-k3-stratified-kernels.md).
   - **Type**: missing formation and operational rules.
   - **Problem**: the only displayed edges connect `from.output` to `to.input` between nodes. Validation and execution
     then quantify over “every external input,” “external outputs,” and inputs whose source is external. No graph input
     row, graph output row, boundary edge, or rule relating boundary ticks and port types is defined. For a graph with
     one node whose sole required input is meant to be supplied externally, “every required input has exactly one edge”
     is currently either false or inexpressible.
   - **Why it matters**: the internal-node induction in P1 is correct. A self-contained theorem for an open process
     still needs the finite boundary object over which its input history and output history range.
   - **Repair**: define `Graph I O` with finite typed boundary rows and either

     ```text
     input(Graph.i, node.j)     output(node.k, Graph.o)
     ```

     edges or explicit input/output maps. Require one source for every node input and one typed driver for every graph
     output. Then the existing node-order induction proves P1 and P2 without change.

4. **L1 has not chosen between set and multiset lineage equality.**
   - **Location**: [34 §5 and Theorem L1](34-candidate-k3-stratified-kernels.md).
   - **Type**: inconsistent definition, although both possible repairs form categories.
   - **Problem**: composition is called a multirelation retaining paths “with multiplicity,” while “duplicate complete
     edges normalize.” Let `L:S→T` contain two byte-identical edges from `s` to `t` with the same trace. Bag semantics
     gives multiplicity two; duplicate normalization gives multiplicity one. They cannot both be the equality of one
     hom-object. The proof's appeal to enumeration “with multiplicity” therefore does not prove the normalized object as
     stated.
   - **Why it matters**: associativity is not endangered once a choice is made. Finite sets of fully traced edges form a
     category under relational composition and union; finite bags form one under multiplicity multiplication and
     addition. But cache keys, lineage equality, and whether two identical derivations remain distinguishable depend on
     the choice.
   - **Repair**: use a finite canonical **set** of complete traced edges and remove every multiplicity claim, or use a
     finite bag and retain counts rather than deduplicating. If distinct derivation instances with identical traces must
     survive, give them canonical derivation ids and include those ids in complete-edge equality.

### Low

1. **“Accepted” should retain the governing resource premise explicitly.** The source proof distinguishes mathematical
   normalization from the implementation's deterministic resource acceptance, and contextual closure requires that the
   meter accept the required output. E1 is sound if “accepted K₃ᴱ term” imports that definition; saying so would prevent
   readers from treating static typing alone as permission to publish an arbitrarily large value.

2. **The immediate block-tick recommendation needs a partial-chunk rule.** The draft honestly refuses to identify its
   explicit register semantics with current code, but “specify the current studio as a block-tick process” is not exact
   yet. `RenderPlan::render` executes chunks of

   ```text
   count = min(frames - written, configured_block_size).
   ```

   With configured block size 512, one call for 512 frames advances the implicit feedback boundary once, whereas four
   calls for 128 frames advance it four times. Thus current feedback is render-chunk-sensitive, not simply
   `Block(r,512)`-causal. The implementation step must either buffer until full nominal blocks, expose a
   `Chunk(r,n≤b)` tick including the caller partition in the semantics, or replace the implicit buffer boundary with
   the proposed explicit IR register. This is a qualification to step 4, not a refutation of P1–P3.

## Regression audit against K₁ and K₂

| Prior failure | K₃ result |
| --- | --- |
| K₁ higher-order opaque primitive defeats normalization | **Repaired.** K₃ transports the governing arrow-free δ boundary. The displayed sum/`Result` scope gap above is separate and routine. |
| K₁ finite-domain `Warp` typed as total | **Repaired.** K₃ retains the total eventually-affine representation and keeps it out of definitional equality. |
| K₁ shifted-feedback ambiguity | **Repaired.** The tick is named and block/frame processes are not definitionally equal. |
| K₁ rank-1/module elaboration claimed without a proof | **Repaired by exclusion.** Candidate T₂ remains outside E1. |
| K₁ non-total `before` constructor | **Repaired by removal.** No event-structure constructor is in K₃. |
| K₁ overly broad strict positivity | **Repaired by exclusion.** K₃ᴱ uses only the governing finite data; Candidate T₂ needs a later proof. |
| K₂ port-DAG cannot execute whole-node `step` | **Repaired.** K₃ validates a conservative whole-node DAG and waits for every node input. |
| K₂ source-less `Generated` lineage is uninhabited | **Repaired.** Every presentation has a root; generation names that root or a rule/site anchor. |
| K₂ structural, semantic, and lineage equality conflated in R1 | **Repaired.** `sem(M)` alone reaches execution; `pres(M)` reaches lineage separately. |
| K₂ cache theorem lacks complete-key collision confirmation | **Repaired conditionally.** C1 includes version, complete semantic arguments, insertion provenance, and collision checking. |

## Clean passes

1. **T1 is correct and remains the right temporal algebra.** `max` extent and finite multiset union make unequal overlay
   total, associative, and commutative without inserting rests. The equal-extent premise belongs only to synchronized
   interchange.
2. **The structural part of T2 is correct.** The term and binding structures are finite and acyclic, every constructor
   is a total finite operation, and the governing normalizer returns one flat canonical timeline. The current kernel
   suite passed: 58 tests across the crate's unit and integration targets.
3. **The exact warp laws remain sound.** Continuity, strict increase, rational piecewise-affine pieces, finitely many
   breakpoints, and a final positive affine ray give totality and closure under composition and tail. Keeping warps as
   typed private pass data avoids contaminating temporal equality.
4. **P1–P3 repair the exact K₂ operational failure.** Once graph boundary rows are supplied, the node DAG gives an
   executable whole-node schedule. Tick induction proves deterministic total execution; the same induction proves
   causality; deleting registers leaves a DAG, so every cycle crosses a register. K₃ does not pretend an extensional
   dependency declaration is executable code.
5. **K₃ is honest about the current studio.** Current scheduling removes dependencies on edges entering a delay node
   which lies on a cycle, and whole nodes process arena buffers in a stored order. There is no explicit registered-edge
   type in the prepared IR or canonical key. K₃ says “most closely,” calls the register implicit, and requires an IR/key
   change before claiming K₃ᴾ semantics. That distinction is correct; the partial-chunk caveat above makes it sharper.
6. **L2 is correct and the K₂ generation hole is closed.** Since even a presentation with no items has a root, every
   generated target has an ordinary source anchor. Total target coverage then composes by the stated two choices. L1
   also becomes correct immediately after choosing set or bag equality.
7. **R1-K₃ uses the right objects.** The current semantic key omits `Origin.definition_span` and `Origin.declaration`
   while retaining `source_span` and `expansion_path`. Sending only `sem(M)` to `prepare_execution` makes N4 equality a
   valid substitution premise; sending `pres(M)` separately allows honest lineage differences. Equal execution
   specifications plus fixed deterministic allocation, identical input histories, and total deterministic node
   transitions give equal frames by P1. K₃ correctly keeps those runtime premises explicit and correctly says the
   governing whole-plan wording needs amendment if “plan” includes lineage.
8. **C1 is a valid conditional cache theorem.** Same version, equality of collision-checked complete arguments, and the
   insertion invariant reduce a hit to substitution into the same deterministic function. It does not make the K₁
   mistake of proving correctness from a digest alone.

## Verified, judged, and not checked

### Verified

- I read the frozen K₃ draft, both preceding proof reviews, the governing elaboration grammar and metatheory, the
  temporal term/normalization laws, and governing R1.
- I inspected the current `ScoreFact::canonical_key`: it contains fact content, scope, source span, and expansion path,
  but not definition span or declaration id, exactly as K₃'s R2 example says.
- I inspected current audio scheduling and rendering: cyclic delay nodes cause incoming dependencies to be deferred;
  processors execute whole-node steps; the render loop uses caller-sensitive chunks no larger than the configured block.
- I ran `cargo test -p musa-kernel -q`; all 58 tests passed.

### Judged

- I judged theorem validity for the displayed calculi, the adequacy of the ownership hypotheses, the category law under
  each possible lineage equality, and whether the K₁/K₂ counterexamples transport.
- The severity classifications are mathematical/design judgments. In particular, I call K₃ **Incomplete** rather than
  **Incorrect** because every unrepaired theorem gap has a local repair and the process, lineage-generation, and R1
  conclusions survive.

### Not checked

- This is not a mechanized proof. I did not verify implementations of the proposed explicit-register IR, root/rule
  anchor encoding, `prepare_execution`, `prepare_lineage`, versioned cache, or Candidate T₂ because they do not yet
  exist as K₃ specifies them.
- I did not test floating-point reproducibility across targets, device callbacks, or allocation strategies; R1-K₃
  correctly makes those separate conformance premises.
- I did not treat later or prospective process documents as repairs to frozen K₃. The verdict is on
  [34](34-candidate-k3-stratified-kernels.md) itself.

## Recommendation

K₃ is worth advancing, but not by declaring the proof closed. Make four small specification repairs first: restrict K₃ᴱ
to the actually governed grammar (or prove sums), name `instantiate_close` as the K₃ᴱ→K₃ᵀ pass, define open graph
boundary rows, and choose set or bag lineage equality. Correct T2 to separate canonical equality from hashing. Then
prototype the explicit registered process IR with a test that renders the same feedback graph under different call and
block partitions. If that test is intentionally unequal, put the partition in the process tick and semantic key; if
partition independence is the goal, the current implicit block boundary must be replaced rather than documented as a
fixed `Block(r,b)` register.

Those repairs are materially smaller than another kernel redesign. The central stratification—total elaboration, finite
temporal placement, explicit causal process execution, and finite traced passes—is the first candidate in this sequence
whose core theorems match the objects they are about.
