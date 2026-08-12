# Candidate K₁ — a total metalanguage with staged deep algebras

**Status: candidate paper calculus. Governs nothing.** This candidate supersedes the recommendation in
[18](18-minimal-recommendation.md). The conclusion “no `world` or `link` keyword yet” survives. The proposed single
symmetric monoidal syntax does not: it confused three compositions whose laws are different.

The design question is:

> Can Musa keep notation, culturally specific musical theories, performance, and audio in one coherent typed program
> without claiming that they have one carrier or one composition law?

Candidate K₁ answers **yes**, by using an ordinary total value language to connect three deep abstract structures:

1. finite data and total functions;
2. finite temporal placement;
3. finite causal-process descriptions.

Clock conversion is a fourth, small bridge structure. Everything called a chord, key, rāga, maqām, tāl, meter,
orchestration, instrument, or analysis remains a library definition in a named theory.

---

## 1. The derivation: why one composition failed

The syntax of [18](18-minimal-recommendation.md) had only `g ∘ f` and `f ⊗ g`. That looked economical because a chord, a
processor graph, and simultaneous musical material could all be drawn as boxes and wires. The economy was false.

### 1.1 Function composition

For total functions `f : A → B` and `g : B → C`, `g ∘ f : A → C` substitutes a computed value. Products are cartesian:
values may be copied and discarded.

### 1.2 Temporal sequence and overlay

For timelines `M,N : Timeline c A`, `M ; N` translates every occurrence of `N` by the ambient extent of `M`, while
`M ⊕ N` takes the maximum extent and unions occurrence multisets. Sequence and overlay have different units. They obey
only synchronized interchange, not the unrestricted interchange law of a monoidal category. Overlay is neither a product
nor mere side-by-side syntax.

### 1.3 Processor wiring

For process descriptions `p : Process I O` and `q : Process O P`, `p >>> q` connects output ports to input ports.
Parallel wiring `p *** q` combines interfaces. There is no automatic right to copy, discard, or feed back a stream;
those operations must be explicit and causal.

### 1.4 Refutation

**Judged.** A notation whose one `⊗` means both timeline overlay and process-interface tensor has no single honest
equational theory. Giving it the symmetric monoidal laws makes the timeline calculus wrong; weakening it to synchronized
interchange makes ordinary wiring needlessly weak.

The right minimum is therefore not one operation with a profound interpretation. It is a small total language containing
several abstract types whose constructors expose exactly their native laws.

## 2. Design commitments

K₁ makes seven commitments.

1. **Plural musical theories.** The core has no pitch, note, chord, key, meter, or voice type.
2. **One pure definition graph.** Cross-domain relationships are total typed functions or values referenced in the same
   program. They do not need a second-class “link” judgment.
3. **Distinct native algebras.** Timeline composition and process wiring have different syntax and laws.
4. **Finite descriptions, possibly unbounded denotations.** A processor or interaction protocol may be finite even when
   its execution is not.
5. **Restricted static indices only.** Indices protect port, rate, layout, and finite-shape compatibility. Musical
   extent and metrical plans remain values.
6. **Explicit approximation.** Exact logical time crosses to physical seconds and frames through named operations.
7. **Opaque implementations.** Domain packages own representations and equality; clients see their signatures.

The candidate is a revision of the existing language, not a demand for a second universal-kernel crate. Much of the
current total core, module stage, temporal kernel, and studio compiler can implement these abstractions.

## 3. Kinds and static indices

The kind grammar is intentionally small:

```text
κ ::= Type | Data | Nat | Clock | Tick | Rate | Ports
```

- `Data` is the subkind of finite values with decidable equality and canonical serialization. `Data <: Type`. Functions
  are not data. A sealed abstract data type must export and law-test its canonical encoder without exposing its
  representation. Timeline payloads use this contract for normalization and hashing.
- `Clock` names a coordinate, not a meter: examples include `WesternBeat`, `CyclePosition`, `Conducted`, and `Seconds`.
- `Tick <: Clock` names a discrete observation coordinate for a causal process. `Frames r : Tick`; an interaction
  protocol may declare another nominal tick without pretending it is a sample rate.
- `Rate` is a positive sample-rate literal or a nominal rate supplied by a host.
- `Ports` is a finite row of distinctly labelled port types.

The only arithmetic index language in K₁ is quantifier-free linear natural arithmetic:

```text
i,j ::= n | α | i + j | k·i        n,k ∈ ℕ
φ   ::= i = j | i ≤ j | φ ∧ φ
```

Clock equality is nominal. Rate equality is literal or nominal equality. Port rows normalize by label and compare their
corresponding types. There is no index-level rational multiplication of variables, finite-set union, metrical-plan
concatenation, general recursion, or user proposition.

**Derived decidability claim.** Type conversion reduces to:

1. nominal equality for clocks and opaque types;
2. canonical row equality for ports;
3. Presburger-validity queries for natural constraints; and
4. structural equality for ordinary types.

Each component is decidable. This is materially smaller than [09](09-the-proposal.md), whose plan union and
concatenation had no specified normal form or solver.

**Judged.** K₁ does not index `Timeline` by extent. Equal extent is not required by overlay, and no current caller needs
compile-time proof of an exact duration. A constructor validates nonnegative exact rational values; a partial operation
returns `Result`.

## 4. Types and values

Let `D` range over nominal, strictly positive algebraic data types and `X` over abstract types exported by structures.

```text
A,B ::= 1 | Bool | Nat | Rat | Text
      | A × B | A + B | A → B
      | D Ā | X | Result A E
      | Vec i A
      | Timeline c A
      | Warp c d
      | Process k I O

I,O ::= ε | { l₁ : A₁, …, lₙ : Aₙ }
```

Formation of `Timeline c A` requires `A : Data`, not merely `A : Type`. `Warp` and `Process` descriptions themselves are
finite data. There is no representation eliminator for any of these opaque constructors. Their public operations are
given below.

Top-level definitions may be rank-1 polymorphic:

```text
f : ∀ ᾱ:κ̄ . A
```

Type abstraction is static: every use is instantiated at monomorphic indices and types before evaluation. There are no
higher-rank values, impredicative types, typecase, or runtime type representations. Local terms remain explicitly typed
call-by-value terms. This is enough to define `map`, collections, generic timeline transforms, and reusable theory
signatures without importing System F as a runtime calculus.

Strictly positive data declarations receive constructors and a structural fold. General recursive functions and negative
recursive types remain absent.

## 5. Ordinary term rules

The value judgment is

```text
Δ ; Γ ⊢ e : A
```

where `Δ` contains kinded static variables, nominal type identities, and discharged linear constraints, while `Γ`
contains value variables. Representative rules are ordinary:

```text
x : A ∈ Γ
──────────── Var
Δ ; Γ ⊢ x : A

Δ ; Γ,x:A ⊢ e : B
──────────────────────── Lam
Δ ; Γ ⊢ fn (x:A) { e } : A → B

Δ ; Γ ⊢ f : A → B    Δ ; Γ ⊢ a : A
──────────────────────────────────── App
Δ ; Γ ⊢ f(a) : B

Δ ; Γ ⊢ a : A    Δ ; Γ ⊢ b : B
──────────────────────────────── Pair
Δ ; Γ ⊢ (a,b) : A × B
```

Sums, pattern matching, and structural folds use the standard rules. Every match is exhaustive. Primitive partiality is
visible as `Option` or `Result`; it never appears as a stuck term.

Evaluation is deterministic, left-to-right call by value. Static arguments and proofs are erased after checking. An
operation which changes a timeline or process is an ordinary runtime value operation and is **not** erased merely
because its result type contains indices.

## 6. Abstract structures and theories

The current Musa structure system gains abstract type members and algebraic data definitions:

```text
signature MelodicPractice {
    data type Intent;
    data type Gesture;
    data type Context;

    let realize : Context × Intent → Result Gesture RealizationError;
}

structure SomePractice : MelodicPractice {
    data type Intent = ...;       // representation sealed outside the structure
    data type Gesture = ...;
    data type Context = ...;
    let realize = ...;
}
```

Matching is nominal for abstract types and transparent for declared manifest aliases. Structure templates are static
functors, as in the current design: they are checked parameterized modules, not runtime values. A functor therefore
“falls out” from the module language without a category-theoretic `Functor` trait.

Two structures can define incompatible pitch, time, or phrase types without either becoming a variant of a global
`Pitch`. Translation requires a named function in a structure which imports both signatures:

```text
translate : A.Intent → Result B.Intent TranslationLoss
```

That result type says more than an untyped theta-link: orientation, failure, and retained loss evidence are all
checkable.

## 7. Temporal placement

For each `c : Clock` and payload type `A`, `Timeline c A` denotes a finite ambient interval with a finite multiset of
placed payload occurrences. Public constructors use exact rational coordinates:

```text
empty    : ∀ c A. Rat≥0 → Timeline c A
place    : ∀ c A. (extent:Rat≥0, start:Rat≥0, end:Rat≥0, A)
                    → Result (Timeline c A) BoundsError
then     : ∀ c A. Timeline c A × Timeline c A → Timeline c A
over     : ∀ c A. Timeline c A × Timeline c A → Timeline c A
map      : ∀ c A B. (A → B) × Timeline c A → Timeline c B
extent   : ∀ c A. Timeline c A → Rat≥0
observe  : ∀ c A. Window × Timeline c A → List (Observation A)
```

The typing rules are direct:

```text
Δ;Γ ⊢ M : Timeline c A    Δ;Γ ⊢ N : Timeline c A
──────────────────────────────────────────────── Then
Δ;Γ ⊢ then(M,N) : Timeline c A

Δ;Γ ⊢ M : Timeline c A    Δ;Γ ⊢ N : Timeline c A
──────────────────────────────────────────────── Over
Δ;Γ ⊢ over(M,N) : Timeline c A

Δ;Γ ⊢ f : A → B    Δ;Γ ⊢ M : Timeline c A
────────────────────────────────────────────── Map-Time
Δ;Γ ⊢ map(f,M) : Timeline c B
```

There is no equal-extent premise for `Over`. Its denotation is the current kernel's maximum-extent multiset union.
Voices may enter and leave without rest payloads. A notational rest, when a notation theory needs one, is an explicit
notation fact or a rendering decision; uncovered ambient time remains absence of the timeline's payload.

`then` is temporal succession. It is not function composition. `over` is simultaneous placement. It is not a product,
voice constructor, chord constructor, or process tensor.

## 8. Clock warps

`Warp c d` is a validated finite description of an order-preserving map from nonnegative `c`-coordinates to nonnegative
`d`-coordinates with `w(0)=0`. Its first candidate representation is a finite piecewise-affine map with positive
rational slopes and explicit discontinuity conventions at breakpoints.

```text
identity : ∀ c. Warp c c
compose  : ∀ a b c. Warp a b × Warp b c → Warp a c
warp     : ∀ c d A. Warp c d × Timeline c A → Timeline d A
```

```text
Δ;Γ ⊢ w : Warp c d    Δ;Γ ⊢ M : Timeline c A
────────────────────────────────────────────── Warp-Time
Δ;Γ ⊢ warp(w,M) : Timeline d A
```

A warp maps the ambient endpoint and every occurrence endpoint. Tempo, rubato, swing, and conducted shaping may be
defined as constructors or combinators for particular warp theories. Meter and tāl are not warps: they are structures
used to interpret or organize positions in some clock.

**Open definition problem.** Fermatas expose a real choice. A discontinuity can insert elapsed performance time at one
logical instant, but occurrences crossing that instant must have an explicit boundary convention. K₁ does not claim the
piecewise-affine candidate is final until that example and live retiming are proved coherent.

Seconds-to-frames is not another exact warp law:

```text
quantize : ∀ r A. RoundingPolicy × Timeline Seconds A → Timeline (Frames r) A
```

It returns quantization evidence or diagnostics. Frame conversion is an approximation boundary and is never used in a
definitional equality.

## 9. Causal process descriptions

`Process k I O` is a finite, typed process graph observed at discrete tick `k`. `I` and `O` are normalized labelled port
rows. A port type says what is presented at one tick. At `k = Frames r`, for example, `Audio n` is an `n`-channel sample
frame, `Control A` is a held control value, and `Events E` is a finite batch of events. A live protocol can use a
nominal interaction tick and event ports without inventing audio samples. Clock or rate conversion is an explicit
processor owned by the relevant runtime theory.

```text
wire       : ∀ k I. Process k I I
serial     : ∀ k I O P. Process k I O × Process k O P → Process k I P
parallel   : ∀ k I O J P. Process k I O × Process k J P → Process k (I ⊗ J) (O ⊗ P)
permute    : ∀ k I J. (π : PortPermutation I J) → Process k I J
primitive  : ∀ k I O. Primitive k I O → Process k I O
feedback₁  : ∀ k I O X. GuardedProcess k (I ⊗ X) (O ⊗ X) → Process k I O
```

```text
Δ;Γ ⊢ p : Process k I O    Δ;Γ ⊢ q : Process k O P
──────────────────────────────────────────────────── Serial
Δ;Γ ⊢ serial(p,q) : Process k I P

Δ;Γ ⊢ p : Process k I O    Δ;Γ ⊢ q : Process k J P
──────────────────────────────────────────────────────── Parallel
Δ;Γ ⊢ parallel(p,q) : Process k (I ⊗ J) (O ⊗ P)
```

Port-row normalization supplies associativity and unit equalities for `⊗`; permutations supply symmetry. Serial and
parallel obey ordinary symmetric-monoidal interchange.

Copy, discard, mix, and feedback are not unrestricted structural privileges:

- fan-out is an explicit primitive for port types whose semantics defines sharing;
- discard is an explicit sink;
- mix is an audio primitive, not codiagonal wiring;
- feedback is accepted only through `feedback₁`, whose witness certifies that every path from the feedback input `X` to
  its output contains at least one frame of explicit delay. This reflects the existing studio's real caller: graph
  cycles through a delay are legal, instantaneous cycles are rejected. The guard is semantic data used to build the
  initial state; it is not a user-written proof term.

A `Process` value is finite and normalizable. Its denotation may be a causal state machine or unbounded stream
transducer. The source evaluator never tries to reduce that stream to a value.

## 10. Stages and typed bridges

There is no mandatory universal pipeline. A notation-first package may define this one:

```text
interpret : Profile × Timeline Written Intent
         → Result (Timeline Performed Gesture) InterpretationError

schedule  : Warp Performed Seconds × Timeline Performed Gesture
         → Timeline Seconds Gesture

prepare   : ∀ r. Bindings × Timeline Seconds Gesture × Process (Frames r) StudioIn StudioOut
         → Result RenderPlan PrepareError
```

Another package can begin from an improvised phrase vocabulary, a drum–dance protocol, or an audio process. The shared
property is that every edge is typed and every lossy or partial edge says so in its result.

The finite stage boundary is:

```text
source definitions
    ↓ total evaluation
finite domain values + Timeline + Process + Warp descriptions
    ↓ prepare / compile
finite render plan
    ↓ causal runtime
audio and interaction
```

This explains notation and audio together without identifying them. They coexist in one acyclic definition graph, and
the render plan retains typed references to the notation/performance choices from which it was prepared.

## 11. Coherent artifact, without `world` or `link`

A Musa artifact is a sealed module whose exported definitions are its views and whose internal definition graph records
their construction:

```text
structure Piece {
    let source_material : Western.Material = ...;
    let written         : Timeline WesternBeat Western.ScoreFact = ...;
    let performed       = Western.interpret(profile, written)?;
    let studio          : Process StudioIn StereoOut = ...;
    let plan            = prepare(bindings, performed, studio)?;
}
```

The artifact does not assert `written = performed = plan`. It asserts that these values inhabit distinct types and that
the named, checked definitions connect them. Provenance records which definition occurrence contributed to a later
value; semantic equality may ignore that provenance while editing and diagnostics retain it.

This is the minimum useful reading of the IUT intuition: do not silently identify coordinate systems; cross them through
explicit maps whose hypotheses and information loss are visible. No `World` term or `ThetaLink` constructor is needed to
obtain that discipline.

## 12. Why CBPV is not load-bearing

K₁ uses polarity only informally:

- the source evaluator computes finite values;
- a `Process` is a finite value describing a future computation;
- runtime execution is outside source normalization.

An ordinary abstract data type plus staging expresses that boundary. A monad or effect system would be useful only if
Musa source programs themselves performed I/O, maintained live state, threw effects, or interacted during evaluation.
They currently do none of those things. CBPV would add two judgments, `F`/`U`, `return`, `force`, and computation types
without discharging a present proof obligation.

If a future live language exposes interaction to user programs, it should add a distinct guarded computation fragment or
effect row in response to those examples. That extension must not retroactively turn pure theory definitions into
computations.

## 13. Erasure is now a precise claim

The following are erased before runtime:

- type arguments;
- natural-index proof evidence;
- port-row equality witnesses;
- abstract-type sealing evidence.

The following are **not** erased:

- `empty(d)`, `place`, `then`, `over`, or an explicit notational rest;
- `warp` and its breakpoint data;
- process primitives and wiring;
- `prepare` and render-plan construction;
- padding or delay operations that produce real logical or physical time.

Thus there is no contradiction between zero-cost static indices and audible silence. `pad` was never an index proof; it
is a value-level temporal operation when a domain chooses to define it.

## 14. What must be proved before adoption

K₁ is not yet a specification. It owes:

1. preservation, progress, determinism, and strong normalization for the total value fragment with abstract data;
2. decidability of kinding, index constraints, type conversion, and module matching;
3. the timeline algebra and canonical normalization already substantially proved in `docs/rules/kernel/`;
4. warp identity, composition, and overlay preservation, including a decision on discontinuities;
5. process port safety, graph-normalization soundness, and causal execution of every admitted primitive;
6. a precise `prepare` coherence theorem with all required hypotheses;
7. encodings which do not translate the counterexamples in [19](19-domain-obligations.md) into dummy Western values.

The next note gives denotational and operational semantics. A later sieve will decide whether abstract types, rank-1
polymorphism, and the three deep structures each remove genuine side conditions or are still furniture.
