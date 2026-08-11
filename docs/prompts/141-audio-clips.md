---
id: 141
slug: audio-clips
status: pending
depends_on: [132, 135, 137, 140]
phase: 4
---

# Recorded Media Reaches the Mix

> **Contingent on prompt 126.** The core-boundary decision may repair this prompt's Design, fold it into another, or
> replace it. Read `docs/core-boundary.md` first.

## Task

Render musical clips and fixed-media cues through the prepared audio plan. Decode and prepare recordings off-thread,
schedule their starts from the appropriate beat/physical-time semantics, apply explicit crop/loop/rate/fade/gain/pan
settings, route named media outputs through buses/sends, and preserve deterministic seek, offline/live, and export-tail
behavior.

## Read

- Prompt 140's normative semantics; prompt 135 asset store; prompt 137 prepared sample playback and resampling; prompt
  130 part isolation; existing offline/live render path, transport seek/loop, and release-tail calculation.
- Roadmap §§13.2, 13.8, 18 Phase 4. Audio recording and waveform editing remain explicitly outside Musa.

## Design

Media is a separate prepared source lane, not a score part, voice, mixer track, or instrument instance. Preparation
resolves each semantic cue/clip identity to immutable decoded audio and compact routing indices. A fixed cue schedules
start at `tempo(b)` and retains natural physical duration. A musical clip uses its transformed beat span and explicit
fit policy. Specify channel conversion, sample-rate conversion, bounds, fades, gain/pan, overlap, retrigger, same-frame
ordering, and end-of-project tail.

`crop` stops at the beat span, `loop` repeats deterministically and truncates at it, and `rate` chooses the documented
constant playback rate required by the source span/policy and changes pitch honestly. No implicit tempo-following or
pitch-preserving warp occurs. Tempo changes inside a loop affect scheduled span boundaries, not fixed-media sample
duration.

Prepare/decode/preload and retire off-thread. Use bounded preloading initially; if a checked realistic recording misses
memory budget, profile and design a control-side streaming/ring-buffer plan with deterministic underrun behavior before
shipping it. Do not perform best-effort file I/O in the callback.

## Target

- Prepared media lanes/sources, routing, render execution, seek/loop/reinstall state, and export-tail calculation.
- CLI offline rendering and desktop playback of local and locked-package recordings.
- Fixtures for overlapping cues, tempo change, repeated fixed cue, loop/crop/rate clip, routing/sends, missing asset,
  seek, and project end.
- Determinism, block partition, offline/live, allocation, NaN, bounded-memory, and routing-isolation tests.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-audio -p musa-engine -p musa-project -p musa
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo deny check
cargo bench -p musa-audio
cargo insta test --workspace --unreferenced=reject
```

Commit as `Render clips and fixed media cues`.

## Stop

- No microphone recording, destructive editing, waveform editor, beat detection, transient slicing, or pitch-preserving
  time stretching.
- No audio bytes or decoder handles cross into the compiler/kernel.
- No silent callback underrun or callback-time asset load.
