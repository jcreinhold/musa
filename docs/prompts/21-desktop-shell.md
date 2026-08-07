---
id: 21
slug: desktop-shell
status: pending
depends_on: [20]
phase: 1.5
---

# Desktop Shell: Tauri and the command boundary

## Task

Wrap prompt 20's UI in a Tauri v2 shell and replace its fixtures with a live `ProjectSession`. After this prompt a user
opens a `.musa` file, sees it engraved, edits the source as plain text, sees diagnostics, and hears it play. The bridge
is the only new concept: a narrow, typed command surface with no semantics in it.

## Read

- `docs/interface/05-states.md` (empty, loading, **stale revision** — the headline behavior), `06-performance.md`
  (B1, B6, B7, B10 land here).
- Roadmap §14.2 (state ownership), §14.7 (invalid edits), §15.9 (the shell is thin: command adaptation, window
  lifecycle, file dialogs, event delivery — nothing else), §14.8 (bundled, zero setup).
- Prompt 19's `ProjectSession`, `ProjectSnapshot`, `ProjectCommand`, `ExportRequest`; prompt 18's transport.

## Design

- Add `apps/musa-desktop/src-tauri`, depending on `musa-project` by workspace path. `musa-project` gains no Tauri
  dependency and no `serde` attribute that exists only for the webview.
- **Command surface** — the complete bridge, and it stays this small:

  ```rust
  #[tauri::command] fn open_project(path: String) -> Result<SnapshotDto, ErrorDto>;
  #[tauri::command] fn new_project(template: TemplateDto) -> Result<SnapshotDto, ErrorDto>;
  #[tauri::command] fn apply(command: CommandDto) -> Result<SnapshotDto, ErrorDto>;
  #[tauri::command] fn transport(cmd: TransportDto) -> Result<(), ErrorDto>;
  #[tauri::command] fn export(request: ExportDto) -> Result<PathBuf, ErrorDto>;
  ```

  DTOs are serde mirrors in `src-tauri`, never re-exported internal types; the surface grows by adding fields. TypeScript
  types are **generated** from the DTOs (`ts-rs` or equivalent) so the two sides cannot drift — a hand-maintained
  `.d.ts` is a defect waiting to happen.
- **Events, not polling** (`06-performance.md` §3): `musa://snapshot` after every successful `apply`, `musa://position`
  from the engine at ~10 Hz **only while playing**, `musa://transport` on state change. The frontend has no timers at
  rest; B10 is asserted.
- **Frontend**: prompt 20's fixture loader is replaced by a single `session` store that owns the snapshot and is the
  only thing that talks to `invoke`. Components read the store; nothing else imports Tauri APIs.
- **Source editing**: the drawer gets a plain `<textarea>` styled per the token system, debounced at 180 ms → one
  `apply(SetSource)`, superseding any compile in flight (`06-performance.md` §3, rule 4). CodeMirror is prompt 26.
- **Stale revision** per `05-states.md` §4 in full: the score does not change, the leaf edge goes `--chalk`, the top
  margin states the revision and problem count, playback continues from the last valid plan, and the drawer opens itself
  the first time in a session. Test this directly — it is the behavior that decides whether the app is pleasant to edit
  in.
- **Empty and launch states** per `05-states.md` §2–§3, including the new-piece template that is immediately audible
  (roadmap §14.8) and the shell painting before the engraver is ready (B6).
- **Window and menu**: native menu with File (New, Open, Save, Export…), Edit (Undo, Redo), View (Zoom, Theme), Help
  (Keyboard sheet). Every menu item also exists in the command palette registry, which is the single source for bindings
  (`03-interaction.md` §6) — the palette itself is prompt 23, the registry starts here.

## Target

- `apps/musa-desktop/src-tauri/` with the five commands, the three events, generated TS types, native menu, file
  dialogs.
- `apps/musa-desktop/ui`: `session` store, live snapshot rendering, debounced source editing, stale-revision and empty
  states, theme following the OS with a manual override.
- Tests: Rust tests for DTO round-trips and that `open → apply(invalid) → snapshot` retains the last-valid artifacts;
  Playwright smoke against `tauri dev` (open `examples/glass-mountain.musa`, score appears, edit to invalid, score
  unchanged and edge is chalk, revert, play issues transport); perf assertions for B1, B6, B7, B10.

## Check

```sh
cargo nextest run -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm ci && npm run check && npm run test && npm run build
cd apps/musa-desktop && cargo tauri dev   # manual: open glass-mountain.musa, break the source, press play
```

Commit as `Add Tauri desktop shell over ProjectSession`.

## Stop

- No structured score editing (prompt 25), no note entry.
- No CodeMirror, no syntax highlighting (prompt 26).
- No Sound or Mix workspace (roadmap §14.4 — Compose and the source drawer only).
- No packaging, installers, icons, or auto-update — `cargo tauri dev` is the bar.
- No MIDI input (prompt 33), no autosave (prompt 33).
