---
id: 173
slug: one-frame-audio
status: pending
depends_on: [172]
phase: 3
---

# Make One Audio Frame the Meaning

## Task

Move every existing native DSP unit, feedback path, modulation source, offline renderer, and engine callback onto the
machine reference semantics. Delete host-block-defined sound and the old public graph-compilation path.

## Read

- The prompt-127a audio rules, research `05-selected-calculus.md` §6, and `17-final-review.md` current-code audit.
- Prompt 149's trusted boundary, prompt 169's K1–K20 matrix, and note 67's frozen adapter theorem. The input language is
  the existing dependent Musa core with checked finite machine values; this prompt migrates their runtime meaning and
  does not recreate the deleted contextual, trait-dictionary, or rank-1 language paths.
- Current `musa-dsp/src/{plan,spec,effects,filter,envelope,synth}.rs`, `musa-playback`, prompt 31 laws, and RT tests.
- `docs/rules/obligations.md` real-time and whole-processor rules after prompt 127a.

## Design

Register each current DSP unit as a registered primitive with one-sample-frame start/step meaning. Typed channel and
event ports remain explicit. A mixer is a primitive from a tuple of frames to one frame; `beside` never mixes. LFO,
modulation, envelopes, smoothing, delays, and feedback advance by frame, not callback.

`prepare_audio(format,machine)` checks the complete primitive registry, format/layout, capacities, memory, worst-case
step cost, and unsupported configuration before allocation. It returns an opaque prepared machine or one stable error.

A whole-machine batch may replace repeated steps only under the exact state/output equality contract. Feedback does not
inherit child batching. Keep a simple frame interpreter as the oracle and differential-test every optimized batch over
many host partitions. Tests are evidence; the conformance theorem remains conditional on the contract.

Delete `compile_graph` and exposed `StudioGraphSpec` as public semantic alternatives once all callers use machine
preparation. Private flattening may retain arrays and schedules but must preserve the structural step.

Only the prompt-149 kernel rechecker may certify source terms. Audio preparation checks formats, primitive
registrations, capacities, and real-time bounds over the already checked machine; it does not elaborate or normalize
source language again.

## Target

- Complete native processor migration to registered primitives and one-frame reference execution.
- One `prepare_audio`/prepared-machine facade shared by offline and live paths.
- Clean deletion of caller-block feedback, block-rate modulation meaning, and obsolete public graph APIs.
- Differential partition, feedback, envelope, modulation, random-seed, NaN, silence, allocation, lock, I/O, logging, and
  plan-retirement tests.

## Check

```sh
cargo nextest run -p musa-dsp -p musa-playback -p musa-project -p musa
cargo nextest run --run-ignored all -p musa-dsp -p musa-playback
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo bench -p musa-dsp
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-dsp
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-playback
```

Commit as `Make one audio frame the reference meaning`.

## Stop

- No compatibility `compile_graph`, block-defined delay, block-rate semantic control, arbitrary closure in a primitive,
  asset/sample feature, plug-in hosting, or public DSP internals.
- No claim that totality alone proves a real-time deadline.
