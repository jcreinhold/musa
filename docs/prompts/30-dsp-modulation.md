---
id: 30
slug: dsp-modulation
status: done
depends_on: [29]
phase: 2
---

# DSP: Envelopes, Filters, and Modulation

## Task

Replace prompt 29's placeholders with real processors for the modulation core: ADSR envelope, LFO, one-pole and biquad
filters, scale/bias/clamp/smoothing control stages — driven by the parameter system's typed units, ranges, and
modulation-combination policies. After this prompt the `glass_pad` patch from §7.1 sounds like a pad, not a buzz.

## Read

- Roadmap §13.5 (envelope in the voice structure), §13.6 (initial processor list), §13.7 (`ParameterDescriptor` and
  modulation as typed connections; no anonymous normalized 0..1), §17.5 (envelope stages, filter response, modulation
  ranges, NaN-freedom).
- Prompt 16's parameter skeleton and prompt 29's placeholders.

## Design

- New processors in `musa-audio` (all satisfying prompt 16's RT contract):
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
  Language-side units were checked in prompt 29 against the same descriptors — verify the descriptors are literally
  shared.
- Per-voice integration: patch structure from §13.5 — allocator → pitch-to-frequency → oscillator bank → amplitude ADSR
  → optional filter envelope → per-voice gain/pan → mix. The prompt-12 placeholder ramp envelope is replaced by the
  default ADSR (fast attack, short release) so unenveloped patches still don't click. Dynamics from prompt 28 profiles
  now scale voice gain (the amplitude boundary designed there).
- Fundsp decision point (§13.6): if hand-rolling biquads/ADSR looks like reimplementing fundsp badly, adopt it **as a
  private implementation detail** for these processors. Either way its types never appear publicly. Record the decision
  in the module docs.
- Remove the envelope/lowpass/lfo/scale/bias placeholders from prompt 29; the §7.1 `glass_pad` patch compiles with zero
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

## Repairs made while implementing

- **Hand-rolled, not fundsp** (§13.6's decision point). An RBJ biquad and a segment-exact ADSR are roughly a hundred
  lines each and are the two things this crate must be able to reason about exactly — frame-exact segment boundaries and
  per-sample coefficient interpolation are the *specification*, not implementation detail. A graph library would own the
  block loop that `RenderPlan::render` already owns. The decision is recorded in `envelope.rs` and `filter.rs`; prompt
  31's reverb is where it is worth revisiting.
- **A modulation targets a parameter, not a port.** `SpecBuilder::modulate(from, from_port, to, param)` is checked in
  `compile_graph`: the source must produce a `Control` port and the target must *declare* the parameter. What happens on
  arrival — combine, clamp, smooth — is the descriptor's business. That is what keeps §13.7's "no anonymous normalized
  0..1" true at the type level rather than by convention.
- **An `envelope` stage configures the synth, it does not become a node.** §13.5 puts the amplitude envelope inside the
  voice, and that is the only placement where overlapping notes fade independently. So `lower_container` maps an
  envelope stage onto the `PolySine`'s `attack`/`decay`/`sustain`/`release` parameters, and the stage keeps its source
  span so a modulation can still address it.
- **`StudioLowering::release_tail`.** The longest release in the studio is now reported to `musa-project`, which adds it
  to the export's tail. An export must contain the end of the *sound*, not the end of the notes; glass-mountain's 3.5 s
  release was previously cut off.
- **Only the spine of a patch is lowered.** `spine()` walks backwards from `output` along first inputs, so `shimmer`'s
  `-15 dB` gain no longer applies to the whole patch. A stage off the path is not on the signal.
- **Sends and master inputs are deduped by source node.** Two parts sharing one patch previously produced two master
  inputs and two sends of the same signal — the patch was heard twice. The loudest send level wins, with a lowering note
  saying so.
- **The default envelope changed the golden audio.** Replacing prompt 12's ramp with an ADSR whose segments are counted
  in frames rather than compared as floats moved 26 of `tuplet-fixture`'s 576,000 samples, all at attack boundaries and
  none by more than one ulp. Verified against a worktree at the previous commit before re-pinning the digest in
  `session_laws.rs`; the rationale is in that test's doc comment.
- **Parameter smoothing alone does not stop zipper on a swept cutoff.** Block-rate smoothing still steps once per block,
  and a 250 Hz sweep steps audibly. The biquad interpolates its coefficients per sample across the block
  (`Coefficients::step_to`/`advance`) and snaps to the target at the block's end. The law that caught this compares the
  worst sample-to-sample step against the signal's own slope.
- **The golden-ear check asserts what is true.** A 1400 Hz low-pass is near-transparent for a pad topping out at 1320
  Hz, and `glass_pad` *adds* a partial, so "less high-frequency energy than the raw sine" was simply false. The claims
  kept are the ones the patch actually makes: the bank brightens the tone, the 3.5 s release is still sounding two
  seconds after the last note-off where a plain voice is silent, and a 500 Hz low-pass takes >80% of the high end off
  broadband noise.
- **`PerformedNote.attack` is now honoured.** Prompt 28's doc assigned it here: it replaces the patch's attack for that
  one note and leaves the rest of the shape alone — a request, not an envelope.
- **NaN-freedom is by construction.** `set_param` refuses non-finite values, so the only way one can enter is a
  modulation; `apply_modulations` substitutes the base value for a non-finite control signal, and `Coefficients::new`
  clamps cutoff and q before computing and returns identity for a non-finite request.
- **Control stages declare `Hz`.** A control signal's dimension is really its target's, and with one target per chain
  the descriptor can simply say what that target is. `Processor::params`'s doc says so, and says to revisit when a
  second dimension has a target.
- **A UI test was synchronising on a diagnostic count.** With four placeholder warnings gone, `origin.spec.ts`'s
  diagnostic test could no longer tell "the piece's own warning" from "the error my edit caused" — a count of one was
  satisfied by the stale list. It now waits for the compiler's words.

## Stop

- No delay/chorus/reverb/buses/sends DSP (prompt 31) — `reverb` stays a placeholder.
- No band-limited/analog oscillators, compressor, distortion (§13.6 "later" list).
- No GUI modulation routing (Sound workspace follows prompt 31).
