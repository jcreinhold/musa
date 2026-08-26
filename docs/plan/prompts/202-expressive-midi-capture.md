---
id: 202
slug: expressive-midi-capture
status: pending
depends_on: [201]
phase: 2
---

# Hear Immediately and Preserve the Performance Evidence

## Task

Replace the note-on-only entry queue with a complete, bounded expressive MIDI capture and audition boundary. A connected
keyboard should sound the selected part without arming an edit, while explicit Capture and the recent-phrase memory
preserve enough evidence for transcription without touching canonical source.

## Read

- Prompt 201's governing workflow; prompts 18, 33, 67, 173, 177–180, 184, 188, and 191; current `MidiInput`, project
  session loop, transport, selected part/voice facts, prepared instrument instances, and desktop bridge.
- Platform `midir`/CoreMIDI timestamp and hot-plug behavior from the versions actually locked in the workspace. Measure
  the callback clock rather than assuming its epoch or monotonicity.
- The pedal distinction in Zhang et al., [ATEPP](https://archives.ismir.net/ismir2022/paper/000053.pdf): key release and
  pedal-extended sound are distinct observations. Preserve both; do not turn CC64 into a longer written note.

## Design

Capture a fixed-layout `Copy` event on the MIDI callback: host/device timestamp, cable/channel, message kind, key or
controller, and value. Admit note on/off with attack/release velocity, CC64 sustain, CC66 sostenuto, CC67 soft pedal,
pitch bend, channel pressure, and polyphonic key pressure. Preserve unsupported channel messages as explicit bounded
facts or counted losses; SysEx and unbounded payloads are refused on this path. Pair repeated note-ons and missing
note-offs deterministically on the control side, never in the callback.

Maintain two clocks. Raw callback timestamps are immutable evidence. A measured affine/offset calibration maps them to
the project's monotonic performance clock for audition and transcription, detects jumps/wraps, and records calibration
quality. Filtering may remove transport-clock error; it may not smooth away expressive timing. A take begins at an exact
project revision, selected part/voice, transport state, tempo/meter context, device identity, and calibration record.

Audition routes MIDI through the selected part's already prepared instrument instance using bounded RT queues. Note,
velocity, pressure, bend, and pedal reach only controls the source-declared signature maps; unsupported controls are
reported, never guessed. Audition does not compile, edit, allocate, lock, perform I/O, or wait for engraving. Device
latency and input-to-sound p50/p95/max are measured separately from transcription.

The recent phrase memory is a preallocated ring bounded by both time and event count, defaulting to a measured useful
window rather than an arbitrary large history. It is memory-only, visible, clearable, and disabled by preference.
`Keep that` freezes a consistent suffix beginning at a detected silence or user-adjustable boundary. Explicit Capture
creates a finite take with optional count-in/metronome context. Frozen takes move to control-side immutable storage and
survive review, not application restart.

Device selection is explicit when more than one input exists, remembers a stable platform identity when available,
handles hot-plug/reconnect, and never silently switches an armed capture to a different keyboard. Disconnect ends or
pauses capture with a visible reason while preserving the take and source.

## Target

- Deep `musa-playback` input/audition facade and project-owned immutable MIDI-take facts; no public `midir` or CoreMIDI
  type and no serializable alternative project model.
- Desktop device picker/status, always-listen indication, Capture/stop, recent-memory indication, Keep that, clear, and
  disconnect/reconnect states. Source remains byte-identical throughout.
- Fake-device/fake-clock laws for message decoding, timestamp calibration, ordering, repeated notes, dropped note-offs,
  pedal separation, queue overflow, clock discontinuity, hot-plug, and take bounds.
- Native audition differential against the same prepared instrument event history, plus callback allocation/lock/I/O/log
  instrumentation and measured latency report.

## Check

```sh
cargo nextest run -p musa-playback -p musa-project -p musa-desktop
cargo clippy --all-targets -p musa-playback -p musa-project -p musa-desktop -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
```

Commit as `Capture expressive MIDI without editing source`.

## Stop

- No transcription, quantization, beat inference, candidate score, source edit, MIDI file import, or DAW
  synchronization.
- No audio recording, pitch/onset detection, FFT, waveform, disk-backed take, telemetry, or cloud processing.
- No callback allocation, lock, I/O, log, compiler call, decoder call, or large destruction.
