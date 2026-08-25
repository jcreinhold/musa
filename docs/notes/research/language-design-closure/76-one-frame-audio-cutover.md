# One frame became the audio meaning

Prompt 173 completes the runtime cutover selected by the machine calculus. This note records the implementation
evidence; it does not govern the design.

## The boundary that shipped

Production audio now follows one path:

```text
ScoreSnapshot
  → EventTrack<PerformedTime, Gesture> per part
  → exact performed-time/physical-time map
  → checked Schedule<Gesture>
  → registered native primitive preparation
  → opaque PreparedAudio
  → repeated one-stereo-frame steps
```

`Gesture` retains written pitch, exact rational loudness and attack, notated times, event identity, and complete origin.
It chooses neither tuning nor frames. `IntegratedTempoMap` no longer stores a sample rate: legacy frame exporters must
name one at the call, while audio scheduling queries exact rational physical seconds. This follows the selected-calculus
§6 boundary and the event rules' separation of exact musical/performed time from the physical frame lattice.

The legacy `PerformancePlan` is not translated into this path. It remains only for the MIDI/debug consumers assigned to
prompt 174. Translating its floating frequency and already-rounded frame events would have created a second schedule and
invalidated note 67's one-interface theorem.

## Native primitive discipline

Every existing processor family maps exhaustively to one entry in a closed, versioned registry. An entry names its
configuration codec, retained state layout, and conservative base step work. Parameterized resource accounting adds the
actual fixed voice-pool, delay, chorus, reverb, limiter, output-buffer, and limiter-scan bounds at the selected sample
rate. `prepare_audio` checks, before processor-state allocation:

- stereo layout and nonzero sample rate;
- positive finite tuning;
- complete registry membership and supported graph output;
- primitive count, retained-state bytes, and conservative one-frame work;
- schedule/table limits and finite total extent; and
- an explicit stochastic seed.

The start and step dispatches are exhaustive over the same private processor enum. Registry uniqueness and construction
witness laws make adding a processor without pricing or dispatch a compile/test failure. This is a finite native
registry, not arbitrary closures or plug-in hosting; totality does not claim a real-time deadline.

## The semantic change and its oracle

The old graph allocated caller-sized buffers. Feedback read the previous caller block, modulation targets changed at
block boundaries, and filter interpolation covered the block. The audio-bridge fixture therefore had two WAV meanings:

| Host partition | Old digest | One-frame digest |
| --- | --- | --- |
| 64 frames | `ade054ca22b410f5` | `850e24083d0dd472` |
| 256 frames | `469bbde3e360bc5c` | `850e24083d0dd472` |

The private plan now allocates one-frame buffers by construction; there is no block-size option. Events are applied
before that frame's processor steps. Feedback, LFOs, modulation, smoothing, filters, delays, envelopes, effects, and the
master limiter each advance exactly once. Offline rendering and playback callbacks fill arbitrary host buffers only by
repeating this reference operation. Their allocation, logging, bounded-progress, retirement, loop, seek, and partition
laws all exercise the same opaque `PreparedAudio`.

`compile_graph`, the public `StudioGraphSpec`, and the public `RenderPlan` disappeared. The graph and flattening remain
crate-private implementation machinery, and their detailed laws are compiled as internal tests rather than exported
through a hidden production backdoor.

## Performance consequence

The semantic repair makes work proportional to audio frames rather than callback count. Prompt 173's benchmark records
the exact gesture/schedule/preparation chain outside the timed region and measures repeated one-frame rendering. On the
prompt machine, the 254,400-frame audio-bridge render measured a 41.02 ms median across 30 samples. The timed offline
wrapper made two output-vector allocations (65.53 KiB initially, growing to its 2.129 MiB final capacity); the callback
step law separately measured zero allocations. The schedule cursor's 4,000 steps measured 2.124 µs median. These are
throughput measurements, not permission to replace repeated steps with an unproved batch. No current primitive
advertises such a batch.
