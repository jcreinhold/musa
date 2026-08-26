---
id: 211
slug: aligned-stem-rendering
status: pending
depends_on: [210]
phase: 4
---

# Render Aligned Stems from the Existing Mix

## Task

Make the source-declared routing graph render deterministic, frame-aligned part-output and named-bus/return stems beside
the existing master mix. A stem is a tap on an already checked route, not a second mixer model or a new language
construct.

## Read

- Prompt 210's governing DAW contract; prompts 173, 179–180, 188, and 191;
  `docs/rules/language/08-performance-and-sound.md` §§5–8; `docs/rules/desktop/09-sound-and-mix.md`; the current
  source-to-runtime studio projection.
- Current `musa-dsp` graph preparation/offline renderer, `musa-project` WAV export, part/bus identity, tail handling,
  routing laws, cache arguments, and callback instrumentation.

## Design

Derive taps only from checked source routing: one for every part output and every explicitly named bus/return, plus the
existing master. Do not add a Rust stem declaration, hidden solo/mute state, or source syntax. The host projection may
hold compact tap indices, but each index must differential-test back to one stable source identity and tap point.

Every output begins at project frame zero and has the same final frame count, including declared release/effect tails
and silence before or after a tap is active. Write 32-bit floating-point WAV at the requested sample rate and channel
layout, using the same seed, preparation arguments, processor order, rounding, and one-frame semantics as master export.

Name files from display names only for readability. Resolve normalization, forbidden characters, case folding, Unicode
normalization, and collisions with a deterministic suffix derived from the stable semantic identity; the manifest in
prompt 212 remains the authoritative mapping.

Do not promise that summing stems recreates the master. Sends duplicate signal, returns can share nonlinear processing,
and a nonlinear master chain cannot be reconstructed from pre-master taps. Instead record every tap point and routing
edge needed to understand the projection, and test exact master parity against the existing master path.

Render all requested outputs in one bounded traversal when measurement shows that this preserves the one-frame laws and
reduces work; otherwise share immutable preparation and render independent state instances. Measure before choosing.
Never multiply decoded asset storage per stem.

## Target

- A narrow project-owned offline multitrack export operation and private DSP tap projection; public types state format,
  alignment, tap identity, and produced-file metadata rather than exposing graph nodes.
- Deterministic part/bus/master WAV fixtures covering silence, sends, shared returns, nonlinear master processing, media
  tails, duplicate display names, and non-ASCII names.
- Laws for equal master bytes, common frame count, block-partition independence, stable names, bounded memory, and
  source-to-runtime tap differential parity.
- Measured cost against one representative many-part/many-return work, recorded with prompt 191's method.

## Check

```sh
cargo nextest run -p musa-dsp -p musa-project
cargo clippy --all-targets -p musa-dsp -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
```

Commit as `Render aligned DAW stems`.

## Stop

- No DAW bundle, MIDI transport, Audio Unit, UI, normalization/mastering, waveform editor, or destructive audio work.
- No claim that exported taps are additive when the declared graph is not.
- No graph internals, decoder state, or mutable render state crossing `musa-dsp`'s facade.
