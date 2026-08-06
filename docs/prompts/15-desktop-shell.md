---
id: 15
slug: desktop-shell
status: pending
depends_on: [14]
phase: 1.5
---

# Desktop Shell: Tauri + Svelte + Verovio

## Task

Create `apps/musa-desktop`: a thin Tauri shell with a Svelte/TypeScript frontend that
renders the score as interactive SVG via Verovio WebAssembly, wired to
`ProjectSession` through Tauri commands. After this prompt a user can open a piece,
see the score, press play, and see diagnostics — with the score updating on source
edits.

## Read

- Roadmap §14.1 (technology choices and why), §14.2 (state ownership: Rust owns
  semantics; frontend owns only ephemeral UI state), §14.3 (default workspace
  layout), §14.4 (workspaces — build **Compose** only), §14.7 (invalid-edit
  behavior), §14.8 (minimal setup: Verovio bundled), §15.9 (the shell is thin).
- Prompt 14's `ProjectSnapshot`, prompt 08's `xml:id` contract.

## Design

- Scaffold `apps/musa-desktop/{src-tauri,ui}` with Tauri v2 + Svelte + Vite +
  TypeScript. `src-tauri` depends on `musa-project` via workspace path.
- Tauri command surface (the **only** bridge; §15.9 — command adaptation, file
  dialogs, event delivery, nothing semantic):

  ```rust
  #[tauri::command] fn open_project(path: String) -> Result<SnapshotDto, ErrorDto>;
  #[tauri::command] fn apply_source(source: String) -> Result<SnapshotDto, ErrorDto>;
  #[tauri::command] fn transport(cmd: TransportDto) -> Result<(), ErrorDto>;
  #[tauri::command] fn export(request: ExportDto) -> Result<PathBuf, ErrorDto>;
  ```

  DTOs are serde mirrors of `musa-project` types; add fields by extending, never by
  passing raw internal types.
- Frontend:
  - Compose workspace layout per §14.3: transport bar (play/stop/loop, position,
    tempo), parts list (from snapshot), score pane (Verovio WASM rendering the
    snapshot's MEI to SVG), diagnostics/source drawer at the bottom, inspector column
    on the right (empty placeholder sections for now).
  - Source editing via a plain textarea or CodeMirror 6 (§14.1 recommends CM6 — use
    it, with musa syntax highlighting deferred; plain text mode is fine now). Edits
    are **debounced** (§10.7: ~200 ms) then sent as `apply_source`. This drawer is the
    seed of §14.4's **Source** workspace (source + diagnostics + score preview); build
    it so it can grow into a full workspace rather than a throwaway panel.
  - Invalid source: show diagnostics immediately, keep the last-valid SVG visible,
    badge the transport as "last valid revision N" (§14.7 — the snapshot carries the
    flag; the frontend only displays it).
  - Verovio loaded as a bundled WASM module (§14.8: zero setup). Pin the version;
    render options minimal (adjust page width to pane).
  - Score interaction for this prompt: click a note → resolve `xml:id` → `EventId`
    (prompt 08's contract) → highlight it and show basic event info (pitch, duration,
    part/voice, origin expansion path) in the inspector. No editing yet (prompt 16).
- Playback: transport commands go to the session's engine; position display polls or
  receives an event at ~10 Hz. Playback cursor/highlight on the SVG is **not**
  required here (nice-to-have; the event id plumbing exists for it).
- The Rust core must stay unaware of the webview: no Tauri types in `musa-project`.

## Target

- `apps/musa-desktop/` scaffold; Tauri commands above; Compose workspace UI; bundled
  Verovio; debounced source editing with §14.7 behavior; click-to-inspect.
- UI tests: at minimum, type-checked build + one Playwright/webdriver smoke test
  (open example, SVG appears, play button issues command) if practical to run headless;
  otherwise a documented manual checklist in `apps/musa-desktop/README.md`.
- Package script so `cargo tauri dev` runs the app.

## Check

```sh
cargo nextest run -p musa-project   # untouched crates stay green
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm ci && npm run build
cd apps/musa-desktop && cargo tauri dev   # manual: open glass-mountain.musa, see score, press play
```

Commit as `Add Tauri desktop shell with Verovio score view`.

## Stop

- No score editing commands (prompt 16), no note entry.
- No Sound/Mix/Source workspaces (§14.4 — Compose only; the source drawer is a panel,
  not the Source workspace).
- No MIDI input, no playback cursor animation requirement.
- No menu/system-tray/packaging polish (icons, installers) — `tauri dev` is the bar.
