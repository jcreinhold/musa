---
id: 188
slug: audio-clips
status: done
depends_on: [173, 179, 182, 184, 187]
phase: 4
---

# Recorded Media Reaches the Mix

> **Governed by the event-track and machine core installed by prompts 127a–127e and 171–174.** Clips and cues become
> registered source machines connected to explicit mix machines.

## Task

Render musical clips and fixed-media cues through the prepared audio plan. Decode and prepare recordings off-thread,
schedule their starts from the appropriate beat/physical-time semantics, apply the source-owned crop/loop/rate and gain
settings, route named media outputs through buses/sends, and preserve deterministic seek, offline/live, and export-tail
behavior.

## Read

- Prompt 187's normative semantics; prompt 182 asset store; prompt 184 prepared sample playback and resampling; prompt
  177 part isolation; existing offline/live render path, transport seek/loop, and release-tail calculation.
- Roadmap §§13.2, 13.8, 18 Phase 4. Audio recording and waveform editing remain explicitly outside Musa.
- Note 79: preparation consumes exact projections of source media declarations; only decoded bytes, routing indices,
  schedules, and playback state are private Rust data.

## Design

Media is a separate prepared source machine, not a score part, voice, mixer track, or instrument instance. Preparation
resolves each semantic cue/clip identity to immutable decoded audio and compact routing indices. A fixed cue schedules
start at `tempo(b)` and retains natural physical duration. A musical clip uses its transformed beat span and explicit
fit policy. Specify channel conversion, sample-rate conversion, bounds, gain, overlap, retrigger, same-frame ordering,
and end-of-project tail.

No public Rust cue/clip schema may supply policy absent from the checked source value. The preparation projection is
opaque/read-only and covered by source-to-projection differential laws.

`MediaPlayback` currently declares only exact `gain_db`; this prompt therefore implements no independently authored fade
or pan policy. Those controls require a later governing source amendment that states their units, ranges, interaction
with restriction, and concise authoring path before native preparation may project them. Neutral native fade/pan
constants are implementation facts, not falsely advertised source settings.

`crop` stops at the beat span, `loop` repeats deterministically and truncates at it, and `rate` chooses the documented
constant playback rate required by the source span/policy and changes pitch honestly. No implicit tempo-following or
pitch-preserving warp occurs. Tempo changes inside a loop affect scheduled span boundaries, not fixed-media sample
duration.

Prepare/decode/preload and retire off-thread. Use bounded preloading initially; if a checked realistic recording misses
memory budget, profile and design a control-side streaming/ring-buffer plan with deterministic underrun behavior before
shipping it. Do not perform best-effort file I/O in the callback.

Edition one prepares canonical PCM/float WAV through the deterministic native decoder. The declaration and asset layers
may identify other audio adapters, but audio preparation must refuse those formats explicitly until their deterministic
decoder is specified and installed; verification alone must never imply render support.

## Target

- Prepared media source machines, routing, render execution, seek/loop/reinstall state, and export-tail calculation.
- CLI offline rendering and desktop playback of local and locked-package recordings.
- Fixtures for overlapping cues, tempo change, repeated fixed cue, loop/crop/rate clip, routing/sends, missing asset,
  seek, and project end.
- Determinism, block partition, offline/live, allocation, NaN, bounded-memory, and routing-isolation tests.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-dsp -p musa-playback -p musa-project -p musa
cargo clippy --workspace --all-targets -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo deny check
cargo bench -p musa-dsp
cargo insta test --workspace --unreferenced=reject
```

Commit as `Render clips and fixed media cues`.

## Stop

- No microphone recording, destructive editing, waveform editor, beat detection, transient slicing, or pitch-preserving
  time stretching.
- No audio bytes or decoder handles cross into the compiler/event track.
- No silent callback underrun or callback-time asset load.
