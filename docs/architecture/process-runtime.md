# Preparing an audio graph for real-time use

This page maps the audio-graph rules to `musa-audio` and `musa-engine`.

## 1. Crate boundary

`musa-audio` owns:

- the private process-graph representation;
- graph validation and whole-node scheduling;
- the closed processor registry;
- processor state formats;
- audio preparation; and
- the offline implementation of one semantic audio step.

`musa-engine` owns:

- device negotiation;
- installing and retiring prepared plans;
- transport state; and
- calling the prepared step from the audio callback.

The engine does not inspect graph nodes or buffers. Offline rendering runs the same prepared step without CPAL.

## 2. Preparation happens before the callback

`prepare_execution` performs these steps:

1. Check every option and instrument or studio binding.
2. Build closed processor nodes with typed ports.
3. Add only the registers and delays that the source or processor contract requested.
4. Build the ordinary-wire dependency graph over whole nodes.
5. Reject missing inputs, type mismatches, duplicate drivers, and cycles without a register.
6. Choose a fixed node order.
7. Compute buffer and state sizes and allocate control-side storage.
8. Return an opaque prepared plan or a stable preparation error.

Every recoverable failure occurs here. The callback never discovers that a port is missing or that more memory is
needed.

## 3. Store a whole-node schedule

The prepared plan stores node order, not port order. During one semantic step, a node runs once after all of its current
inputs are available. Graph inputs and old register values are available at the start. New register values are committed
after all nodes run.

Grouping independent nodes or processing several steps with SIMD is allowed only when tests show the same output and
next state as the simple step rule.

## 4. Fix the semantic step in the plan

Caller buffer size must not define feedback delay. The prepared options choose one semantic step. One frame is the
simplest initial choice. A larger fixed step is allowed only when every processor and delay contract names it.

The engine can satisfy an arbitrary device request by repeating or slicing semantic steps and keeping any partial-step
state private. Tests render the same duration under many caller-buffer partitions. Feedback, modulation, envelopes, and
media playback must agree in every partition.

Today’s caller-buffer-sensitive feedback does not meet this rule. It remains a known implementation gap.

## 5. Processor contract

Each registered processor provides:

- input, output, and state formats;
- deterministic initialization from prepared parameters and seed;
- a total finite step function;
- memory and work bounds;
- rules for clipping, NaN, and infinity; and
- an operation version.

A native processor receives no arbitrary closure or host callback. A future plug-in adapter must state what happens on
failure or nondeterminism. It does not automatically inherit the native processor theorem.

## 6. Publishing a plan to the callback

The control thread creates every buffer, state value, schedule table, media map, and processor instance. It sends the
finished plan through the existing bounded lock-free queue.

The callback:

- allocates no memory;
- takes no lock;
- performs no file, network, or device-setup I/O;
- writes no log;
- reads only validated compact indices;
- sends old plans back for destruction on the control thread; and
- reports only bounded real-time-safe counters.

Sample rate and channel layout are preparation inputs. A device change creates a new prepared plan; it does not mutate
the current plan behind the callback’s back.

## 7. Required tests

The implementation is not complete until it passes:

- the known graph whose ports look acyclic but whose whole nodes cannot be scheduled;
- missing, duplicate, and mismatched port tests;
- rejection of ordinary-wire cycles and acceptance of registered feedback;
- totality and determinism tests over small reference processors;
- causality tests on every input prefix;
- all caller-buffer partitions for feedback and other stateful processors;
- equality between offline and live use of the prepared step; and
- instrumentation that detects allocation, locks, I/O, logging, or large destruction in the callback.
