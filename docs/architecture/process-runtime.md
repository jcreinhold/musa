# Process IR and runtime architecture

## 1. Ownership

`musa-audio` owns the finite process IR, formation checker, scheduler, processor registry, state schemas, and offline
tick implementation. These types are private. `StudioGraphSpec`, instrument bodies, media voices, and gesture bindings
compile into this IR during `prepare_execution`.

`musa-engine` owns device negotiation, plan install/retirement, transport, and callback invocation. It does not inspect
node topology. Offline rendering calls the same prepared transition without CPAL.

## 2. Preparation phases

```text
resolved gesture/studio/instrument intent
  1. validate exact options and bindings
  2. elaborate closed processor nodes and typed ports
  3. insert only explicit semantic registers/delays
  4. build whole-node dependency graph
  5. reject combinational cycles/missing inputs/type mismatch
  6. choose canonical node schedule
  7. compute exact capacities and allocate control-side storage
  8. return opaque PreparedExecution or canonical PrepareError
```

Preparation never depends on a callback. Every failure occurs before publication of a plan.

## 3. Whole-node schedule

The scheduler stores node order, not an arbitrary port order. A node transition is called once per semantic tick only
after every same-tick input is available. Boundary inputs and prior register values are available at tick start.
Register outputs are committed after all nodes step.

This representation directly implements `docs/spec/03-process-calculus.md`. An optimization may group independent nodes
or vectorize ticks only after a differential law shows the same returned state/output.

## 4. Fixed semantic tick

The prepared options choose the semantic tick independently of caller request size. The simplest conforming launch
choice is one frame; a fixed multi-frame tick is allowed when its processor contracts and feedback delay say so
explicitly. The engine may request any number of frames by iterating/slicing semantic ticks while retaining partial-tick
state privately.

Tests compare rendering one request against every partition of the same request. Feedback, modulation, media playback,
and envelopes must agree. Current caller-block-sensitive feedback is a known nonconformance, not the intended contract.

## 5. Processor boundary

A registered processor supplies concrete closed operations:

- port and state schemas;
- deterministic initialization from prepared parameters/seed;
- a total finite tick transition;
- resource/capacity bounds;
- numeric/NaN behavior; and
- canonical operation version.

No processor receives a closure or arbitrary host callback. Later plug-in hosting requires an adapter process with an
explicit failure/nondeterminism/conformance policy; it does not silently satisfy the native theorem.

## 6. Real-time publication

All buffers, node states, schedule tables, media maps, and processor instances are created on the control side. A
prepared plan crosses to the callback through the existing bounded lock-free queue. The callback:

- takes no allocation or lock;
- performs no I/O or logging;
- reads only prevalidated compact indices;
- returns retired plans for control-side destruction; and
- reports bounded counters through RT-safe channels.

Device sample-rate/channel negotiation is an input to preparation options. A mismatch requires re-preparation; it is not
an ambient mutation of a running plan.

## 7. Verification

The implementation gate includes:

- the exact whole-node/acyclic-port counterexample from the K₂ proof review;
- missing/duplicate/incompatible port negative tests;
- combinational-cycle rejection and registered-cycle acceptance;
- tick totality/determinism properties over a small reference processor family;
- causality prefix tests;
- all caller-block partitions of feedback and stateful processors;
- offline/live prepared-transition equality; and
- callback allocation/lock/I/O/destruction instrumentation.
