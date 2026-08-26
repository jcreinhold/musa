---
id: 209
slug: logic-midi-processor
status: pending
depends_on: [204, 206, 208]
phase: 4
---

# Project a Musa Piece through Logic's MIDI Processor Surface

## Task

Implement the Audio Unit MIDI Processor admitted by prompt 206 so Logic Pro can schedule a checked Musa piece on its own
sample timeline and route the result to prompt 207's Music Device or another instrument. Keep GarageBand explicitly out
of this path unless the trial proved a supported host surface.

## Read

- Prompt 206's measured MIDI Processor contract and repaired version of this prompt; prompts 67, 72–75, 177–180,
  203–205, 207–208; the shared MIDI packet schedule introduced by prompt 204.
- Apple's current AU MIDI-output/render-event, musical-context, transport-state, offline-rendering, state-restoration,
  and Logic MIDI FX documentation. Treat absence from GarageBand's documented extension surface as unsupported, not as
  an invitation to use a private API.

## Design

Compile source, resolve packages/assets needed for performance semantics, realize the selected seed/profile, and build
prompt 204's finite immutable MIDI event schedule on a control worker. The AU render path receives host sample time,
musical context, transport state, cycle bounds, and maximum frame count, then performs a bounded random-access range
query and emits only events in that render interval at exact sample offsets. It never advances a mutable composition
cursor that must be replayed from zero after seek.

Define half-open block boundaries, same-sample ordering, note spans crossing seek/loop boundaries, preroll, stopped
rendering, discontinuities, cycle wrap, offline bounce, tempo/time-signature changes, negative host sample times, and
missing/inconsistent host context. On seek or loop, derive the bounded active-note/controller re-entry state from a
precomputed index; do not scan the whole piece or allocate in the callback.

The host timeline is the performance edge. Importing the same piece as performance MIDI and running the processor at
once would duplicate events, so state and documentation make the modes mutually exclusive. Host tempo can either drive
an explicitly selected host-relative projection or be ignored in favor of the piece's exact physical schedule; the
choice is part of saved state and never implicit. Polytempo that cannot fit the chosen host-relative mode is refused or
reported exactly as prompt 201 requires.

The processor uses the same portable source/lock identity, stable part/channel mapping, loss records, and document-state
rules as the Music Device. It produces MIDI only; it does not instantiate an instrument, render audio, mutate source, or
reach into Logic's project document.

## Target

- Production `aumi` component inside `apps/musa-audio-unit/`, sharing the audited control-side closure and prompt 204
  schedule without sharing mutable RT state with the Music Device.
- Indexed random-access event selection and active-state recovery with differential parity against parsed performance
  MIDI for linear, seeked, looped, and offline host timelines.
- Automated host harness/`auval` checks for component `aumi:msmp:MUSA`, RT instrumentation, corrupt/missing state,
  extension restart, and absent/malformed host context.
- Logic setup/use documentation and a clear GarageBand unsupported statement unless prompt 206 recorded contrary
  primary/documented evidence.

## Check

```sh
cargo nextest run -p musa-notation -p musa-project -p musa-au
cargo clippy --all-targets -p musa-notation -p musa-project -p musa-au -- -D warnings
xcodebuild -project apps/musa-audio-unit/MusaAudioUnit.xcodeproj -scheme MusaMIDIScheduler -configuration Debug CODE_SIGNING_ALLOWED=NO test
scripts/check-audio-unit.sh midi-processor
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
```

Commit as `Schedule Musa pieces through Logic MIDI FX`.

## Stop

- No GarageBand private API, AU instrument hosting inside the processor, audio render, proprietary Logic document, or
  bidirectional DAW round trip.
- No mutable forward-only sequencer in the callback and no whole-piece scan on seek or loop.
- No silent host-tempo/polytempo flattening or simultaneous imported-MIDI duplication.
