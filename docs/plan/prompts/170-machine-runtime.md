---
id: 170
slug: machine-runtime
status: pending
depends_on: [127d, 169]
phase: 3
---

# Give Every Machine One Exact Next Step

## Task

Implement the reference machine semantics in `musa-dsp`: a functional build-local primitive registry, validated machine
preparation, private combined state, explicit start, and one total deterministic next step.

## Read

- The governing machine rules from prompt 127a and research `05-selected-calculus.md` §§5–6.
- `docs/plan/code-map/process-runtime.md`, current `StudioGraphSpec`, `RenderPlan`, processors, feedback scheduling, and
  engine/offline callers.
- The K2 and K3 counterexamples retained in the research final review.

## Design

Each primitive registration fixes id/version, exact configuration codec, port schemas, private state layout,
deterministic initialization, total step, memory/work bounds, and optional batch contract. Reject conflicting
registrations. Microphone and controller values arrive through typed inputs; primitives read no hidden input.

Prepare structural machine forms by recursion. Define state and start/step exactly as the governing equations say.
Initialized feedback stores the old feedback value and commits the returned next value after the child step. Do not
schedule ports or infer a loop delay from graph shape.

Keep state, buffers, primitive instances, node order, and flattened layout private. The first implementation may execute
the structural tree directly. A later flattening is valid only when differential tests show the same state and outputs.
`musa-playback` receives an opaque prepared machine.

This prompt uses small deterministic reference primitives, not the full studio catalogue. Prompt 172 migrates existing
DSP units after the semantics pass.

## Target

- Functional primitive registry, preparation, opaque prepared state, start, and one-step interpreter.
- Tests for every constructor, typed port failure, registry conflict, first feedback output, Boolean negation with
  stored delay, causality, determinism, and the old whole-node scheduling counterexample.
- Offline and engine test harnesses that call the same prepared step.
- Narrow facade and module-design audit.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-dsp -p musa-playback
cargo clippy --all-targets -p musa-compiler -p musa-dsp -p musa-playback -- -D warnings
cargo fmt --check
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-dsp
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-playback
```

Commit as `Define one exact step for every machine`.

## Stop

- No event scheduling, full DSP migration, batching optimization, asset decoder, plug-in API, public state, or public
  graph scheduler.
- No caller-block-defined feedback and no zero-delay loop.
