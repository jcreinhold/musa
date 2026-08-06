---
id: 13
slug: engine-transport
status: pending
depends_on: [12]
phase: 1
---

# Audio Engine and Transport

## Task

Implement `musa-engine`: CPAL device negotiation and output stream lifecycle, the real-time command boundary (`rtrb`
queues, plan installation, retired-plan return), and transport (play, stop, seek, loop). `musa play piece.musa` plays
the piece live. This completes the Phase 1 CLI slice.

## Read

- Roadmap §13.1–§13.2 (real-time separation: the callback never allocates, locks, parses, logs, or destroys), §15.6
  (crate ownership and the `AudioEngine` facade), §14.8 (default output selection algorithm; zero setup).
- Prompt 11's RT contract, prompt 12's orchestration shim.

## Design

- Create `musa-engine` with dependencies: `musa-compiler`, `musa-audio`, `cpal`, `rtrb`, `tracing`, `thiserror`.
  (`midir` arrives at prompt 23.)
- Public surface exactly per §15.6:

  ```rust
  pub struct AudioEngine { /* hidden: stream, queues, callback state */ }

  impl AudioEngine {
      pub fn open(config: EngineConfig) -> Result<Self, EngineError>;
      pub fn install(&self, plan: PreparedPlaybackPlan) -> Result<(), EngineError>;
      pub fn command(&self, command: TransportCommand) -> Result<(), EngineError>;
  }

  pub enum TransportCommand { Play, Stop, Seek { frame: u64 },
      SetLoop { start: u64, end: u64 }, ClearLoop }
  ```

  The rest of the application never sees a CPAL stream, stream config, or sample
  format (§15.6). `EngineConfig` picks the default output device with a documented
  fallback order (§14.8).
- Real-time boundary (§13.2):
  - Control → audio: one `rtrb` SPSC queue carrying `TransportCommand` and `Install(PreparedPlaybackPlan)`.
  - Audio → control: a second `rtrb` queue returning **retired plans** so large structures are dropped on the control
    thread, never in the callback.
  - The callback consumes commands at block boundaries, executes the installed `RenderPlan::render` (prompt 11's
    allocation-free contract), and handles underruns by emitting silence, never by panicking or blocking.
- `PreparedPlaybackPlan` = compiled `RenderPlan` + scheduled `PerformancePlan` events
  + loop state; prepared entirely on the control side. Sample-rate mismatch between
  the plan (48 kHz default) and the negotiated device rate: resample is out of scope —
  instead negotiate the stream at the plan's rate and error clearly if unsupported
  (record the decision; a resampler is a later measured feature).
- `musa play <file>`: reuse the prompt-12 orchestration shim to build the plan, open the engine, install, play to
  completion (or `--loop`), exit on Ctrl-C. Keep it thin; prompt 14 moves orchestration into `musa-project`.
- Instrumentation (§17.5): a test mode running the callback logic against a fake output, asserting no
  allocation/lock/destruction in the callback path and measuring per-block processing time budget. This test is the RT
  contract's enforcement.

## Target

- `musa-engine`: `AudioEngine`, `EngineConfig`, `PreparedPlaybackPlan`, `TransportCommand`, RT queues, callback,
  instrumentation tests.
- `musa-cli`: `play` subcommand.
- Tests: command queue round-trips; seek/loop state machine; callback instrumentation; graceful behavior when no audio
  device exists (clear `EngineError`, not a panic — CI may have no device).

## Check

```sh
cargo nextest run -p musa-engine -p musa-cli
cargo clippy --all-targets -p musa-engine -p musa-cli -- -D warnings
cargo fmt --check
# manual, on a machine with audio:
cargo run -p musa-cli -- play examples/glass-mountain.musa
```

Commit as `Add audio engine with real-time-safe transport`.

## Stop

- No MIDI input (prompt 23), no recording (rejected, §4).
- No resampling, no device hot-plug, no per-part mixer UI.
- No GUI; the desktop app drives this same facade in prompt 15.
