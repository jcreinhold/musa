# Candidate K₃ — three small kernels and one coherence discipline

**Status: fixed candidate and proof draft for independent review; governs nothing.** K₂ remains incorrect as frozen;
[30](30-proof-review-k2.md) gives the counterexamples. K₃ does not repair it by adding more machinery. It deletes the
unsupported premise that `Timeline`, `Warp`, and `Process` must all be value types of one source calculus.

K₃ is a stratified design:

```text
total elaboration core
    ↓ evaluates to
finite temporal term/value
    ↓ interpreted with profiles and exact clock maps
finite process plan
    ↓ allocated and executed at a named tick
causal runtime history
```

Typed passes with finite lineage and loss evidence join the stages. Each kernel has only the compositions and equality
its denotation supports.

This is close to Musa's implemented architecture. The major proposed source change is Candidate T₂'s nominal first-order
data and abstract type members, and even that remains prototype-first. The immediate architectural changes are to
lineage and process contracts, not to the temporal kernel.

---

## 1. K₃ᴱ — total elaboration

K₃ᴱ is the existing monomorphic, pure, call-by-value calculus of `docs/rules/language/02-core-calculus.md`:

```text
A ::= scalar | A×B | A+B | Option A | List A | A→B | Music
e ::= x | c | fn(x:A){e} | e(e) | let x=e in e
    | constructors and exhaustive match
    | finite structural eliminators
    | saturated first-order δ-operations
    | controlled Music operations
```

The load-bearing boundary already matches K₂'s successful repair:

- a δ-operation has only inert first-order argument and result types, with no arrow anywhere;
- partiality returns `Option`/`Result` rather than getting stuck;
- higher-order `map`, folds, filtering, ranges, and repetition are separately specified structural eliminators; and
- named value dependencies are finite and acyclic.

K₃ᴱ does **not** contain `Timeline`, `Warp`, `Process`, `Signal`, structure, syntax, or DSP-node values. `Music` is an
opaque finite contextual recipe; evaluating accepted source yields a finite checked temporal term. The source cannot
inspect the temporal normal form and feed that observation back into elaboration.

Candidate T₂ may conservatively add first-order polynomial nominal data. It must not add general recursion, higher-order
foreign operations, arbitrary quotient equality, first-class structures, or value-dependent type conversion.

### Theorem E1 — conditional totality boundary

Assume the exact structural-eliminator and contextual-music compatibility lemmas in the governing calculus, and assume
each foreign δ-operation satisfies its stated totality, purity, typing, and finiteness contracts. Then every accepted
closed K₃ᴱ term evaluates to one finite value.

*Proof.* This is the governing reducibility proof's theorem, not a new proof by analogy. The arrow-free δ restriction
closes the hostile higher-order primitive counterexample from [25](25-proof-review.md). Structural eliminators decrease
their finite constructor measure and apply callbacks already in the arrow reducibility candidate. Acyclic named
definitions elaborate to a finite nest of non-recursive lets. Preservation and progress give a typed value; reduction
determinism gives uniqueness. ∎

**Proof boundary.** K₃ does not claim Candidate T₂'s module/abstract-type elaboration is already covered. Its path,
sealing, generativity, and preservation theorem remain required before implementation.

## 2. K₃ᵀ — finite temporal placement

K₃ᵀ is the existing kernel term calculus and denotation, unchanged:

```text
t ::= timeline d {(s,e,a)*}
    | seq(t̄) | over(t̄) | scale(r,t) | restrict(I,t)
    | let x=t in t | x | x@mark

⟦t⟧ = (d,E)
```

`d,s,e∈ℚ≥0`; `E` is a finite multiset; every occurrence lies in `[0,d]`. Sequence translates the second operand by the
first extent. Overlay takes `max` extent and multiset union. Marks select a consumer-owned total payload map without
making arbitrary functions term syntax.

K₃ᵀ has no lambdas, recursion, process wiring, signal values, extent indices, clock-plan types, or global musical
payload. Its parametric payload owner supplies canonical equality and serialization under an admission record.

### Theorem T1 — unequal overlay is the natural total operation

For any two well-formed timelines of the same payload type, `M⊕N` is well formed, associative, and commutative. It needs
no equal-extent premise and inserts no payload.

*Proof.* For `M=(d,E)` and `N=(e,F)`, define `M⊕N=(max(d,e),E⊎F)`. Every old occurrence remains inside the maximum
extent. Associativity and commutativity follow from `max` and multiset union. ∎

Equal extent remains exactly where it belongs: as a premise of synchronized interchange, not of overlay formation.

### Theorem T2 — temporal normalization is total

Every closed well-formed K₃ᵀ term evaluates to one finite flat timeline, and its canonical form decides the admitted
semantic equality up to the documented hash collision bound.

*Proof.* Structural recursion over the finite acyclic term and finite binding environment. Every constructor invokes a
total operation on finite timelines. The governing N1–N6 normalization and T1–T4 proofs establish determinism and the
canonical comparison. ∎

## 3. Exact clock maps are a pass structure, not a term former

A performance pass may use a total exact warp

```text
w : ℚ≥0 → ℚ≥0
```

represented as a continuous strictly increasing rational piecewise-affine function with finitely many breakpoints and a
final positive affine ray. Composition and tail are closed, and endpoint action obeys the laws proved in
[29 §6](29-metatheory-of-k2.md), one of K₂'s clean passes.

`Warp` is a private exact performance value unless two source callers justify exposing it. It is not part of temporal
definitional equality. A finite observed tempo curve needs an explicit final extension policy before becoming total.
Ametric or interaction time need not be forced through a beat warp; a pass may instead consume a nominal clock and
produce timed observations with explicit approximation.

Frame quantization returns a result containing the quantized plan, rounding/collision decisions, errors, and lineage. It
is a real pass and cannot erase as static evidence.

## 4. K₃ᴾ — whole-tick process graphs with explicit registers

K₃ᴾ is a finite typed graph, not a source function calculus. Fix a nominal discrete tick `k`. Every primitive node `p`
supplies:

```text
I_p, O_p                        finite typed port rows
S_p                             finite private runtime-state layout
init_p : Params_p × Seed → S_p
step_p : S_p × Frame(I_p) → S_p × Frame(O_p)
```

`init_p` and `step_p` are total deterministic ownership contracts. State is local to one node instance.

A graph edge either is instantaneous or is an explicit one-tick register:

```text
e ::= wire(from.output,to.input)
    | reg(initial,from.output,to.input)
```

Rows require distinct labels; parallel graph composition uses tagged disjoint union and explicit rename/permute maps.
Every required input has exactly one edge. Types of connected ports are equal at the named tick.

### 4.1 Validation and execution

Construct the **node dependency graph** containing `p→q` for every instantaneous wire from an output of `p` to an input
of `q`. Registered edges add no current-tick dependency. `close` accepts iff this finite graph is acyclic and every port
contract is satisfied. It stores one deterministic topological node order, choosing the least stable node id among
currently available nodes.

At tick `n`:

1. read each registered edge's current stored value and every external input;
2. invoke each whole-node `step_p` once in stored node order after **all** its inputs are available;
3. expose external outputs; and
4. commit the source outputs of registered edges as their values for tick `n+1`, together with node next states.

This interface intentionally rejects the more permissive port-DAG example from [30](30-proof-review-k2.md): even if an
output mathematically ignores one input, a whole-node step is scheduled only after every input is present. A finer
acceptance rule first needs executable per-output functions as specified in [33](33-studio-feedback-semantics.md), not
merely a dependency claim.

### Theorem P1 — accepted execution is total and deterministic

For fixed external input history, parameters, seeds, and initial register values, every accepted K₃ᴾ graph has exactly
one state and output at every finite tick.

*Proof.* The instantaneous node graph is a finite DAG, so its stored topological order contains every node after every
instantaneous predecessor. Induct on ticks. At tick zero, external and registered inputs and node states are fixed. Now
induct on the node order: every input of the current node is either external, a register value read at the start, or an
instantaneous predecessor output already computed. Total deterministic `step_p` therefore gives one output and next
state. After the finite order, register and node next states are uniquely committed. The same argument advances every
tick. ∎

### Theorem P2 — accepted execution is causal

If two external input histories agree through tick `n`, the graph's external outputs agree through tick `n`.

*Proof.* Simultaneous induction on ticks and, within each tick, on the stored node order. At tick zero the fixed initial
states and equal external inputs give equal node outputs. If executions agree through tick `m−1`, their node and
register states at `m` agree. Equal external inputs at `m` and node-order induction then give equal current outputs and
next states. ∎

### Corollary P3 — every accepted feedback cycle crosses an explicit tick

Every directed graph cycle contains a `reg` edge.

*Proof.* Removing registered edges leaves the accepted instantaneous DAG. A cycle without `reg` would remain a cycle in
that DAG. ∎

K₃ᴾ is parametric in tick. A block-register process has tick `Block(r,b)` and may depend sonically on `b`. A frame
process has tick `Frame(r)`. They are not definitionally equal. Current Musa feedback most closely implements an
**implicit** block register on edges entering a delay node; K₃ requires that register to become explicit in the prepared
IR and canonical key before claiming this semantics.

Long term, Musa should prefer the frame-causal per-output design in [33](33-studio-feedback-semantics.md) if authored
feedback must be independent of callback partition. That design is an extension of K₃ᴾ's primitive contract, not a
reinterpretation of P1.

## 5. K₃ᴸ — lineage with no source-less edge

Every finite presentation `S` has a finite anchor view with a distinguished root:

```text
Anchors(S) = { root_S } ∪ item anchors ∪ rule/site anchors.
```

The root represents the presentation or declaration site as a whole. A generated default therefore still has a real
source: the root or explicit rule/site anchor responsible for generating it.

A lineage edge is exactly:

```text
DerivationEdge S T = {
    source : AnchorId_S,
    target : AnchorId_T,
    trace  : List DerivationStep,
}

DerivationStep = {
    pass : PassId,
    role : Preserved | Split | Merged | Generated | Approximated | Selected,
    evidence : CanonicalEvidence,
}
```

The identity edge from anchor `a` to itself has an empty trace. A primitive derived edge has a one-step trace.
Composition matches the intermediate anchor, retains every resulting path as a multirelation edge, and concatenates
traces. Duplicate complete edges normalize; parallel derivations remain distinct. Region information belongs in typed
anchor facts or canonical step evidence; K₃ does not claim an unspecified `RegionMap` composition.

Formation requires:

1. every source and target id exists;
2. every target has at least one incoming edge;
3. a `Generated` step points to the root or a named rule/site anchor, never to nowhere; and
4. ids, steps, and evidence have canonical finite encodings.

### Theorem L1 — lineage is a category

Finite typed lineages with the composition above are associative and have identity lineages as units.

*Proof.* Underlying finite multirelation composition is associative: both groupings enumerate the same composable paths
through the two intermediate anchor sets, with multiplicity. Trace concatenation is associative. Canonical sorting and
duplicate normalization depend on the complete resulting edge, not grouping. Identity relation supplies the same
endpoint with an empty trace, and list concatenation with empty is a unit. ∎

### Theorem L2 — total target coverage composes

If every target anchor of `L:S→T` has an incoming edge and every target anchor of `M:T→U` has an incoming edge, then
every target anchor of `M∘L:S→U` has an incoming edge.

*Proof.* Take `u∈Anchors(U)`. Choose an edge `t→u` from totality of `M`; choose an edge `s→t` from totality of `L`.
Their composite is an edge `s→u`. Generated targets cause no exceptional case because their edge names a root or rule
anchor in the ordinary source anchor set. ∎

This representation handles a pass-introduced default honestly: its lineage leads to the pass invocation/root and the
step evidence names the default rule. It avoids the uninhabited source-less `DerivationEdge` refuted in [30].

## 6. Preparation has two results and two equalities

Let `sem(M)` be the canonical-key semantic normal form governed by kernel N3–N4. It may quotient away payload fields.
Let `pres(M)` be a canonical **presentation** containing full anchor and derivation data required for lineage.

Preparation returns:

```text
PreparedArtifact = {
    execution : PreparedSpec,
    lineage   : Lineage Gesture Plan,
    losses    : List RealizationLoss,
}
```

with conceptual factorization:

```text
prepare_execution : sem(M) × Bindings × Seed × Options
                  → Result PreparedSpec PrepareError

prepare_lineage   : pres(M) × PreparedSpec
                  → Lineage Gesture Plan × List RealizationLoss.
```

`prepare_execution` is either a checked total function or an explicitly trusted pure deterministic implementation
contract. `prepare_lineage` may observe presentation facts which semantic equality omits, but those facts never affect
the execution value.

### Theorem R1-K₃ — rendering is a function of semantic meaning

If `M≡N` under governing semantic equality and bindings, seed, and options are equal, then successful
`prepare_execution` results are equal; under fixed deterministic runtime inputs they render equal frames.

*Proof.* `M≡N` is equality of `sem(M)` and `sem(N)`. The remaining arguments are equal, so purity and determinism of
`prepare_execution` give equal `Result` values. Equal execution specifications, fixed allocation semantics, identical
input histories, and deterministic processor transitions give identical frames by P1. These runtime premises remain
explicit; source factorization alone does not prove floating-point or device conformance. ∎

### Non-theorem R2 — lineage equality

`M≡N` need not imply `prepare(M).lineage = prepare(N).lineage`. Current `ScoreFact` semantic keys omit `definition_span`
and declaration id while the full payload retains them. A lineage-bearing wrapper may and should distinguish those
presentations without violating R1-K₃.

The governing `core-boundary.md` currently writes equality of the whole “prepared plan.” If future prepared-plan
equality includes lineage, that sentence must be amended to name the `execution` projection. This is a clarification
forced by the current N3 quotient, not permission for lineage to affect sound.

### Theorem C1 — versioned cache correctness

Suppose a cache entry stores `(version,complete_semantic_args,result)` and insertion occurs only by evaluating the
current `prepare_execution` on those arguments. A collision-checked hit at the same version returns the same `Result` as
recomputation.

*Proof.* By the insertion invariant, the stored result equals the versioned function applied to its stored arguments.
Collision checking establishes equality of the complete arguments, and version equality selects the same function.
Substitution yields the recomputed result. Invalidation deletes entries and preserves the invariant; a version change
cannot hit an old entry. ∎

Lineage may be recomputed from `pres(M)` on a cached execution hit or cached separately under its stronger presentation
key.

## 7. The coherent artifact

K₃'s stages are joined by the artifact diagram in [32](32-the-coherent-object-is-the-diagram.md):

```text
source presentation
  ── K₃ᴱ ──▶ temporal presentation
  ── performance pass ──▶ gesture presentation
  ── preparation ──▶ execution spec + lineage
  ── allocation ──▶ runtime process state
```

Every finite pass returns a target, K₃ᴸ lineage, and explicit losses. Runtime histories are denotations of the process
node, observed through K₃ᴾ. The diagram, not a common carrier, is the coherent object.

## 8. What is minimal and what is optional

| Structure | Status | Reason |
| --- | --- | --- |
| Existing total elaboration calculus | retain | already supplies higher-order finite definition language |
| Existing finite temporal kernel | retain unchanged | laws and unequal overlay survived both reviews |
| Whole-tick registered process IR | specify | exact causal operational boundary missing today |
| Lineage-bearing pass result | add | required for notation↔performance↔plan inspectability |
| Total exact warp | private pass structure first | useful laws; no need for source syntax yet |
| Nominal data + abstract type members | prototype, then likely add | smallest path to plural theory carriers |
| Natural-number dependent indices | defer | no current core caller beyond ordinary finite container lengths |
| CBPV/world/link/profunctor syntax | reject for now | no caller survives the simpler staging/modules/passes account |
| Event structures | defer | proposed prime constructor refuted; current mobile form has bespoke finite data |

## 9. Immediate recommendation

1. **Do not index the temporal kernel by extent. Do not restrict overlay. Do not add padding.**
2. Treat the current source calculus and temporal kernel as two intentionally different stages, not fragments awaiting
   unification.
3. Repair prompts 156–158 so score→gesture→prepared execution carries finite lineage and explicit loss, while R1 names
   execution equality separately from lineage equality.
4. Specify the current studio as a block-tick process with an explicit register in every admitted cycle; add the
   block-size differential test. Prototype frame-causal SCC execution before promising block-partition independence for
   feedback.
5. Prototype Candidate T₂ with one Western package and one phrase/gesture-primary package. Only then write its source
   elaboration proof and implementation prompts.
6. Do not rewrite all governing architecture around K₃. Amend only the exact R1, lineage, and feedback claims which are
   already contradicted or underspecified after the independent review passes.

This is less spectacular than a musical motive. It is also a smaller language with cleaner rules and a direct path to a
notation/audio workbench whose views are genuinely related.
