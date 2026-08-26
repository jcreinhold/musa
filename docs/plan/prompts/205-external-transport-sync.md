---
id: 205
slug: external-transport-sync
status: pending
depends_on: [204]
phase: 4
---

# Give Live MIDI One Clock Authority

## Task

Synchronize Musa and Logic Pro over CoreMIDI without creating two competing transports. Implement explicit leader and
follower modes for the MIDI clock/MTC protocols Logic documents, measure their timing, and state every representational
limit.

## Read

- Prompt 204; prompts 15, 18, 43, 67, 72–75, 172–173; the exact tempo/time maps, frame policy, transport command queue,
  seek/loop state, and live MIDI callback.
- Apple's current Logic synchronization guide cited by prompt 201 and CoreMIDI packet/timestamp documentation. Verify
  which of MIDI clock, Song Position Pointer, Machine Control, and MTC Logic sends/receives before implementing a mode.

## Design

Each session chooses exactly one authority:

- `musa-leads`: Musa owns play/stop/continue/seek and sends the documented clock/position messages with prompt 204's
  performance packets; or
- `external-leads`: Musa follows one selected external source and translates its start/stop/continue/position/clock
  state into ordinary transport commands.

Reject a configuration that sends and follows the same protocol, has two selected authorities, or changes authority
while running. Clock reception uses a bounded allocation-free parser/state machine in the MIDI callback and hands
observations through an RT-safe queue; smoothing, dropout detection, tempo estimation, and transport mutation live on
the control side. Preserve raw timestamps and expose lock/acquiring/lost state and measured drift/jitter.

MIDI clock's 24 pulses per quarter and MTC's physical-time frames are not Musa's exact tempo maps. Define the explicit
conversion and resynchronization policy. A single DAW tempo lane cannot denote independent simultaneous polytempo; the
operation either follows one named reference scope and reports the others as unsynchronized, or refuses when no scope is
selected. Polymeter does not itself require flattening because the shared beat clock and each source scope remain
distinct.

Loop/seek discontinuities flush scheduled MIDI and active notes before restarting from the selected boundary. Never
infer source edits, rewrite tempo declarations, or let jitter enter semantic identity. For sample-accurate in-host
rendering, direct users to prompts 207–209's Audio Unit path.

## Target

- Versioned sync options/status in the project and CLI facades, with leader/follower lifecycle and exactly-one-authority
  validation.
- MIDI clock and the minimum documented Logic transport/position protocols, using a deterministic virtual clock in
  tests.
- Parser fragmentation/running-status/realtime-interleaving tests; drift, dropout, relock, seek, loop, panic, and queue
  saturation laws; measured loopback jitter recorded by host and OS version.
- Logic setup documentation that labels measured behavior and does not claim GarageBand synchronization unless Apple's
  current guide and a reproducible probe support it.

## Check

```sh
cargo nextest run -p musa-playback -p musa-project -p musa
cargo clippy --all-targets -p musa-playback -p musa-project -p musa -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
```

Commit as `Synchronize one external MIDI transport`.

## Stop

- No Ableton Link, OSC, network session discovery, MIDI 2.0, or undocumented GarageBand control.
- No clock smoothing or tempo estimate in the RT callback and no observed jitter in source identity.
- No silent polytempo collapse, two-way authority, or claim of sample accuracy across CoreMIDI clients.
