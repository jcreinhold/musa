# Sieve II — which structure is house, and which is furniture?

**Status: superseded prototype plan; retained as the sieve for the larger candidate. Governs nothing.** This note
compares Candidates [R](11-candidate-relational-presentation.md), [S](12-candidate-process-worlds.md),
[M](13-candidate-modules.md), [L](14-candidate-temporal-locality.md), [F₂](15-candidate-frames.md), and
[E₂](16-candidate-fibred-equipment.md). It records the current smallest surviving theory and the tests which may still
kill it.

**Current disposition.** The supposed minimum here still assumes that worlds and modules are required. The smaller
[current recommendation](18-minimal-recommendation.md) begins with one multi-sorted process syntax and treats both as
later escalations.

---

## 1. The target phenomenon

The desired language must make the following simple for the same structural reasons:

- build a chord or independent polyphony from parallel tones;
- build an audio graph from serial and parallel processors;
- describe a key or other pitch frame without installing 12-TET as the universe of pitch;
- interpret a structure compositionally when a genuine functor exists;
- relate notation, performance, analysis, and audio when the relationship is partial or many-valued;
- restrict to an excerpt and relate several clocks;
- normalize finite syntax without deciding musical or signal equivalence.

It must also degrade honestly. A repertoire without key should not receive a dummy key. A written rest should not be
created to satisfy a type. A nonfunctional performance choice should not be wrapped in a fake deterministic function.

---

## 2. Comparative level audit

| Candidate | What it gets naturally | What it forces or forgets | Disposition |
| --- | --- | --- | --- |
| Common residue (01) | shared invariants | forgets the cross-realization link | **Rejected level** |
| Indexed CBPV (09) | finite/coinductive staging | forces one polarity boundary and equal-extent overlay; key concepts remain primitives | **Rejected synthesis** |
| R: regular presentation | non-identification, hiding, relational composition | native maps and all domain construction can be opaque predicates | **Concrete substrate only** |
| S: monoidal process world | serial/parallel construction, functorial models | forces functional cross-world maps; no context/locality | **Necessary component** |
| M: modules | many-valued natural links; functors representable | no time/frame; composition may require coends | **Necessary component if real links use actions** |
| L: temporal locality | restriction, support, several clocks | extension and global contracts need more; no pitch context | **Semantic component** |
| F₂: frames | transposition, collection, conditional key orbit | harmonic grammar does not follow; group model is local | **Domain/indexing component** |
| E₂: fibred equipment | combines the surviving constructions without identification | semantic and coherence complexity may exceed the value | **Prototype candidate** |

The level audit yields one firm correction: the exact object is a diagram carrying native maps **and** cross-world link
witnesses. A common quotient and a discrete relation are shadows of it.

---

## 3. Five-example sieve

### 3.1 Parallel tones in a frame

Let `p` and `q` be tone states in one temporal and pitch-frame fiber.

- Candidate S constructs `p ⊗ q`.
- Candidate L ensures restriction of `p ⊗ q` is the tensor of restrictions.
- Candidate F₂ maps the resulting voiced family to a pitch multiset when that forgetting is desired and transports it
  under transposition.

**Falls out:** parallel carrier, excerpt naturality, transposition.

**Still supplied:** the tone generator, temperament, voicing identity, and any rule classifying the result as a chord.

**Side condition:** both tones must be viewed in one ambient context. Candidate L makes this the enclosing observation
region rather than equal active duration. If constructing that context requires explicit padding, the synthesis has
regressed to 09 and fails.

### 3.2 String and wind `staccato`

- The written mark and gesture live in native notation and performance worlds.
- A profile module has several or profile-indexed witnesses between them.
- Native transformations on either side act on witnesses.
- A deterministic profile compiler, if it exists, represents the relevant submodule by a functor.

**Falls out:** the distinction between general interpretation and its deterministic special case.

**Still supplied:** the actual string and wind profile laws.

**Falsifier:** if changing a native notation or gesture map never transports a profile witness in actual code, a
profunctor is unused generality and Candidate R's relation is enough.

### 3.3 Entering voice and reverb tail

- Both voices inhabit one enclosing notated region with different supports; tensor needs no rest or padding.
- The gesture-to-audio module relates an initiating gesture to an audio process.
- The audio observation has its own support and may continue after the gesture.

**Falls out:** unequal support and non-identical clocks.

**Still supplied:** the causal reverb processor and the clock/link witness identifying which output window to render.

**Falsifier:** if every operation still demands a single scalar duration and manual extension before composition, the
locality layer is not present in the term language.

### 3.4 Tempo text, numeric tempo, and rubato

- A verbal mark is a notation object and may have a nonfunctional performance module.
- A numeric tempo declaration may represent a deterministic local clock functor for its admitted region.
- Rubato and fermata remain families of clock witnesses rather than failed functions.

**Falls out:** functor when determined, module when not.

**Still supplied:** the style-specific semantics of “Meno mosso,” rubato constraints, and performed timing.

**Falsifier:** if the system needs a universal pulse-layer index before it can state these relations, it has forced one
meter model into the base.

### 3.5 Audio graph and score binding

- Oscillators, filters, mixers, and delays are native audio maps.
- Serial and parallel graph construction use composition and tensor.
- Part/patch binding is a cross-world witness or represented compiler map.
- A joint piece presentation contains the score, studio, and binding instead of declaring one “coefficient data” by
consumer count.

**Falls out:** graph structure and the exact location of the score/audio link.

**Still supplied:** DSP primitives, real-time constraints, instrument selection, and the executable compiler.

**Falsifier:** if tensor cannot model port-level parallelism without exposing audio internals across crate boundaries,
the proposed shared process structure is at the wrong implementation layer.

### 3.6 Ossia and mobile: the boundary case

An explicit alternative is not merely several solutions to one opaque relation. The notation may need to retain both
branches while a performance selects one.

Candidates M and E₂ can model a module with branch witnesses, but their basic finite diagram has no generic sum.
Possible constructions are:

1. a native coproduct in the notation world;
2. finite coherent disjunction in the presentation layer;
3. an explicit choice object and selection module.

No option has yet fallen out as uniquely right. This example blocks a claim that the connective set is final.

---

## 4. Smallest prototype language

The semantic synthesis should not be implemented directly. The discriminating prototype needs only the following
declarations and term forms.

### 4.1 Declarations

```text
world W
context b @ W
object A @ (W,b)

map f : A -> B
module P : W ~> V
witness p : P(A,B)
```

Each declaration has a stable source origin. A fixed built-in signature supplies context maps and generator typings for
the experiment; user-defined semantic structures can wait.

### 4.2 Terms

```text
id A
f ; g
f | g                 parallel tensor
reindex u f

left f p
right p g
p | q                 monoidal link comparison
link p q hiding y     multi-link composition
```

This is ten structural forms if object tensor and symmetry are inferred from `|`; fewer if identity and symmetry remain
normalizer artifacts. The prototype should reject any request for `pad`, generic `meter`, `PitchClass12`, `key`,
`chord`, `processor`, or `functor` as a kernel form. Those must be definitions or properties in signatures.

### 4.3 Judgments

```text
Γ ⊢ t : A → B @ (W,b)
Γ ⊢ p : P[A,B]
Γ ⊢ t ≡ u structural
```

Type checking uses declared endpoint and context equality plus decidable normalization of any admitted atomic index
expressions. It does not invoke module inhabitance, orbit equality, harmonic analysis, or audio equivalence.

### 4.4 Normal form

The output is the stratified hypergraph of [16](16-candidate-fibred-equipment.md) §5. Two source programs are
definitionally equal only when their normalized graphs agree modulo structural coherence. Richer equations produce
explicit proof/optimization steps, not conversion.

---

## 5. Prototype obligations

Implement or specify the six examples above with one fixed structural checker. The experiment passes only if:

1. every structural form has at least two domain callers;
2. no example adds a generic typing or normalization rule;
3. chord, processor graph, conditional key, and functor are constructed as recorded, not parsed as primitives;
4. a nonfunctional link and a represented link coexist and compose;
5. an entering voice and reverb tail need no explicit rests or padding;
6. restriction commutes with parallel construction by Proposition 14.3;
7. represented link composition agrees with functor composition by Proposition 13.3;
8. normal forms and hashes remain stable under reassociation, identity insertion, and independent reordering;
9. source provenance survives normalization;
10. unresolved execution obligations are reported without making the presentation ill-typed.

The prototype fails if more than one or two examples require a new structural connective. It also fails if the code must
implement general category/profunctor theorem proving rather than local graph checks.

---

## 6. Current theory, stated minimally

The smallest surviving conjecture is:

> **Conjecture 17.1.** A finite dependently sorted calculus of typed process diagrams and natural cross-world link
> witnesses has a decidable structural type system and graph normal form; its models include the context-indexed
> monoidal worlds and modules needed for Musa; and the constructions in §3 elaborate without additional kernel forms.

This is three claims, each independently falsifiable:

1. **Metatheory:** decidable checking and structural normalization.
2. **Semantic adequacy:** the intended fibred/module models interpret the syntax.
3. **Domain adequacy:** the examples elaborate using definitions rather than new rules.

No proof is offered. Candidate E₂ has not been specified tightly enough to prove even the first claim, and the ossia
case shows that its connective set may be incomplete.

### Dependency outline

```text
finite signature and local formation rules
        ├──> typed hypergraph construction ──> termination / decidable structural equality
        ├──> free process-world semantics ──> serial and parallel interpretation
        ├──> module-action semantics ───────> represented-link composition
        └──> context reindex semantics ─────> restriction/frame naturality

all four semantic branches + six example elaborations
        └──> Conjecture 17.1
```

The next mathematical step is to state the syntax as a generalized algebraic theory and prove the free-model/coherence
result for its finite graph construction. The next design step is earlier: decide the context compatibility premises and
the representation of explicit choice. Until those are settled, proving a large theorem would only formalize an unstable
candidate.

---

## 7. Immediate engineering verdict

Do not execute [09](09-the-proposal.md) step 1. Do not add a production “universal” crate from Candidate E₂ either.

The justified action is a disposable checker or a fully formal paper specification in `docs/notes/research/`, with a
fixed signature and the six examples. The existing finite timeline and audio compiler should be used as semantic models
and test oracles, not rewritten to fit the candidate. If the definitions are right, their existing operations should
appear as interpretations of the small syntax with little ceremony. If substantial adapters and special cases are
required, that is evidence against the theory rather than implementation work to push through.
