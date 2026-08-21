---
id: 180
slug: expressive-control-realization
status: pending
depends_on: [174, 177, 178, 179]
phase: 3
---

# A Mark Moves a Musical Control

> **Governed by the event-track and machine core installed by prompts 127a–127e and 171–174.** Controls remain gesture
> payload data until scheduling and instrument-machine binding.

## Task

Complete the typed bridge from score interpretation to sound. Turn gesture-lane controls into each assigned instrument's
exposed controls, resolve them to private DSP parameters during audio preparation, and execute sample- accurate events
or deterministic ramps. Dynamics, hairpins, articulations, slurs, pedal, and custom automation become audible without
making any notation fact denote a DSP operation.

## Read

- `docs/rules/language/08-performance-and-sound.md`; prompt 177 gesture laws, prompt 178 signatures/mappings, prompt 179
  routing.
- Kernel `Progress`; current profile hairpin interpretation; `PerformanceEvent::Parameter`; audio modulation
  combination/smoothing; MIDI control export/loss reporting.
- OMT `007-other-aspects-of-notation.md` and instrument-specific articulation discussion. Cite it for terminology, then
  state Musa's normalized control semantics and mappings as Musa definitions.

## Design

Profiles map notation into standard gestures/controls; instruments map those controls into implementation behavior. At
minimum:

- dynamics and hairpins produce normalized `expression` values/curves;
- accents/marcato produce per-note `emphasis` in addition to any profile gate choice;
- staccato/tenuto produce `separation` while retaining their technique tag;
- slur/legato grouping reaches an instrument as grouping/transition intent, not an ADSR alias;
- pedal produces a typed sustain switch/curve;
- explicitly written automation may target any exposed standard or namespaced custom control of compatible type.

An instrument may map one control to several private parameters with declared typed transfer curves. Score-driven
controls and studio-local modulation combine only by the target descriptor's explicit policy; order is specified and
tested. An unsupported technique or unbound produced control is diagnosed or follows the signature's documented
fallback; silence-by-ignore is forbidden.

Keep exact breakpoints through preparation. Resolve `ControlKey` to compact private parameter indices once, off the
audio thread. Specify frame quantization, same-frame ordering, ramp endpoint behavior, seek/reinstall state, and
block-partition invariance. Delete or privatize the old public `ParameterId` path; a private resolved parameter event is
allowed only after binding.

## Target

- End-to-end standard/custom control production, signature checking, binding, preparation, and render execution.
- Native instruments demonstrating expression mapped to gain+timbre, brightness mapped to filter behavior, legato
  grouping, sustain, and one explicitly physical custom control.
- Positive/negative fixtures, MIDI mapping/loss tests, exact-curve laws, block-size differential tests, and audible WAV
  assertions.
- Remove prompt 93's `ignored-parameter-event` and `graph-topology-modulation-address` ledger entries, which name this
  prompt by slug.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-dsp -p musa-notation -p musa-playback -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- render tests/fixtures/audio-bridge.musa --to wav -o /tmp/musa-expression.wav
```

Commit as `Bind musical controls to instrument sound`.

## Stop

- No rule that crescendo means filter cutoff, slur means ADSR, or dynamic means decibels.
- No frame/control-rate sampling in the event track and no raw graph address in a performance profile.
- No GUI automation editor, sample format, or plug-in hosting.
