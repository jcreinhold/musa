# Candidate L — temporal structure is locality, not an extent equality

**Status: candidate component. Governs nothing.** [12](12-candidate-process-worlds.md) has parallel and serial
composition but no account of temporal observation. [09](09-the-proposal.md) tried to repair that omission by putting an
extent and metrical plan in every timeline type. The external review showed that equal active extent and explicit
padding are the wrong consequences. This note tests a different level.

**One-line claim.** A temporal world varies over observation regions. Restriction to a smaller region is primitive;
compatible local behaviors may glue. Simultaneity is tensor in one ambient region, while active support is internal to a
behavior. Voices may therefore enter and leave without changing the type of overlay or manufacturing rests.

This is an indexed semantic structure, but it is not yet a proposal for dependent syntax. Its first job is to see
whether the musical phenomena become natural at this level.

---

## 1. The examples force regions, restriction, and several clocks

The following facts pull in the same direction.

- An excerpt of a score, performance, or audio observation is meaningful without re-running the whole artifact.
- A voice may be active only on a subregion of the enclosing score.
- A reverb tail may be active after its initiating gesture.
- A chord restricted to a smaller observation region should be the parallel combination of the restricted tones.
- Notated time, performed time, and sample time admit different coordinates and are related by tempo maps, following,
  synchronization, and buffering.
- A fermata or free-time passage may relate those clocks without determining a unique map.

An extent number alone does not provide restriction or gluing. A finite set of pulse layers does not provide several
clocks. Conversely, a general temporal logic is premature if ordinary indexed restriction already explains these
examples.

---

## 2. Temporal bases and local worlds

**Definition 14.1 (temporal base).** A temporal base `B` is a small category whose objects are finite observation
regions and whose maps include admissible embeddings or restrictions of one region into another. A chosen Grothendieck
topology records which families of subregions cover a region.

Examples include rational half-open notated intervals, finite sample windows, and bounded real performance intervals.
They need not be identified. Each world may have its own base.

**Definition 14.2 (local process world).** A local process world over `B` is a pseudofunctor

`W : Bᵒᵖ → SymMonCat`.

For a region `I`, `W(I)` is the symmetric monoidal category of processes observable on `I`. For a map `u : J → I`, the
reindexing functor `u* : W(I) → W(J)` restricts an observation from `I` to `J`. Each `u*` is strong symmetric monoidal,
and identity and composite restrictions agree up to the pseudofunctor's coherent isomorphisms.

The category-valued language is not essential to the first prototype. A set-valued behavior is the special case where
each `W(I)` is discrete. The categorical formulation is retained because native transformations and processor maps from
Candidate S should survive restriction.

### 2.1 Locality of parallel construction

**Proposition 14.3 (parallel construction commutes with restriction).** Let `u : J → I`, and let `x,y` be objects or
maps in `W(I)`. There is a canonical natural isomorphism

`u*(x ⊗_I y) ≅ u*x ⊗_J u*y`,

and `u*(I_I) ≅ I_J`.

**Proof.** These are precisely the binary and nullary comparison isomorphisms of the strong symmetric monoidal functor
`u*`. Their naturality and coherence are part of that structure. ∎

**JUDGED.** If a chord or parallel processor bank is tensor, Proposition 14.3 says taking an excerpt of it gives the
chord or bank of the excerpts. This is a genuine naturality law that 09's metrical-plan index did not state.

---

## 3. Ambient region is not active support

Fix `x ∈ W(I)`. Its type says that it is observable over `I`; it need not say that it is active everywhere in `I`.

**Definition 14.4 (local absence and support).** Suppose each `W(J)` has a distinguished tensor unit `I_J` denoting no
component of the relevant kind on `J`. A behavior `x ∈ W(I)` is locally absent on `u : J → I` when `u*x` is isomorphic
to `I_J`. Its support is the collection of subregions on which it is not locally absent.

This is a semantic definition; a finite implementation may store support directly. It must not identify “absent voice,”
“written rest,” “zero-valued audio,” and “processor bypass.” Each domain chooses what its unit means, and a notated rest
is a native event rather than generic absence.

**Proposition 14.5 (unequal activity needs no padding).** Let `x,y ∈ W(I)` have arbitrary supports. Their tensor `x ⊗ y`
is well-formed in `W(I)` without any equality assumption on those supports. On a subregion where `y` is locally absent,
there is a canonical isomorphism

`(x ⊗ y)|_J ≅ x|_J`.

**Proof.** Tensor is defined for every pair of objects in the same fiber `W(I)`, independently of support. If
`y|_J ≅ I_J`, Proposition 14.3 and the right unit isomorphism give

`(x ⊗ y)|_J ≅ x|_J ⊗ y|_J ≅ x|_J ⊗ I_J ≅ x|_J`.

No rest or extension operation occurs. ∎

The same ambient region can normally be inferred from the enclosing score, render request, or observation window. It is
a context of comparison, not a claim that every voice has equal musical duration.

### 3.1 Existing timelines as a model

**VERIFIED.** The current `Timeline<A>` already separates ambient extent from event support: it stores one extent and a
finite multiset of occurrences whose spans lie inside it. Overlay takes the maximum ambient extent and unions
occurrences. An empty part of the ambient region is not represented by a rest occurrence.

The finite timeline module can therefore model a fragment of Candidate L. What it lacks is a first-class family of
restriction maps and a relationship to performance and sample clocks, not an equal-support type rule.

---

## 4. Gluing and sequence

Restriction is useful by itself, but composition across adjacent regions motivates descent.

**Definition 14.6 (descent condition).** A local world satisfies descent for a cover `{uᵢ : Iᵢ → I}` when compatible
objects and maps in the fibers `W(Iᵢ)` glue to an object or map in `W(I)`, uniquely up to a specified isomorphism.

For set-valued behaviors this is the ordinary sheaf condition. For categories it is stack-like descent. The stronger
word is intentionally avoided until the required 2-categorical coherence is fixed.

Sequence may then arise in two different, non-equivalent ways:

1. **Gluing:** construct one behavior on a union of adjacent regions from compatible local behaviors.
2. **Process composition:** connect the output boundary state of one process to the input boundary state of another.

The existing timeline `sequence` looks like gluing plus a translation of the second region. An audio processor chain is
process composition. Treating these as one untyped operator because both are written `;` would be a forced
identification. Candidate S supplies the second; Candidate L supplies the first.

**Open choice.** The kernel syntax may need distinct terms for temporal pasting and port composition even if both have
categorical semantics. An example in which boundary state matters—legato connection, a tied note, or filter state—is
needed to decide this.

---

## 5. Clock relationships

Let `B_N`, `B_P`, and `B_A` be notated, performed, and audio/sample temporal bases.

A deterministic tempo or resampling map may induce a functor between bases and hence a reindexing of local worlds. But
not every relationship is functional:

- a fermata permits a family of performed durations;
- rubato may be constrained but not fixed by notation;
- score following relates a sample window to several possible score regions;
- swing depends on style, performer, and local context;
- free-time notation may preserve order and cues without supplying a metric map.

These are candidates for modules between local worlds, not equality of clocks. A clock module must itself respect
restriction: a witness linking observations on `I` and `J` should restrict to witnesses on subregions.

**JUDGED.** Meter is then content in a notated or analytical world, not the temporal base itself. A finite pulse-layer
plan can be one object in a regular-meter model. Gregorian rhythm, senza misura, feathered beams, Carter modulation, and
performed rubato may use other local objects and clock links without changing the structural definition of region.

---

## 6. Normalization and execution

The generic structural equalities are small:

- identity and composite restriction;
- monoidal preservation by restriction;
- selected descent uniqueness up to isomorphism.

They should not be oriented as arbitrary object-language rewrites. A concrete finite syntax can normalize region maps
and push restrictions to primitive observations. Domain modules normalize their local data.

Execution is request-relative:

1. choose a finite observation region in the target world;
2. use clock/link witnesses to identify required source regions;
3. restrict or obtain local source behaviors;
4. execute the oriented process diagram for that window;
5. retain state required across adjacent windows.

This makes block audio evaluation a semantic use of finite observation, not evidence that an infinite signal has become
a finite occurrence.

---

## 7. Failure modes

1. **Trivial base.** If every artifact is placed over one global region and restriction is unused, locality is
   decoration.
2. **False descent.** Global obligations such as “this dissonance eventually resolves” may hold locally on every small
   region and fail globally. Contracts may be subpresheaves without satisfying sheaf gluing.
3. **Hidden extension.** Moving a behavior from a small region to a larger one is not supplied by restriction. An
   extension-by-absence operation must be justified per domain; it cannot silently reintroduce generic padding.
4. **Stateful boundaries.** Naive interval gluing can lose filter state, sustain, resonance, or phrase continuation.
5. **Too many bases.** If every musical feature invents a temporal base, indexing has replaced rather than solved the
   ontology problem.
6. **Undecidable support.** Semantic equivalence to the tensor unit may be impossible to decide. Type checking must not
   depend on it.

**Verdict.** Candidate L explains unequal activity, excerpts, and multiple clocks more naturally than extent-indexed
overlay. It also reveals that temporal pasting and process composition are distinct. It does not produce pitch frames,
keys, or harmonic function; those belong to contextual organization rather than temporal locality.
