# Candidate R — a piece is a presentation of coherent realizations

**Status: candidate coordination substrate; not the house. Governs nothing.** This note reopens the premise of
[01](01-realizations-and-residue.md), not just the conclusion of [09](09-the-proposal.md). The external review in
[10](10-external-review.md) says why 09's proposed kernel change should not be made. This note asks the constructive
question that review did not answer: what small language could represent notation, performance, analysis, and audio
together without pretending that they are the same kind of object?

**One-line claim.** A Musa artifact is a **finite typed presentation of the relation that makes several native objects
coherent realizations of one another**. The native objects remain in different sorts. The links, not their common
residue, are the content that joins them.

The candidate kernel is correspondingly small: indexed sorts, first-order operations, relation symbols, conjunction, and
hiding. Its concrete normal form is a finite typed constraint hypergraph. It is not CBPV, a signal calculus, a theorem
prover, or a claim that every musical tradition has one pitch or metrical ontology.

This is a candidate, not a disguised work order. It should be rejected if the examples at the end cannot be encoded
without adding structural rules one repertoire at a time.

**Disposition after the emergence test.** The candidate passes a non-identification test and a composition-of-links
test. It fails as a motive on its own. If `chord`, `key`, `processor`, and `realization` are merely primitive relation
symbols, regular logic is a neutral database language in which nothing musical falls out. The rest of the notebook now
treats this candidate as the concrete normal form for a richer compositional theory, not as that theory itself. See
[12](12-candidate-process-worlds.md) and [13](13-candidate-modules.md).

---

## 1. What changed: residue forgets the link

[01](01-realizations-and-residue.md) looked for the intersection of what engraving, performance, sound, analysis, MIDI,
and equational reasoning retain. That operation is not conservative. Projections do not determine a relation.

Let `N` be a notation domain and `A` an audio domain. A piece of cross-domain meaning is a relation

```text
R ⊆ N × A.
```

Take `N = {staccato, tenuto}` and `A = {short, long}`. The two relations

```text
R₁ = {(staccato, short), (tenuto, long)}
R₂ = {(staccato, long),  (tenuto, short)}
```

have exactly the same projections to `N` and `A`. Their separate notation and audio residues are therefore identical,
but their meanings are opposite.

> **Proposition 11.1 (derived).** For the displayed sets `N` and `A`, the map
>
> `image : ℘(N × A) → ℘(N) × ℘(A)`
>
> sending a relation to its two projections is not injective. Consequently it has no left inverse, so no function of the
> two projections alone recovers every coherence relation.

**Proof.** The displayed relations satisfy `R₁ ≠ R₂`, because `(staccato,short) ∈ R₁` but `(staccato,short) ∉ R₂`. Both
project to all of `N` and all of `A`, however, so `image(R₁) = image(R₂)`. Thus `image` is not injective. Any map with a
left inverse is injective, so `image` has no left inverse. ∎

This is elementary, but it changes the search. The desired object is not the greatest common quotient of notation and
audio. It must retain the joint presentation from which either side can be forgotten.

The local IUT literature supplies a useful warning, not a transferable construction. In
[the discussion of “redundant copies”](../../../../papers/arithmetic-geometry/anabelian-and-iut/on-the-essential-logical-structure-of-inter-universal-teichmuller-theory/02-section-2.md),
identifying objects which look redundant destroys a logical conjunction that depends on keeping both copies present. The
musical analogy is limited but exact at this point: a written mark and a performed or sounding event may be linked, but
identifying them destroys the very relation under study. No claim here identifies a Musa link with an IUT Theta-link.

### 1.1 The repository already contains the counterexamples

**VERIFIED.** [`profile-fixture.musa`](../../../examples/profile-fixture.musa) states that `staccato` is not numeric
until an instrument profile interprets it. The same written mark receives different gate and attack data under the
string and wind profiles. The profile-dependent relation is not recoverable from a notation-only residue or from the two
sets of numeric values.

**VERIFIED.** [`tempo-changes.musa`](../../../examples/tempo-changes.musa) distinguishes a printed tempo marking from a
performance clock. A numeric `tempo 1/4 = 120 "Doppio movimento"` affects both projections; the words `"Meno mosso"`
print but deliberately do not determine an engine tempo. The notation-to-performance link is partial and contextual, not
an identity.

**VERIFIED.** [`glass-mountain.musa`](../../../examples/glass-mountain.musa) already places a score and a studio in one
source artifact, including patches, part routes, sends, a reverb bus, and a master. Syntax is not the separation. The
missing object is an explicit semantic account of how the score, performance gestures, bindings, graph, and sound are
related.

This refutes [01](01-realizations-and-residue.md) E8's inference from “only sound reads the patch” to “the patch is not
part of the piece.” A score-only work need not specify a patch. An electroacoustic Musa artifact may. “Part of the
piece” is not a global Boolean decided by counting consumers; it is determined by the presentation and by which
forgetful view of that presentation is being discussed. The patch implementation can still be realization parameter
data, while the fact that a named part is bound to it is part of the artifact's cross-domain relation.

---

## 2. The house: signatures, presentations, and models

The local references point to a plainer foundation than a motive or a multimodal lambda calculus.

- [Jacobs, equational logic](../../../../papers/logic-and-computation/type-theory/categorical-logic-and-type-theory/03-equational-logic.md)
  starts from many-typed signatures and their generic term models.
- [Jacobs, first-order predicate logic](../../../../papers/logic-and-computation/type-theory/categorical-logic-and-type-theory/04-first-order-predicate-logic.md)
  isolates regular logic: equality, truth, conjunction, and existential quantification. In its category of relations,
  identity is equality and composition is existentially hiding the common variable after conjunction.
- [Caramello, classifying toposes](../../../../papers/category-theory/theories-sites-toposes/06-chapter-02-classifying-toposes-and-the-bridge-technique.md)
  gives a genuine universal semantics: a theory has a universal model, and its different presentations may be studied
  through the classifying topos. This is a semantic construction, not a proposed Rust data structure.

The right placement of the three notions is:

1. A **signature** says which native kinds of thing and primitive links exist.
2. A **presentation** is a finite, typed network of particular things and constraints. A score or project elaborates to
   one.
3. A **model** gives the signature meaning in sets, temporal behaviors, signals, engraving objects, or another suitable
   semantic category.

Notation and audio are therefore not asserted to be Morita-equivalent presentations of one theory. That would be a
strong and probably false claim. They are distinct parts of a joint signature, connected by explicit relations. A
notation-only or audio-only view is a reduct or projection of a joint presentation.

> **Definition 11.2 (judged).** A Musa artifact over a base theory `T` is a finite formula-in-context
> `[x₁:A₁, …, xₙ:Aₙ | φ]`, where the `Aᵢ` are native sorts and `φ` is a regular formula. Its meaning in a model `M` of
> `T` is the collection of tuples satisfying `φ` in `M`.

The exposed variables are the artifact's interface. Existentially quantified variables are internal decisions or
intermediate representations. A concrete realization is a satisfying tuple, or a projection of one, not a value obtained
by collapsing all domains to a common carrier.

### 2.1 Why a presentation rather than “the piece is a theory”

A whole musical language, tuning system, performance practice, or DSP vocabulary can sensibly be a theory. A particular
piece is normally data and constraints within one or more such theories. Calling every piece a theory extension is
possible—add constants and axioms—but it obscures the simpler object: one formula in context, equivalently one object of
the regular syntactic category.

This distinction also makes abstraction ordinary. Hiding the studio variables gives a score-facing projection. Hiding
engraving choices gives a performance-facing projection. Neither projection is declared to be the uniquely true work.
They are views that forget specified parts of one presentation.

### 2.2 What is universal, if anything

The implementable universal property belongs to the syntax, not to a mystical per-piece carrier. The regular syntactic
category is the generic model generated by the signature and axioms: an interpretation of those generators in any
regular model extends in the prescribed way to the whole syntax. A geometric completion has a classifying topos and
universal model.

That is a real factorization result, but it must not be oversold. It says that interpretations of the *theory* factor
through its generic syntax. It does not prove that engraving, performance, and audio are faithful, equivalent, or
recoverable from one another. Their actual relationships are stated by the presentation.

---

## 3. The proposed kernel language

The language has two deliberately separated static layers.

### 3.1 A decidable index layer

Index sorts and operations describe shapes which must agree before an object can be connected:

```text
Rate       Channels       PortShape       ClockKind       …
48_000     stereo         audio(48_000, stereo)
```

Each index theory admitted to the kernel must provide a terminating, confluent decision procedure for its definitional
equality. Rational normalization and finite enumerations qualify. An arbitrary user predicate does not.

Object sorts may be parameterized by normalized index terms:

```text
Signal(rate, channels)
Port(shape)
Interval(clock)
Section(behavior, interval)
```

This is **stratified indexing**, not full dependent type theory. Object values do not appear in index terms; there are
no universes, dependent functions, identity proof terms, or conversion by arbitrary theorem proving.

The restriction is evidence-led.
[Jacobs's first-order dependent type theory](../../../../papers/logic-and-computation/type-theory/categorical-logic-and-type-theory/10-first-order-dependent-type-theory.md)
shows why dependent families can make invariants concise, but also records that strong extensional equality makes
conversion depend on inhabitation and hence undecidable. Musa needs the former result and has no demonstrated caller for
the latter machinery.

### 3.2 First-order terms and regular constraints

For a typed signature `Σ`, the core judgments are only:

```text
Δ ⊢ i : I                         index term
Δ ⊢ i ≡ j : I                     decidable index conversion
Δ ; Γ ⊢ t : A[ī]                  first-order object term
Δ ; Γ ⊢ φ prop                       constraint
```

The object and constraint grammar is:

```text
t ::= x | f(t₁, …, tₙ)
φ ::= ⊤ | t = u | R(t₁, …, tₙ) | φ ∧ ψ | ∃ x:A. φ
```

An operation `f` is deterministic structure native to one domain. A predicate `R` may be partial, many-valued, or
cross-domain. Examples are:

```text
sounds_as      : WrittenEvent × Profile × Gesture
drives         : Gesture × Binding × SignalGraph
observed_as    : Performance × AudioObservation
analyzes_as    : Passage × AnalysisContext × AnalysisClaim
synchronizes   : NotatedClock × PerformanceClock
```

These names illustrate arities, not a settled Musa signature.

There is no primitive `compose-link`. Regular logic already derives it:

```text
(S ∘ R)(x, z) := ∃ y. R(x, y) ∧ S(y, z)
```

This is the first candidate in the notebook where the main correspondence operation falls out of the definitions without
a musical side condition.

### 3.3 Concrete normal form: a typed constraint hypergraph

A finite regular presentation can be stored directly as:

- typed vertices for variables and first-order values;
- typed hyperedges labelled by operation graphs or primitive predicates;
- an ordered boundary of exposed vertices;
- internal vertices, corresponding to existentially hidden variables.

Conjunction is graph union with shared boundary vertices identified. Hiding moves a boundary vertex to the interior.
Composition is gluing on an equal-typed boundary followed by hiding it. Type checking is local to vertices and incident
edges.

The public structural API could therefore be as small as:

```text
check(signature, presentation)
compose(left, right, shared_boundary)
hide(presentation, boundary_vertices)
normalize(presentation)
```

This follows the repository's deep-module discipline: the generic kernel knows graph structure and typing; notation,
meter, tuning, analysis, and DSP define signatures and interpreters behind their own module boundaries.

---

## 4. Normalization and equality

This candidate draws a hard line between three equalities which 09 blurred.

### 4.1 Index conversion

Index expressions normalize using the decision procedure attached to their index theory. Type checking may call only
this equality. Adding an index domain is therefore an admission decision with a metatheoretic obligation, not a free
extension of the grammar.

### 4.2 Structural presentation equality

Presentation normalization performs only structure-preserving work:

1. normalize index and first-order data terms;
2. quotient explicit variable equalities with typed union-find;
3. alpha-normalize internal vertices;
4. flatten conjunction and sort independent hyperedges;
5. remove syntactic identities and unreachable internal vertices where sound.

Each pass is finite and terminating. The result gives a stable structural hash modulo the explicitly supported
congruences. It does **not** decide whether two arbitrary constraint systems have the same solutions in every model.

### 4.3 Domain and denotational equality

The existing temporal kernel's flatten-and-sort normal form is valuable and can remain the definitional equality of the
finite-timeline module. Rational arithmetic has its own normal form. A DSP optimizer may have sound graph rewrites. A
pitch theory may declare a finite canonicalization.

Such equations enter only through a domain module that supplies a terminating normalizer or through explicit proof
evidence. Arbitrary audio equivalence, orchestral equivalence, or analytical equivalence is not conversion. Treating it
as conversion would make type checking an open-ended semantic problem.

**JUDGED.** This modest equality is a feature. The user-facing language can support richer claims and proofs without
making the trusted kernel pretend to solve them. If a canonical hypergraph representation is still too expensive or
unstable for incremental editing, that is a direct falsifier for this candidate's engineering form.

Finite disjunction is deliberately absent from the first kernel. A primitive relation can be nondeterministic, so live
performance and score following do not require disjunction. But an ossia or mobile whose *alternatives themselves must
remain inspectable* may require the coherent-logic extension `φ ∨ ψ` and `⊥`. That is one orthogonal extension to test,
not a reason to begin with full dependent type theory.

---

## 5. Denotational semantics

Let `E` be a regular category. A model `M` assigns:

- an object of `E` to every sort;
- a morphism to every operation;
- a subobject of the appropriate product to every predicate;
- interpretations satisfying the declared axioms.

Equality is interpreted by the diagonal, conjunction by pullback/intersection, and existential quantification by stable
image. A presentation `[x̄:Ā | φ]` denotes a subobject of the product interpreting `Ā`. In `Set`, it is simply the set of
satisfying tuples.

This semantics is useful because it does not require every domain to have the same internal ontology.

- Written pitch may be interpreted as spelling-bearing notation objects.
- Performance may be interpreted as timed trajectories or gestures.
- A signal sort may denote streams or finite-observation behaviors.
- An analysis predicate may denote a context-sensitive set of warranted claims.

Static objects can live as constant temporal behaviors while time-varying objects use local sections. A sheaf or
presheaf category over intervals is a regular category, so restriction and gluing of finite observations can be part of
a model without making an infinite signal an inductive kernel payload. The finite syntax denotes a potentially
coinductive behavior; it does not enumerate that behavior.

The classifying topos is a good semantic north star because it supplies a universal model and makes presentation
independence precise. It is not required in the compiler, and “notation model” and “audio model” should not be called
two sites for one topos unless an actual Morita equivalence has been proved.

> **Proposition 11.3 (derived).** Forgetting all audio variables from a joint presentation is existential projection. It
> preserves exactly the score tuples which admit at least one coherent audio completion, not the link or the audio
> choices themselves.

**Proof.** Write the joint formula as `φ(n,a)`, where `n` contains the non-audio variables and `a` the audio variables.
Regular semantics interprets hiding `a` as the image of the projection, which is the formula `∃a. φ(n,a)`. A tuple `n`
belongs to this image exactly when some `a` makes `φ(n,a)` hold. The existential formula records neither that witness
`a` nor the proof or link data establishing `φ(n,a)`. ∎

This says precisely what a notation-only consumer loses. It also explains why two projects with identical printed scores
can be semantically distinct without forcing every score to specify a studio.

---

## 6. Operational semantics

Relations are intentionally unoriented in the denotation. Execution requires more information.

A primitive relation may have one or more **orientations**:

```text
inputs ⇒ outputs
```

An executable orientation declares or certifies the properties its scheduler needs: totality on an admitted input
domain, determinism or controlled nondeterminism, causality, latency, and resource bounds. Trusted DSP primitives and
compiler passes can supply implementations. An analysis relation may instead be queried by a solver. Score following may
orient observed audio and a score toward a distribution over score positions.

Operational compilation then has four steps:

1. choose an orientation for the requested outputs;
2. construct the dependency graph and reject unoriented obligations;
3. schedule acyclic finite work topologically, and require a delay/guard on causal signal cycles;
4. evaluate finite objects to completion and causal objects block by block.

The denotation remains one relation while different tasks orient it differently. Engraving, synthesis, transcription,
analysis, and live following need not be forced into one evaluation direction.

CBPV may still be a useful implementation language inside an interpreter which stages pure preparation before effectful
or coinductive execution. It is not load-bearing in the kernel. The relation between a written mark and a performance is
often partial or many-valued, and no `U`/`F` polarity supplies that relation.

Likewise, multimodal dependent type theory is informative but too strong as the starting point. The local
[MTT account](../../../../papers/logic-and-computation/type-theory/multimodal-dependent-type-theory/text.md) puts a full
dependent type theory at every mode and connects modes by dependent right-adjoint-like modalities, with locks and modal
eliminators. Its
[normalization theory](../../../../papers/logic-and-computation/type-theory/normalization-for-multimodal-type-theory/text.md)
is correspondingly substantial and makes decidable type checking conditional on decidable equality in the mode theory.
Musa's cross-domain links have not been shown to be modalities or functors, much less right adjoints. A relation symbol
states the observed structure directly.

Guarded recursion belongs in the certified causal-signal interpreter if that interpreter needs an internal calculus. It
need not be paid for by engraving or finite score normalization.

---

## 7. The hard examples

These are not implementations. They are the sieve the signature prototype must pass.

### 7.1 Voices with unequal extents

No generic rule requires equal extents. A voice has an active region; an enclosing score has an ambient region; silence,
absence, and a written rest remain different payload facts. The existing timeline module may keep total overlay with
extent `max`. A consumer which needs synchronized blocks states a synchronization constraint or constructs local padding
in its own representation.

This avoids turning the interchange law into a global admission test for ordinary entrances, cutoffs, pickups, and
reverb tails.

### 7.2 Meter, rubato, swing, and free time

There is no universal finite set of pulse layers in the kernel. A regular-meter module may define such layers. More
generally, notated, performed, and sample clocks are separate objects connected by `synchronizes` relations.

- Swing constrains selected notated subdivisions to a performance-time relation.
- Rubato and accelerando constrain a varying clock map.
- A fermata weakens or opens the duration relation at one location.
- Metric modulation relates two pulse descriptions at a boundary.
- Senza misura, chant, and ametrical notation need no fake pulse set; they can use order, phrasing, or locally supplied
  timing constraints.

The proposal does not claim that one clock theory already covers these. It says the structural kernel should let their
native theories and correspondences coexist without changing its logic.

### 7.3 Pitch and tuning

`WrittenPitch`, `Frequency`, `MidiPitch`, `PitchClass(n)`, `ScaleDegree(context)`, and analytical set classes are
different sorts or families. Temperament, tuning, instrument, register, and analytical context parameterize relations
among them. A 12-TET quotient is one model, not the carrier of pitch itself.

Modal, just-intonation, microtonal, spectral, and non-Western modules can therefore decline to instantiate the
common-practice relations without changing the structural calculus. This does not make those modules easy to design; it
makes their disagreement visible instead of encoding one as the default quotient tower.

### 7.4 Harmonic function and motif

Harmonic function is an analysis predicate depending on a passage and an analytical context, not an orbit under tonic
translation. Motif correspondence is likewise a declared or inferred relation among passages and transformations. If a
musically meaningful projector is later defined, its idempotence can be proved. The kernel gets no “split idempotent”
primitive from the pun.

### 7.5 Articulation profiles

The written mark, instrument profile, and produced gesture inhabit three vertices of one relation. The current
`profile-fixture.musa` behavior is the direct model:

```text
sounds_as(written_staccato, string_profile, string_gesture)
sounds_as(written_staccato, wind_profile,   wind_gesture)
```

Nothing requires `written_staccato` to equal either gate fraction.

### 7.6 Studio graph and reverb tail

A studio graph is a finite term or presentation even when its signal denotation is unbounded. The binding edge joins a
gesture source to an input port; the graph's causal semantics may continue after the gesture's active region. No
notational `pad` is required to make the reverb tail exist.

### 7.7 Live and reverse directions

The same correspondence can support synthesis in one direction and score following or transcription in another only when
suitable orientations exist. A many-valued or probabilistic observation relation may require an enriched semantic
module; the structural presentation can still name its inputs and outputs without claiming determinism.

### 7.8 Ossia and mobile

Regular predicates can denote multiple solutions, but they do not retain the syntax of two alternatives. This is the
cleanest present pressure for coherent finite disjunction. The prototype must encode one real ossia and one mobile both
ways—opaque nondeterminism and explicit `or`—before deciding whether `or` belongs in the trusted kernel.

---

## 8. What survives from the existing design

This candidate is a layer around deep domain modules, not a demand to discard working semantics.

- The finite temporal kernel remains a good normal form for finite exact occurrences.
- Exact rational musical time remains correct inside that module.
- Total overlay remains musically useful.
- `ScoreFact` and performance gesture remain distinct payloads.
- A prepared render plan remains a valuable real-time boundary object.
- The source remains canonical, and the joint presentation is derived and disposable.
- Signals remain coinductive denotations rather than occurrence payloads.
- Studio graph validation and compilation remain owned by `musa-dsp`.

What changes is the claimed outer shape. The occurrence kernel is one native theory in a larger presentation language,
not the universal ontology of every realization. The audio graph is another finite native theory, not merely coefficient
data that cannot be related to the piece.

---

## 9. What could kill this candidate

The regular-presentation frame is dangerously capable of becoming “everything is a graph.” It survives only if the
shared structural operations do real work.

1. **Vacuity.** If every meaningful link is an opaque callback and no composition, hiding, type checking, or
   normalization law is reused, there is no common kernel.
2. **Equality cost.** If structural normal forms are unstable under ordinary edits or graph isomorphism dominates
   incremental compilation, the proposed concrete representation is wrong.
3. **Temporal mismatch.** A sheaf-style finite-observation model has not been proved adequate for feedback, live input,
   latency, stochastic following, or offline noncausal processing in Musa.
4. **Choice.** Regular logic may be too weak if explicit alternatives pervade real scores; coherent logic may then be
   the actual minimum.
5. **Quantitative semantics.** Probabilities, costs, loudness, and approximation may require enriched relations rather
   than ordinary subobjects.
6. **Layout and provenance.** The candidate has not yet shown that source spans, engraving alternatives, stable IDs, and
   round-tripping survive normalization.
7. **Theory boundaries.** The correct division among base signature, repertoire package, piece presentation, profile,
   and realization request remains unsettled.
8. **No useful reverse orientation.** Relational denotation does not guarantee a tractable solver for analysis or
   transcription. If reverse uses are the only reason for relations, the candidate may be buying unused generality.

These are experimental obligations, not details to wave away with category theory.

---

## 10. Concrete next experiment

Do not amend the governing ontology or write a new production crate yet. Do not prototype this candidate in isolation;
that would merely demonstrate that typed graphs can encode data. Its concrete hypergraph normal form should instead be
tested as an implementation of the compositional candidates that follow.

Encode these artifacts without special structural cases:

1. the string/wind `staccato` example;
2. a score with an entering voice and a reverb tail;
3. numeric tempo plus a purely verbal tempo mark;
4. an ossia or mobile with both an engraving view and one performed branch;
5. one tuning where enharmonic spellings coincide in frequency and one where they do not;
6. one live score-following observation.

For each artifact, require:

- local type checking with a documented index equality procedure;
- a stable structural normal form and hash;
- at least two projections obtained by hiding, not bespoke traversals;
- one forward executable orientation;
- a statement of what the relation allows but the implementation cannot compute;
- preservation of source provenance.

The experiment passes only if new musical vocabulary is added as signature data or a deep domain module while the six
structural rules and their semantics stay fixed. If each example asks for another generic connective, modality, or
conversion rule, the kernel is wrong.

### Immediate verdict on 09 step 1

**Do not index the existing temporal kernel by extent, restrict overlay to equal extents, or reintroduce generic `pad`
now.** This candidate removes the alleged reason: synchronization is a relation stated where a consumer needs it, not a
precondition imposed on every overlay.

The constructive next step is the finite-presentation experiment above. If it succeeds, the governing boundary should be
reopened deliberately to admit a small cross-domain presentation layer while retaining the occurrence and audio modules
as deep implementations. If it fails, the failure should be recorded here before another production kernel is proposed.
