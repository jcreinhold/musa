# Candidate K₂ — the smallest coherent Musa core

**Status: fixed candidate for proof review; governs nothing.** This note repairs K₁ after the independent attack in
[25](25-proof-review.md). It does not edit the failed draft in place. The counterexamples remain part of the notebook.

K₂ is deliberately less ambitious as a programming language and more explicit as a musical workbench. Its thesis is:

> Musa needs one total language in which several finite, typed presentations can be related. It does not need one
> composition law, one time model, one pitch ontology, or one source-level universe containing both finite scores and
> unbounded signals.

The finite artifact is coherent because its passes return **values, lineage, and explicit loss evidence**. Audio
execution is the denotation of a finite process description at one named tick. Notation and audio are therefore not
separated into unrelated products, but neither is one identified with the other.

---

## 1. What K₂ removes

K₂ removes every feature which was present mainly to make the proposal look general.

1. There is no CBPV polarity in the source language. Finite evaluation versus unbounded execution is already expressed
   by the distinction between a process *description* and running that description.
2. There is no `world`, `link`, theta-link, profunctor, or universal `Morphism` constructor. Named nominal types prevent
   accidental identification; ordinary typed passes express transformation; finite lineage expresses derivation.
3. There is no user-level dependent type theory. Types contain only nominal identities, finite port rows, and a small
   linear-natural index language. Musical extent, meter, tuning, key, and theory evidence are values.
4. There is initially no user-level polymorphism theorem. Compiler-owned operations are schematic families, while every
   checked core term is monomorphic. A source module elaborator may later add explicit rank-1 schemes, but it must be
   specified and proved separately before inheriting core totality.
5. There is no primitive event-structure algebra. Candidate E₃'s `before` is not closed under choice
   ([23](23-candidate-event-presentations.md)); adding general/disjunctive enabling would enlarge the kernel before a
   caller has justified it.
6. There is no finite-domain `Warp` masquerading as a total operation and no generic feedback theorem whose shift is
   ambiguous.

This leaves four small structures: total finite values, timelines, total clock warps, and validated causal-process
graphs. Lineage is a finite library structure carried by passes.

## 2. Static boundary

### 2.1 Kinds and indices

```text
κ ::= Type | Data | Clock | Tick | Rate | Ports

i,j ::= n | α | i + j | k·i                    n,k ∈ ℕ
φ   ::= i = j | i ≤ j | φ ∧ φ
```

The solver may use full Presburger formulas internally, including negation, but users can state only the positive
fragment above. A static context is well formed only if its finite conjunction of constraints is satisfiable:

```text
PresburgerSat(constraints(Δ))
────────────────────────────── Δ-Ok
⊢ Δ ok
```

Conversion of two *given* index expressions asks whether `Δ ⊨ i=j`. K₂ makes no claim that arbitrary omitted indices can
be inferred. Static arguments are explicit whenever they are not fixed syntactically by the declared operation. This
avoids confusing decidable equality checking with existential solving.

Clock and tick equality are nominal. A tick states the granularity at which a process transition occurs. In particular,
`Frame r` and `Block r b` are different ticks even when both ultimately operate on samples at rate `r`.

Port rows are finite maps with distinct labels. Their tensor is **tagged disjoint union**:

```text
I ⊗ J = { left.l : A | l:A ∈ I } ∪ { right.l : A | l:A ∈ J }.
```

Explicit `rename` and `permute` operations change the tags. Parallel composition never silently captures equal labels.

### 2.2 Types

```text
A,B ::= 1 | Bool | Nat | Rat | Text
      | A × B | A + B | A → B | Result A E
      | D Ā | X | Vec i A
      | Timeline c A | Warp c d | Process k I O
```

`X` is a nominal abstract type. `Data` is the least subkind containing scalar canonical data, products, sums, finite
vectors, admitted algebraic data, and the opaque finite descriptions above; it excludes arrows transitively.
`Timeline c A` requires `A : Data`.

Initial algebraic data are **first-order polynomial** declarations. Recursive positions may occur in finite products,
sums, and finite vectors, but never beneath an arrow or an arbitrary user type constructor. Every value therefore has a
finite constructor tree and a well-founded size. K₂ does not use the broader phrase “strictly positive” as a substitute
for this definition.

Each nominal `Data` type has one owner-supplied semantic equality and canonical finite encoder. Totality, determinism,
purity, and encoder completeness are trusted ownership-boundary contracts unless their implementation is itself written
in the checked core.

## 3. Exact monomorphic value core

The proof core is substitution-based, left-to-right call by value. Every term is monomorphic under a fixed satisfiable
`Δ`.

```text
e ::= x | () | true | false | n | q | text
    | fn (x:A) { e } | e e | let x=e in e
    | (e,e) | fst e | snd e
    | inl[B] e | inr[A] e | case e of inl x => e | inr y => e
    | C(ē) | fold_D[R](e; branches)
    | op(ē)
    | timeline_map(e,e)

v ::= () | true | false | n | q | text | fn (x:A) { e }
    | (v,v) | inl[B] v | inr[A] v | C(v̄) | u
```

`u` is a closed canonical value of an opaque `Data` type: a vector, timeline, warp, process description, or nominal
domain value. Primitive syntax is saturated; a bare foreign primitive is not a first-class function value.

Foreign operations have only first-order signatures:

```text
op : A₁ × ⋯ × Aₙ ⇒ B          A₁,…,Aₙ,B : Data.
```

For every tuple of closed canonical arguments, `δ(op,v̄)` is one closed canonical value of `B`. An operation which can
fail returns a canonical `Result`. A foreign operation cannot return an arrow or hide an arrow in data. This rules out
the hostile higher-order primitive from [25 §High 1](25-proof-review.md).

`timeline_map` is not such a foreign operation. It is a checked higher-order eliminator over a finite timeline and is
handled by the core operational semantics.

### 3.1 Representative typing rules

The ordinary STLC rules are standard. The rules which fix K₂'s boundary are:

```text
Δ;Γ ⊢ f : A → B    Δ;Γ ⊢ M : Timeline c A    A,B : Data
──────────────────────────────────────────────────────── T-Map
Δ;Γ ⊢ timeline_map(f,M) : Timeline c B

op : A₁×⋯×Aₙ⇒B    Δ;Γ ⊢ e₁:A₁ … Δ;Γ ⊢ eₙ:Aₙ
─────────────────────────────────────────────────────── T-Op
Δ;Γ ⊢ op(e₁,…,eₙ) : B

Δ;Γ ⊢ e : D Ā    each branch has the polynomial fold type into R
────────────────────────────────────────────────────────────── T-Fold
Δ;Γ ⊢ fold_D[R](e; branches) : R

Δ;Γ ⊢ e:A    Δ ⊢ A ≡ B
──────────────────────────── T-Conv
Δ;Γ ⊢ e:B
```

Type conversion has no term evaluation. It is structural equality plus nominal identity, normalized port-row equality,
and Presburger entailment for already given natural expressions.

### 3.2 Reduction

Evaluation contexts `E` select the leftmost outermost call-by-value redex:

```text
E ::= [] | E e | v E | let x=E in e
    | (E,e) | (v,E) | fst E | snd E
    | inl E | inr E | case E of … | C(v̄,E,ē)
    | fold_D(E; branches) | op(v̄,E,ē)
    | timeline_map(E,e) | timeline_map(v,E)
    | timeline_build(d; spans before; (s,t,E); spans after)
```

The principal reductions are:

```text
(fn (x:A) { e }) v                         ↦ e[v/x]
let x=v in e                               ↦ e[v/x]
fst (v₁,v₂)                                ↦ v₁
snd (v₁,v₂)                                ↦ v₂
case (inl v) of inl x=>e₁ | inr y=>e₂      ↦ e₁[v/x]
case (inr v) of inl x=>e₁ | inr y=>e₂      ↦ e₂[v/y]
fold_D(C(v̄); branches)                     ↦ branch_C(v̄, recursive folds)
op(v̄)                                     ↦ δ(op,v̄)
```

For a canonical timeline

```text
u = timeline(d; [(s₁,t₁,a₁), …, (sₙ,tₙ,aₙ)]),
```

ordered only for representation, mapping exposes the finite administrative form

```text
timeline_map(f,u)
  ↦ timeline_build(d; [(s₁,t₁,f a₁), …, (sₙ,tₙ,f aₙ)]).
```

`timeline_build` evaluates the `n` payload expressions left to right and then returns one canonical timeline. It is not
source syntax and cannot inspect or change spans. This rule makes callback execution visible to the normalization proof
instead of hiding it in a supposedly one-step total primitive.

## 4. Timeline algebra

For a nominal clock `c`, exact nonnegative rationals `Q`, and canonical payload set `A`:

```text
T_c(A) = { (d,E) |
    d ∈ Q,
    E is a finite multiset of (s,e,a),
    0 ≤ s ≤ e ≤ d,
    a ∈ A }.
```

The opaque operations have schematic signatures:

```text
empty  : Rat≥0 ⇒ Timeline c A
place  : Rat≥0 × Rat≥0 × Rat≥0 × A ⇒ Result (Timeline c A) BoundsError
then   : Timeline c A × Timeline c A ⇒ Timeline c A
over   : Timeline c A × Timeline c A ⇒ Timeline c A
extent : Timeline c A ⇒ Rat≥0
```

For `M=(d,E)` and `N=(e,F)`:

```text
M ; N = (d+e, E ⊎ τ_d(F))
M ⊕ N = (max(d,e), E ⊎ F).
```

Overlay requires no extent equality and creates no rests. A voice may enter or leave. A notation package may insert an
explicit rest when a notational convention needs a glyph or a counted absence; the temporal ontology does not confuse
absence of payload with a universal musical rest.

The normal form retains exact extent and multiplicity and sorts occurrences by `(start,end,enc(payload))`. Payload
encoding includes whichever identity/provenance the nominal payload declares. Content equality, presentation equality,
and derivation equality remain separate relations.

## 5. Total clock warps

`Warp c d` is a finite representation of a **total** function

```text
w : ℚ≥0 → ℚ≥0
```

which is continuous, strictly increasing, sends zero to zero, is rational piecewise affine, has finitely many rational
breakpoints, has positive rational slopes, and has a final affine ray extending to infinity. Thus every admitted warp is
eventually affine and covers every finite timeline.

```text
identity : Warp c c
compose  : Warp c d × Warp d e ⇒ Warp c e
tail     : Warp c d × Rat≥0 ⇒ Warp c d
warp     : Warp c d × Timeline c A ⇒ Timeline d A
```

```text
tail(w,a)(t) = w(a+t)-w(a).
```

Finite local measurements do not become warps until a theory chooses an extension policy for the final ray. An API may
instead return a finite `WarpEvidence` value, but applying that evidence outside its observed interval returns `Result`.
The total `Warp` type never has an implicit failure boundary.

A fermata is not forced into `Warp`: a point-attached hold requires a convention about which neighbouring occurrence or
gesture owns the added time. Performance elaboration can resolve it to a total warp plus changed performed occurrences.
Swing, rubato, accelerando, metric modulation, and measured tempo curves can use total warps when their interpretation
is functional and monotone. Senza misura and conducted/free time may use another nominal clock and a partial observation
relation rather than a fabricated beat coordinate.

Quantization is separate:

```text
quantize : Policy × Rate × Timeline Seconds A
        ⇒ Result (Quantized r A) QuantizeError
```

`Quantized r A` privately contains the frame timeline, rounding errors, collision decisions, and approximation lineage.
Quantization is a real computation, never erased as an index proof.

## 6. Causal process descriptions

`Process k I O` is an opaque finite graph which has already passed interface and causality validation. Its chosen tick
`k` is part of the nominal type.

Every primitive processor owns:

```text
State_p                     finite runtime layout
init_p(params,seed)         total deterministic initial state
step_p : State_p × Frame(I) → State_p × Frame(O)
dep_p ⊆ InputPort_p × OutputPort_p
```

`dep_p` is a conservative relation of **instantaneous** dependencies: `(i,o)∈dep_p` when output `o` at the current tick
may depend on input `i` at the current tick. A delayed dependency is absent from `dep_p` and instead passes through
`State_p`. State is not shared between nodes except through explicit graph edges.

The process operations are:

```text
wire     : Process k I I
serial   : Process k I O × Process k O P ⇒ Process k I P
parallel : Process k I O × Process k J P ⇒ Process k (I⊗J) (O⊗P)
rename   : PortIso I J × Process k I O × PortIso O P ⇒ Process k J P

close : OpenGraph k I O ⇒ Result (Process k I O) GraphError
```

`close` constructs the graph whose vertices are individual ports and whose edges are wires together with the internal
instantaneous dependencies `dep_p`. It accepts exactly when that finite directed graph is acyclic and all interfaces are
connected according to their contracts. It stores a topological schedule. A cycle which crosses a delay can be accepted
because that delayed relation is not an instantaneous edge; a multiport node which delays one path but passes another
through cannot accidentally license the pass-through cycle.

There is no generic source term `feedback₁` and no external `shift`. Feedback is ordinary graph wiring followed by
`close`. At each tick:

1. read old node states;
2. evaluate current-tick port computations in the stored topological order; and
3. atomically commit next node states.

This defines one deterministic state-machine transition. It applies equally to a frame-tick graph or a block-tick graph,
but those are distinct process types and need not be byte-identical realizations. Musa's current feedback scheduler is a
block-boundary implementation; K₂ does not call it a one-frame recurrence.

## 7. The coherence structure: typed passes and lineage

K₂ adopts [27](27-lineage-is-the-link.md) as a library design, not a new type-theoretic connective:

```text
Derived S T = {
    value   : T,
    lineage : Lineage S T,
    losses  : List LossEvidence
}

Pass S T E = S → Result (Derived S T) E.
```

`Lineage S T` is a finite typed multirelation from target anchors to source anchors. Its identity and composition are
ordinary finite data operations. Nominal anchor types keep notation, performance, plan, and analysis anchors distinct.

The central artifact can therefore have the shape

```text
source presentation
  ── elaborate ──▶ score timeline + source→score lineage
  ── perform   ──▶ gesture timeline + score→gesture lineage
  ── prepare   ──▶ render spec + gesture→plan lineage
  ── allocate  ──▶ executable process state
```

The composed lineage makes a score selection, a performed gesture, and a planned audio interval inspectably related. It
does not assert that a sample is a note or that a score is a signal.

## 8. Domain theories remain libraries

The kernel contains no pitch, interval, scale, chord, key, meter, voice, instrument, or notation type. A package defines
the structures natural to its practice and exports total functions or evidence-producing analyses.

For example, a common-practice package may define a spelled-pitch action and construct a chord by applying an interval
pattern to a root. A rāga package may instead make ascent/descent, characteristic phrases, intonational regions, and
ornament obligations part of a melodic context; an Arabic maqām package may expose jins and sayr; a gamelan package may
make ensemble-specific tuning and paired-instrument relations primary. None must implement a global `Pitch` trait or
reduce its identity to 12-TET.

Likewise, a key is not a primitive kernel value and harmonic function is not translation by a tonic. A theory which uses
keys can define them as contexts carrying whatever collection, spelling, hierarchy, tendencies, and evidence that the
theory actually needs. Chord construction “falls out” of finite data plus actions only within a package whose musical
definitions justify those actions.

Static structure templates can serve as module functors once their source elaboration is specified. K₂ does not call an
ordinary template a categorical functor theorem, nor require the kernel to know the objects and morphisms of every music
theory.

## 9. R1 and cache correctness

Let `nf_T`, `canon_B`, and `nf_P` be the complete identity-sensitive canonical values required by preparation, including
current `Origin` where the governing kernel does. Define

```text
prepare(M,B,P,s,o) = prepare_spec(nf_T(M),canon_B(B),nf_P(P),s,o).
```

Then equality of the five arguments implies equality of the `Result PreparedSpec PrepareError`. That is the narrow
factorization law R1. Structural recursion proves termination of `prepare_spec`; it does not prove this quotient law.

A cache theorem additionally assumes:

```text
CacheInvariant(C,v) iff
  every entry (key_v(args),result) in C satisfies result = prepare_spec_v(args),
  and a lookup hit confirms v and the complete canonical args after digest lookup.
```

Only under that invariant does a hit equal recomputation. A separate content-only render theorem would require a
separate content quotient and proof that identity/lineage differences cannot affect frames.

## 10. Trust ledger and non-claims

K₂'s checked core can establish type safety and normalization only relative to these owned contracts:

- foreign first-order `δ` operations are total, deterministic, pure, and return canonical finite data;
- canonical encoders implement their one declared equality;
- warp constructors validate the total eventually-affine representation;
- processor `init`, `step`, and instantaneous port-dependency declarations are sound and total;
- preparation normalizers terminate and canonicalize the complete inputs they claim to canonicalize.

Tests can attack these contracts; they do not turn Rust into a total proof language.

K₂ does not claim:

- that all music has notation, notes, pitch, meter, voices, or a closed work identity;
- that every temporal relationship is a monotone function;
- that every performance is determined by a score;
- that causal-process equivalence is decidable;
- that content-equivalent scores render identically;
- that the current source module language already has a proved elaboration; or
- that the Western examples are more than one concrete launch package.

## 11. Immediate decision

K₂ rejects step 1 of [09](09-the-proposal.md)'s work order. Do **not** index the kernel by extent, make overlay
fibre-local, or require padding. The existing unequal-extent timeline algebra is the part of K₁ that survived hostile
proof review without repair.

The next implementation decision is smaller:

1. retain the current finite temporal kernel and its unequal-extent overlay;
2. specify current compiler passes as `Derived` values with composed lineage;
3. make score→gesture→prepared-plan identity and loss explicit in prompts 156–158;
4. specify the studio's actual process tick and port-level instantaneous dependency contract; and
5. add total exact warps only when one score/performance caller demonstrates the need.

Do not rewrite the governing architecture on this candidate until the metatheory in [29](29-metatheory-of-k2.md) has
received an independent proof review.
