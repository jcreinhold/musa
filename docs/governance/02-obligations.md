# Rules that follow from the core design

These rules turn [01-constitution.md](01-constitution.md) into tests for new designs. Each rule gives a concrete mistake
to avoid.

## 1. Every conversion is named

A written note, a sounding frequency, and a MIDI note number are different values. Moving between them requires a named
function or compiler pass with all needed inputs, such as tuning, register, and spelling policy.

This forbids a compiler from silently choosing 12-tone equal temperament, a tonic, or a MIDI spelling because two types
happen to contain the same integer.

## 2. Built-in Western theory remains optional

Keys, scale degrees, 12-note pitch classes, tonal chord functions, and regular bars are useful built-in tools. None may
be required by the temporal kernel or by every music project.

A new music-theory package must be able to reach notation, performance gestures, or audio through its own explicit
conversions. It need not pretend that its values are Western keys or chords.

## 3. Overlay accepts parts of different lengths

If `M` has length `d` and `N` has length `e`, their overlay has length `max(d, e)` and contains all events from both.
The shorter part simply has no events after it ends.

This forbids an equal-length type check and automatic rest insertion. Authors write rests when the notation or analysis
needs rests, not to satisfy the timeline implementation.

## 4. Metre is not built into every timeline type

Bars, pulse layers, tāla, swing, rubato, fermatas, gradual tempo changes, and unmeasured music need different models.
They may guide a named conversion from written or structural time to performance time. They are not mandatory type
parameters of every timeline.

This forbids treating `senza misura` as a type error or choosing one piecewise-constant pulse model for all music.

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
    -> prepared audio plan or error
```

`options` includes every choice that can change acceptance or execution, such as sample rate, channels, block policy,
render bounds, and quality settings. Origin data used only for editor navigation is handled separately.

Equal arguments to a pure deterministic implementation give equal prepared results. Equal rendered samples also require
equal external input, initial state, allocation rules, and conforming processors. This forbids claiming unconditional
bit-for-bit audio equality across devices.

## 8. Audio schedules whole processors

A processor runs once per audio step after all of its current inputs are ready. The scheduler must therefore order whole
processors, not individual ports. A connection cycle is legal only if at least one edge reads a stored value from an
earlier step.

This forbids a port graph that looks acyclic but cannot run the processor API, as well as zero-delay feedback whose
meaning changes with the caller’s buffer size.

## 9. Origin paths keep their intermediate steps

If source `A` produced score event `B`, which then produced MIDI event `C`, the recorded path keeps `B` and both
conversions. A generated event also keeps the source root and generation site that explain where it came from.

This forbids flattening the path to a list that can no longer be checked, or inventing a generated event with no source
or generation site.

## 10. New language features need real examples

Musa should add a type-system feature only when ordinary finite data, total functions, and modules make at least two
real musical operations unclear or unsafe. Nominal data and private constructors have such examples: different theory
packages need to hide their representations.

Dependent types, recursive data, first-class modules, “worlds,” and equality proofs do not enter the language merely
because they fit an analogy. The failed examples must come first.

## 11. A formal compiler stage need not be source syntax

The timeline rules, audio-graph rules, and origin-path rules need precise definitions. That does not mean composers must
write those internal terms. A compiler-owned form should remain private when exposing it would not make musical ideas
easier to express.

This forbids adding one source operator for chords, phrases, timelines, and audio graphs just because each has some form
of composition.

## 12. Real-time code must implement the same audio rules

Preallocation, fixed buffers, queues, and vectorization may change how fast the engine runs. They may not change what a
processor step means. The audio callback allocates no memory, takes no lock, performs no file or network I/O, writes no
log, and destroys no large object.

This forbids a fast path with different feedback timing or a cached plan that omits a setting which changes execution.
