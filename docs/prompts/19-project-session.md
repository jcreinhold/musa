---
id: 19
slug: project-session
status: done
depends_on: [18]
phase: 1.5
---

# Project Session

## Task

Implement `musa-project`, the application's deepest module: one `ProjectSession` facade that owns the source document,
revisions, compile/render/playback orchestration, commands, undo/redo, and exports. After this prompt, neither the CLI
nor the desktop app ever chains compiler → render → engine calls themselves.

## Read

- Roadmap §3 (this is the canonical deep module), §11 (the project source is canonical; one persistent truth),
  §14.6–§14.7 (command flow, invalid-edit behavior), §15.7 (ownership list and facade), §16 (one-file projects now,
  directory projects deferred).
- The orchestration shim left in `musa` by prompt 17/18 (to be deleted and moved here).

## Design

- Create `musa-project` with dependencies per §15.7: all five semantic crates, `serde`, `toml`, `tracing`.
- Public surface (§15.7, fixed):

  ```rust
  pub struct ProjectSession { /* hidden: document, revision history,
      last-good compilation, prepared plan */ }

  impl ProjectSession {
      pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectError>;
      pub fn create(path: impl AsRef<Path>, template: Template) -> Result<Self, ProjectError>;
      pub fn snapshot(&self) -> ProjectSnapshot;
      pub fn apply(&mut self, command: ProjectCommand)
          -> Result<ProjectUpdate, ProjectError>;
      pub fn export(&self, request: ExportRequest)
          -> Result<ExportArtifact, ProjectError>;
      pub fn undo(&mut self) -> Result<ProjectUpdate, ProjectError>;
      pub fn redo(&mut self) -> Result<ProjectUpdate, ProjectError>;
  }
  ```

- `ProjectCommand` at this prompt: `SetSource(String)` (full-document replace — the editor's primitive now),
  `ApplyEdits(Vec<TextEdit>)` (uses prompt 04's `apply_edits`). Structured score commands (`InsertNote` etc.) are prompt
  16 and must be expressible without changing this enum's shape.
- Revisions: every successful `apply` bumps a revision; undo/redo walk the revision history (command-inverse or
  snapshot-based — choose snapshot-based: simplest correct thing for a text-canonical system; document the choice).
- Last-valid-revision behavior (§14.7): the session keeps the last **successful** compilation, rendered MEI, and
  prepared playback plan. When the source is temporarily invalid, `snapshot()` reports current diagnostics **plus** the
  last-valid artifacts and a flag saying so. Playback keeps working from the last-valid plan. This is a headline
  behavior — test it directly.
- `ProjectSnapshot` is the frontend's whole view: source text, diagnostics, current score MEI (rendered through
  `musa-render`), playback state, revision. The frontend owns nothing semantic (§14.2).
- Move the prompt-12/13 orchestration here: `export(ExportRequest::{Wav, Mei, LilyPond, PerformanceDump})` and an
  internal `prepare_playback()` that rebuilds and reinstalls the engine plan after each valid compile (debouncing/engine
  ownership policy: session owns an optional `AudioEngine`; `play`/`stop`/`seek` commands pass through to it).
- Rewrite `musa` on top of `ProjectSession` — every subcommand becomes a few lines (§15.8). This is the proof the facade
  is deep: the CLI shrinks.
- Debounced compilation (§10.7) is a caller concern (the GUI debounces keystrokes); the session compiles synchronously
  per `apply`. Note the decision; worker-thread compilation arrives only if profiling demands it.

## Target

- `musa-project`: `ProjectSession` and the types above; revision/undo machinery; last-valid artifact retention; export +
  playback orchestration.
- `musa`: rewritten on `ProjectSession` with no behavior change.
- Tests: open/create/apply/undo/redo round-trips; invalid source keeps last-valid snapshot and still plays; export
  equivalence with prompt-12 golden WAV (same bytes); concurrent-free single-threaded session semantics.

## Check

```sh
cargo nextest run -p musa-project -p musa
cargo clippy --all-targets -p musa-project -p musa -- -D warnings
cargo fmt --check
# CLI behavior unchanged end-to-end:
cargo run -p musa -- check examples/glass-mountain.musa
cargo run -p musa -- render examples/glass-mountain.musa --to wav -o /tmp/gm3.wav
cmp /tmp/gm3.wav /tmp/gm.wav   # same bytes as prompt 17's golden output
```

Commit as `Add project session facade and rebuild CLI on it`.

## Stop

- No structured score-edit commands (prompt 25) — only whole-source and text-edit commands.
- No autosave (prompt 33), no directory/album projects (§16 later part).
- No compilation on a worker thread or Salsa (§10.7).
- No Tauri/desktop code (prompt 21).
