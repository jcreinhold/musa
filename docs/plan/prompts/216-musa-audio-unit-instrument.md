---
id: 216
slug: musa-audio-unit-instrument
status: pending
depends_on: [215]
phase: 4
---

# Host a Musa Instrument as an AUv3 Music Device

## Task

Ship a macOS AUv3 Music Device that loads one source-declared Musa instrument and renders it from host MIDI with the
same prepared-machine semantics as native playback. Keep Apple plumbing in a thin extension and all semantic and DSP
work behind an audited Rust boundary.

## Read

- Prompt 215's accepted architecture/report and repaired version of this prompt; prompts 174, 178–180, 182–184, 188, and
  191; the current checked-value projection, asset store, prepared instrument, render plan, and RT instrumentation.
- Apple's APIs named by prompt 215, especially `AUAudioUnitFactory`, `AUAudioUnit`, `internalRenderBlock`, render
  events, `allocateRenderResources`, `reset`, `fullStateForDocument`, tail/latency, and out-of-process debugging.

## Design

Add a narrow Rust `staticlib`/C ABI owned by a new shell crate only if prompt 215 proves it is the smallest maintainable
boundary. Its control-side operations validate a complete source/package/asset closure, select an instrument
declaration, compile and prepare it for an exact audio format, and return an immutable installable handle plus
diagnostics. Its render operation accepts plain fixed-layout MIDI/parameter events and noninterleaved/known-layout
buffers; no Rust, CPAL, DSP-node, compiler, asset-store, or allocator type crosses the ABI. Every unsafe boundary has a
written ownership, threading, lifetime, aliasing, panic, and error invariant with negative tests.

The extension's control worker performs source checking, package/asset resolution, decode, preparation, and atomic plan
replacement. `internalRenderBlock` consumes the installed plan, host sample time, and already supplied render events,
renders at most `maximumFramesToRender`, and publishes bounded counters. It allocates, locks, performs I/O, logs,
compiles, decodes, invokes UI code, or destroys large state never. Reset and resource allocation obey the measured trial
contract; old plans return through a control-side retirement queue.

MIDI is an edge projection. The component documents its MIDI 1.0 channel/pitch/controller support and reports losses for
Musa controls the host stream cannot carry. It does not reinterpret notation or load a whole piece. The same instrument,
seed, format, input event history, parameter history, and initial state must render the same frames as the native
prepared path for every host block partition.

`fullStateForDocument` stores the versioned portable closure admitted by prompt 215, selected declaration and complete
parameter-address mapping; presets store only state the trial proves appropriate for `fullState`. Restoration validates
all exact identities off-thread and yields silence plus an actionable visible diagnostic until ready. Missing locked
assets are never replaced by same-named bytes.

The containing app is the source/asset manager and diagnostic UI, not a second editor. It can open an existing Musa
project, choose an instrument, install its exact closure into the extension's App Group, and reveal the canonical source
in Musa desktop. It does not recreate instrument declarations in Swift.

## Target

- Production AUv3 Music Device and containing app under `apps/musa-audio-unit/`, plus the smallest audited Rust shell
  crate/library and generated C header justified by prompt 215.
- Control-worker preparation, versioned state restoration, App Group asset resolution, missing/incompatible-state
  diagnostics, and native/AU differential renderer.
- Automated host tests for MIDI/event boundaries, reset, format changes, offline render, discontinuity behavior, tail,
  silence-before-ready, extension relaunch, and corrupted state.
- RT allocation/lock/I/O/log/destruction instrumentation across Swift/Objective-C and Rust, Thread Sanitizer coverage,
  `auval` validation for component `aumu:musa:MUSA`, and dependency/layer audits.

## Check

```sh
cargo nextest run -p musa-dsp -p musa-project -p musa-au
cargo clippy --all-targets -p musa-dsp -p musa-project -p musa-au -- -D warnings
xcodebuild -project apps/musa-audio-unit/MusaAudioUnit.xcodeproj -scheme MusaAudioUnit -configuration Debug CODE_SIGNING_ALLOWED=NO test
scripts/check-audio-unit.sh instrument
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo deny check
```

Commit as `Host Musa instruments as an Audio Unit`.

## Stop

- No whole-piece sequencer, MIDI Processor, exposed-control parameter tree beyond the minimum trial control, or
  multi-output routing; prompts 217–218 own those.
- No callback compile/decode/I/O, dynamic plug-in hosting, proprietary DAW document, or alternative editable instrument.
- No signing identity, notarization, App Store submission, installer, or release publication.
