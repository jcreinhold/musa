---
id: 31
slug: dsp-effects-mix
status: pending
depends_on: [30]
phase: 2
---

# DSP: Time Effects, Buses, and the Mix Workspace

## Task

Complete the built-in studio: delay, feedback delay, chorus, and algorithmic reverb processors; bus and send/return
routing execution; a master peak limiter. Then add the desktop **Sound** and **Mix** workspaces (§14.4) so patches,
buses, sends, and levels are visible and adjustable — as structured editors of studio source, never a second authority.

## Read

- Roadmap §13.3 (feedback only through explicit delay), §13.6 (time effects, routing: mixer/splitter/send/return;
  limiter on master), §5.6 (feedback causality is visible), §14.4 (Sound and Mix workspaces), §11 (`AssignPatch` edit
  command — workspace controls produce source edits).
- Prompt 29's bus/send/modulate spec, prompt 30's processors.

## Design

- Processors (`musa-audio`): mono/stereo delay (time in `ms`, max preallocated), feedback delay (feedback gain — this is
  the graph's legal cycle: it must pass through the delay node, §13.3), chorus (modulated short delay, LFO-driven),
  algorithmic reverb (Schroeder/Freeverb-class: comb + allpass network; `room`, `damping` parameters per §7.1). Peak
  limiter on the master bus.
- Routing execution: `bus hall { reverb(...) }` compiles to a bus subgraph; `send violin -> hall at -18 dB` compiles to
  a send connection with the dB gain converted **at the compiler edge** (dB→linear happens once, at the spec→graph
  lowering; the graph carries linear gains — document where unit conversion lives). `route hall -> master` completes the
  return. Master always ends in the limiter.
- Cycle validation now has its positive case: a feedback path containing a delay is accepted; any other cycle is
  rejected. Test both.
- Desktop workspaces (§14.4):
  - **Sound**: selected part's patch shown as a readable node list (not a free-form graph canvas — a vertical chain view
    matching the `|>` source structure). Parameter controls (unit-labeled sliders) issue `EditStudio`-style project
    commands that rewrite the studio source (parameter literal replacement), keeping §11's one-truth rule. `AssignPatch`
    command implemented (§11's enum) with a patch picker per part.
  - **Mix**: per-part level (gain), buses with levels, send amounts — same mechanism: controls → source edits →
    recompile → reinstall plan. Live-feeling adjustments ride the existing debounce; do not build a separate real-time
    parameter channel now (note it as a measured-later optimization if recompile latency annoys).
- Remove the last placeholders (`reverb`); `glass-mountain.musa` compiles warning-free and the hall send is audible.

## Target

- `musa-audio`: delay, feedback delay, chorus, reverb, limiter; bus/send execution.
- `musa-project`: studio edit commands (`AssignPatch`, parameter-change text edits).
- `apps/musa-desktop`: Sound and Mix workspaces with structured editing.
- Tests (§17.5): delay timing frame-exactness, feedback stability (bounded output at high feedback), reverb decay time
  within tolerance, limiter never exceeds 0 dBFS, cycle acceptance/rejection, NaN-free with long feedback + reverb,
  determinism.

## Check

```sh
cargo nextest run -p musa-audio -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/glass-mountain.musa   # no placeholder warnings
cargo run -p musa-cli -- render examples/glass-mountain.musa --to wav -o /tmp/gm7.wav
cd apps/musa-desktop && cargo tauri dev   # manual: Sound and Mix workspaces edit the piece audibly
```

Commit as `Add time effects, buses, and mix workspaces`.

## Stop

- No convolution reverb, compressor, tape/distortion (§13.6 later list).
- No automation lanes/curves in the GUI (language-level curves are prompt 36).
- No visual node-graph canvas (pannable/zoomable); the chain view is the design.
- No plugin hosting of any kind (§4, Phase 4).
