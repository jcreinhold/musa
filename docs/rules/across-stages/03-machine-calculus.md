# Rules for machines

This chapter defines four things: what a machine is, what one step means, when a machine may be prepared for audio, and
how a finite event track becomes a running source of events. A machine is a source value; the state it carries and the
flattened layout an implementation may choose are private.

This chapter replaces the audio process-graph rules. A graph of nodes, ports, and register edges was a second calculus
with its own scheduler, and the whole-node schedule it required was the mechanism by which host callback size could
change what a piece sounded like. Machines have no scheduler: the step of a composite is defined by the steps of its
parts.

## 1. Primitive units

A primitive instance `p : Primitive<K,A,B>` contains a finite name, a version, and a storable configuration. A unit that
uses chance stores its seed in that configuration.

Its owner supplies a private state type `State(p)` and two total deterministic functions:

```text
start_p : Unit -> State(p)
step_p  : State(p) × A -> State(p) × B
```

`State(p)` is not a source type. Source code can neither inspect nor forge one. A step may read only its configuration,
its private state, and its current input. A microphone sample or a controller message arrives through the input; it is
never hidden global state.

One build uses one finite primitive registry. A pair `(name, version)` selects exactly one state layout, configuration
codec, start function, step function, resource contract, and optional batch contract. Registration rejects a conflict.
This is a build-local execution rule, not a promise of persistent compiled identity.

For audio, the contract also states fixed memory and a worst-case step cost. The type system does not prove a deadline;
preparation checks the contract before the engine accepts the machine.

## 2. Machine values

```text
m, n ::= machine(p)          % a registered primitive instance
       | identity
       | connect(m, n)       % chain: m's output feeds n's input
       | beside(m, n)        % side by side, on a pair
       | feedback(initial, m)
       | copy | drop | swap
```

The typing rules require storable ports:

```text
data A   data B   Γ ⊢ p : Primitive<K,A,B>          data A
──────────────────────────────────────────         ───────────────────────────
Γ ⊢ machine(p) : Machine<K,A,B>                    Γ ⊢ identity : Machine<K,A,A>

Γ ⊢ m : Machine<K,A,B>   Γ ⊢ n : Machine<K,B,D>
─────────────────────────────────────────────────
Γ ⊢ connect(m,n) : Machine<K,A,D>

Γ ⊢ m : Machine<K,A,B>   Γ ⊢ n : Machine<K,D,E>
──────────────────────────────────────────────────────
Γ ⊢ beside(m,n) : Machine<K,(A,D),(B,E)>

data F   Γ ⊢ initial : F   Γ ⊢ m : Machine<K,(A,F),(B,F)>
──────────────────────────────────────────────────────────
Γ ⊢ feedback(initial,m) : Machine<K,A,B>

copy : Machine<K,A,(A,A)>    drop : Machine<K,A,Unit>    swap : Machine<K,(A,B),(B,A)>
```

`copy`, `drop`, and `swap` are wiring, not musical or audio operations. `beside` keeps two outputs; **a mixer is a
primitive** from a pair of frames to one frame, never an implied meaning of parallel placement.

There is no public `lift` from a source function into a machine. The step tag `K` prevents machines whose steps mean
different things from being connected.

## 3. One step

Every machine has a private combined state, a start state, and one step function, defined from its finite syntax:

```text
State(identity) = State(copy) = State(drop) = State(swap) = Unit
State(connect(m,n)) = State(beside(m,n)) = State(m) × State(n)
State(feedback(initial,m)) = State(m) × F

start(machine(p))          = start_p(())
start(connect(m,n))        = (start(m), start(n))
start(beside(m,n))         = (start(m), start(n))
start(feedback(initial,m)) = (start(m), initial)
```

```text
step_identity((), a) = ((), a)

step_connect((s,t), a):
    let (s2, b) = step_m(s, a)
    let (t2, d) = step_n(t, b)
    return ((s2,t2), d)

step_beside((s,t), (a,d)):
    let (s2, b) = step_m(s, a)
    let (t2, e) = step_n(t, d)
    return ((s2,t2), (b,e))

step_feedback((s, old), a):
    let (s2, (b, next)) = step_m(s, (a, old))
    return ((s2, next), b)
```

The feedback rule is the whole feedback rule. `old` is the explicit initial value at step zero and the previous step's
returned value after that. No current output is ever read as a current input, so there is no algebraic loop to reject
and no delay to infer from the shape of a graph.

We write one step as:

```text
m ⊢ (σ, ι) -> (σ', o)
```

Repeating it from `start(m)` on an input history `ι₀, ι₁, …` produces an output history `o₀, o₁, …`. The machine is
finite; the histories need not be.

## 4. What one audio step is

For audio, `K` is `AudioFrameStep` and **one step is one sample frame** at the prepared rate. A mono frame carries one
sample, a stereo frame two. The control input for a frame is a finite prepared event batch.

The sample rate is a value in an explicit `AudioFormat`, not a type index. A user may choose it at run time.

A batch method over `n` frames may replace `n` repeated steps only under this contract:

```text
batch(n, state, inputs[0..n])
  =
step once on inputs[0], then on inputs[1], …, then on inputs[n-1]
```

for every valid state and input block. A feedback-free machine inherits a valid batch method from valid child methods
through `connect` and `beside`. **A feedback machine does not**: its later feedback inputs are earlier outputs of the
same block, so it runs frame by frame unless a separately checked whole-machine batch method exists. A unit that needs
internal blocks, such as an FFT effect, buffers frames in its private state and states its latency.

Running `n + m` frames in one request therefore gives the same samples and the same final state as running `n`, keeping
the state, and running `m` more. That is a consequence of the step rule, not an extra requirement on it.

## 5. Preparation

Before allocation:

```text
prepare_audio(format, machine) -> Result<PreparedMachine, PrepareError>
```

checks that every primitive in the machine is registered and accepts the chosen format, that external frame layouts
agree, and that the stated memory limits and worst-case step costs are satisfied. A well-typed machine may still fail
preparation — an oscillator configured for 44.1 kHz in a 48 kHz render is a configuration error, reported as one.

The product-level operation `prepare_execution(gestures, bindings, seed, options)` builds the machine from gestures,
instrument implementations, routing, and effects, and then runs this check. `04-identity-and-realization.md` states its
determinism law.

## 6. Scheduling an event track into a running source

```text
schedule(format, policy, time_map, track)
    -> Result<Schedule<A>, ScheduleError>

Schedule<A> = {
    machine   : Machine<AudioFrameStep, Unit, EventBatch<A>>,
    decisions : List<TimeDecision<C>>,
}
```

`TimeMap<C>` maps source positions in coordinate `C` to exact physical time. It may encode strict tempo, rubato, swing,
fermatas, or a timing captured from a person. `SchedulePolicy` maps those exact times to bounded integer frames, fixes
the order of messages landing on one frame, and says which collapses are allowed. Both are ordinary checked data
supplied by the caller.

The algorithm is finite:

1. sort the track's occurrences by start, end, and the payload's exact encoding, retaining a copy number for exact
   duplicates;
2. ask the time map and policy for each occurrence's start and end frame;
3. reject an unrepresentable or negative frame, an end before its start, an unanswered boundary, a reversal of two
   ordered source boundaries, or a collapse the policy forbids;
4. create one opaque handle and the required boundary messages per occurrence;
5. collect messages by frame and sort each batch by the policy's fixed key, which never inspects handle spelling; and
6. store the finite table, the decisions, a cursor, and a bounded countdown in the returned source machine.

A non-point occurrence normally emits `Begin(handle, payload)` at its start frame and `End(handle)` at its end frame; a
point emits `Point(handle, payload)`. A collapse policy may instead reject the occurrence, expand it to a minimum
duration, or place an ordered `Begin` and `End` in one batch — and the decision record says which happened.

At frame `j` a connected machine reads the batch **before** producing output frame `j`. A `Begin` therefore affects its
start frame and an `End` prevents the occurrence from sounding on its end frame, which is the audio meaning of the
half-open span in §3 of `01-stage-judgments.md`.

After the last stored batch the source enters `Finished`: it returns `Finished` and an empty batch forever, so it uses
fixed memory no matter how long the engine keeps stepping. An instrument may still produce a release or reverb tail from
its own state.

Handles are opaque. A primitive may compare them for equality and use them to pair a start with an end. Merging two
scheduled sources relabels left and right handles into disjoint sets before applying the policy's order, so handles
created by one source cannot collide with equal-looking handles created by the other. That merge is a registered
`merge_event_batches(policy)` machine, not plain pair wiring.

## 7. Basic theorems

**Theorem M1: one step has one result.** For a well-typed `m : Machine<K,A,B>`, a valid state `s`, and an input `a : A`,
there is exactly one `(s2, b)` with `step_m(s,a) = (s2,b)`.

**Proof.** Induct on the finite syntax of `m`. A primitive has one result by its registered contract. `identity`,
`copy`, `drop`, and `swap` are total functions. `connect` applies the hypothesis for `m`, whose output has exactly `n`'s
input type, then the hypothesis for `n`. `beside` applies the two hypotheses to the two components. `feedback` supplies
the stored value at the required type and applies the hypothesis for its child, then splits the returned pair. Each case
finishes after finitely many child steps. ∎

**Theorem M2: machines are causal.** If two input histories agree through step `n`, the same machine started from the
same description produces equal outputs through step `n`.

**Proof.** Induct on the step number. The start states are equal because the description, including every explicit seed,
is equal. Step zero follows from M1 on equal inputs. If states and inputs agree at step `j`, M1 gives equal output and
state at `j+1`. Feedback reads only the value stored at step `j-1`, with the explicit initial value at zero. ∎

**Corollary M3: feedback always has a first output.** The initial combined state of `feedback(initial,m)` contains both
`start(m)` and `initial`, so M1 applies at step zero. This is what an ordinary-wire cycle could not supply, and it is
supplied without a scheduler.

**Theorem M4: chain composition has identity and associativity.** Under behavioural equality, `identity` is the left and
right unit of `connect`, and `connect` is associative.

**Proof.** One `identity` step returns its input and changes no state. For associativity, one step of
`connect(connect(m,n),p)` and one step of `connect(m,connect(n,p))` both run `m`, then `n`, then `p` on the same
intermediate values; their state products differ only by parentheses. Induct on the input history. Explicit seeds stay
in the same leaves, so regrouping reseeds nothing. ∎

**Theorem M5: interchange.** `connect(beside(m,n), beside(p,q))` is behaviourally equal to
`beside(connect(m,p), connect(n,q))`.

**Proof.** Both take `(a,d)`, run `m` on `a` and `n` on `d`, then run `p` and `q` on the respective outputs; their state
products differ by a fixed rearrangement. Induct on the input history. ∎

M4 and M5 say that machine wiring is a symmetric monoidal category: it has chain composition and a side-by-side
operation with the stated regrouping laws. **No categorical product and no trace law is claimed.** `copy` and `drop`
exist as fixed machines, but they need not commute with an arbitrary stateful machine, and nothing in that name says a
score is a sound.

**Theorem M6: scheduling emits every boundary exactly once.** If scheduling succeeds, stepping the scheduled source
through the final assigned frame emits exactly the boundary messages of §6, in their stored order, and nothing else.

**Proof.** The algorithm visits each occurrence once after sorting, creates the one message set the collapse policy
requires, and inserts each message into exactly one frame entry. The per-frame sort changes only order. The source
stores that finite table with a cursor and a bounded countdown; induction through the table shows every entry is emitted
at its assigned frame, after which `Finished` emits only empty batches. ∎

**Theorem M7: scheduling preserves simultaneous placement.** Call a policy **occurrence-local** when each boundary's
frame depends only on that occurrence, the time map, and the policy; when exact duplicate occurrences receive equal time
decisions, their copy numbers only keeping handles apart; and when a same-frame collision is resolved by ordering alone,
never by dropping or shifting.

Suppose the combined call and both separate calls succeed under one such policy. Then scheduling `together(x,y)` is
behaviourally equal — up to a consistent one-to-one renaming of private handles — to scheduling `x` and `y` separately,
running the two sources beside one another, and connecting the pair to `merge_event_batches(policy)`.

**Proof.** `together` is multiset union and moves no position. Occurrence-locality gives every occurrence the same
decision in the combined call as in its own call, and equal copies equal decisions. The combined table therefore assigns
the union of the two message sets to every frame. The merger's left and right injections keep the separate handles
distinct; rename those injected handles to the combined table's. The merge sorts the same union by the same policy,
which does not inspect handle spelling, so the two constructions emit equal batches at every frame. ∎

**Corollary M8: the same audio follows.** If handles are opaque in the sense of `obligations.md` §15 — a primitive may
compare them and may rename its private tables under a consistent renaming, but its non-handle outputs do not change —
then connecting either construction of M7 to the same instrument produces the same audio history.

**There is no corresponding unconditional theorem for `follow`,** and inventing one would be a lie. A nonlinear time map
or a non-additive rounding rule may send `d + s` to a frame other than the sum of the separately converted `d` and `s`.
Scheduling the combined track is authoritative and its decision record exposes the difference. Under an additive time
map and an additive frame conversion, `follow` is preserved by delaying the second scheduled source by the scheduled
duration of the first.

## 8. What the private runtime may and may not do

An implementation may flatten a machine tree into arrays of units and buffers, preallocate, reuse buffers, and
vectorize. It must preserve the step equations of §3 exactly: the public meaning is those equations, not whatever order
a layout pass happens to choose. Differential tests against a plain structural interpreter are the evidence that a
flattening is faithful; they are not a substitute for the contract in §4.
