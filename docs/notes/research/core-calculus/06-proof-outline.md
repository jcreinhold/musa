# Proof outline for the selected calculus

**Status: proof outline. This document does not set Musa's rules.**

## Purpose

This note states the claims the selected calculus needs and gives the shortest proof route. It does not claim a
mechanized proof or an implementation conformance result.

## 1. Assumptions

The claims below use these assumptions.

**A1.** Every source primitive has a closed rank-1 type scheme whose complete argument and result are storable data.
When applied to complete well-typed finite arguments, it returns one well-typed finite value of its declared result
type. An operation that can fail declares an ordinary `Result` type. A returned primitive or machine description
contains no source closure.

**A2.** Every machine primitive supplies the exact private state, start function, and step function required by
[the selected calculus](05-selected-calculus.md#51-primitive-units). Its start and step functions are total and
deterministic. One finite build-local registry maps each primitive id and version to one exact definition and rejects a
conflict.

**A3.** Every nominal recursive data type is strictly positive. Source code can consume it only through its generated
structural fold. There is no recursive value definition.

**A4.** `Length<C>` contains a nonnegative exact rational. Its zero, addition, comparison, and maximum operations are
total.

**A5.** Time-map and scheduling-policy operations are first-order source operations satisfying A1. Frame assignments are
bounded integers and preserve the order of the finite source-boundary set. The scheduler itself is the six-step finite
algorithm in the selected calculus §7.1. Its source state uses a finite cursor and bounded countdown, then a fixed
`Finished` state. The event-batch merger relabels its two inputs into disjoint handle sets before sorting the union.

**A6.** A scheduler advertised as occurrence-local assigns a boundary from that occurrence, the time map, and the policy
alone. Equal occurrences receive equal time decisions; their private copy numbers only keep handles apart. Merging
same-frame messages sorts them by the fixed policy and neither drops nor shifts them.

**A7.** The source evaluator uses a versioned nonnegative integer cost for each reduction and constructed value. Its
remaining budget is part of the evaluation state. Costs do not read wall time, allocator behavior, or machine load.

**A8.** Event handles are opaque. A machine primitive may test them only for equality. Consistently renaming handles may
rename the primitive's private table but does not change any non-handle output, including audio.

A real-time deadline is not among these assumptions. Totality and determinism do not imply that a step is fast enough
for audio. The runtime acceptance check has a separate resource contract.

## 2. Source typing and evaluation

### Theorem 2.1: type inference terminates and returns a principal type

For every finite resolved source term `e` and finite type environment `Gamma`, the inference algorithm either reports a
type error or returns a type scheme more general than every other valid rank-1 type scheme of the same value/data class
for `e` in `Gamma`.

**Proof outline.** Use Algorithm W with two classes of type variable: any value and storable data. Each syntax case
makes finitely many recursive calls on strict subterms and performs first-order unification with an occurs check.
Unification also rejects replacing a data variable with a function or a container that contains one. `EventTrack`,
`Primitive`, and `Machine` are ordinary rigid type constructors with data payloads or ports. The tags `C` and `K` are
ordinary nominal types, so they add no value equations. Non-recursive `let` generalizes exactly the variables not free
in `Gamma`, while retaining each variable's class. Nominal constructors and primitive schemes enter the environment like
any other declared constant. Storable-data admission for each finite recursive declaration group is computed before
expression inference by the check in the selected calculus §2.1. The standard principal-unifier argument then applies
within each class. ∎

### Theorem 2.2: evaluation preserves types

If a typed evaluation configuration for result `A` takes one step, the next configuration is also for result `A`.

**Proof outline.** Prove substitution first. The ordinary lambda, `let`, constructor, case, and fold rules are standard.
Track operations return the result type shown in their typing rule. Machine operations only build a machine syntax value
whose type follows from the premises. A1 supplies the primitive case. An ordinary expression step keeps type `A`;
`done(v)` requires `v : A`; and the resource transition retains the configuration's recorded result `A`. No machine step
runs during source evaluation. ∎

### Theorem 2.3: a closed typed source term can make progress

If `run(budget,e)` is closed and well typed, then it can take one step. A terminal typed configuration is either
`done(v)` or `failed(ResourceError)`.

**Proof outline.** If the next fixed charge exceeds the remaining budget, take the resource transition. Otherwise induct
on the typing derivation. Exhaustive matches cover every constructor. A fold over a constructor tree has a next
structural case. Track and machine operations either wait for their left-to-right arguments or reduce on values. A1
supplies the source primitive case, whose declared `Result` value covers ordinary primitive failures. ∎

### Theorem 2.4: source evaluation is deterministic

A closed source term has at most one next step.

**Proof outline.** The evaluation contexts choose one leftmost unevaluated argument. Match branches are disjoint after
the coverage checker has compiled them. Track operations are functions. Machine operations are constructors. A1 makes
primitive reduction deterministic. A7 makes the resource boundary deterministic. ∎

### Theorem 2.5: every accepted source term terminates

Every closed well-typed source term with a fixed finite budget reaches `done(v)` or `failed(ResourceError)` after
finitely many steps. With no resource bound, it reaches `done(v)`.

**Proof outline.** Use a reducibility argument for the simply typed lambda calculus with strictly positive data. At an
arrow type, a value is reducible when it maps every reducible input to a reducible output. At a data type, every child
is reducible. Structural folds recurse only into strict constructor children. `map_events` traverses a finite occurrence
multiset and applies a reducible callback to each payload. Machine terms do not recurse: they build finite syntax trees.
A1 supplies reducibility for source primitives because their results are storable finite data. A finite budget can only
stop this reduction earlier. There is no fixpoint or recursive value binding. ∎

The direct proof is shorter than a translation because the only new higher-order operation, `map_events`, has an obvious
finite measure. A later formalization may still translate the ordinary fragment into a known strongly normalizing core
as a cross-check.

## 3. Event-track claims

### Lemma 3.1: track operations preserve bounds

If all occurrences of `x` and `y` lie within their track lengths, then every occurrence of `follow(x,y)`,
`together(x,y)`, and `map_events(f,x)` lies within the returned length.

**Proof.** Write `x = (d,E)` and `y = (q,F)`.

- `follow` keeps every event in `E` inside `[0,d]`. An event `(s,e,a)` in `F` satisfies `0 <= s <= e <= q`; after adding
  `d`, it satisfies `d <= d+s <= d+e <= d+q`.
- `together` returns length `max(d,q)`, which is at least both old lengths.
- `map_events` does not change starts, ends, or length.

Thus all returned occurrences satisfy the bounds. ∎

### Theorem 3.2: track construction is total

Every well-typed closed track expression evaluates to one finite well-formed `EventTrack`.

**Proof.** Theorem 2.5 gives termination. `empty` and `event` are finite and well formed by construction. Lemma 3.1
handles every combining operation. ∎

### Theorem 3.3: the stated track laws hold

`follow` is associative with `empty(0)` as unit. `together` is associative and commutative with `empty(0)` as unit.
`map_events` preserves both operations and obeys the identity and composition laws.

**Proof.** `follow` reduces to rational addition, translation, and multiset union. Their associativity gives the result.
`together` reduces to `max` and multiset union; both are associative and commutative, and zero and the empty multiset
are their units on nonnegative lengths. Mapping changes only payloads, so it distributes over translations and multiset
union. ∎

The theorem does not claim that `together` is idempotent. Multiset union preserves two equal events.

## 4. Machine claims

### Theorem 4.1: every machine has one total next step

For any well-typed `m : Machine<K,A,B>`, valid private state `s`, and input `a : A`, there is exactly one pair `(s2,b)`
such that `step_m(s,a) = (s2,b)`.

**Proof.** Induct on the finite syntax of `m`.

- A primitive has one result by A2.
- `identity`, `copy`, `drop`, and `swap` are direct total functions.
- For `connect(m,n)`, the induction hypothesis gives one result from `m`; its output has the exact input type of `n`,
  whose induction hypothesis gives one result.
- For `beside(m,n)`, apply the two induction hypotheses to the two input components.
- For `feedback(initial,m)`, the stored feedback value has the required type. The induction hypothesis for `m` gives one
  output pair. Store its feedback component and return its public component.

Every case terminates after finitely many child steps. ∎

### Theorem 4.2: machine output is causal

If two input histories agree through step `n`, then a machine started from the same description produces equal outputs
through step `n`.

**Proof.** Induct on the step number. The initial private states are equal because the machine description, including
every explicit seed, is equal. At step zero, equal inputs and Theorem 4.1 give equal output and next state. If the
states and inputs agree at step `j`, Theorem 4.1 gives equal output and state at `j+1`. Feedback reads only the stored
value from step `j-1`, with the explicit initial value used at zero. ∎

### Corollary 4.3: feedback always has a first output

Every well-typed `feedback(initial,m)` can take step zero.

**Proof.** Its initial combined state contains both the start state of `m` and the supplied `initial` value. Theorem 4.1
then applies. ∎

This is the claim that Drafts A and B lacked. It does not admit an instantaneous algebraic loop.

### Theorem 4.4: machine connection has identity and associativity

Under behavioral equality, `identity` is the left and right unit of `connect`, and `connect` is associative.

**Proof.** For the unit laws, one identity step returns its input unchanged and changes no state. For associativity,
compare one step of

```text
connect(connect(m,n),p)
```

with one step of

```text
connect(m,connect(n,p)).
```

Both run `m`, then `n`, then `p` on the same intermediate values. Their state products differ only by parentheses.
Induction on the input history gives equal output histories. Explicit primitive seeds stay in the same leaves, so
regrouping does not change them. ∎

### Theorem 4.5: side-by-side connection obeys interchange

Given machines with matching types,

```text
connect(beside(m,n), beside(p,q))
```

is behaviorally equal to

```text
beside(connect(m,p), connect(n,q)).
```

**Proof.** Both sides take `(a,d)`, run `m` on `a` and `n` on `d`, then run `p` and `q` on the respective outputs. Their
state products differ only by a fixed rearrangement. Induct on the input history. ∎

These two theorems say that machine wiring forms a symmetric monoidal category: it has chain composition and a
side-by-side operation, both with the stated regrouping laws. No categorical product or stronger trace law is claimed.

## 5. Scheduling and audio

### Definition 5.1: boundary messages

For each non-point occurrence, a scheduler normally emits one `Begin(id,payload)` message at its chosen start frame and
one `End(id)` message at its chosen end frame. A point occurrence emits one `Point(id,payload)` message. A collapse
policy may instead reject a non-point occurrence, expand it to one or more frames, or place an ordered `Begin` and `End`
in one batch. `id` is an opaque handle used only for equality and for pairing that occurrence's boundaries.

Messages assigned to one frame use the exact order stored in the schedule table. The scheduler version fixes how that
order is produced. A successful generic scheduler therefore requires a canonical finite encoding for its payload type,
or an explicit ordering function supplied as an argument. Handle spellings do not take part in this order.

### Theorem 5.2: successful scheduling emits every boundary exactly once

If scheduling succeeds, stepping the scheduled source through the final assigned frame emits exactly the boundary
messages described in Definition 5.1, in their fixed order, with no extra messages.

**Proof.** The algorithm visits every occurrence once after sorting. For each successful occurrence it creates the one
set of messages required by the collapse policy and inserts each message into exactly one frame entry. It creates no
other messages. The final per-frame sort changes only order. The returned source stores that finite table, a cursor, and
a bounded countdown to the next entry. Induction through the finite table proves that it emits every stored entry at its
assigned frame. After the final entry it reaches `Finished`, whose only output is an empty batch. ∎

### Theorem 5.3: a prepared score-to-audio machine has one audio history

Suppose scheduling succeeds, every connected machine type matches, and every primitive satisfies A2. For fixed live
input, the connected scheduler, instrument, and effects produce exactly one audio frame at every finite step.

**Proof.** The scheduler is a machine by Theorem 5.2. Repeated use of Theorem 4.1 gives one next frame and state. ∎

This theorem is deliberately weaker than “the score determines the sound.” Performance choices, time map, patch, format,
seeds, live input, and effect settings are all explicit premises.

### Theorem 5.4: scheduling preserves simultaneous placement

Suppose the combined call and both separate calls succeed for one time map and one occurrence-local scheduling policy.
Then scheduling `together(x,y)` is behaviorally equal, up to a consistent one-to-one renaming of private handles, to
scheduling `x` and `y` separately, running the two sources beside one another, and connecting them to the specified
event-batch merger.

**Proof.** `together` takes multiset union without changing any position. A6 gives equal time decisions to equal copies
and makes every other decision depend only on that occurrence. The combined schedule table therefore assigns the union
of the two separate message sets to every frame. The merger's left and right injections keep the separate handles
distinct. Rename those injected handles to the handles chosen by the combined table. The fixed merge operation sorts
that same union by the same policy, which does not inspect handle spellings. The two source machines then emit equal
batches at every frame. ∎

There is no unconditional corresponding theorem for `follow`. A nonlinear time map or non-additive rounding rule may
send `d + s` to a different frame from the sum of the separately converted `d` and `s`. Under explicit additivity
hypotheses, the proof reduces to rational addition and table shifting.

### Corollary 5.5: simultaneous scheduling remains equal after an instrument

Under A8, connecting the two sources in Theorem 5.4 to the same instrument produces the same audio history.

**Proof.** Theorem 5.4 supplies a consistent handle renaming. A8 says that the instrument's non-handle outputs are
unchanged by that renaming. Repeated machine steps therefore give equal audio histories. ∎

### Theorem 5.6: valid whole-machine batching preserves sound

If a machine's `batch(n)` method equals `n` repeated calls to that machine's frame step for every valid state and input
block, then replacing those repeated calls with `batch(n)` does not change the machine's state or outputs.

**Proof.** This is the stated whole-machine contract. A feedback-free machine inherits such a method from valid child
batches by structural induction through `connect` and `beside`. A feedback machine does not inherit one by this
argument: its later feedback inputs are earlier outputs from the same block. It must use repeated frame steps or supply
a separately checked whole-machine batch. ∎

The primitive owner must state this contract for each optimized implementation. A proof or exhaustive check can
establish it. Differential tests can find violations and prevent regressions, but testing alone does not prove the
universal claim. The type system does not establish it.

## 6. What is proved, assumed, and still open

### Derived in this note

- event-track closure and algebra;
- total deterministic machine steps from A2;
- causal stored feedback;
- connection, side-by-side composition, and their laws; and
- conditional scheduling and audio-history results.

### Standard proof route, not written in full

- principal Hindley–Milner inference;
- preservation, progress, and deterministic source evaluation; and
- strong normalization with strictly positive folds.

### Assumed ownership contracts

- source primitives satisfy A1;
- machine primitives satisfy A2;
- time-map and scheduling-policy operations satisfy A5; and
- optimized audio batches satisfy Theorem 5.6's whole-machine premise.

### Not proved

- a real-time deadline for arbitrary prepared machines;
- semantic equality of arbitrary machines;
- adequacy of any culture-specific music library;
- uniqueness of performance, transcription, or analysis;
- correctness of the current Musa compiler or audio engine against this candidate; or
- preservation of source lineage through future lowering passes.
