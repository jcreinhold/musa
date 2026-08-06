---
id: 11
slug: audio-core
status: pending
depends_on: [10]
phase: 1
---

# Audio Core: Graph Compilation and Offline Rendering

## Task

Implement `musa-audio`'s foundation: the declarative `StudioGraphSpec` → compiled,
preallocated `RenderPlan` pipeline, typed ports, and the first processors (sine
oscillator, gain, pan, mixer, constant), plus deterministic offline block rendering.
No CPAL, no live stream — this prompt produces the pure DSP engine everything
real-time later executes.

## Read

- Roadmap §5.6 (the signal algebra: serial/parallel/merge/split/feedback-through-delay
  — conceptually, not as shared types with the score algebra), §13.1–§13.4
  (architecture, real-time separation, spec-vs-plan, typed ports), §13.5 (oscillator
  structure), §13.8 (offline rendering == live rendering), §17.5 (DSP tests).
- Roadmap §3: the graph compiler hides validation, topological sort, and buffer
  allocation. Callers see `build` and `render`, never the internals.

## Design

- Create `musa-audio` with dependencies: `musa-compiler`, `hound` (WAV writing is used
  by the offline render test harness and prompt 12), `rtrb` (not needed until 13 —
  defer unless a type requires it), `thiserror`. Do **not** add `fundsp` yet; decide
  at this prompt whether the first processors are hand-rolled (recommended for sine /
  gain / pan / mixer — they are small and the roadmap §13.6 only lists fundsp as "an
  implementation backend or reference"). If you do adopt fundsp, its types must not
  appear in any public signature (§13.6).
- Public surface:

  ```rust
  pub struct StudioGraphSpec { /* nodes, connections, parameters — declarative,
      editable, private layout */ }

  pub enum PortKind { Audio { channels: u8 }, Control, Gate, NoteEvents }

  pub fn compile_graph(
      spec: &StudioGraphSpec,
      options: &GraphOptions,   // sample rate, block size
  ) -> Result<RenderPlan, GraphError>;

  pub struct RenderPlan { /* processors, schedule, buffer arena — opaque */ }
  impl RenderPlan {
      /// Render `frames` samples of the graph with `events` scheduled.
      /// Real-time-safe: no allocation, no locks.
      pub fn render(&mut self, events: &EventSlice, output: &mut [f32], frames: usize);
  }
  ```

- Graph validation (§13.3): port-kind compatibility, channel-count compatibility,
  disconnected nodes, unreachable processors, and cycle rejection — a cycle is legal
  only through an explicit delay node (none exists yet, so all cycles are errors now;
  encode the rule, not just the current case).
- Processors at this prompt: sine oscillator (phase-continuous, §13.5 formula), noise
  (deterministic seed), constant control, gain, pan, mixer (n-in), splitter,
  mono↔stereo adapters (§13.4's explicit adapters).
- Parameter system skeleton (§13.7): `ParameterDescriptor { unit, range, default,
  smoothing, combination }` — no modulation sources yet (prompt 20), but descriptors
  exist now so adding modulation is additive.
- The render function must satisfy the §13.2 rules even though nothing is live yet:
  preallocate everything in `compile_graph`; `render` takes `&mut self` and never
  allocates. Test this with an allocation-counting harness (a test-only global
  allocator wrapper) — it is the cheapest way to make the RT contract a test, not a
  hope.
- Offline rendering executes the same `RenderPlan::render` a live stream will (§13.8).

## Target

- `musa-audio`: `StudioGraphSpec`, `PortKind`, `compile_graph`, `RenderPlan`,
  processor set above, allocation-free render harness.
- Tests (§17.5): oscillator frequency accuracy (zero-crossing/FFT-lite) and phase
  continuity across blocks; mixer/gain arithmetic; cycle rejection; disconnected-graph
  silence; determinism (two renders, byte equality); NaN/infinity absence with
  adversarial parameters; allocation-free `render`.
- insta snapshots of validation diagnostics (they will become user-facing via prompt
  19's language).

## Check

```sh
cargo nextest run -p musa-audio
cargo clippy --all-targets -p musa-audio -- -D warnings
cargo fmt --check
```

Commit as `Add audio graph compiler and offline renderer`.

## Stop

- No voice allocation or note-event → oscillator routing (prompt 12).
- No ADSR/LFO/filters/effects (prompts 20–21).
- No CPAL or threads (prompt 13).
- No fundsp types in public API; if undecided, hand-roll.
