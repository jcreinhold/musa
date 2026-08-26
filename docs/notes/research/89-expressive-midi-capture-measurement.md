# Expressive MIDI capture measurement

**Status:** implementation record for prompt 202. This page governs nothing; the workflow is governed by
[`docs/rules/desktop/10-keyboard-composition.md`](../../rules/desktop/10-keyboard-composition.md).

## What was measured

On 2026-08-26, an optimized build at commit `43ca754a` plus prompt 202's working tree was measured on an Apple M4 Pro
with 24 GiB RAM, macOS 26.6, Rust's monotonic `Instant`, `midir` 0.11.0, and `cpal` 0.18.1. The reproducible command is:

```sh
cargo test --release -p musa-playback --test suite \
  latency_probe_reports_bounded_software_paths -- --nocapture
```

Two thousand observations produced these software-path results:

| Boundary | p50 | p95 | max |
| --- | ---: | ---: | ---: |
| MIDI message decode, fixed event construction, ring push, and test-side pop | 3 ns | 4 ns | 21 ns |
| Prepared audition event consumption and one 128-frame stereo render block | 6.417 µs | 6.791 µs | 22.583 µs |

The first row batches 64 messages per clock reading because a single operation is below this host clock's useful
resolution. It is a throughput-derived per-message cost, not a claim that a physical key reaches Musa in three
nanoseconds. The audio row times a complete callback invocation and includes the first rendered samples.

The ordinary test suite separately counts allocations around both exact paths. One thousand MIDI callback invocations
and an audio callback window containing audition, a prepared-plan replacement, plan retirement, and transport work
allocate zero times. The callback module's structural audit rejects logging; the paths contain no lock or I/O call.

The recent-memory bounds are 30 seconds and 4,096 fixed events. A `CapturedMidiEvent` is 96 bytes on this target, so the
published event bound is 393,216 bytes; the phrase-start index and deque bookkeeping remain below the one-mebibyte
project budget asserted by `the_published_recent_ring_stays_below_one_mebibyte`. At 30 seconds the event bound admits
136 messages per second before time does, leaving room above note-on/off traffic for pedals and pressure. Thirty seconds
also contains eight bars down to 64 beats per minute in common 4/4, or sixteen bars at 128. This is a disclosed product
default, not a claim about phrase structure: the ring retains observed all-keys-and-sustain-up boundaries, and Review
lets the musician move the chosen one. An uninterrupted performance exceeding either bound is marked as a truncated
suffix and cannot be passed off as a complete Keep-that phrase.

## What was not available

No physical MIDI input was connected to this host, and there was no acoustic or electrical loopback from an audio
output. Hardware key-scan time, USB/Bluetooth transport, CoreMIDI delivery, CPAL callback phase, device buffering, DAC,
and acoustic travel therefore have no p50, p95, or maximum in this measurement. Reporting the software figures as
“input-to-sound latency” would be false.

The application keeps those terms separate. The desktop control loop polls the already-filled MIDI ring every 2 ms; an
event can therefore wait 0–2 ms there. It then enters a preallocated audio ring and waits for the next device callback.
CPAL currently accepts the output device's default buffer size, so that wait and the device's own latency are device
facts, not constants Musa can derive from the 48 kHz sample rate. A hardware report must timestamp a physical stimulus
and looped-back output on the same measurement clock and publish its device identities and negotiated buffer. Until that
experiment exists, the interface and this record say the device contribution is unavailable.

## Why capture does not use arrival time directly

The locked `midir` API describes its callback timestamp as microseconds from an unspecified connection-stable epoch; the
CoreMIDI backend converts host time to microseconds. Musa therefore preserves that raw value and the independently
sampled callback arrival. A bounded online affine fit maps device time to the connection-local project clock, records
residual and discontinuity facts, and starts a new segment after a backward or implausibly large jump. Audition uses the
bounded queues; transcription receives the immutable raw observation and calibration instead of timing already rounded
to an audio block.

Pedal is equally non-destructive. Following Zhang et al.'s ATEPP distinction, key release and CC64 remain separate
facts. Sustain changes prepared audition state through the instrument's checked source binding; it never lengthens the
captured note or manufactures a written duration.
