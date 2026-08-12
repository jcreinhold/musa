---
id: 31
slug: dsp-effects-mix
status: done
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
cargo run -p musa -- check examples/glass-mountain.musa   # no placeholder warnings
cargo run -p musa -- render examples/glass-mountain.musa --to wav -o /tmp/gm7.wav
cd apps/musa-desktop && cargo tauri dev   # manual: Sound and Mix workspaces edit the piece audibly
```

Commit as `Add time effects, buses, and mix workspaces`.

## Repairs made while implementing

- **A legal cycle is one that passes through a delay, and it is implemented as a deferred read.** `schedule_order` finds
  every `Delay` node that can reach itself, drops its in-edges from the topological sort, and lets it read the previous
  block. Any other cycle is still `GraphError::Cycle`. One block of latency inside a loop already measured in hundreds
  of milliseconds is not audible, and it is the only way to keep the render plan a straight-line schedule.
- **The limiter has no lookahead, deliberately.** Instantaneous attack with a `min` against the allowed gain makes the
  guarantee exact per sample, and it keeps the master bus latency-free, so an offline render and a live one stay
  sample-aligned (§13.8). The cost — distortion when it engages hard — is documented on the processor.
- **The golden audio digest was re-pinned.** Measured against a worktree at the previous commit: the old master peaked
  at 3.80 with 189,980 samples over 0 dBFS. It now peaks at exactly 1.0. `session_laws` records both numbers beside the
  new digest so the change reads as the fix it is rather than as drift.
- **`is_placeholder` and its warning were deleted, not left always-false.** With `reverb` implemented there is no
  processor without DSP, and a check that can never fire is a lie about the language.
- **The compiler now records where each written value is.** `StudioNode.param_spans`, `StudioNode.span`,
  `Send.level_span`, `Assignment.patch_span`, and `StudioSpec::span()` are what a knob rewrites. Regenerating a whole
  call instead would normalize `30 ms` to `0.03 s`, reorder named arguments, and drop any comment inside the parentheses
  — so a written value is replaced in the scale it was written in, an unwritten one is *added* in the unit it is
  declared in, and a part with no `assign` gets a new statement inside the `studio` block rather than a refusal.
- **Those spans are trimmed.** `span_of` includes a node's leading trivia, so the first fader move produced `at-6.5 dB`.
  They use `trimmed_span` now.
- **`ParamSpec` gained a writable `range`.** A slider needs bounds in the unit the composer writes (`-60`…`+12` dB), and
  `musa-audio`'s `ParameterDescriptor` bounds something else — what the DSP accepts, in linear terms. Two questions, two
  answers; the doc on `ParamSpec::range` says which is which.
- **Mix shows only the levels the language has.** There is no per-part fader in `.musa`, so the Mix workspace does not
  draw one: a part's level is the `gain` stage its patch actually writes, plus its sends. Inventing a control would have
  meant inventing a second authority for the value behind it (§11).
- **A parameter change is committed on release, not while dragging.** One gesture is one edit, one revision, one entry
  in the undo history. This is the "ride the existing debounce" the Design section asked for; no separate real-time
  parameter channel was built, and none was needed at this rate.
- **A modulated parameter says so.** `ParamFacts.modulated_by` is filled from the resolved `modulate` statements, and
  the row prints it, because a knob that appeared to disagree with what is heard would be worse than no knob.
- **`--s-5` did not exist.** Four existing screens used `var(--s-5)` for gaps and padding, so those declarations were
  silently dropped. It is now part of the 4px chrome scale.
- **`ParamControl` takes its element id from the caller.** Two oscillators in one patch both have a `frequency`, so a
  label pointing at `param-frequency` pointed at the wrong control.
- **The workspace switcher and the command registry now list four workspaces.** `⌘2` and `⌘3` were reserved and bound to
  nothing; `docs/rules/desktop/03-interaction.md` and the stale comment in `examples/glass-mountain.musa` about
  pass-through stages were updated to say what is now true.

## Stop

- No convolution reverb, compressor, tape/distortion (§13.6 later list).
- No automation lanes/curves in the GUI (language-level curves are prompt 36).
- No visual node-graph canvas (pannable/zoomable); the chain view is the design.
- No plugin hosting of any kind (§4, Phase 4).
