---
id: 124
slug: expressive-control-realization
status: pending
depends_on: [121, 122, 123]
phase: 3
---

# A Mark Moves a Musical Control

## Task

Complete the typed bridge from score interpretation to sound. Turn gesture-lane controls into each assigned
instrument's exposed controls, resolve them to private DSP parameters during audio preparation, and execute sample-
accurate events or deterministic ramps. Dynamics, hairpins, articulations, slurs, pedal, and custom automation become
audible without making any notation fact denote a DSP operation.

## Read

- `docs/language/08-performance-and-sound.md`; prompt 121 gesture laws, prompt 122 signatures/mappings, prompt 123
  routing.
- Kernel `Progress`; current profile hairpin interpretation; `PerformanceEvent::Parameter`; audio modulation
  combination/smoothing; MIDI control export/loss reporting.
- OMT `007-other-aspects-of-notation.md` and instrument-specific articulation discussion. Cite it for terminology, then
  state Musa's normalized control semantics and mappings as Musa definitions.

## Design

Profiles map notation into standard gestures/controls; instruments map those controls into implementation behavior.
At minimum:

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
- Remove prompt 93's ignored-parameter and graph-address ledger entries.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-audio -p musa-render -p musa-engine -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa-cli -- render tests/fixtures/audio-bridge.musa --to wav -o /tmp/musa-expression.wav
```

Commit as `Bind musical controls to instrument sound`.

## Stop

- No rule that crescendo means filter cutoff, slur means ADSR, or dynamic means decibels.
- No frame/control-rate sampling in the kernel and no raw graph address in a performance profile.
- No GUI automation editor, sample format, or plug-in hosting.
