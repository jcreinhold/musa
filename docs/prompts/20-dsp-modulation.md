---
id: 20
slug: dsp-modulation
status: pending
depends_on: [19]
phase: 2
---

# DSP: Envelopes, Filters, and Modulation

## Task

Replace prompt 19's placeholders with real processors for the modulation core: ADSR envelope, LFO, one-pole and biquad
filters, scale/bias/clamp/smoothing control stages — driven by the parameter system's typed units, ranges, and
modulation-combination policies. After this prompt the `glass_pad` patch from §7.1 sounds like a pad, not a buzz.

## Read

- Roadmap §13.5 (envelope in the voice structure), §13.6 (initial processor list), §13.7 (`ParameterDescriptor` and
  modulation as typed connections; no anonymous normalized 0..1), §17.5 (envelope stages, filter response, modulation
  ranges, NaN-freedom).
- Prompt 11's parameter skeleton and prompt 19's placeholders.

## Design

- New processors in `musa-audio` (all satisfying prompt 11's RT contract):
  - **ADSR**: gate-triggered, segment-exact state machine; per-voice in polysynth patches. Times from unit-typed
    parameters (`ms`/`s`), sustain a level (0..1).
  - **LFO**: oscillator at control rate (or audio rate with control output — pick control rate for cost; document),
    waveforms sine/triangle/square, frequency in Hz.
  - **Filters**: one-pole low-pass; biquad low-pass/high-pass with `cutoff: Hz` and `q`. Coefficient updates smoothed;
    no zipper noise; stable at extreme settings (clamp cutoff to `[10 Hz, 0.45·sr]`).
  - **Control stages**: `scale`, `bias`, `clamp`, `smoothing` — matching §7.1's chain vocabulary.
- Parameter system completion (§13.7): `ParameterDescriptor { unit, range, default, smoothing, combination }` is now
  load-bearing. `modulate lfo -> glass_pad.lowpass.cutoff` compiles to a control connection; at render time the
  connection's value is combined per `combination` (Add/Multiply/Replace), clamped to `range`, smoothed per `smoothing`.
  Language-side units were checked in prompt 19 against the same descriptors — verify the descriptors are literally
  shared.
- Per-voice integration: patch structure from §13.5 — allocator → pitch-to-frequency → oscillator bank → amplitude ADSR
  → optional filter envelope → per-voice gain/pan → mix. The prompt-12 placeholder ramp envelope is replaced by the
  default ADSR (fast attack, short release) so unenveloped patches still don't click. Dynamics from prompt 18 profiles
  now scale voice gain (the amplitude boundary designed there).
- Fundsp decision point (§13.6): if hand-rolling biquads/ADSR looks like reimplementing fundsp badly, adopt it **as a
  private implementation detail** for these processors. Either way its types never appear publicly. Record the decision
  in the module docs.
- Remove the envelope/lowpass/lfo/scale/bias placeholders from prompt 19; the §7.1 `glass_pad` patch compiles with zero
  placeholder warnings.

## Target

- `musa-audio`: ADSR, LFO, one-pole, biquads, control stages; modulation connection execution; per-voice patch
  integration.
- Tests (§17.5): ADSR stage timing (frame-exact segment boundaries), biquad frequency response (sine sweep at cutoff ≈
  −3 dB), one-pole impulse response, modulation combination/clamping at range extremes, NaN/infinity absence under
  adversarial modulation, determinism.
- Golden-ear check: render `glass-mountain.musa` and compare spectral character (filtered pad vs raw sine)
  programmatically — e.g. high-frequency energy below a bound relative to the prompt-12 render.

## Check

```sh
cargo nextest run -p musa-audio
cargo clippy --all-targets -p musa-audio -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- render examples/glass-mountain.musa --to wav -o /tmp/gm6.wav
```

Commit as `Add envelopes, filters, and typed modulation`.

## Stop

- No delay/chorus/reverb/buses/sends DSP (prompt 21) — `reverb` stays a placeholder.
- No band-limited/analog oscillators, compressor, distortion (§13.6 "later" list).
- No GUI modulation routing (Sound workspace follows prompt 21).
