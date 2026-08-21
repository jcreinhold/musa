---
id: 183
slug: sampler-runtime
status: pending
depends_on: [173, 176, 177, 178, 179, 181]
phase: 4
---

# A Sample Map Is an Instrument Implementation

> **Governed by the event-track and machine core installed by prompts 127a–127e and 170–173.** The sampler is a
> registered machine primitive family behind an instrument contract.

## Task

Implement a deterministic native sample-instrument runtime behind the instrument contract. Notes select prepared sample
regions by pitch, velocity/expression, technique, grouping, pedal, and sequence policy; each voice performs pitch-rate
conversion, looping, envelopes, release behavior, gain/pan, and stealing without allocation, I/O, locking, or random
choice on the audio thread.

## Read

- `docs/rules/language/08-performance-and-sound.md` and `09-assets-and-packages.md`; roadmap §§13.2, 13.5, 17.5, 18
  Phase 4.
- Prompt 176 gestures, prompt 177 instrument implementation family, prompt 178 instance routing, prompt 179 controls,
  prompt 181 asset store; current voice allocator, oscillator instrument, render plan, and engine retirement path.
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

- Concrete sample implementation variant and private region/voice runtime in `musa-dsp`.
- Project/audio asset preparation and bounded-memory reporting; no decoder state in compiler facts.
- Tiny native sample-map fixture exercising pitch regions, velocity layers, round-robin, loop, release, pedal, and
  instrument swapping.
- Determinism, isolation, frame/block partition, loop, resampling, voice-stealing, NaN, allocation, and offline/live
  laws.

## Check

```sh
cargo nextest run -p musa-dsp -p musa-playback -p musa-project -p musa
cargo clippy --all-targets -p musa-dsp -p musa-playback -p musa-project -p musa -- -D warnings
cargo fmt --check
cargo deny check
cargo bench -p musa-dsp
```

Commit as `Add the deterministic sampler runtime`.

## Stop

- No SFZ/SoundFont parser yet, no disk streaming, convolution, time stretching, or waveform editing.
- No sampler types in the event track or score snapshot and no file access in the callback.
- No consumer-time aleatory choice detached from the declared realization seed and event identity.
