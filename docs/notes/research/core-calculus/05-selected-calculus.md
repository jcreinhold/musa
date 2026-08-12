# The selected calculus

**Status: research candidate. This document does not set Musa's rules.**

## Purpose

This document gives one small core language for Musa. It has one job:

> A finite source program builds finite values, finite event tracks, and finite machines. Event tracks state what occurs
> when. Machines state what happens one input step at a time.

The core includes audio without treating an endless audio stream as a finite source value. A finite machine produces the
stream step by step.

## 1. The whole design in one example

The following is paper syntax, not yet accepted `.musa` code.

```musa
let gestures = interpret(score, performance_choices)?
let source = schedule(audio_format, schedule_policy, tempo, gestures)?
let synth = instrument(audio_format, patch, render_seed)?
let space = room(audio_format, hall)?

let sound = connect(source.machine,
    connect(synth, space))
```

The values have these types:

```text
score            : EventTrack<WrittenTime, WrittenFact>
gestures         : EventTrack<PerformedTime, Gesture>
source.machine   : Machine<AudioFrameStep, Unit, EventBatch<Gesture>>
synth            : Machine<AudioFrameStep, EventBatch<Gesture>, StereoFrame>
space            : Machine<AudioFrameStep, StereoFrame, StereoFrame>
sound            : Machine<AudioFrameStep, Unit, StereoFrame>
```

Compilation evaluates the source and finishes with the finite `sound` value. Audio preparation checks that every unit
uses the chosen rate and layout. Running the prepared machine advances it once per audio frame. The engine may process
many frames in one host callback, but that batching may not change the result.

This path is optional. A piece may begin with a microphone, a live controller, a sample, a phrase description, or a
score. The type system requires only that connected inputs and outputs agree.

## 2. Ordinary types and programs

The ordinary language is strict, pure, and total. Its core types are:

```text
A, B ::= a
       | Unit | Bool | Nat | Ratio | Text
       | A * B
       | A + B
       | List<A>
       | N<A, ...>
       | A -> B
       | Length<C>
       | EventTrack<C, A>
       | Primitive<K, A, B>
       | Machine<K, A, B>
```

`a` is a type variable. `N` is a nominal data type declared by a library. `C` is an ordinary nominal tag for a time
coordinate. `K` is an ordinary nominal tag for one kind of machine step. They are type parameters, not values inside
types.

### 2.1 Values and storable data

Every well-formed type is a **value type**. Some are also **storable data types**. Storable data contains no source
function at any depth and has a versioned, finite, exact structural encoding. A digest may help find an encoding; only
the full encoding decides equality.

The base types and `Length<C>` are storable data. A product, sum, list, or nominal constructor is storable data when all
of its stored fields are. `EventTrack<C,A>` is storable data when `A` is storable data. `Primitive<K,A,B>` and
`Machine<K,A,B>` are storable data when `A` and `B` are storable data and every stored configuration value is storable
data. These conditions are also their well-formedness rules.

A source function `A -> B` is a value type but never storable data. Therefore a list of source functions may be used
during source evaluation, but it cannot be put in an event track, machine port, feedback value, primitive configuration,
or foreign primitive call.

An abstract compiler-owned type counts as storable data only when its owner guarantees that its hidden representation
contains no source closure and supplies the exact encoding. A machine primitive contains an id and version for runtime
code, not a source closure.

The compiler checks nominal declarations once. It groups mutually recursive types, rejects any stored function field,
and accepts the group as storable data when every field leaving the group is already storable data. Thus `Tree<Nat>` may
be storable data while `Box<Unit -> Unit>` is not. The check terminates because the declaration graph is finite.

Type variables carry the same distinction. An ordinary variable `a` may stand for any value type. A data variable `d`
may stand only for storable data. Unification never replaces `d` with a function or a container that holds one. This is
a small check on ordinary Hindley–Milner inference; it is not subtyping, overloading, or a source-visible type class.

The core terms are:

```text
e ::= x | literal | (e, e) | inl e | inr e
    | fn x => e | e(e) | let x = e in e
    | Constructor(e, ...) | match e { cases }
    | fold_N(e, cases)
    | track operation
    | machine operation
```

The language has:

- non-recursive `let`;
- finite strictly positive data with a generated fold;
- exhaustive pattern matching;
- rank-1 Hindley–Milner type inference; and
- build-local nominal type identity.

It has no general recursion, subtyping, overloading, implicit conversion, higher-rank type, or value-dependent type.
Ordinary calls are complete. A written multi-argument function takes one product argument. A closure is written
explicitly.

Evaluation is left to right and call by value. These choices make the value of a term and its resource charge easy to
follow from the source.

### 2.2 Source evaluation

A source value is a literal, constructor whose fields are values, pair of values, closure, finished event track,
primitive description, or finished machine description. Evaluation follows these rules:

1. evaluate a function, then its complete argument, then its body;
2. evaluate a `let` binding before its body;
3. evaluate the subject of a match, choose its one matching branch, then evaluate that branch;
4. evaluate constructor fields and operation arguments from left to right; and
5. call a source primitive only after every argument is a value.

A fold replaces one constructor layer with its declared case and folds only the constructor's strict children. Track
operations traverse finite occurrence sets. Machine operations build finite descriptions; they never call a machine's
step function during source evaluation.

A typed evaluation configuration for a source result `A` is one of:

```text
run(remaining_budget, e)   where e : A
done(v)                    where v : A
failed(ResourceError)      still recorded as an evaluation of A
```

A versioned table assigns a nonnegative integer cost to each reduction and each constructed value. A step subtracts its
fixed cost. A step that would cross zero enters `failed(ResourceError)`. Costs never read wall time, allocator behavior,
or machine load. The budget can stop evaluation, but it cannot change the value returned by an accepted term.

`run(budget,v)` enters `done(v)` at zero cost when `v` is already a value. Otherwise the one leftmost next reduction is
chosen first; its fixed charge either permits that step or yields the one resource failure. These priorities leave no
case in which both success and failure are possible.

A source primitive that can fail returns an ordinary `Result` value. It does not create a second hidden error channel.

### 2.3 Why this follows the functional-language literature

Peyton Jones recommends a rich surface followed by a small enriched lambda language, with `let`, constructors, and cases
kept long enough to preserve sharing and compile patterns cleanly. Musa uses that compiler shape. It does not copy the
book's laziness, unrestricted recursion, or graph-reduction runtime
(`implementation-of-functional-programming-languages/03-translating-a-high-level-functional-language-into-the-lambda-calculus.md`,
`06-transforming-the-enriched-lambda-calculus.md`).

## 3. Exact lengths

`Length<C>` contains a nonnegative exact rational number tagged by `C`. The owner of `C` supplies named constructors so
source code does not need type arguments. For example, a score library may supply:

```text
beats   : Ratio -> Result<Length<WrittenTime>, TimeError>
seconds : Ratio -> Result<Length<SecondTime>, TimeError>
```

Lengths with different tags do not unify. Adding a written beat to an audio-frame length is a type error.

The core uses only zero, addition, order, and maximum on lengths with the same tag. Meter is not present. A meter,
groove, tempo, or conducted timing plan is ordinary library data.

## 4. Finite event tracks

### 4.1 Definition

An `EventTrack<C,A>` is a pair `(d,E)` where:

- `d` is a `Length<C>`; and
- `E` is a finite multiset of occurrences `(s,e,a)` with `0 <= s <= e <= d`.

A positive occurrence occupies the half-open span `[s,e)`: it includes its start and excludes its end. An occurrence
with `s = e` is a point at `s`.

The multiset keeps duplicates. Two violinists playing equal written notes are two occurrences.

The public core operations are:

```text
empty      : Length<C> -> EventTrack<C, A>
event      : Length<C> -> A -> EventTrack<C, A>
follow     : EventTrack<C, A> * EventTrack<C, A> -> EventTrack<C, A>
together   : EventTrack<C, A> * EventTrack<C, A> -> EventTrack<C, A>
map_events : (A -> B) * EventTrack<C, A> -> EventTrack<C, B>
length     : EventTrack<C, A> -> Length<C>
```

The typing rules are direct:

```text
data A    Gamma |- d : Length<C>
---------------------------------- Track-empty
Gamma |- empty(d) : EventTrack<C,A>

data A    Gamma |- d : Length<C>    Gamma |- a : A
-------------------------------------------------- Track-event
Gamma |- event(d,a) : EventTrack<C,A>

Gamma |- x : EventTrack<C,A>    Gamma |- y : EventTrack<C,A>
---------------------------------------------------------------- Track-follow
Gamma |- follow(x,y) : EventTrack<C,A>

Gamma |- x : EventTrack<C,A>    Gamma |- y : EventTrack<C,A>
---------------------------------------------------------------- Track-together
Gamma |- together(x,y) : EventTrack<C,A>

data A    data B    Gamma |- f : A -> B    Gamma |- x : EventTrack<C,A>
--------------------------------------------------------------------- Track-map
Gamma |- map_events(f,x) : EventTrack<C,B>
```

### 4.2 Meaning

Let `x = (d,E)` and `y = (q,F)`.

```text
empty(d)        = (d, empty multiset)
event(d,a)      = (d, {(0,d,a)})
follow(x,y)     = (d + q, E union shift(d,F))
together(x,y)   = (max(d,q), E union F)
map_events(f,x) = (d, {(s,e,f(a)) | (s,e,a) in E})
```

Here `union` is multiset union. `shift(d,F)` adds `d` to every start and end in `F`.

All five operations are total on well-typed values. `event(0,a)` is a point event. A late event is built by following an
empty track with an event. A trailing empty track fixes a longer surrounding length.

### 4.3 Laws

The following equations use event-track equality: equal length and equal occurrence multiset.

```text
follow(follow(x,y),z) = follow(x,follow(y,z))
follow(empty(0),x) = x = follow(x,empty(0))

together(together(x,y),z) = together(x,together(y,z))
together(x,y) = together(y,x)
together(empty(0),x) = x

map_events(identity,x) = x
map_events(g,map_events(f,x)) = map_events(g after f,x)
```

`map_events` also preserves `follow` and `together`.

`together(x,x)` is not equal to `x`; duplicates remain. `follow` and `together` do not satisfy a general interchange
law. They are two different ways to build time.

### 4.4 What the track does not say

An empty interval is absence of `A`. It is not a rest unless `A` contains a written-rest value. It is not audible
silence unless a later audio unit produces zero samples.

An event payload owns musical meaning. A notation package may preserve ties, separate noteheads, spelling, staff,
articulation, and meter. A performance package may use gestures. The track knows only placement.

For this reason, `EventTrack` is a clearer name than `Timeline`. It names the stored object: finite events placed within
a finite length. `length` is clearer than “musical extent.”

## 5. Machines that run one step at a time

### 5.1 Primitive units

A primitive instance `p : Primitive<K,A,B>` contains a finite name, version, and configuration. A random primitive
stores its seed in that configuration. Its owner supplies a private state type `State(p)` and two total deterministic
functions:

```text
start_p : Unit -> State(p)
step_p  : State(p) * A -> State(p) * B
```

`State(p)` is not a source type. Source code cannot inspect or forge it. A step may read only its configuration, private
state, and current input. A microphone sample or controller message arrives through the input; it is not hidden global
state.

One build uses one finite primitive registry. A pair `(name,version)` selects one exact state layout, configuration
codec, start function, step function, resource contract, and optional batch contract. Registration rejects a conflict.
This is a build-local execution rule, not a promise of persistent compiled identity.

Foreign **source** primitives are separate from machine primitives. A source primitive is first order: none of its
argument or result types contains a source function, even inside a list, constructor, event track, primitive
configuration, or machine description. Higher-order finite operations such as `map_events` are defined by the source
calculus and use its termination proof. This rule prevents foreign code from hiding a looping source closure inside an
apparently finite value. In the type checker, this says simply that every complete primitive argument and result is
storable data.

For audio, the primitive contract also states fixed memory and a worst-case step cost. The ordinary type system does not
prove a deadline. Audio preparation checks those resource contracts before the engine accepts a machine.

### 5.2 Machine values

A `Machine<K,A,B>` is one of these finite values:

```text
machine(p)            where p : Primitive<K,A,B>
identity
connect(m,n)
beside(m,n)
feedback(initial,m)
copy
drop
swap
```

There is no public `lift` from an arbitrary source function.

The main typing rules are:

```text
data A    data B    Gamma |- p : Primitive<K,A,B>
-------------------------------- Machine-primitive
Gamma |- machine(p) : Machine<K,A,B>

data A
-------------------------------- Machine-identity
Gamma |- identity : Machine<K,A,A>

Gamma |- m : Machine<K,A,B>    Gamma |- n : Machine<K,B,D>
---------------------------------------------------------------- Machine-connect
Gamma |- connect(m,n) : Machine<K,A,D>

Gamma |- m : Machine<K,A,B>    Gamma |- n : Machine<K,D,E>
----------------------------------------------------------------------- Machine-beside
Gamma |- beside(m,n) : Machine<K,(A,D),(B,E)>

data F    Gamma |- initial : F    Gamma |- m : Machine<K,(A,F),(B,F)>
---------------------------------------------------------------- Machine-feedback
Gamma |- feedback(initial,m) : Machine<K,A,B>
```

`copy`, `drop`, and `swap` have the expected types:

```text
copy : Machine<K,A,(A,A)>
drop : Machine<K,A,Unit>
swap : Machine<K,(A,B),(B,A)>
```

These are wiring, not musical or audio operations. `beside` keeps two outputs. An audio mixer is a primitive machine
from a pair of audio frames to one audio frame.

### 5.3 Exact step meaning

Every machine has a private combined state, a start state, and one step function. These are defined from its finite
syntax.

For a primitive, use the owner's state and functions. For the other forms:

```text
State(identity)       = Unit
State(connect(m,n))   = State(m) * State(n)
State(beside(m,n))    = State(m) * State(n)
State(feedback(initial,m)) = State(m) * F
```

`copy`, `drop`, and `swap` have `Unit` state.

The start equations are just as direct:

```text
start_machine(p)          = start_p(())
start_identity            = ()
start_connect(m,n)        = (start_m, start_n)
start_beside(m,n)         = (start_m, start_n)
start_feedback(initial,m) = (start_m, initial)
```

`copy`, `drop`, and `swap` also start in `()`.

The step equations are:

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
    let (s2, (b,next)) = step_m(s, (a,old))
    return ((s2,next), b)
```

The last rule is the whole feedback rule. `old` comes from the initial value at step zero and from the previous step
after that. No current output is read as a current input.

The fixed wiring units copy, discard, or exchange their current input and keep `Unit` state.

### 5.4 Seeds

Allocation has no hidden root seed. A unit that uses chance stores an explicit seed in its primitive configuration.
Preparation may take one render seed and split it into named unit seeds, but that split is part of the finite machine
description.

Using one primitive value twice creates two private states with the same explicit configuration. If two independent
random paths are wanted, preparation constructs two primitive values with two seeds. This rule makes regrouping a
machine tree harmless: `connect(connect(m,n),p)` and `connect(m,connect(n,p))` do not silently reseed any unit.

### 5.5 Machine laws

Two machine values are **behaviorally equal** when, for every input history, they produce the same output history.
Private states may differ in representation.

Under that equality:

- `identity` is the left and right unit of `connect`;
- `connect` is associative;
- `beside` is associative up to the explicit product rearrangement;
- `swap` gives symmetry; and
- connecting two side-by-side machines equals putting the two connections side by side, after the same rearrangement.

Formally, these operations give a symmetric monoidal category: machines can be wired in a chain or set beside one
another, and regrouping the drawing does not change what comes out. This does **not** claim that `beside` is a
categorical product. The fixed `copy` and `drop` machines exist, but they need not commute with an arbitrary stateful
machine.

Structural equality is different. Within one fixed primitive registry, it compares primitive ids, versions,
configurations, and the exact machine tree. Musa may decide structural equality. It must not try to decide behavioral
equality during type checking or caching.

Research on signal-flow diagrams and causal machines gives a broader mathematical setting for these laws
([Coya](https://arxiv.org/abs/1805.08290), [Bonchi, Di Lavore, and Román](https://arxiv.org/abs/2410.10627)). Musa needs
only the concrete rules above.

## 6. Audio has one-frame reference semantics

For audio, one `AudioFrameStep` means one sample frame at the prepared sample rate. A mono frame has one sample; a
stereo frame has two. Control input for that frame is a finite prepared event batch.

The sample rate is an explicit `AudioFormat` value, not a source type. A user may choose it at run time. Each audio
primitive records the formats it accepts. Before allocation,

```text
prepare_audio : AudioFormat * Machine<AudioFrameStep,A,B>
             -> Result<PreparedMachine<A,B>, PrepareError>
```

checks every primitive, the external frame layouts, memory limits, and step-cost contracts. A machine can therefore be
well typed yet fail preparation because an oscillator was configured for 44.1 kHz while the render requests 48 kHz. That
is a clear configuration error, not a reason to add value-dependent types.

The engine usually asks for a block. A batch implementation for a whole machine must equal repeated frame steps:

```text
run_block(n, state, inputs[0..n])
=
run step once for inputs[0], then once for inputs[1], ..., then once for inputs[n-1]
```

A feedback-free machine may combine valid primitive batch methods through `connect` and `beside`. Feedback is different:
the feedback value for a later frame comes from an earlier frame in the same block. A feedback machine therefore runs
its frame steps in order unless a specialized batch method for the **whole feedback machine** satisfies the displayed
contract. A unit that needs internal blocks, such as an FFT effect, buffers frames in its private state and states its
latency. Host callback size does not become musical or audio meaning.

This rule removes the current ambiguity where feedback may change with callback partition.

An unbounded run is not a source value. Given a machine, its start state, and an input history, the step rule defines an
output history one frame at a time. Any finite prefix is computable. Recording the first `n` frames returns a finite
`AudioClip`.

## 7. Scheduling connects tracks to audio

### 7.1 The checked conversion

For one chosen audio format, a scheduler has a type like:

```text
schedule : AudioFormat
        * SchedulePolicy
        * TimeMap<C>
        * EventTrack<C,A>
       -> Result<Scheduled<A>, ScheduleError>

Scheduled<A> = {
    machine : Machine<AudioFrameStep, Unit, EventBatch<A>>,
    decisions : List<TimeDecision<C>>
}
```

`TimeMap<C>` says how source positions map to physical time. It may encode strict tempo, rubato, swing, fermatas, or a
performance captured from a person. `SchedulePolicy` says how exact physical times become bounded integer frame numbers,
how same-frame messages are ordered, and which collapse choices are allowed. Both are ordinary checked library data, not
hidden global state. Scheduling asks the map only about the finite set of event boundaries in the input track. Success
requires a representable nonnegative frame assignment with every end at or after its start. A map or policy that cannot
answer one of those finite questions returns an error. Scheduling also checks order on the finite boundary set: if
source boundary `x` is no later than `y`, neither its exact physical time nor its assigned frame may be later than
`y`'s.

Rational positions need not land on integer frames. `schedule` records every rounding, collision, and ordering decision
or returns an error. It never calls approximate values definitionally equal.

The scheduler also creates an opaque handle for each occurrence so an instrument can pair its start and end. A unit may
compare handles for equality but may not inspect their numeric spelling. If a unit needs a random value per gesture,
that value belongs in the gesture payload or explicit primitive configuration, not in the accidental handle number.

Two scheduled sources are equal **up to handle renaming** when a consistent one-to-one renaming of their private handles
makes every event batch equal. Every instrument primitive must preserve this equality: renaming handles may rename its
private voice table, but may not change its audio output.

Merging two event sources must first relabel their handles into disjoint left and right sets, then apply the policy's
fixed message order. This is a registered `merge_event_batches(policy)` machine, not plain pair wiring. It prevents a
handle created by the left source from colliding with an equal-looking private handle created by the right source.

At frame `j`, the scheduled machine emits exactly the finite ordered messages assigned to `j`. The scheduling policy
states what happens when a start and end round to the same frame: it may reject the event, expand it to a minimum
length, or emit an ordered start-and-end pair. The decision record says which occurred. After the finite track ends the
machine emits empty batches. An instrument may continue producing a release or reverb tail through its state.

The instrument reads frame `j`'s batch before it emits audio frame `j`. A `Begin` message therefore affects its start
frame. An `End` message prevents the event from sounding on its end frame. This is the audio meaning of the half-open
span in §4.1. A `Point` message is read once at its assigned frame.

The scheduler is this finite algorithm:

1. sort the track's occurrences by start, end, and the payload's exact storable-data encoding; retain a copy number for
   exact duplicates;
2. ask the time map and policy for the start and end frame of each occurrence;
3. reject an unrepresentable or negative frame, an end before its start, an unanswered boundary, a reversal of two
   ordered source boundaries, or a collapse the policy forbids;
4. create one private handle and the required boundary messages for each occurrence;
5. collect messages by frame and sort each batch by the policy's fixed key, which does not inspect handle spelling; and
6. store the finite table, the decisions, a cursor, and a bounded countdown to the next batch in the returned source
   machine.

Every loop is over the finite occurrence list or one of its finite batches. The algorithm therefore returns a finite
schedule or a stated error.

After the source emits its last stored batch, it enters `Finished`. A step in `Finished` returns `Finished` and an empty
batch. The source therefore uses fixed memory no matter how long the engine keeps asking it to step.

### 7.2 What scheduling preserves

Scheduling is the exact algebraic connection between a finite track and a running source.

Call a scheduling policy **occurrence-local** when the frame chosen for one boundary depends only on its occurrence, the
time map, and the policy—not on neighboring events—and when a same-frame collision is resolved only by ordering, never
by dropping or shifting a message. Two occurrences with the same start, end, and payload must receive the same time
decision. The policy may use the private copy number only to keep their handles apart, not to move one copy.

If the combined call and both separate calls succeed under one occurrence-local policy, scheduling `together(x,y)` has
the same behavior, up to handle renaming, as scheduling `x` and `y` separately, running the two source machines beside
one another, and connecting their paired output to `merge_event_batches(policy)`. Both constructions emit the same
combined assignment table after private handles are renamed. For a policy that moves or removes one event in response to
another, this law is not claimed; the combined schedule and its decision record are authoritative.

The same unconditional law does **not** hold for `follow`. If a nonlinear time map or rounding policy maps `d + s`, it
need not equal the sum of the separately rounded images of `d` and `s`. Scheduling the combined track is authoritative;
the decision record exposes the difference. Under an additive time map and an additive frame conversion, `follow` is
preserved by delaying the second scheduled source by the scheduled length of the first.

This limited result is important. Notation and audio are connected by a real structure-preserving operation where the
structures agree. The design does not force a false law where tempo and rounding break it.

### 7.3 A notation-led chain

```text
ScoreDocument
    -- a theory-owned interpretation with explicit choices -->
EventTrack<WrittenTime,WrittenFact>
    -- performance choices -->
EventTrack<PerformedTime,Gesture>
    -- schedule and record time decisions -->
Machine<AudioFrameStep,Unit,EventBatch<Gesture>>
    -- connect --> instrument
    -- connect --> effects
    -- repeated step --> audio history
```

Each arrow is an ordinary typed function or a machine connection. No universal `link`, `world`, or `Description` term is
needed.

### 7.4 A sound-led chain

A live microphone is an engine input. A score follower is a machine from audio frames to finite observations. An offline
transcription is a total finite operation from an `AudioClip` to a list of candidates and evidence.

```text
Machine<AudioFrameStep,MicrophoneFrame,ObservationBatch>
AudioClip -> List<TranscriptionCandidate>
```

The core does not promise that transcription is unique or total for every musical concept. Its result type states the
actual promise.

## 8. What musical ideas become simple

### 8.1 Chords and texture

A theory package defines notes and chords. The core does not.

One possible construction maps each chord tone to an event of the same length and joins the results with `together`.
Heterophony and polyphony use the same neutral placement operation while keeping distinct payloads and parts. Unequal
entries and exits require no padding.

### 8.2 Keys and harmonic function

Key, scale degree, chord spelling, voicing, and harmonic function are separate library types. A key may help construct
or analyze a chord. It is not a global pitch coordinate, and harmonic function is not reduced to transposition by a
tonic.

### 8.3 Meter and free time

An event track has exact positions but no meter. Meter, polymeter, hypermeter, swing, rubato, and perceived grouping are
library values or analyses. An under-specified phrase remains a phrase value until performance choices produce a timed
track.

### 8.4 Timbre and orchestration

An orchestration payload can name instruments, techniques, and roles. An attack-sustain effect places a short attack
event and a longer sustain event together. The chosen instrument machines make the timbre audible. Audio is therefore
part of the same typed program, not an unrelated export backend.

### 8.5 Audio-led composition

A synthesizer, microphone, sampler, and effect chain can be the primary exported value. Written or analytical views may
be derived later. Goldfrapp's retained key noise is represented by running the synth path and microphone path beside one
another, then connecting both to an explicit mixer.

### 8.6 Live music

A finite protocol can build a machine with a live-input type. The machine may run without a fixed end. Its source still
normalizes because the source builds the machine; it does not execute the whole live history.

## 9. What is not in the core

The first version does not include:

- a universal `Music` value;
- a `Description` or `Match` calculus;
- dependent or refinement types;
- call-by-push-value terms;
- effects during source evaluation;
- first-class signals or streams;
- arbitrary feedback;
- public lifting of source functions into real-time machines;
- general recursion;
- type-directed macros; or
- built-in notes, chords, keys, meters, instruments, or cultural theories.

Surface syntax adapters may turn `c#4/4` or another package-owned notation into ordinary constructor calls. Expansion
finishes before type inference and cannot inspect inferred types. It is not part of the semantic core.

These omissions are deliberate. A new feature must remove a real side condition in at least two different musical uses,
or close a safety boundary that the current rules cannot state.

## 10. The compiler path

The compiler needs four source forms and two targets:

```text
source text
-> lossless syntax
-> resolved source-like terms
-> typed terms with inferred types
-> small evaluation core
-> finite values, EventTrack values, and Machine values

Machine value
-> validated allocated machine
-> repeated step execution
```

Pattern compilation, modules, and syntax adapters belong before the small evaluation core. Event-track operations and
machine constructors remain explicit because later passes need them. This is the useful lesson from Peyton Jones's
enriched lambda calculus: lower a feature only after the information it carries is no longer needed.

The private runtime may flatten a machine tree into arrays of units and buffers. That is an implementation choice. The
flattening pass must preserve the structural step equations in §5.3. The public meaning is not “whatever order a graph
scheduler happens to choose.”

## 11. Recommended names

| Old name | Proposed name | Reason |
| --- | --- | --- |
| `Timeline<A>` | `EventTrack<C,A>` | It is a finite set of placed events, not an audio stream or a universal timeline. |
| extent | length | It is the finite surrounding length of one track. |
| musical time | written time or performed time | The coordinate must say whose time it is. |
| process graph | machine | The public value is defined by state and repeated steps; the graph is a private layout. |
| tick | step | One call to the state transition. |
| signal | audio history | Use “signal” only when the mathematical signal is really meant. |
| block semantics | frame semantics with batching | Host callback size is not meaning. |

Names are part of the design. If a reader cannot guess what a type stores or what an operation does, the definition is
not yet good enough.

## 12. Immediate recommendation

Use this calculus as the next formal candidate. Do not amend governing documents or compiler code yet.

The next proof must establish:

1. source type inference is decidable and principal;
2. every accepted source term terminates;
3. every event-track operation preserves well-formedness;
4. every machine has one total deterministic next step;
5. feedback is causal because it uses stored prior data;
6. connection and side-by-side composition obey their stated laws;
7. scheduling emits each event exactly once and records all time conversion decisions; and
8. validated batching preserves frame-by-frame audio meaning.
