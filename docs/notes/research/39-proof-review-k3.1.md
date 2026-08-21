# Proof review: K₃.1 closure

**Status: independent review of the frozen K₃.1 corrigendum; governs nothing.** I reviewed [38](38-k3.1-closure.md)
against the frozen K₃ candidate [34](34-candidate-k3-stratified-kernels.md), its review [35](35-proof-review-k3.md), the
governing source and temporal specifications, and the current kernel and audio code. The question was closure: whether
K₃.1 repairs the exact gaps in review 35 without silently changing semantic equality, R1, or cache correctness.

## Verdict

- **Decision**: **Incorrect.**
- **Basis**: E1.1, B1, the open whole-node process construction, and the caller-chunk qualification survive. But T2.1
  identifies semantic equality with current N5 bytes, and those bytes are not injective on canonical timelines. There is
  an exact pre-hash collision in the implemented generic kernel. Consequently T2.1 is false, and using those bytes as
  “complete canonical arguments” would invalidate C1 and strengthen R1 to a coarser, unintended equality.
- **Second failure**: the finite-set choice repairs duplicate multiplicity, but L1.1 still does not preserve the
  generated-anchor formation condition under composition. A flat trace drops the intermediate root/rule anchor that
  justified generation.
- **Severity**: two errors fatal to the claimed closure, two low statement qualifications, and no defect found in the
  process construction itself.

## Findings

### Fatal to the fixed closure proof

1. **Current N5 bytes are not a complete canonical form.**
   - **Location**: [38 §3 and Theorem T2.1](38-k3.1-closure.md), `docs/rules/events/05-normalization.md` N4–N6,
     `crates/musa-kernel/src/timeline.rs::write_canonical`, and `crates/musa-kernel/src/occurrence.rs`.
   - **Type**: false theorem / encoding–semantic-equality mismatch.
   - **Problem**: `write_canonical` inserts `Canonical::canonical_key()` directly between unescaped textual delimiters:

     ```text
     occurrence <payload-key> from <start> to <end>;
     ```

     The `Canonical` contract requires a payload key to be deterministic, total, and injective as a key; it does not
     forbid newlines or delimiter text. The current generic kernel even implements `Canonical for String` by returning
     the string unchanged. Therefore record boundaries are not recoverable from N5 bytes.

     Here is an exact counterexample using valid `Timeline<String>` values. Let

     ```text
     M = timeline 2 {
           occurrence "a from 0 to 1;\n  occurrence b" from 1 to 2;
         }

     N = timeline 2 {
           occurrence "a" from 0 to 1;
           occurrence "b" from 1 to 2;
         }.
     ```

     The quotation marks above delimit mathematical strings; they are not emitted by N5. Both timelines serialize to
     exactly

     ```text
     timeline 2 {
       occurrence a from 0 to 1;
       occurrence b from 1 to 2;
     }
     ```

     Yet current `Timeline::semantic_eq` returns false: `M` has one canonical occurrence at `[1,2]` with one payload
     key, while `N` has two occurrences at `[0,1]` and `[1,2]`. Multiplicity and spans differ. The individual string
     keys are injective, so the counterexample does not violate N3's payload-owner premise.
   - **Consequences**:
     - `can(M)=can(N)` but `M≢N` under governing structural N4, refuting T2.1's claimed equivalence.
     - `h(M)=h(N)` before FNV-1a has any opportunity to collide. This is a serialization collision, not the accepted
       finite-hash probability.
     - If `≡_T` is redefined by these bytes, it is strictly coarser than current N4. R1 would then require equal
       execution for some timelines the governing semantics distinguishes. That is an accidental strengthening despite
       §1's “no theorem strengthened” promise.
     - If C1 confirms a cache hit by equality of these N5 bytes, the “complete semantic arguments” premise is false: an
       entry for `M` may be returned for `N`.
   - **Repair**: keep exact semantic equality as structured comparison of

     ```text
     (extent, [(start,end,payload-key-bytes), ...]).
     ```

     Define `can` to be that structure, not current display text. If a byte encoding is required, make it injective by
     length-prefixing every variable byte string (and using a fixed rational encoding) or by rigorously escaping and
     delimiting payload keys. Change `write_canonical` and N5 accordingly, and add the counterexample above as a
     regression. C1 must compare the structured arguments or the repaired injective encoding after its hash lookup.

2. **Flat set-valued traces are not closed under the generated-anchor formation rule.**
   - **Location**: [38 §6 and Theorem L1.1](38-k3.1-closure.md), inheriting the step fields and formation rule from
     [34 §5](34-candidate-k3-stratified-kernels.md).
   - **Type**: false category-closure claim / lost intermediate witness.
   - **Problem**: an edge stores only its outer source, outer target, and `List Step`. A `Generated` primitive edge must
     name the source presentation's root or a rule/site anchor. Relational composition removes the intermediate anchor,
     while a `Step` as defined in K₃ stores only pass, role, and canonical evidence—not the typed source and target
     anchors of that hop.

     Take presentations with anchors

     ```text
     Anchors(S) = {root_S, s}
     Anchors(T) = {root_T}
     Anchors(U) = {root_U, u}.
     ```

     Let the well-formed lineages contain

     ```text
     L : S→T = {(s, root_T, [Preserved])}
     M : T→U = {(root_T, u, [Generated])}
     ```

     together with any edges needed to cover `root_U`. The generated edge in `M` is valid because its source is
     `root_T`. The displayed composition produces

     ```text
     (s, u, [Preserved, Generated]).
     ```

     Its outer source `s` is neither a root nor a rule/site anchor. If the generated-anchor condition is checked against
     the composite edge's source, the composite is not a lineage, so the alleged hom-sets are not closed under
     composition. If the condition is meant to refer to the intermediate `root_T`, the flat composed edge no longer
     contains that anchor or even the intermediate presentation needed to validate it. Exact set deduplication does not
     repair this loss.
   - **Why it matters**: L1.1 proves associativity of endpoint/trace set comprehension, but not that the result
     satisfies all lineage formation rules. The point of root/rule anchors was to make generation inspectable rather
     than merely label it `Generated`.
   - **Repair**: make a trace a typed path of primitive hops, each retaining its source presentation/anchor, target
     presentation/anchor, pass, role, and evidence. Composition concatenates paths; generation's root/rule witness then
     survives literally. A globally qualified, canonically encoded generation-site reference inside every generated step
     could also work, but the formation and lookup rule must be explicit. Re-prove closure before associativity and
     identity. L2.1 then applies to the outer endpoints exactly as written.

### Low

1. **P2.1 should state that initial states and process parameters are the same in the two executions.** Its proof says
   “initial states are fixed,” and this is the intended causal-function reading. The theorem statement mentions only
   agreement of input histories. Without the fixed-state premise, a registered boundary delay with different initial
   register values has different tick-zero outputs under identical inputs. Write “for a fixed accepted process,
   parameters, seeds, and node/register initial state.” The process construction itself is sound.

2. **“Resource preflight” is broader than the governing implementation terminology.** The governing meter charges nested
   evaluator work as operations are reached, although aggregate output is preflighted before its allocation and no
   partial value is published. E1.1 is correct if “resource preflight accepted” means the complete deterministic
   resource-acceptance judgment for that run. It should not imply that every nested charge is statically predicted in
   one pass before evaluation.

## Claims that survived

### E1.1 — exact source totality

E1.1 now quantifies over exactly the governing grammar: base values, unit, booleans, naturals, rationals, products,
options, lists, arrows, and contextual `Music`. General sums, `Result`, Candidate T₂a nominal data, recursion, and
higher-order foreign operations are absent. The existing option/list/match and structural-eliminator compatibility cases
and the D1–D4 foreign boundary cover the displayed terms. Subject to the resource wording above, this is a legitimate
transport of the governing deterministic-total-evaluation theorem rather than a proof by analogy.

### B1 — accepted instantiate/close bridge

The bridge is correctly a partial **compiler judgment**, not a partial K₃ᴱ term operation. Ordinary evaluation returns
`ContextualMusic`; checked context, placement, resource acceptance, instantiation, and closure produce the closed
temporal term. The governing contextual-music theorem supplies finite occurrence accounting and a finite acyclic binding
environment; the temporal evaluator supplies the unique normalized timeline. Under the stated deterministic environment,
placement, marking, binding completion order, and traversal, B1's output is unique. No other source value is forced
through this bridge.

### P1.1–P3.1 — open whole-node processes

I found no accepted graph lacking a current-tick value. Every node input is driven by exactly one of a boundary input,
an old register value, or an instantaneous predecessor output. The node dependency DAG orders the last case; whole-node
totality handles all outputs at once. Every register source is either a boundary input known at tick start or a node
output known by the end of the node order, so atomic commit cannot affect its current-tick sink.

The boundary-only edge cases behave correctly:

- `wire(In(i),Out(o))` is current-tick identity/permutation;
- `reg(r,a₀,In(i),Out(o))` outputs `a₀` at tick zero and input `i_n` at tick `n+1`;
- a register into a node supplies the old value before the node runs; and
- a register from a node to a boundary output exposes the old value and commits the node's current output only for the
  next tick.

Boundary outputs cannot source edges, and boundary inputs cannot be sinks, so they cannot smuggle an instantaneous cycle
around the node DAG. Hence P1.1 and P3.1 hold, and P2.1 holds with the fixed-state premise made explicit.

### Current audio qualification

Section 5 correctly withdraws the false fixed-`Block(r,b)` shorthand. Current `RenderPlan::render` uses
`min(frames-written, configured_block_size)`, so four 128-frame calls and one 512-frame call advance the implicit cyclic
delay boundary differently. The three proposed routes distinguish fixed internal ticks, partition-keyed semantics, and
frame-causal execution honestly. No proposed K₃ᴾ semantics is claimed to be implemented already.

### Set deduplication and target coverage

Choosing finite sets fixes review 35's bag-versus-set contradiction. Exact deduplication itself preserves associativity
of set comprehension and cannot remove the last representative needed for L2.1. Target coverage therefore composes on
the underlying endpoint relation. What fails is preservation of generated-step formation evidence, not set algebra or
the L2 choice argument.

### R1 and C1, conditionally unchanged

K₃.1 does not directly add a content-only rendering law, lineage equality, device conformance, or cache correctness from
a digest alone. The frozen K₃ R1 proof remains valid when `sem(M)` means governing structured N4 equality and when
runtime allocation, inputs, and processor transitions retain their explicit deterministic premises. C1 remains valid
when a hit compares genuinely complete structured semantic arguments at the same implementation version.

They do **not** survive replacement of N4 by equality of current N5 bytes. That replacement is the accidental
strengthening/weakening identified in the fatal finding, and must be removed before K₃.1 can claim closure.

## Verified, judged, and not checked

### Verified

- I read the complete frozen K₃.1 note, the K₃ candidate and prior review, the governing source grammar and §§4, 5.6–5.8
  proofs, and kernel N4–N6.
- I inspected the current generic implementation. `Canonical for String` returns its contents unchanged;
  `Timeline::semantic_eq` compares the structured extent and canonical occurrence keys; `write_canonical` interpolates
  payload keys without framing; and `semantic_hash` hashes exactly those bytes. The N5 counterexample follows from those
  definitions without assuming an FNV collision.
- I inspected current audio scheduling/render chunking for the block-partition qualification.
- I ran `cargo test -p musa-kernel -q`; all 58 tests passed. The suite does not contain the serialization-injectivity
  counterexample above.

### Judged

- I checked each theorem against the exact displayed syntax/data rather than an intended richer implementation. I judged
  E1.1 and B1 under their imported ownership contracts, enumerated every source/sink form in the process graph, and
  checked lineage formation as well as endpoint relation algebra.
- I classify the result as **Incorrect**, not merely Incomplete, because T2.1 has an exact counterexample and L1.1's
  claimed category is not closed under the stated formation rule.

### Not checked

- This is not a mechanized proof. The proposed process IR, typed lineage paths, preparation split, and versioned cache
  do not yet exist in K₃.1 form, so I did not implementation-verify them.
- I did not check cross-target floating-point reproduction or host callback behavior beyond the current deterministic
  chunk loop. Those remain explicit R1 conformance premises.
- I did not treat any later draft as a repair to frozen [38](38-k3.1-closure.md).

## Recommendation

Do not reopen the stratified architecture. Repair two representations:

1. make canonical semantic comparison structured, and make any stored byte encoding injective by construction; and
2. make lineage traces typed paths which retain every intermediate anchor, especially the root/rule witness for a
   generated hop.

Then add the N5 injection regression and the two-hop generated-lineage counterexample to the law suites. The open
whole-node process definition is ready for a narrow prototype once P2.1 names its fixed initial state. E1.1 and B1 need
no new language mechanism.
