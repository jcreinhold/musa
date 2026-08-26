---
id: 33
slug: midi-entry-autosave
status: done
depends_on: [25, 31]
phase: 2
---

# MIDI Input, Step Entry, and Autosave

> **Historical implementation evidence.** Prompt 201 replaces step entry as the intended workflow with always-audible
> keyboard performance, finite Capture/Keep-that evidence, Review, and source acceptance. This implementation remains
> only as a migration bridge until prompt 209 proves the replacement and deletes it; autosave remains current.

## Task

Connect a MIDI keyboard: live input through `midir` in the engine, step-entry note input in the desktop app (play a key,
get a note at the cursor with the selected duration), and session autosave so no work is lost. These roadmap Phase 2
items ("MIDI step entry", "project undo and autosave") are sequenced here because they depend on the desktop shell and
edit commands, not because their scope changed.

## Read

- Roadmap §12.5 (live MIDI input belongs in the engine through `midir`), §14.5 (note entry: pitch from MIDI keyboard,
  duration from numeric shortcuts), §15.6 (engine owns MIDI input), §15.7 (autosave is project-owned).
- Prompt 18's engine, prompt 25's edit commands and entry UX.

## Design

- `musa-playback` (add `midir`): `EngineConfig` gains optional MIDI input selection (default: first available port,
  documented). Incoming note-on/note-off becomes `MidiInputEvent`s delivered to the control side via a dedicated `rtrb`
  queue — the callback never touches midir callbacks directly beyond queueing (§13.2 rules apply to the MIDI thread too:
  no allocation, no locks; midir gives you a callback thread, treat it like the audio one).
- Step entry flow: desktop receives input events (engine → project → Tauri event), the Compose workspace in entry mode
  maps note-on pitch → `InsertNote` at the cursor with the currently selected duration (prompt 25's command path —
  nothing new semantically), cursor advances. Held-chord entry: overlapping note-ons within a small window (or a chord
  modifier) issue one chord insertion. MIDI note number → `WrittenPitch` spelling happens at this edge (§12.5: MIDI is
  an edge format); spelling heuristic: prefer sharps in sharp keys, flats in flat keys, diatonic preference in the
  current key signature — document the rule and make it testable.
- The engine must run without any MIDI device (CI/laptops): absence is normal, not an error.
- Autosave (`musa-project`): after every successful `apply`, mark dirty; a debounced saver writes the source to disk
  (and a `.musa.bak` rotation or a recovery copy — pick the simplest correct policy: write-through after N seconds idle,
  plus explicit save on command). Undo history survives autosave; recovery on open if the file was dirty at crash
  (compare mtime/content; prompt the user via the snapshot).
- GUI: save-state indicator, MIDI device status in the transport bar, recovery prompt.

## Target

- `musa-playback`: MIDI input queue and event type.
- `musa-project`: autosave/recovery policy; MIDI event surfacing.
- `apps/musa-desktop`: step entry with MIDI, chord entry, indicators.
- Tests: MIDI queue round-trips with synthetic events; spelling heuristic table tests; autosave/recovery round-trips on
  temp dirs; no-device robustness.

## Check

```sh
cargo nextest run -p musa-playback -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
# manual with a MIDI keyboard:
cd apps/musa-desktop && cargo tauri dev   # step-enter a melody, kill the app, verify recovery
```

Commit as `Add MIDI step entry and autosave`.

## Repairs made while implementing

- **MIDI input is its own facade, not a field of `EngineConfig`.** The Design sketched the keyboard as engine
  configuration, but a composer entering notes has not necessarily pressed play, and an entry path that depended on an
  open audio stream would make "write a melody" require a working output device. `MidiInput::open` is therefore a small
  independent type in `musa-playback`, and `ProjectSession::listen_to_midi` opens it on demand.
- **The keyboard is read while note entry is on, and at no other time.** There is no MIDI thread polling in the
  background: the desktop session thread blocks until something happens, and only entry mode gives it a 15 ms tick. This
  keeps roadmap §14.8's "nothing is scheduled at rest" true of the keyboard as well as of the transport.
- **Spelling and chord grouping live in `musa-project`, not in the frontend.** Both are musical judgements — which
  letter a black key is, and whether two presses are one chord — and `docs/rules/desktop/03-interaction.md` §7 forbids
  the frontend from computing musical facts. `MidiEntry` crosses the bridge already spelled and already grouped; the
  interface's only contribution is the notated duration, which the keyboard cannot know.
- **The spelling rule is stated and table-tested.** A note the key already spells is written the key's way; a note
  outside the key is an alteration of its neighbour, raised in a sharp signature and lowered in a flat one. A second law
  asserts that every spelling sounds back at the note it came from, including where the letter and the number disagree
  about the octave (`bs3` is 60).
- **Velocity does not cross the queue.** The Stop list rejects velocity interpretation, so carrying it would be a public
  field with no caller; `decode` uses it to tell a zero-velocity note-on from a press and then drops it.
- **Autosave is a recovery copy beside the file, not a write-through to the file itself.** The Design offered a choice;
  this is the simplest correct one. Writing through would mean a crash could leave the *file* half-rewritten, and would
  make "save" meaningless as an act. The copy is written atomically (temp + rename) after every command that changes the
  source, and removed on save.
- **No debounce inside the session.** The Design asked for "write-through after N seconds idle", but the editor already
  debounces keystrokes before they reach the session (roadmap §10.7), so a command is already a settled edit. A second
  timer would mean the session owning a clock and a thread to answer a question the caller has answered.
- **Undo history is not saved.** It is a session's working memory, and a recovered file that claimed a history it could
  no longer reach would misreport what undo does.
- **Recovery is offered, never applied.** `open` reports a differing copy as `snapshot.recovery`; `RestoreRecovery`
  makes it the source as an ordinary undoable edit, and `DiscardRecovery` forgets it. A copy identical to the file is
  cleared rather than offered, because that is a save that landed and a copy that outlived it.
- **`KeyMap::fifths` moved into `musa-compiler`.** MIDI spelling and the notation plan both need the circle-of-fifths
  position of a key; two copies of that table would be two chances to disagree.

## Stop

- No MIDI recording of performances (rejected, §4) — input is entry only.
- No MIDI output to external devices, no MIDI clock sync.
- No velocity/aftertouch interpretation in entry (fixed default; profiles remain the dynamics mechanism).
