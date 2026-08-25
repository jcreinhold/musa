# Preparing a machine for real-time use

This page maps the machine rules of `../../rules/across-stages/03-machine-calculus.md` to their implementation in
`musa-dsp` and `musa-playback`. Prompt 171 supplies the checked reference semantics in `musa-dsp/src/machine.rs`: typed
preparation, a closed functional primitive table, explicit start, private combined state, and one structural step.
Prompt 173 migrated the production path directly from exact gesture tracks through checked schedules into opaque
`PreparedAudio`. `StudioGraphSpec`, its validation error, and the flattened `RenderPlan` are crate-private preparation
details; `compile_graph` and caller-block sound no longer exist.

## 1. Crate boundary

`musa-dsp` owns:

- the exact checked projection from source-declared studio/instrument values into private preparation data; the source
  declarations and their documentation remain in `stdlib/`;
- preparing registered primitives, `identity`, `connect`, `beside`, `feedback`, `copy`, `drop`, and `swap` from the
  compiler's immutable `MachineSpec`;
- machine validation and structural step semantics;
- the closed primitive registry;
- primitive state formats;
- checked scheduling and bounded audio preparation; and
- the reference implementation of one audio step, which is one sample frame.

`musa-playback` owns:

- device negotiation;
- installing and retiring prepared audio machines in the production callback;
- transport state; and
- calling the prepared step from the audio callback.

The engine does not inspect primitives or state. Offline rendering and the device callback both fill host buffers by
repeating `PreparedAudio::step`; their partition equality is an executable law.

## 2. Reference preparation happens before start

`prepare_machine` now performs these steps:

1. Check the finite bottom-up projection is one tree, with children preceding parents, and enforce node and depth
   bounds.
2. Match every primitive id/version against both the compiler descriptor and the closed runtime registration.
3. Solve the structural port equations by first-order unification with an occurs check, fixing the root to the exact
   owned port schemas the compiler handed across the trusted boundary.
4. Decode primitive configuration and explicit feedback initials once, at their solved storable schemas.
5. Sum declared private-state and step-work bounds.
6. Build a private structural tree and return it inert as `PreparedMachine`.

`PreparedMachine::start` explicitly constructs the combined state, returned only inside `StartedMachine`. A dynamic
caller can still offer a value of the wrong external schema; `step` refuses it before state changes. No source syntax,
core term, resolver, evaluator, or payload checker is available at this boundary.

## 3. Execute the structural equations, not a schedule

The reference implementation deliberately executes the finite structural tree. `connect` runs its left child and hands
that current result immediately to its right child; `beside` runs the two components on the two input members;
`feedback` supplies its stored old value, takes one child step, and commits the returned next value afterward. There is
no node order, port schedule, inferred register edge, or zero-delay loop.

A later flattening may store a control program rather than the tree, and batching may group repeated steps, only when
differential tests show the same outputs and final private state as this implementation.

## 4. The step is one sample frame

**One audio step is one sample frame** (`../../rules/constitution.md` §4, `../../rules/obligations.md` rule 8). The
caller's buffer size never defines it, and preparation does not get to choose a larger one. A host block is an
optimization: replacing `n` repeated steps by one `batch(n)` call is legal only where that machine's `batch`
implementation satisfies the contract in `../../rules/across-stages/03-machine-calculus.md` §4, and a machine containing
`feedback` does not inherit a valid batch from its parts.

The engine can satisfy an arbitrary device request by repeating steps, or by a valid batch, and keeping any partial-step
state private. Tests render the same duration under many caller-buffer partitions. Feedback, modulation, envelopes, and
media playback must agree in every partition (R1-batch).

Every current native feedback, modulation, envelope, smoother, and effect now advances under this rule. The old 64-frame
and 256-frame audio-bridge digests differed; the migrated oracle makes them byte-identical.

## 5. Registered-primitive contract

Each reference registration now provides:

- input, output, and state formats;
- deterministic initialization from prepared parameters and seed;
- a total finite step function;
- memory and work bounds;
- exact total arithmetic or a finite non-arithmetic operation; and
- an operation version.

A native primitive receives no arbitrary closure or host callback. A future plug-in adapter must state what happens on
failure or nondeterminism. It does not automatically inherit the native-primitive theorem.

## 6. Publishing a prepared machine to the callback

The control thread creates every buffer, state value, step-order table, media map, and primitive instance. It sends the
finished machine through the existing bounded lock-free queue.

The callback:

- allocates no memory;
- takes no lock;
- performs no file, network, or device-setup I/O;
- writes no log;
- reads only validated compact indices;
- sends old prepared machines back for destruction on the control thread; and
- reports only bounded real-time-safe counters.

Sample rate and channel layout are preparation inputs. A device change creates a new prepared machine; it does not
mutate the current one behind the callback’s back.

## 7. Current and successor tests

Prompt 171 covers:

- every structural constructor and every reference primitive;
- malformed trees, registry conflicts, stored-value codecs, and typed port mismatch;
- totality and determinism over the small reference family;
- causality tests on every input prefix;
- the first output and subsequent Boolean negation of initialized feedback;
- the old whole-node scheduling counterexample; and
- equality between offline and device-free playback use of the prepared step.

Prompts 172–173 add scheduling, stereo-format, tuning, seed, registry, capacity, retained-memory and worst-step-work
checks, caller-buffer partition laws, and callback RT instrumentation. Primitive graph laws remain internal tests of the
private flattening; no graph constructor or compiler is a public semantic alternative.
