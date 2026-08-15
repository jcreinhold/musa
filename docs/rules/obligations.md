# Rules that follow from the core design

These rules turn [constitution.md](constitution.md) into tests for new designs. Each rule gives a concrete mistake to
avoid.

## 1. Every conversion is named

A written note, a sounding frequency, and a MIDI note number are different values. Moving between them requires a named
function or compiler pass with all needed inputs, such as tuning, register, and spelling policy.

This forbids a compiler from silently choosing 12-tone equal temperament, a tonic, or a MIDI spelling because two types
happen to contain the same integer.

## 2. Built-in Western theory remains optional

Keys, scale degrees, 12-note pitch classes, tonal chord functions, and regular bars are useful built-in tools. None may
be required by the event-track core or by every music project.

A new music-theory package must be able to reach notation, performance gestures, or sound through its own explicit
conversions. It need not pretend that its values are Western keys or chords.

## 3. Placing tracks together accepts different durations

If `M` has duration `d` and `N` has duration `e`, `together(M, N)` has duration `max(d, e)` and contains every
occurrence of both, with multiplicity preserved. The shorter part simply has no occurrences after it ends.

This forbids an equal-duration type check and automatic rest insertion. Authors write rests when the notation or
analysis needs rests, not to satisfy the event-track implementation.

## 4. Metre is not built into every track type

Bars, pulse layers, tāla, swing, rubato, fermatas, gradual tempo changes, and unmeasured music need different models.
They may guide a named conversion from written or structural time to performed or physical time. They are not mandatory
type parameters of every event track.

This forbids treating `senza misura` as a type error or choosing one piecewise-constant pulse model for all music.

The one type parameter a track does carry beside its payload is its time coordinate, and it is carried for the reason in
`constitution.md` §8: a written beat and a physical second are indistinguishable until the sound is wrong.

## 5. An equality key must say what it ignores

A type may deliberately treat two stored values as equal even when some fields differ. For example, a musical-event
comparison may ignore source-layout fields. Its equality key must state which fields it reads, which it ignores, and its
version.

This forbids claiming both that a key distinguishes every stored value and that it omits fields. Changing the chosen
fields requires a new version or a checked migration.

## 6. A hash only narrows a search

If two deterministic hashes differ, their input bytes differ. If the hashes match, the input bytes may still differ. A
cache must compare the full encoded arguments before it returns a correctness-sensitive hit.

This forbids hash-only cache hits and type identities based only on a digest.

## 7. Audio preparation receives every choice as an argument

Audio preparation has this shape:

```text
prepare_execution(gestures, bindings, seed, options)
    -> prepared machine or error
```

`options` includes every choice that can change acceptance or execution, such as sample rate, channel layout, batching
policy, render bounds, and quality settings. Origin data used only for editor navigation is handled separately.

Equal arguments to a pure deterministic implementation give equal prepared results. Equal rendered frames also require
equal external input, initial state, allocation rules, and conforming primitives. This forbids claiming unconditional
bit-for-bit audio equality across devices.

Preparation is also where the core check happens: every primitive in the machine must accept the chosen format, every
connected port layout must agree, and every stated memory and worst-case-step contract must be satisfied before anything
is allocated. A machine may therefore be well typed and still fail preparation — an oscillator configured for 44.1 kHz
in a 48 kHz render is a configuration error, and it is reported as one rather than avoided by making the type system
depend on values.

## 8. One audio step is one sample frame

The reference meaning of an audio machine is its frame-by-frame step. A batch method over `n` frames may replace `n`
repeated steps only when it produces the same next state and the same `n` outputs for every valid state and input block.

A feedback-free machine may combine valid batch methods for its parts through chain and side-by-side connection. A
feedback machine may not: its later feedback inputs are earlier outputs from the same block, so it runs frame by frame
unless a separately checked batch method for the **whole** feedback machine exists. A unit that genuinely needs internal
blocks, such as an FFT effect, buffers frames in its private state and states its latency.

This forbids sound that changes with host callback size, and it forbids inheriting a batch method through feedback by
analogy with the feedback-free case.

## 9. Origin paths keep their intermediate steps

If source `A` produced occurrence `B`, which then produced MIDI event `C`, the recorded path keeps `B` and both
conversions. A generated occurrence also keeps the source root and generation site that explain where it came from.

This forbids flattening the path to a list that can no longer be checked, or inventing a generated occurrence with no
source or generation site.

## 10. New language features need real examples

A type-system feature enters Musa on evidence, by one of two routes. Whichever route is used, **the failed examples come
first**: the program that could not be written is written badly, committed, and measured before the feature that fixes
it is designed.

**The musical route.** Ordinary finite data, total functions, and modules make at least two real musical operations
unclear or unsafe, and the smallest failing term is recorded. Nominal data and private constructors entered this way:
different theory packages need to hide their representations.

**The engineering route.** A committed Musa program — the standard library, an adapter, a fixture — is measurably
deformed by the language itself. This route requires all four of: the failing program named and committed at a stated
revision; its size measured in lines and bytes; the compensating constructs enumerated and counted, not described; and a
later prompt that rewrites the same program on the new feature and reports the new measurement. A prediction is not
evidence and a rewrite that does not shrink the program is the feature failing, whatever else it improved.

`stdlib/src/adapters/staff.musa` is the first admission on this route: 2,404 lines and 93,252 bytes of Musa to read
staff notation, of which six hand-written `call1`–`call7` argument builders, 27 distinct hand-allocated role integers
across 56 `syntax_built` calls, an eight-field product destructured in full to read one field, 21 `text_equal` tests
against token-kind spellings, and a reading algorithm that runs backwards because a list cannot be constructed are
compensation for the language rather than facts about notation. The record is
[`../notes/research/language-design-closure/42-dependent-core-decision.md`](../notes/research/language-design-closure/42-dependent-core-decision.md).

Neither route is satisfied by an analogy. General recursion, call-by-push-value, first-class signals, macros that
dispatch on an inferred type, and “worlds” do not enter the language because they fit one, and none of them is admitted
by the amendment that opened the engineering route.

## 11. A formal compiler stage need not be source syntax

The event-track rules, machine rules, and origin-path rules need precise definitions. That does not mean composers must
write those internal terms. A compiler-owned form should remain private when exposing it would not make musical ideas
easier to express.

This forbids adding one source operator for chords, phrases, tracks, and machines just because each has some form of
composition.

## 12. Real-time code must implement the same step rules

Preallocation, fixed buffers, queues, flattening a machine tree into arrays, and vectorization may change how fast the
engine runs. They may not change what a step means. The audio callback allocates no memory, takes no lock, performs no
file or network I/O, writes no log, and destroys no large object.

This forbids a fast path with different feedback timing, and it forbids a private flattening whose execution order
differs from the structural step equations.

## 13. Storable data contains no source function

Only storable data may be an occurrence payload, a machine port or feedback value, a primitive configuration, or an
argument or result of a foreign primitive. A type is storable data when it contains no source function at any depth and
has a versioned, finite, exact encoding.

The check is recursive and covers containers: a list of functions, a constructor with a function field, and an abstract
type whose hidden representation holds a closure are all rejected. An abstract compiler-owned type counts as storable
data only when its owner guarantees the absence of a closure and supplies the exact encoding.

This forbids smuggling a looping or non-total closure into a value that must be finite, exactly comparable, or runnable
under a real-time deadline.

## 14. Scheduling records every decision it makes

The operation from an event track to a running event source either succeeds with a complete record of its conversions,
or fails with a stated error. The record names, for each occurrence boundary, the exact source position, the exact
physical time, the assigned frame, the rounding or collision choice, and the policy version.

Success additionally requires that the assignment be nonnegative, representable, and order-preserving on the finite set
of source boundaries, with each end no earlier than its start.

This forbids a hidden default policy, a silently dropped or shifted occurrence, an unbounded counter in the running
source, and any claim that a rounded time and its exact original are the same value.

## 15. Private identity stays private

An event handle created by scheduling is opaque. A primitive may compare two handles for equality and may use them to
pair one occurrence's start with its end. It may not read their numeric spelling, derive randomness from them, or order
messages by them.

Merging two scheduled sources relabels their handles into disjoint sets before applying the policy's fixed message
order, so a handle from one source can never collide with an equal-looking handle from the other.

This forbids an instrument whose output changes when handles are consistently renamed, which is the premise every
overlay-preservation result rests on.

## 16. Seeds are explicit configuration

A unit that uses chance stores its seed in its own primitive configuration. Preparation may split one render seed into
named per-unit seeds, but that split is part of the finite machine description.

This forbids a hidden root seed, and it forbids regrouping a machine tree from reseeding a unit:
`connect(connect(m,n),p)` and `connect(m,connect(n,p))` contain the same leaves with the same configurations, so they
run the same.
