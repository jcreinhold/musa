---
id: 215
slug: audio-unit-architecture-trial
status: done
depends_on: [210, 214]
phase: 4
---

# Prove the Audio Unit Shape Before Shipping It

## Task

Build the smallest macOS AUv3 trial that can falsify the proposed boundary, then fix prompts 216–218 to the measured
answer. Prove component discovery, out-of-process rendering, MIDI/event timing, state restoration, source/asset access,
parameter identity, output buses, and real-time behavior before coupling the production runtime to Xcode.

## Read

- Prompt 210's contract; prompts 173, 178–180, 182–184, 188, and 191; current prepared instrument/runtime and project
  source/asset closure APIs.
- Apple's current [`AUAudioUnit`](https://developer.apple.com/documentation/audiotoolbox/auaudiounit),
  [`renderBlock`](https://developer.apple.com/documentation/audiotoolbox/auaudiounit/renderblock),
  [`fullStateForDocument`](https://developer.apple.com/documentation/audiotoolbox/auaudiounit/fullstatefordocument),
  parameter-tree, render-event, bus, sandbox/App Group, extension packaging, and validation documentation. Read the
  current Logic and GarageBand Audio Unit guides cited by prompt 210.
- The installed Xcode templates/SDK headers and `auval`; do not infer constants, property-list keys, component
  categories, signing behavior, or callback guarantees from an old article.

## Design

Trial two deliberately small components inside one containing macOS app:

1. an `aumu` Music Device receiving timestamped host MIDI/parameter events and producing stereo audio; and
2. an `aumi` MIDI Processor emitting a fixed finite schedule from host sample/musical/transport context.

The trial uses a tiny deterministic oscillator/schedule, not Musa's compiler or DSP graph. It answers whether the split
itself works. Exercise in-process and out-of-process instantiation where the SDK permits it, offline rendering,
noncontiguous sample times, loop/reset, maximum render frames, parameter ramps, state save/restore, dynamic parameter
tree replacement, multiple output buses, missing host context, extension termination/relaunch, and asset access through
the sandbox.

The expected production boundary is: compile/resolve/decode/prepare on a control worker; publish an immutable prepared
instrument through a tiny versioned C ABI; consume only preallocated state and scheduled events in
`internalRenderBlock`; retire old plans off-thread. The Music Device is MIDI-driven and therefore does not seek a whole
composition. The MIDI Processor owns a finite random-access event schedule and uses host time to select events; it never
steps a synthesizer from the beginning after a seek. Trial evidence may refine this, but may not weaken the RT or
source-authority laws.

For state, test a property-list-safe document snapshot containing version, exact source/package text closure, lock and
asset identities, selected declaration, parameter-address table, and user parameter values. Large asset bytes remain in
the verified project/App Group store and are diagnosed by digest when missing; absolute author-machine paths are not
portable state. Test the smallest viable security-scoped/App Group arrangement.

Measure every callback for allocation, lock, file/network access, logging, Objective-C/Swift runtime work that may
block, and destruction. Use Thread Sanitizer and Audio Workgroup/real-time diagnostics where available. Record
unsupported or host-dependent behavior plainly. Then amend prompts 216–218 before completing this prompt so their
Targets and Checks match the evidence; changing only pending prompts is ordinary stack repair.

## Target

- `apps/musa-audio-unit-trial/`: minimal containing app, Music Device extension, MIDI Processor extension, and automated
  host harness, with no production Musa runtime linkage.
- A checked-in research report with SDK/macOS/Xcode versions, component descriptions, commands, results, timing traces,
  state/sandbox findings, and the accepted/rejected production architecture.
- Updated prompts 216–218 and code-map planned entries whose claims are each backed by the trial or Apple primary docs.
- A reproducible validation script using `xcodebuild`, the host harness, and `auval`; manual Logic/GarageBand
  observations are recorded when those apps are installed but are not silently substituted for automated contract tests.

## Check

```sh
xcodebuild -project apps/musa-audio-unit-trial/MusaAudioUnitTrial.xcodeproj -scheme MusaAudioUnitTrial -configuration Debug CODE_SIGNING_ALLOWED=NO build
scripts/check-audio-unit-trial.sh
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
make docs-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Prove the Audio Unit integration shape`.

## Stop

- No production Rust FFI, Musa source compilation, shipping plug-in, notarization, distribution, or App Store work.
- No whole-piece stateful synthesizer seek, callback compilation/decoding, or undocumented host assumption.
- No third-party plug-in hosting or cross-platform wrapper framework.
