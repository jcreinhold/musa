# Draft B: tracks and machines

**Status: draft under review. This document does not set Musa's rules.**

## Purpose

Draft A failed because finite placed events and unbounded audio have different shapes. This draft keeps both shapes in
one total language.

## 1. The ordinary language

The base is a strict, pure functional language. It has inferred rank-1 polymorphic types, products, sums, finite data,
functions, `let`, exhaustive matching, and finite folds. It has no general recursion or source effects.

A source program always returns a finite value. Two new finite value types do the musical work:

```text
EventTrack<C, A>
Machine<K, A, B>
```

`C` names a time coordinate. `A` is an event payload. `K` names what one machine step means. A machine reads one `A` and
writes one `B` per step.

These are ordinary type constructors. Hindley–Milner inference handles them without dependent types.

## 2. Finite event tracks

An `EventTrack<C,A>` contains:

- a nonnegative exact rational length; and
- a finite multiset of occurrences `(start, end, payload)` with `0 <= start <= end <= length`.

The tag `C` stops a written beat from being used as an audio frame. It has no built-in meter. A package may use tags
such as `WrittenTime`, `PerformedTime`, or `SecondTime`.

The constructors are:

```text
empty       : Length<C> -> EventTrack<C, A>
event       : Length<C> -> A -> EventTrack<C, A>
follow      : EventTrack<C, A> -> EventTrack<C, A> -> EventTrack<C, A>
together    : EventTrack<C, A> -> EventTrack<C, A> -> EventTrack<C, A>
map_events  : (A -> B) -> EventTrack<C, A> -> EventTrack<C, B>
```

`empty(d)` has length `d` and no events. It means absence of this payload, not a written rest and not zero-valued audio.

`event(d,a)` has length `d` and one occurrence of `a` from `0` through `d`. A point event uses `d = 0`.

`follow(x,y)` places `y` after the length of `x`. `together(x,y)` retains both sets of occurrences and uses the greater
length. It inserts no rests.

Any finite occurrence set can be built from these operations. To place `a` from time `s` to `e` in a track of length
`d`, write the equivalent of:

```text
together(empty(d), follow(empty(s), event(e - s, a)))
```

The surface language and libraries may provide clearer builders.

## 3. Running machines

A `Machine<K,A,B>` is a finite description of a unit that advances one `K` step at a time. It may have private state.

The draft operations are:

```text
identity  : Machine<K, A, A>
lift      : (A -> B) -> Machine<K, A, B>
connect   : Machine<K, A, B> -> Machine<K, B, D> -> Machine<K, A, D>
beside    : Machine<K, A, B> -> Machine<K, D, E> -> Machine<K, (A, D), (B, E)>
loop      : Machine<K, (A, F), (B, F)> -> Machine<K, A, B>
```

`connect(m,n)` sends each output of `m` to `n` during the same step. `beside(m,n)` runs both once per step and keeps
their outputs separate. A mixer is therefore an explicit machine from `(Audio,Audio)` to `Audio`.

`lift(f)` treats a pure function as a stateless machine. `loop(m)` connects the second output of `m` to its second
input.

This structure has familiar laws. `identity` is the unit of `connect`. Connection is associative. `beside` is
associative up to product rearrangement. Connecting two side-by-side pairs agrees with placing the two connections side
by side.

## 4. A direct path to audio

Suppose a library defines these types:

```text
WrittenFact
Gesture
AudioEvent
StereoFrame
WrittenTime
PerformedTime
AudioStep
```

It may then offer:

```text
interpret : PerformanceChoices
         -> EventTrack<WrittenTime, WrittenFact>
         -> Result<EventTrack<PerformedTime, Gesture>, PerformanceError>

schedule  : TempoMap
         -> EventTrack<PerformedTime, Gesture>
         -> Machine<AudioStep, Unit, EventBatch<Gesture>>

instrument : Patch
          -> Machine<AudioStep, EventBatch<Gesture>, StereoFrame>

room : Room -> Machine<AudioStep, StereoFrame, StereoFrame>
```

The rendered system is an ordinary finite value:

```text
connect(schedule(tempo, gestures),
  connect(instrument(patch), room(hall)))
```

At each audio step the scheduler emits the gestures that begin or end there. The instrument updates its private state
and emits a stereo frame. The room updates its state and emits the final frame.

This is a real typed connection between written material and audio. It is not an assertion that a written note is an
audio frame.

## 5. Audio-led work

Nothing requires a score. A microphone and synthesizer can start the graph:

```text
let paths = beside(bass_synth(patch), key_microphone)
let sound = connect(paths, mix)
```

A recording operation can run a closed machine for a finite number of steps and return an `AudioClip`. A transcription
function can return zero or more written candidates with evidence. Both are ordinary typed operations.

## 6. What this draft claims

The draft claims that one total language plus `EventTrack` and `Machine` is enough. It does not claim that the two types
are one object. It claims that typed functions can build them and that a scheduler can turn a finite track into the
input source of a running machine.

