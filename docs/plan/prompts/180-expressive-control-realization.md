---
id: 180
slug: expressive-control-realization
status: pending
depends_on: [174, 177, 178, 179]
phase: 3
---

# A Source-Declared Musical Control Moves Sound

## Task

Complete the typed bridge from source performance profiles to source instrument mappings and private sound. Bind exact
gesture controls to each selected instrument signature, evaluate its declared transfer mappings during preparation,
resolve the result to private DSP parameters, and execute sample-accurate events or deterministic ramps. No notation
fact denotes a DSP operation and no Rust table defines musical control policy.

## Read

- `docs/rules/language/08-performance-and-sound.md` §§0, 2–4, 7; repaired prompts 177–179 and note 79.
- Source `ControlKey<K>`/`ControlValue<K>`, instrument mappings, `Progress`, current modulation/smoothing, and MIDI
  mapping/loss paths.
- OMT 007 and instrument-specific articulation discussion. Cite terminology while stating normalized control semantics
  and mappings as Musa definitions.
- Pattern-unification laws: key, value, and mapping domain share one source index and use no domain-specific solver.

## Design

Profiles map notation into standard source-declared gestures/controls; instruments map those controls into
implementation behavior through source definitions. At minimum, the edition-pinned library defines expression, emphasis,
separation, phrase/legato grouping, sustain, brightness, and explicit namespaced custom controls without claiming
universality.

Instrument mappings are total source functions/finite mapping data checked at the shared control index. One control may
map to several private primitive parameters with typed transfer curves. Score-driven and studio-local modulation combine
only by the registered target descriptor's host-owned combination policy. An unsupported technique/control follows the
source signature's explicit fallback or is diagnosed; silence-by-ignore is forbidden.

Finite preparation evaluates mappings before the runtime boundary. Rust receives exact dimensioned target values from
one checked source mapping, resolves stable primitive/parameter keys to compact private indices, and quantizes/samples
only downstream. It does not carry `ControlKey` as a closed enum or reevaluate source functions on the audio thread.

Keep exact breakpoints through preparation. Specify frame quantization, same-frame ordering, ramp endpoints,
seek/reinstall state, and block-partition invariance. Delete or privatize the old public `ParameterId` path.

## Target

- End-to-end source standard/custom control production, indexed checking, source instrument mapping, preparation, and
  render execution.
- Standard-library native instruments demonstrating expression→gain+timbre, brightness behavior, grouping, sustain, and
  one physical custom control.
- Positive/negative pattern-unification and compatibility fixtures; MIDI loss tests; exact curve and block-differential
  laws; audible WAV assertions.
- Private resolved parameter events only after binding; no authoritative Rust control/mapping vocabulary.
- Removal of prompt 93's ignored-parameter and graph-address ledger rows.

## Check

```sh
cargo nextest run -p musa-calculus -p musa-compiler -p musa-dsp -p musa-notation -p musa-playback -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- render tests/fixtures/audio-bridge.musa --to wav -o /tmp/musa-expression.wav
```

Commit as `Bind source musical controls to instrument sound`.

## Stop

- No universal rule that crescendo means cutoff, slur means ADSR, or dynamic means decibels.
- No frame sampling in event tracks, raw graph address in a profile, source function in the callback, or Rust musical
  control enum.
- No GUI automation editor, sample format, plug-in hosting, special unifier, coercion, or inferred default control kind.
