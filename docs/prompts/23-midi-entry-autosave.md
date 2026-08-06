---
id: 23
slug: midi-entry-autosave
status: pending
depends_on: [16, 21]
phase: 2
---

# MIDI Input, Step Entry, and Autosave

## Task

Connect a MIDI keyboard: live input through `midir` in the engine, step-entry note
input in the desktop app (play a key, get a note at the cursor with the selected
duration), and session autosave so no work is lost. These roadmap Phase 2 items
("MIDI step entry", "project undo and autosave") are sequenced here because they
depend on the desktop shell and edit commands, not because their scope changed.

## Read

- Roadmap §12.5 (live MIDI input belongs in the engine through `midir`), §14.5
  (note entry: pitch from MIDI keyboard, duration from numeric shortcuts), §15.6
  (engine owns MIDI input), §15.7 (autosave is project-owned).
- Prompt 13's engine, prompt 16's edit commands and entry UX.

## Design

- `musa-engine` (add `midir`): `EngineConfig` gains optional MIDI input selection
  (default: first available port, documented). Incoming note-on/note-off becomes
  `MidiInputEvent`s delivered to the control side via a dedicated `rtrb` queue — the
  callback never touches midir callbacks directly beyond queueing (§13.2 rules
  apply to the MIDI thread too: no allocation, no locks; midir gives you a callback
  thread, treat it like the audio one).
- Step entry flow: desktop receives input events (engine → project → Tauri event),
  the Compose workspace in entry mode maps note-on pitch → `InsertNote` at the cursor
  with the currently selected duration (prompt 16's command path — nothing new
  semantically), cursor advances. Held-chord entry: overlapping note-ons within a
  small window (or a chord modifier) issue one chord insertion. MIDI note number →
  `WrittenPitch` spelling happens at this edge (§12.5: MIDI is an edge format);
  spelling heuristic: prefer sharps in sharp keys, flats in flat keys, diatonic
  preference in the current key signature — document the rule and make it testable.
- The engine must run without any MIDI device (CI/laptops): absence is normal, not an
  error.
- Autosave (`musa-project`): after every successful `apply`, mark dirty; a debounced
  saver writes the source to disk (and a `.musa.bak` rotation or a recovery copy —
  pick the simplest correct policy: write-through after N seconds idle, plus explicit
  save on command). Undo history survives autosave; recovery on open if the file was
  dirty at crash (compare mtime/content; prompt the user via the snapshot).
- GUI: save-state indicator, MIDI device status in the transport bar, recovery
  prompt.

## Target

- `musa-engine`: MIDI input queue and event type.
- `musa-project`: autosave/recovery policy; MIDI event surfacing.
- `apps/musa-desktop`: step entry with MIDI, chord entry, indicators.
- Tests: MIDI queue round-trips with synthetic events; spelling heuristic table tests;
  autosave/recovery round-trips on temp dirs; no-device robustness.

## Check

```sh
cargo nextest run -p musa-engine -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
# manual with a MIDI keyboard:
cd apps/musa-desktop && cargo tauri dev   # step-enter a melody, kill the app, verify recovery
```

Commit as `Add MIDI step entry and autosave`.

## Stop

- No MIDI recording of performances (rejected, §4) — input is entry only.
- No MIDI output to external devices, no MIDI clock sync.
- No velocity/aftertouch interpretation in entry (fixed default; profiles remain the
  dynamics mechanism).
