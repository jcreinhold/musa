# Preparing a machine for real-time use

This page maps the machine rules of `../../rules/across-stages/03-machine-calculus.md` to `musa-dsp` and
`musa-playback`. The Rust identifiers in the workspace still carry their pre-127a spellings; the pairs are in
[`../clean-break-ledger.md`](../clean-break-ledger.md).

## 1. Crate boundary

`musa-dsp` owns:

- machine construction from registered primitives, `identity`, `connect`, `beside`, `feedback`, `copy`, `drop`, `swap`;
- machine validation and whole-machine step ordering;
- the closed primitive registry;
- primitive state formats;
- scheduling and audio preparation; and
- the offline implementation of one audio step, which is one sample frame.

`musa-playback` owns:

- device negotiation;
- installing and retiring prepared machines;
- transport state; and
- calling the prepared step from the audio callback.

The engine does not inspect a machine's primitives or buffers. Offline rendering runs the same prepared step without
CPAL.

## 2. Preparation happens before the callback

`prepare_execution` performs these steps:

1. Check every option and instrument or studio binding.
2. Build closed registered primitives with typed ports.
3. Add only the `feedback` edges that the source or a primitive's contract requested.
4. Build the ordinary-wire dependency graph over whole primitives.
5. Reject missing inputs, type mismatches, duplicate drivers, and cycles that cross no `feedback`.
6. Choose a fixed step order.
7. Compute buffer and state sizes and allocate control-side storage.
8. Return an opaque prepared machine or a stable preparation error.

Every recoverable failure occurs here. The callback never discovers that a port is missing or that more memory is
needed.

## 3. Store a whole-machine step order

The prepared machine stores primitive order, not port order. During one step, a primitive runs once after all of its
current inputs are available. Machine inputs and old `feedback` values are available at the start. New `feedback` values
are committed after all primitives run.

Grouping independent primitives or processing several steps with SIMD is allowed only when tests show the same output
and next state as the simple step rule.

## 4. The step is one sample frame

**One audio step is one sample frame** (`../../rules/constitution.md` §4, `../../rules/obligations.md` rule 8). The
caller's buffer size never defines it, and preparation does not get to choose a larger one. A host block is an
optimization: replacing `n` repeated steps by one `batch(n)` call is legal only where that machine's `batch`
implementation satisfies the contract in `../../rules/across-stages/03-machine-calculus.md` §4, and a machine containing
`feedback` does not inherit a valid batch from its parts.

The engine can satisfy an arbitrary device request by repeating steps, or by a valid batch, and keeping any partial-step
state private. Tests render the same duration under many caller-buffer partitions. Feedback, modulation, envelopes, and
media playback must agree in every partition (R1-batch).

Today’s caller-buffer-sensitive feedback does not meet this rule. It remains a known implementation gap.

## 5. Registered-primitive contract

Each registered primitive provides:

- input, output, and state formats;
- deterministic initialization from prepared parameters and seed;
- a total finite step function;
- memory and work bounds;
- rules for clipping, NaN, and infinity; and
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

## 7. Required tests

The implementation is not complete until it passes:

- the known machine whose ports look acyclic but whose whole primitives cannot be ordered;
- missing, duplicate, and mismatched port tests;
- rejection of ordinary-wire cycles and acceptance of `feedback` edges;
- totality and determinism tests over small reference primitives;
- causality tests on every input prefix;
- all caller-buffer partitions for feedback and other stateful primitives;
- equality between offline and live use of the prepared step; and
- instrumentation that detects allocation, locks, I/O, logging, or large destruction in the callback.
