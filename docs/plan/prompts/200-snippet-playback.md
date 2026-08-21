---
id: 200
slug: snippet-playback
status: pending
depends_on: [173, 188, 199]
phase: 5
---

# Deferred: Snippet Playback in the Page

## Task

**Deferred — do not start until prompts 196–199 are done and a real user need is demonstrated.** Give `<musa-score>` an
opt-in play button: the snippet's score is offline-rendered to PCM in the browser and played through an AudioWorklet,
with the playhead's position mapped back onto the engraved score via the provenance contract.

## Read

- Prompt 194 (the wasm shell this extends — the audio pipeline must join it without CPAL or `musa-playback`, which are
  native-only) and prompts 15–17 (performance plan, offline audio core), 185–186 (clip/cue semantics the playback must
  respect).
- Prompt 198's provenance interaction — playhead highlighting is `highlight(eventId)` driven by a clock, not a new
  mechanism.

## Design

To be written when this prompt is scheduled. The shape it must take, fixed now so earlier prompts do not foreclose it:
`musa-dsp`'s prepared machine and offline renderer cross to wasm behind a feature flag (CPAL and the engine stay
native); the web package gains `playback?: boolean` per score; playback state (playhead time → covering events) is
computed from the scheduled event decisions, not by parsing SVG. Determinism and exact rational time are inherited, not
re-derived.

## Target

- Deferred.

## Check

- Deferred.

## Stop

- Everything, until scheduled. In particular: do not let prompts 196–199 bake in assumptions that make this hard (a
  `typeset` result that discards the `PerformancePlan`, an engraver option that loses event ids) — that is the only
  obligation this prompt places on them today.
