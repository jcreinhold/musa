---
id: 12
slug: polysynth-wav
status: pending
depends_on: [11]
phase: 1
---

# Polyphonic Synth and WAV Export

## Task

Make the compiler's `PerformancePlan` audible: a polyphonic sine synthesizer with a
voice allocator, wired end-to-end so `musa render piece.musa --to wav` compiles the
piece, lowers the performance, builds the graph, and renders a deterministic WAV file
offline.

## Read

- Roadmap §13.1 (pipeline: scheduled events → voice allocators → instrument DSP →
  master), §13.5 (playable synth structure: allocator, pitch-to-frequency, oscillator
  bank, envelope, per-voice gain/pan), §13.8 (hound WAV; offline == live), §4 (core
  use case step 10: play must work with zero setup).
- Prompt 10's `PerformancePlan`, prompt 11's `RenderPlan`.

## Design

- `musa-audio` additions:
  - A `VoiceAllocator` (fixed voice pool, e.g. 16 voices, steal-oldest policy) turning
    `NoteOn`/`NoteOff` events into per-voice gate + frequency control.
  - A sine polysynth instrument built from prompt 11's processors: per-voice
    oscillator + simple attack/release gain smoothing (a full ADSR is prompt 20; a
    short linear ramp now to avoid clicks — document that it is a placeholder
    envelope, not an articulation model).
  - A **default instrument**: when the source has no `studio` block (always, until
    prompt 19), every part gets this sine polysynth (§14.8: a new piece is immediately
    audible).
  - Offline renderer:

    ```rust
    pub fn render_offline(
        plan: &mut RenderPlan,
        events: &PerformanceEvents,
        frames: u64,
    ) -> RenderedAudio;   // interleaved f32 + sample rate
    ```

- Where does orchestration live? `musa-cli`'s `render --to wav` currently must chain
  compile → lower_performance → compile_graph → render_offline. That chain is exactly
  what `musa-project` will own (prompt 14). Until then, put the chain in **one**
  function `musa_audio::render_piece_wav(...)` — no, do not: audio must not depend on
  language parsing. Instead put a small `pub fn render_to_wav(source, options)` in
  `musa-cli` behind one `orchestrate` module with a `// TODO(prompt-14): move to
  musa-project` marker. Keep it under 50 lines so the migration is mechanical.
- WAV: 32-bit float, stereo, 48 kHz default via `hound` (§13.8). Duration = piece span
  + release tail (fixed 1 s until envelopes exist).
- Deterministic: same source → byte-identical WAV. This is a headline test (§17.5).

## Target

- `musa-audio`: voice allocator, sine polysynth default instrument, `render_offline`.
- `musa-cli`: `render --to wav` with the marked orchestration shim.
- Tests: allocator voice-stealing and note-off matching; click-free on/off ramps
  (max sample discontinuity bound); end-to-end WAV determinism for all examples; a
  golden-frequency test (render a single 440 Hz A4, verify dominant frequency within
  tolerance).
- Render all three examples to WAV and listen or inspect one manually — this is the
  first audible output of the system.

## Check

```sh
cargo nextest run -p musa-audio -p musa-cli
cargo clippy --all-targets -p musa-audio -p musa-cli -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- render examples/glass-mountain.musa --to wav -o /tmp/gm.wav
cargo run -p musa-cli -- render examples/glass-mountain.musa --to wav -o /tmp/gm2.wav
cmp /tmp/gm.wav /tmp/gm2.wav   # deterministic
afinfo /tmp/gm.wav 2>/dev/null || file /tmp/gm.wav
```

Commit as `Add polyphonic sine synth and WAV export`.

## Stop

- No CPAL live playback (prompt 13).
- No ADSR/LFO/filter/effects, no studio DSL (prompts 19–21).
- No MIDI export (prompt 18).
- Do not add project/session orchestration beyond the marked shim.
