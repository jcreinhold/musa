---
id: 184
slug: sampler-runtime
status: done
depends_on: [174, 177, 178, 179, 180, 180a, 182]
phase: 4
---

# A Sample Map Is an Instrument Implementation

> **Governed by the event-track and machine core installed by prompts 127a–127e and 171–174.** The sampler is a
> registered machine primitive family behind an instrument contract.

## Task

Implement a deterministic native sample-instrument runtime behind the instrument contract. Notes select prepared sample
regions by pitch, velocity/expression, technique, grouping, pedal, and sequence policy; each voice performs pitch-rate
conversion, looping, envelopes, release behavior, gain/pan, and stealing without allocation, I/O, locking, or random
choice on the audio thread.

## Read

- `docs/rules/language/08-performance-and-sound.md` and `09-assets-and-packages.md`; roadmap §§13.2, 13.5, 17.5, 18
  Phase 4.
- Prompt 177 gestures, prompt 178 instrument implementation family, prompt 179 instance routing, prompt 180 controls,
  prompt 182 asset store; current voice allocator, oscillator instrument, render plan, and engine retirement path.
- Existing allowed dependency lists and decoder/resampler implementations. Measure before adding a large dependency;
  keep its types private if one is justified.
- Note 79 and the repaired instrument prompts: sample maps and instrument declarations are source values; normalized
  region tables, decoder state, voices, and resamplers are private runtime projections.

## Design

Define the author-facing sample-map/region vocabulary as ordinary source data, then derive a private normalized runtime
projection with regions selected by bounded predicates: key/range, velocity or normalized expression range,
standard/custom technique, trigger/release condition, pedal state, sequence position, and deterministic weighted/random
selector. A region declares sample asset, root pitch, tuning, gain/pan, playback and loop bounds/mode, envelope,
exclusive group, and optional release sample.

Selection is prepared or computed from compact immutable tables. Round-robin is a per-instance deterministic sequence;
randomized selection is a stateless stable function of realization seed, instrument instance, semantic event identity,
and region group. It never consumes ambient RNG state and therefore survives block-size, seek, and offline/live changes.

Decode validated PCM and build resampling/loop metadata off-thread. Start with bounded preloading; do not promise disk
streaming until measured assets require it. Pitch ratio derives from sounding frequency/root frequency at the DSP edge.
Specify interpolation quality, loop boundary behavior, stereo/mono handling, note-off/release, pedal, voice stealing,
and denormal/NaN safety. Asset failure prevents plan installation rather than failing inside render.

The source map and private normalized table have complete differential laws. Rust callers cannot construct an
authoritative sample instrument without a checked source map; foreign adapters in 185–186 produce the same declared
contract before private normalization.

## Target

- Source-declared sample map/instrument implementation plus private normalized region/voice runtime in `musa-dsp`.
- Project/audio asset preparation and bounded-memory reporting; no decoder state in compiler facts.
- Tiny native sample-map fixture exercising pitch regions, velocity layers, round-robin, loop, release, pedal, and
  instrument swapping.
- Determinism, isolation, frame/block partition, loop, resampling, voice-stealing, NaN, allocation, and offline/live
  laws.

## Check

```sh
cargo nextest run -p musa-dsp -p musa-playback -p musa-project -p musa
cargo clippy --all-targets -p musa-dsp -p musa-playback -p musa-project -p musa -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo deny check
cargo bench -p musa-dsp
```

Commit as `Add the deterministic sampler runtime`.

## Stop

- No SFZ/SoundFont parser yet, no disk streaming, convolution, time stretching, or waveform editing.
- No sampler types in the event track or score snapshot and no file access in the callback.
- No consumer-time aleatory choice detached from the declared realization seed and event identity.
