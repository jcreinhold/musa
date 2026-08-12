# Current recommendation — begin with one compositional typed syntax

**Status: current recommendation for the next research step. Governs nothing.** This note answers a clarity failure in
[16](16-candidate-fibred-equipment.md) and [17](17-sieve-and-prototype.md): neither “world” nor “link” has yet earned a
place in the kernel. They name possible semantic structure, but the notebook moved to them before showing that ordinary
typed composition is inadequate.

**Recommendation.** Begin with one small, multi-sorted, symmetric monoidal term language. Use atomic types to keep
notation, gesture, analysis, render plans, and signal interfaces distinct. Use typed morphisms for both native and
cross-domain operations. Add worlds, general relations/modules, dependency, choice, or guarded computation only when a
specific example cannot be expressed naturally without them.

This is smaller than Candidate E₂ and is now the preferred prototype.

---

## 1. Why no `world` yet

Candidate E₂ used a world for each native domain so that notation and audio could retain different objects, maps, and
equalities. That separation is important, but a first-order type system already provides it:

```text
WrittenPitch      Gesture       Signal(48_000, stereo)      AnalysisClaim
```

A notation operation accepts notation types. An audio processor accepts signal-interface types. A cross-domain compiler
has a source type in one domain and a target type in another. Nothing requires the carriers or equalities of those types
to be identified.

Rust crates and signature modules can own the native equations and normalizers. The generic syntax need not reify each
module as a categorical world merely to preserve this boundary.

**Escalation criterion W.** Add first-class worlds only if two real domain theories require incompatible structural
equalities or composition laws which cannot be kept sound by ordinary types, signatures, and module ownership. Require
two such examples before changing the kernel.

---

## 2. Why no `link` yet

Candidate M introduced a module/profunctor because a written object may have many performances or sounds.
Many-valuedness does not by itself require a new judgment. It can be an ordinary output type:

```text
interpret_mark : Mark ⊗ Profile → Gesture
prepare        : Gesture ⊗ Bindings ⊗ Seed → RenderPlan
follow         : Score ⊗ AudioWindow → Distribution(ScorePosition)
realizations   : Passage → Finite(PerformancePlan)
validate       : Notation ⊗ AudioPlan → Diagnostic
```

These types preserve direction, failure, alternatives, uncertainty, and evidence explicitly. A relation can likewise be
represented by a predicate or by a function into a collection of candidates if a domain actually needs it.

A profunctor supplies more: native maps on both sides act coherently on every correspondence witness, and composition is
defined through an intermediate category. That is mathematically attractive, but no inspected Musa caller currently
requires those actions as first-class data.

**Escalation criterion L.** Add a general link/module judgment only after an example needs all three properties:

1. the correspondence is not adequately oriented as a typed morphism with an explicit result type;
2. transformations on both sides must transport correspondence witnesses;
3. this transport must compose generically through at least one intermediate domain.

Until then, a cross-domain operation is just a well-typed operation. It is not ontologically second-class because its
source and target types belong to different modules.

---

## 3. The minimum calculus to specify

Fix a multi-sorted signature `Σ` of atomic types and typed generators. The structural type grammar is initially:

```text
A, B ::= X | I | A ⊗ B
```

Here `X` is an atomic type, `I` the unit interface, and `⊗` parallel interface. The term grammar is:

```text
f, g ::= primitive | id_A | g ∘ f | f ⊗ g | swap_(A,B)
```

with judgments `Σ ⊢ f : A → B`. The equations are exactly the category, symmetric monoidal, and interchange laws.

This is Candidate S without first-class worlds. Its free syntax has a direct graph reading:

- primitives are typed boxes;
- composition connects outputs to inputs;
- tensor places diagrams in parallel;
- symmetry rewires independent ports;
- the normal form is a typed string diagram.

### 3.1 What should be library structure

Do not add these as generic term forms:

- `chord`;
- `key`;
- `meter`;
- `processor`;
- `functor`;
- `prepare`;
- `pad`.

Instead:

- a processor is an audio generator or defined audio morphism;
- a processor graph is built with `∘` and `⊗`;
- a parallel tone configuration is a tensor of tone states;
- a chord is a musical structure or classification on such a configuration;
- a key is a domain definition such as the orbit of a scale frame, when that theory applies;
- `prepare` is an ordinary cross-domain morphism;
- a realization functor is the semantic interpretation induced by assigning meanings to the free generators.

This is the sense in which the concepts fall out: their generic construction uses existing structure, while their
musical or audio content remains in domain definitions.

### 3.2 Features not admitted automatically

- Add finite sum `A + B` only if the ossia/mobile example shows that alternatives must remain structural rather than an
  ordinary domain value.
- Add indexed atomic types such as `Signal(rate,channels)` or `Pitch(frame)` only with a decidable index equality and a
  concrete type-safety caller.
- Add temporal restriction as a domain operation first. Promote context reindexing only if the same laws are reused by
  multiple domains.
- Add guarded recursion or trace only inside an executable causal-signal theory with an explicit delay/productivity
  obligation.
- Do not add full dependent types, modal locks, universes, or proof terms during this experiment.

---

## 4. Denotational semantics

Let `F⊗(Σ)` be the free symmetric monoidal category generated by `Σ`. A denotational model assigns an object and map in
a semantic symmetric monoidal category `C` to each generator. By the free universal property, this assignment extends to
a strong symmetric monoidal interpretation

`⟦-⟧ : F⊗(Σ) → C`.

Different interpretations can target finite score structures, engraving plans, performance transformations, or signal
processors. They need not use one carrier. The common object is the finite typed syntax and its compositional laws, not
an identification of the semantic values.

A combined Musa signature may also contain cross-domain generators. An engraving-only interpretation can target a reduct
which omits studio generators; a full project interpretation handles them. The exact treatment of partial signatures is
part of the paper specification, not a reason to add worlds immediately.

---

## 5. Operational semantics

The finite diagram is evaluated by an interpreter for the chosen signature:

1. verify local port types;
2. normalize structural wiring;
3. orient the requested output computation;
4. schedule primitive boxes in dependency order;
5. delegate each primitive to its owning domain module.

An audio primitive may compile to a causal block processor. A score primitive may normalize to the existing finite
timeline. A cross-domain primitive such as `prepare` runs the existing compiler pass. The generic kernel does not
evaluate signals, solve analysis, or compare audio denotations.

If a typed graph contains feedback, the first prototype rejects it. The audio domain may later admit feedback only
through explicit delayed primitives and an independently justified guarded execution rule.

---

## 6. The prototype sequence

### Step 1: paper calculus

Write the complete syntax, typing rules, structural equations, graph normal form, denotational universal property, and
finite operational semantics for the grammar in §3. The specification must fit in one short note. If it needs a large
coherence apparatus in the user-facing judgments, stop.

### Step 2: encode four constructive examples

Use one fixed signature to encode:

1. parallel pitched tones and a derived chord view;
2. a serial/parallel audio processor graph;
3. `Mark ⊗ Profile → Gesture → RenderPlan`;
4. a score and studio joined by typed bindings and `prepare`.

These establish the positive case for one typed compositional syntax.

### Step 3: attack it with four boundary examples

Try:

1. an ossia or mobile;
2. live score following with uncertainty;
3. a voice entrance followed by an audio tail;
4. frame-relative tuning or enharmonic reinterpretation.

For each failure, try an ordinary data type or domain operation first. Record exactly which law cannot be expressed
before adding a structural feature.

### Step 4: escalate one feature at a time

Use the criteria above:

- sums for structural alternatives;
- restricted indexing for port or frame compatibility;
- contextual reindexing for reused restriction laws;
- modules only for natural, bidirectionally acted-on correspondences;
- guarded trace only for causal feedback.

Each addition must remove a side condition in at least two examples. Otherwise it is furniture.

---

## 7. Concrete repository recommendation

1. Do not execute [09](09-the-proposal.md) step 1.
2. Do not implement Candidate E₂ or create a production universal-kernel crate.
3. Write `19-minimal-process-calculus.md` as the complete paper specification from §6 step 1.
4. Then write a disposable checker or executable model for the eight examples. Keep the existing timeline and audio
   compiler unchanged and use them as interpreters/test oracles.
5. Revisit worlds, modules, temporal fibrations, and dependent indexing only in response to named failed examples.

The likely production outcome, if the experiment succeeds, is not replacement of `musa-kernel`. It is a small typed
composition/presentation layer above the existing deep timeline and audio modules, with their native types and
normalizers still private to their owners.
