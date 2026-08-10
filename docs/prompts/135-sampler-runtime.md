---
id: 135
slug: sampler-runtime
status: pending
depends_on: [128, 129, 130, 131, 133]
phase: 4
---

# A Sample Map Is an Instrument Implementation

## Task

Implement a deterministic native sample-instrument runtime behind the instrument contract. Notes select prepared sample
regions by pitch, velocity/expression, technique, grouping, pedal, and sequence policy; each voice performs pitch-rate
conversion, looping, envelopes, release behavior, gain/pan, and stealing without allocation, I/O, locking, or random
choice on the audio thread.

## Read

- `docs/language/08-performance-and-sound.md` and `09-assets-and-packages.md`; roadmap §§13.2, 13.5, 17.5, 18 Phase 4.
- Prompt 128 gestures, prompt 129 instrument implementation family, prompt 130 instance routing, prompt 131 controls,
  prompt 133 asset store; current voice allocator, oscillator instrument, render plan, and engine retirement path.
- Existing allowed dependency lists and decoder/resampler implementations. Measure before adding a large dependency;
  keep its types private if one is justified.

## Design

Define a private normalized sample-map representation with regions selected by bounded predicates: key/range, velocity
or normalized expression range, standard/custom technique, trigger/release condition, pedal state, sequence position,
and deterministic weighted/random selector. A region declares sample asset, root pitch, tuning, gain/pan, playback and
loop bounds/mode, envelope, exclusive group, and optional release sample.

Selection is prepared or computed from compact immutable tables. Round-robin is a per-instance deterministic sequence;
randomized selection is a stateless stable function of realization seed, instrument instance, semantic event identity,
and region group. It never consumes ambient RNG state and therefore survives block-size, seek, and offline/live changes.

Decode validated PCM and build resampling/loop metadata off-thread. Start with bounded preloading; do not promise disk
streaming until measured assets require it. Pitch ratio derives from sounding frequency/root frequency at the DSP edge.
Specify interpolation quality, loop boundary behavior, stereo/mono handling, note-off/release, pedal, voice stealing,
and denormal/NaN safety. Asset failure prevents plan installation rather than failing inside render.

## Target

- Concrete sample implementation variant and private region/voice runtime in `musa-audio`.
- Project/audio asset preparation and bounded-memory reporting; no decoder state in compiler facts.
- Tiny native sample-map fixture exercising pitch regions, velocity layers, round-robin, loop, release, pedal, and
  instrument swapping.
- Determinism, isolation, frame/block partition, loop, resampling, voice-stealing, NaN, allocation, and offline/live laws.

## Check

```sh
cargo nextest run -p musa-audio -p musa-engine -p musa-project -p musa
cargo clippy --all-targets -p musa-audio -p musa-engine -p musa-project -p musa -- -D warnings
cargo fmt --check
cargo deny check
cargo bench -p musa-audio
```

Commit as `Add the deterministic sampler runtime`.

## Stop

- No SFZ/SoundFont parser yet, no disk streaming, convolution, time stretching, or waveform editing.
- No sampler types in the kernel or score snapshot and no file access in the callback.
- No consumer-time aleatory choice detached from the declared realization seed and event identity.
