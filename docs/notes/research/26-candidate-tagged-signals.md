# Candidate H — tagged signals as a heterogeneous semantic envelope

**Status: denotational candidate, not a source-language recommendation. Governs nothing.** Candidate K₁'s hardest
remaining question is not how to spell a chord. It is how a logical score clock, a performed clock, physical seconds,
device-event time, and audio frames compose without silently identifying their notions of simultaneity and causality.

There is an established framework aimed at this exact shape of problem: the tagged signal model. Its fundamental event
is a value paired with a tag; different models of time arise from different tag structures; processes constrain tuples
of signals; morphisms between tag structures relate heterogeneous models. See Lee and Sangiovanni-Vincentelli,
[“The Tagged Signal Model”](https://ptolemy.berkeley.edu/papers/96/denotational/), Liu,
[“Semantic Foundation of the Tagged Signal Model”](https://ptolemy.berkeley.edu/projects/chess/pubs/55.html), and
Benveniste et al.,
[“Composing Heterogeneous Reactive Systems”](https://ptolemy.berkeley.edu/projects/chess/pubs/336.html).

This is much closer to the user's IUT/theta-link intuition than the notebook's unexplained `world` and `link`:
coordinate systems remain separate, and a cross-domain map is mathematical data. The analogy still stops before any IUT
theorem or arithmetic geometry enters.

---

## 1. The minimal semantic data

A **tag structure** is initially a decidable ordered set

```text
K = (T_K, ≤_K)
```

whose order expresses the domain's information about temporal precedence or refinement. Examples might include:

```text
WrittenBeat        PerformedBeat        Seconds        Frames(48000)
DeviceStamp        InteractionStep
```

These examples are names, not identifications. `WrittenBeat` need not use the same equality, density, or notion of
coincidence as `Frames(48000)`.

For value space `A`, an event is `(t,a)∈T_K×A`. A signal is a set, sequence, multiset, partial function, or total
function of such events depending on the model of computation. That choice is essential:

- a score occurrence signal must retain multiple equal labelled occurrences and spans;
- a device event signal is sparse and ordered;
- an audio signal has exactly one frame value at every frame tag;
- a nondeterministic protocol denotes several possible signal histories.

There is therefore no single concrete `Signal K A` representation. A **signal model over K** specifies the admitted
signal space `Σ_K(A)` and its prefix/observation structure.

A process with interface `I→O` denotes a relation

```text
P ⊆ Σ(I) × Σ(O).
```

A deterministic causal state machine is the special case where this relation is the graph of a causal function. A
constraint, partial specification, or nondeterministic interaction can use a genuinely relational process.

## 2. Heterogeneous links

A tag morphism

```text
h : K → L
```

is an order-respecting map with whatever continuity or finite-prefix condition the two signal models require. Together
with a value translation `f:A→B`, it induces a retagging operation when collisions and absence have explicit policies:

```text
retag(h,f,policy) : Σ_K(A) → Σ_L(B).
```

Examples:

- a tempo/rubato warp maps `WrittenBeat → PerformedBeat`;
- a performance clock maps `PerformedBeat → Seconds`;
- quantization maps `Seconds → Frames(r)` with rounding/collision evidence;
- a score follower produces an estimated reverse correspondence from audio-frame prefixes to distributions over score
  positions, not an inverse function.

This is the first concrete justification for the old word `link`: heterogeneous tag domains sometimes need maps,
relations, or estimators which are not native operations of either side. But nothing yet justifies a `link` keyword.
`Warp`, `quantize`, and `follow` are clearer typed operations with domain-specific laws.

## 3. How present Musa objects embed

### 3.1 Finite timelines

A Musa timeline `(d,E)` is a bounded finite multiset signal whose tags are spans in an exact rational clock:

```text
tag = (start,end) with 0≤start≤end≤d.
```

The ambient bound `d` is additional signal-domain data. Sequence, overlay, and observation are operations of this model,
not generic operations on every tagged signal.

### 3.2 Event presentations

Candidate E₃ is an untimed causal/conflict model. A configuration is one possible signal history; a schedule is a map
from its event identities to span tags. Scheduling is therefore a model-changing link from a finite event structure to a
bounded timeline, not an equality.

### 3.3 Audio

At rate `r`, an audio signal is a total dense discrete signal

```text
ℕ → Float^channels.
```

A processor graph describes a causal relation/function on such signals. A prepared render plan combines finite event
signals with this process description and an explicit synchronization policy.

### 3.4 Notation

Notation is not merely another tag domain. It includes spatial, graphical, linguistic, and grouping relationships in
addition to musical-time anchors. A notation plan can expose a time-tagged reduct, but the full engraving model needs
its own finite data. Tagged signals therefore do not collapse notation into a note timeline.

## 4. Composition

Processes with compatible signal interfaces compose relationally by synchronization and hiding:

```text
Q ∘ P = { (x,z) | ∃y. (x,y)∈P ∧ (y,z)∈Q }.
```

Parallel composition is product of independent interfaces, with an explicit synchronization algebra when events may
coincide or communicate. Identity is equality/copycat on the shared signal interface.

This explains two earlier observations:

1. process composition is not temporal sequence;
2. a genuinely relational link becomes useful only when an intermediate behavior must be synchronized and hidden.

K₁'s ordinary deterministic functions are the simpler special case. Candidate M's profunctors were premature because the
notebook had not yet produced this caller.

## 5. The inter-universal reading, made precise

The safe part of the analogy is:

- a **universe/world** is a signal model with its own tags, admissible histories, equality, and native composition;
- a **link** is an explicit morphism or relation connecting selected observations across models;
- coherence is a commuting or comparison law for a named diagram;
- no global coordinate identifies written beat, performed beat, seconds, frames, and analytical position.

For example, the intended forward timing square is

```text
Timeline WrittenBeat A  ── interpret ──▶  Timeline PerformedBeat G
          │ warp_w                              │ warp_v
          ▼                                     ▼
Timeline PerformedBeat A ─ interpret' ─▶  Timeline Seconds G
```

Whether it commutes exactly, approximately, or not at all is a property of `interpret`, `w`, and `v`; the framework does
not assert it. Quantization to frames makes the lower comparison approximate and must record the policy.

This is “inter-universal” in the modest useful sense: preserve distinct equalities and make transport data explicit. It
does not imply that music has an analogue of a theta-link, Frobenioid, or anabelian reconstruction theorem.

## 6. Why this is not yet the kernel language

The general tagged-signal semantics is too permissive for source checking:

1. an arbitrary process relation has no computable normal form;
2. relational composition may require unbounded existential search;
3. equality of processes or signal sets is generally undecidable;
4. dense audio signals are coinductive and cannot be source-evaluated to normal form;
5. tag morphism validity depends on the chosen model's order/continuity axioms;
6. the framework supplies no musical vocabulary, chord theory, rāga practice, notation grammar, or orchestration
   semantics by itself.

Turning all of this into source terms would trade K₁'s simple totality proof for a general heterogeneous reactive
language. Musa does not have evidence to pay that price.

## 7. What K₁ should learn from it

Candidate H changes the semantic specification, not the user syntax:

1. Define `Clock` as a nominal tag structure interface rather than a bare name.
2. Define `Warp c d` as the computable monotone tag-map fragment for exact rational clocks.
3. Keep `Timeline`, `EventPlan`, and `Process` as different signal/process models with native equalities.
4. Make cross-model operations (`schedule`, `interpret`, `quantize`, `follow`, device capture) explicit and typed.
5. State one coherence law per real bridge; never claim a universal commuting diagram.
6. In architecture, run different model-of-computation regions separately and connect them through explicit bounded
   queues/adapters. Do not pretend two scheduler clocks form one process graph.

The cross-clock Case 5 from [22](22-encodings-and-sieve.md) therefore has a clean answer:

```text
interaction process
  ── emits device-stamped events ──▶ bounded queue
  ── timestamp/calibration policy ─▶ finite Seconds event batch
  ── quantization policy ──────────▶ Frames(r) event batch
  ── audio process input ──────────▶ signal
```

The queue and policies belong to the prepared runtime plan. They are not erased indices and not generic function
composition inside one synchronous graph.

## 8. Verdict

**Candidate H is the best current answer to “what do worlds and links buy?”** They buy a semantics of heterogeneous time
and computation in which each model retains its native synchronization and equality. That is real work.

**It is not the best current source kernel.** K₁ remains smaller: a total value/module language plus several opaque deep
algebras and explicit bridges. Tagged signals should guide the denotational and architecture documents if K₁ survives,
especially the definition of clock conversion and runtime adapters.

The revised “motive” claim is consequently weaker but more defensible:

> Musa need not find one object through which every realization factors. It needs a typed finite presentation of the
> chosen objects and bridges, together with a heterogeneous semantics in which their different tag structures and
> process models compose without being identified.

That is ordinary enough to implement and strong enough to say why notation and audio belong to one coherent artifact.
