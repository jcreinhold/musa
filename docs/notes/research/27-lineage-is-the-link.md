# Lineage is the link the artifact actually needs

**Status: corrective candidate; governs nothing.** [20](20-candidate-staged-algebras.md) said that typed definitions and
the source definition graph make notation, performance, and audio one coherent artifact. That is incomplete. A function
type proves that a whole target value was computed from a whole source value. It does not tell an editor which target
event came from which source event.

Musa already depends on the finer relation:

- each `ScoreFact` carries `Origin` with source/definition spans, declaration identity, and expansion path;
- motif applications and repeats retain both definition and call/iteration provenance;
- score projections preserve event identity for selection and Origin view;
- `performance.rs` copies score-event origin into performance events;
- the interface specification expects every displayed note to trace to source.

The notation/audio problem therefore needs two different structures:

1. a typed pass says which *values* can be transformed;
2. finite lineage says which *identified parts* of the output derive from which parts of the input.

This is a concrete reason for a link. It is smaller than a general profunctor and more informative than an ordinary
function.

---

## 1. Anchored presentations

A value can participate in lineage only through a finite **anchor view**:

```text
Anchors A = finite map AnchorId_A ↦ AnchorFact_A
```

An anchor is a stable identity for a selectable or diagnosable part of a finite presentation. Examples:

- a written occurrence;
- a phrase or region;
- a performance gesture;
- a processor node or parameter binding;
- a scheduled render event or frame interval;
- an analysis claim.

Not every scalar or byte receives an anchor. A 10-minute render must not allocate a provenance node per sample. The
prepared plan anchors scheduled intervals and processor contributions; samples are observed through those ranges.

An anchor type is nominal. `ScoreAnchor`, `GestureAnchor`, `PlanAnchor`, and `AnalysisAnchor` cannot be confused merely
because their runtime IDs share a representation.

## 2. Finite lineage

For source presentation `S` and target presentation `T`, define

```text
Lineage S T = finite multirelation
    AnchorId_T ↦ NonEmpty (DerivationEdge S T)

DerivationEdge S T = {
    source : AnchorId_S,
    role   : DerivationRole,
    reason : OriginStep,
    region : Option RegionMap
}
```

`DerivationRole` is a small structural vocabulary, not musical analysis:

```text
Preserved | Split | Merged | Generated | Approximated | Selected
```

- **Preserved:** one conceptual item survives a representation change.
- **Split:** one source contributes several target anchors.
- **Merged:** several sources contribute one target anchor.
- **Generated:** a target is introduced by an explicit rule or default; the edge names that rule/site.
- **Approximated:** the map records a lossy conversion, such as quantization.
- **Selected:** a target realizes one alternative from a source freedom.

These names say how identities relate, not why a chord is dominant or a phrase is a prayoga. Domain packages may attach
typed evidence beside them.

## 3. Formation obligations

A lineage value is accepted only if:

1. every source and target ID exists in the corresponding anchor view;
2. every target anchor has at least one incoming edge or one explicit `Generated` edge naming a source site/rule;
3. duplicate edges normalize by their full canonical data;
4. any `RegionMap` lies inside both anchors' declared regions;
5. identities and origin steps serialize canonically; and
6. no edge points from a later compiler stage back to mutable runtime state.

Lineage is finite even when the target process denotes an unbounded signal. It describes the finite plan and its
interfaces, not every future observation.

## 4. Identity and composition

Identity lineage maps every anchor to itself with role `Preserved`.

Given `L : Lineage S T` and `M : Lineage T U`, composition is relational path composition:

```text
(M ∘ L)(u) = {
    compose_edge(s→t, t→u)
    | t→u ∈ M(u), s→t ∈ L(t)
}.
```

The composed reason retains the finite path of origin steps. Roles compose by a small conservative table: any lossy edge
makes the composite `Approximated`; a split followed by a merge remains a path carrying both steps rather than being
guessed back to `Preserved`; generated targets remain generated.

### Proposition L1 — associativity

Lineage composition is associative up to canonical association of reason paths.

*Argument.* Underlying relation composition is associative. Reason paths compose by list concatenation, which is
associative. Canonical role summarization is defined as a fold over the full path, so it is independent of grouping. ∎

### Proposition L2 — identity

Identity lineage is a two-sided unit.

*Argument.* Relational identity adds no intermediate alternatives; the empty reason-path contribution and `Preserved`
role leave each edge unchanged. ∎

Lineages therefore form an ordinary category of anchored finite presentations. This is the modest categorical structure
the editor actually calls.

## 5. Passes return values with lineage

A provenance-preserving compiler pass has the semantic shape

```text
Pass S T E = S → Result (Derived S T) E

Derived S T = {
    value   : T,
    lineage : Lineage S T,
    losses  : List LossEvidence
}
```

Pass composition is:

```text
thenPass(p,q)(s) =
    let (t,L,loss₁) = p(s)?;
    let (u,M,loss₂) = q(t)?;
    return (u, M∘L, loss₁++loss₂);
```

This is an ordinary result/writer construction in the total language. It needs no `link` keyword, modal type, or
first-class world.

The familiar pipeline becomes:

```text
notation
  ── Pass ──▶ score timeline
  ── Pass ──▶ gesture timeline
  ── Pass ──▶ prepared render spec
  ── allocate ──▶ runtime plan
```

Each arrow returns a value and a finite lineage. A score-following estimator is different: it returns time-varying
claims/distributions and evidence, not compiler lineage, because observation does not establish derivation.

## 6. Examples

### 6.1 One note to several gestures

A written note anchor may produce attack, sustain, and release target anchors. Each has a `Split` edge back to the note,
plus a reason naming the profile rule. Selecting any gesture in the performance view can reveal the note and rule.

### 6.2 Several tones to one analysis

A chord claim has `Merged` lineage from the observed tone anchors, while its typed evidence names the analysis theory.
The lineage does not make the claim true; it says what the claim was about.

### 6.3 Orchestration

One source line may split across instruments, or several sources merge into one timbral mass. Ordinary one-origin fields
cannot represent this faithfully; a multirelation can.

### 6.4 Quantization

A scheduled event in seconds maps to a frame interval with an `Approximated` edge carrying rounding error and policy.
There is no fabricated equality between exact and frame time.

### 6.5 Audio samples

The runtime plan records which scheduled anchors and processor nodes feed each output buffer range. The editor can
highlight a planned region while audio plays. It does not record a relation edge per sample, and it does not claim that
reverb frames derive from exactly one note when state contains contributions from many prior inputs.

## 7. Equality must be separated

Lineage forces three equalities which the specification must not conflate:

1. **content equality:** the musically/semantically relevant finite facts agree after the domain's declared quotient;
2. **presentation equality:** anchors, alternatives, and structure agree;
3. **derivation equality:** lineage paths and source identities agree.

**Correction after the K₂ proof review.** An earlier draft said that `ScoreFact`'s key “includes `Origin`.” That was too
coarse and repeated the notebook's original failure mode: naming the right object without checking its exact definition.
[30](30-proof-review-k2.md) checked the implementation and governing N3. The key includes `origin.source_span` and
`origin.expansion_path`, but omits `origin.definition_span` and `origin.declaration`. A normalized Rust timeline retains
the full `ScoreFact` values even though semantic equality compares their narrower canonical keys. Current semantic
equality is therefore provenance-sensitive but is not full derivation equality.

Current R1 can remain a law for the execution-bearing preparation result:

That current law can remain:

```text
R1-semantic (current R1):
    M ≡ N ⇒ prepare(M,B,s) = prepare(N,B,s)
```

But a result carrying full lineage may legitimately distinguish two semantically equal timelines whose omitted
definition span or declaration differs. The architecture must not ask one equality to do both jobs. Split the result:

```text
PreparedArtifact = {
    execution : PreparedSpec,
    lineage   : Lineage Gesture Plan,
    losses    : List RealizationLoss
}
```

R1 governs `execution`; presentation/derivation equality governs `lineage`. If the prepared artifact retains full event
identity for editing, two semantically equal inputs may yield different lineage while their execution specs and frames
remain equal. A useful additional law would be:

```text
R1-content (new, if wanted):
    content_equal(M,N) ⇒ render(prepare(M,B,s)) = render(prepare(N,B,s))
```

`R1-content` is strictly stronger in a different direction and needs proof that preparation's lineage/identity
differences cannot influence samples. It must not be smuggled in as a reading of current R1.

K₁'s proof in [24](24-metatheory-of-k1.md) proves factorization only through whichever argument equality is chosen. It
does not prove that semantic equality transports full lineage, and it does not prove the proposed content law.

## 8. Does this require worlds or profunctors?

No source syntax does. Nominal source/target anchor types already prevent universe confusion. Composition is ordinary
typed relational composition over finite data.

A profunctorial semantics could later describe how transformations of the source and target act on correspondence
witnesses. That becomes useful if Musa exposes generic bidirectional editing across arbitrary views. The current
compiler/editor needs a directed derivation relation and reverse lookup; `Lineage S T` supplies both without the general
apparatus.

## 9. Concrete recommendation

Add lineage to the K₁ architecture before any governing rewrite:

1. treat current `Origin` as the `Source → ScoreFact` special case;
2. define stable nominal anchors for gesture and prepared-plan events;
3. require prompts 130–132 to return or retain explicit score→gesture→plan lineage;
4. keep lineage out of the generic temporal algebra—it rides in payload/adjacent presentation data;
5. state content, presentation, and derivation equality separately; and
6. test one-to-many, many-to-one, generated-default, selected-alternative, and quantized-region examples.

This is the structure that makes notation and audio feel like views of one artifact to a user. The type system keeps the
views distinct; lineage makes their relationship inspectable.
