---
id: 204
slug: coremidi-performance-output
status: pending
depends_on: [201, 203]
phase: 4
---

# Project Performance through CoreMIDI

## Task

Send Musa's checked score or performance MIDI projection to Logic Pro, GarageBand, and other macOS clients through
timestamped CoreMIDI virtual sources. Reuse the Standard MIDI exporter’s musical decisions rather than implementing a
second performance interpretation in `musa-playback`.

## Read

- Prompts 28, 33, 67, 72–75, 177–180, 201, and 203; roadmap §12.5; the current `GesturePlan`, MIDI writer, MIDI-input
  callback, transport, and project/CLI facades.
- Apple's CoreMIDI documentation and Logic's current virtual MIDI device/input documentation. Use Apple terminology:
  Musa publishes virtual **sources** from which a DAW receives; a physical/DAW endpoint selected for sending is a
  destination.

## Design

Extract one immutable, host-neutral MIDI packet schedule from the existing score/performance MIDI decision path. SMF
writing and live output consume that schedule; neither may reinterpret dynamics, articulation, tempo, controller curves,
note ordering, channel allocation, or tuning independently. The schedule retains stable part/event origin and exact
physical time before a live adapter turns it into CoreMIDI host timestamps.

On macOS, publish one named virtual source per sounding part when the host supports it, plus an explicit single-source
channelized mode for simpler GarageBand projects. Names and channel assignments derive deterministically from stable
part identities and are returned in a connection report. More than sixteen channels, microtonal pitch, MPE/per-note
expression, controller collisions, and unsupported SysEx are explicit refusal or loss outcomes; do not steal channels or
quantize silently.

All allocation, endpoint creation, scheduling-window refill, and plan replacement happen on the control side. The MIDI
send path uses bounded preallocated packets and monotonic timestamps, never parses source or locks in an OS callback.
Late packets follow one documented policy and increment observable counters. Stop/panic sends the bounded note-off/reset
sequence required by active state.

Expose project/CLI controls for listing endpoints, starting score/performance output, selecting virtual-source versus
destination mode, and stopping. Keep MIDI input and output types private behind the playback facade. This prompt uses
Musa's own transport only; external clock authority belongs to prompt 205.

## Target

- A shared MIDI packet schedule consumed differentially by SMF and live output, including origins and loss records.
- macOS CoreMIDI output behind a platform module, with a deterministic fake-clock/fake-endpoint backend for all CI laws.
- CLI/project lifecycle, endpoint reports, part mappings, late/drop counters, panic behavior, and actionable errors.
- Differential fixtures proving live packets equal the parsed performance/score MIDI event stream modulo container and
  timestamp representation; RT allocation/lock instrumentation and bounded-queue saturation tests.

## Check

```sh
cargo nextest run -p musa-notation -p musa-playback -p musa-project -p musa
cargo clippy --all-targets -p musa-notation -p musa-playback -p musa-project -p musa -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
```

Commit as `Stream Musa performance through CoreMIDI`.

## Stop

- No MIDI clock, MTC, Ableton Link, network MIDI configuration, Audio Unit, MPE, or MIDI 2.0.
- No MIDI output in the event-track ontology and no public CoreMIDI or `midir` type.
- No promise of sample-accurate audio/MIDI alignment across processes; prompt 205 measures the edge and Audio Units own
  the in-host sample-time path.
