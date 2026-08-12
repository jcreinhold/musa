# The coherent object is the derivation diagram, not a musical motive

**Status: conceptual synthesis; governs nothing.** The notebook began by asking for one object through which notation,
performance, sound, analysis, MIDI, and equational reasoning factor. The search has now failed in a useful way:

- common residue forgets the correspondences among realizations ([11](11-candidate-relational-presentation.md));
- one monoidal calculus conflates function, temporal, and process composition ([20](20-candidate-staged-algebras.md));
- first-class worlds and profunctor links are more structure than any current caller needs
  ([18](18-minimal-recommendation.md)); and
- score, process description, and running signal have different finiteness and composition laws
  ([28](28-candidate-k2.md)).

The constructive replacement is:

> A Musa artifact is a finite typed derivation diagram of native presentations. Each edge records a successful pass,
> finite lineage, and explicit loss. A running process is a denotation of one finite node, not another finite node of
> the same kind.

This is not merely “the compiler pipeline.” It says exactly what makes several views one inspectable artifact and what
coherence among those views means.

---

## 1. Native presentations

A **presentation family** `P` consists of:

1. a nominal data type `Value(P)`;
2. a finite anchor view `Anchors(P,x)` for each finite `x:Value(P)`;
3. its own semantic, presentation, and derivation equalities where those distinctions matter; and
4. its native operations and laws, owned by its theory module.

Examples are source syntax, a normalized score timeline, an analytical claim set, a gesture timeline, an instrument
binding set, and a prepared render specification. They are not variants of a universal `MusicObject`.

A process description is a finite presentation. The history it denotes is not: it is observed through causal process
semantics at a named tick. A recording asset is finite bytes plus metadata, while the physical or decoded signal it
represents may be observed over physical time. Finiteness is a property of the presentation, not an assertion that all
musical phenomena are finite.

## 2. Derivation edges

For presentation families `S,T`, a successful derivation edge contains:

```text
Edge S T = {
    pass_id  : PassId,
    source   : Value(S),
    target   : Value(T),
    lineage  : Lineage S T,
    losses   : List LossEvidence,
    options  : CanonicalOptions,
}
```

It is well formed only if re-running the named pure pass at its version and options returns the recorded target,
lineage, and losses. Runtime allocation is kept outside this equality when it contains addresses or host resources.

The edge is oriented because derivation is oriented. An engraving derives from a score; an audio observation does not
therefore invert to one score. A score follower has another type and returns evidence-bearing claims or distributions.
Many-valuedness is expressed in its target data, not by pretending every relation is the same kind of link.

## 3. Artifact diagrams

An **artifact diagram** is a finite typed acyclic hypergraph:

```text
Artifact = {
    nodes       : finite map NodeId ↦ exists P. Value(P),
    derivations : finite typed hyperedges (P₁,…,Pₙ) → Q,
    comparisons : finite path-coherence obligations,
}
```

A hyperedge admits several inputs because performance may depend on score plus profile, and preparation on gestures plus
instrument bindings, mix intent, seed, and render options. Every derived node has one recorded generating hyperedge;
alternative derivations produce alternative nodes until a comparison explicitly relates them.

The graph is acyclic at finite preparation stages. A cycle in a live audio or interaction process belongs inside the
validated `Process` node and is guarded by its tick semantics; it is not a recursive compiler derivation.

### Proposition 32.1 — successful deterministic derivation is unique

Fix root node values. Suppose every hyperedge operation is a pure deterministic total-or-`Result` function of its
predecessor values, and every non-root node has one generator. If all generators succeed, the artifact's derived values
are unique and independent of the chosen topological evaluation order.

*Proof.* Induct over the finite predecessor partial order. Root values are fixed. For a non-root node, every predecessor
has a unique value by the induction hypothesis. Purity and determinism give one successful edge result on that tuple, so
the node has one value. Any topological ordering evaluates a node only after the same uniquely valued predecessors and
therefore obtains the same result. ∎

The theorem does not say every interpretation is deterministic. An indeterminate performance stores a choice/evidence
value as an input node or returns alternatives. Once an artifact records the realization seed and decisions, its
derivation can be deterministic without claiming the musical practice was predetermined.

## 4. Lineage supplies the user-visible unity

Derivation edges compose their finite lineage relations. Therefore a path

```text
source → score → gesture → prepared interval
```

gives reverse lookup from the prepared interval to the gesture, score occurrence, and source site. A selection in any
finite view can navigate the path in either lookup direction even though the derivation function itself is not
invertible.

This is the unity a notation/audio workbench actually exposes:

- selecting notation can highlight the gesture and planned audio interval;
- a playback cursor can reveal the source and performance decision responsible for the active plan region;
- an analysis claim can name the exact anchors it concerns without becoming part of score truth; and
- a quantized event carries the exact time it approximated and the policy/error of that approximation.

There is no edge per audio sample. Prepared intervals and processor contributions are finite anchors; the process
semantics answers which runtime region they can influence.

## 5. Coherence is a named comparison, not global definitional equality

Two paths may reach the same presentation family:

```text
score ── direct MIDI ───────────────▶ MIDI
  └──── performance ── MIDI map ───▶ MIDI
```

It is usually wrong to demand literal equality. The direct route may omit expressive timing; the performed route may
quantize differently. An artifact can state a **comparison obligation**:

```text
compare : Projection × Value(T) × Value(T) → Verdict Evidence.
```

The projection names what should agree: pitches and onsets, notation content, frame output, anchor coverage, or another
theory-owned observation. A successful comparison carries evidence; a failure records a counterexample or declared loss.
Definitional equality is reserved for representations whose owners really intend it.

Examples:

- two engraving backends agree on score anchor coverage, not layout pixels;
- symbolic and performed MIDI agree under a declared loss projection;
- identity-sensitive prepared plans may differ in lineage while audio frames agree under an explicit rendering law;
- two theory analyses can disagree while both remain well-formed claims about the same anchors.

This is a more useful coherence condition than “everything factors through one value.”

## 6. The earned universal property

There is one modest universal construction here. Forget node values and retain the typed graph of presentations and
derivation generators. Its paths form the **free category** `Path(A)` on that graph.

### Proposition 32.2 — free extension of a pass interpretation

Any assignment which maps each presentation node type to an object of a category `C` and each derivation generator to a
well-typed morphism of `C` extends uniquely to a functor `Path(A)→C`.

*Proof.* Send an identity path to the corresponding identity and a finite generator path to the composite of its
assigned morphisms. Category associativity makes this independent of parenthesization, and identity laws handle empty
paths. Every functor preserving the generator assignment is forced to act this way on paths, proving uniqueness. ∎

Declared comparison equations may quotient this free category when they really are equalities. Evidence-bearing
comparisons and lossy projections remain labelled data instead of false equations.

This universal property explains generic pass composition and lineage composition. It does **not** discover chords,
keys, rāgas, meters, or timbres. Those definitions live inside theory modules. The construction is universal syntax for
derivation, not a motive for music.

## 7. Relation to worlds, theta-links, and the Grothendieck method

There is a restrained inter-universal reading:

- a sealed theory structure supplies a native universe of types, equality, and operations;
- a typed pass relates values from two such structures without identifying their carriers;
- lineage and loss say what survived that passage; and
- path comparisons say which composites agree under which observation.

That captures the useful warning behind a theta-link analogy: do not silently transport all structure or use one
universe's equality in another. But it does not justify an IUT-like source calculus. In Musa, structures and typed
passes already carry the distinction, and every additional coherence datum has a concrete editor, compiler, or backend
caller.

If many translations later require generic two-sided action by source and target transformations, the pass/lineage
family may acquire a profunctor or equipment semantics. That is an escalation theorem, not present evidence.

The Grothendieck method earns its keep here as **concept-finding discipline**:

1. retain the several native theories rather than intersecting away their differences;
2. form the total collection of their typed presentations and explicit translations;
3. ask which structure is stable under change of theory; and
4. implement only the finite generators and witnesses current consumers inspect.

The “total category” is a useful denotational view of all `(theory,presentation)` pairs and their passes. It need not be
a source-level universe.

## 8. What falls out, and what does not

From the artifact definitions, the following genuinely follow:

- deterministic successful finite preparation has one result;
- lineage composes across stages;
- native types and equalities are not confused;
- lossy and approximate passages retain evidence;
- parallel derivation paths can state local coherence obligations; and
- a finite plan can coherently denote an unbounded causal process.

The following do not follow and must remain theory definitions:

- chordhood, voicing, harmonic function, or key;
- pitch, tuning, interval, or octave equivalence;
- meter, pulse, phrase, gesture, or voice;
- orchestration and timbral identity;
- which performance realizes a notation well; and
- when two sounds count as the same music.

This division is healthy. A representation language should make good domain definitions easy to state and compose; it
should not manufacture them from generic category words.

## 9. Concrete Musa representation

K₂ and Candidate T₂ already supply the pieces:

```text
data Derived S T = Derived(T, Lineage S T, List LossEvidence)
type Pass S T E = S → Result (Derived S T) E
```

The compiler does not need a public generic `Artifact` graph initially. It can preserve the same structure through
narrow stage results:

```text
Compilation
  score              : Timeline ScoreFact
  source_score       : Lineage Source Score

PerformanceIntent
  gestures           : Timeline Gesture
  score_gesture      : Lineage Score Gesture

PreparedAudio
  spec               : opaque PreparedSpec
  gesture_plan       : Lineage Gesture Plan
  losses             : List RealizationLoss
```

Composition at the project facade supplies source-to-plan lookup. This respects the repo's deep-module standard and does
not expose pass internals or a second editable AST.

## 10. Verdict

Stop searching for a single finite value which is simultaneously score, performance, analysis, and sound. The
mathematically and architecturally coherent object is the typed derivation diagram, together with the native denotation
of each node.

This replacement is more constructive than abandoning the motive question:

- it says exactly what “one artifact” means;
- it gives a checkable coherence discipline;
- it identifies lineage as the missing current feature;
- it leaves room for distinct cultural theories through nominal modules; and
- it does not require the temporal kernel, audio process semantics, or source evaluator to share false laws.

The next work should therefore formalize and prototype **theory modules plus lineage-bearing passes**, while preserving
the existing timeline algebra and specifying the studio graph at its actual tick. No `world`, `link`, CBPV polarity, or
extent dependency is needed in Musa syntax.
