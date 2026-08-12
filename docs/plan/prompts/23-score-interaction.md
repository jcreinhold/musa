---
id: 23
slug: score-interaction
status: done
depends_on: [22]
phase: 1.5
---

# Score Interaction: selection, keyboard, playhead, accessibility

## Task

Make the score navigable and playable without a mouse: the full selection model including a first-class caret, the
keyboard map, the command palette, transport with a smooth playhead and follow modes, loop from selection, and the
accessibility floor. After this prompt the app is a usable score reader and player; prompt 25 adds writing.

## Read

- `docs/rules/desktop/03-interaction.md` in full — this prompt's specification.
- `05-states.md` §6 (results and confirmations), `06-performance.md` B3, B4, B5, B9, B10.
- Prompt 18's transport facade and position events; prompt 21's event channel and command registry.

## Design

- **Selection model** (`03-interaction.md` §1) exactly as typed there, including `caret`. The caret is required *now*,
  not at prompt 25 — inserting into an empty voice has no representation without it, and retrofitting it later means
  rewriting every navigation path.
- **Pointer** (§2) with the local-first rule: the halo is drawn from the clicked element's `xml:id` in the same frame
  (B3), and the Rust round trip only enriches the inspector (B4). Drag on the leaf is range selection and nothing else.
- **Keyboard map** (§3) in one file, `src/lib/commands/map.ts`, which is also the source for the command palette and the
  `?` keyboard sheet — bindings and their documentation cannot drift because there is one list.
- **Command palette** (§6): `⌘K`, leaf-styled, every command with its binding, verbs in sentence case matching their
  result messages (`05-states.md` §1).
- **Transport and playhead** (§4):
  - position comes only from engine events; the frontend interpolates on `requestAnimationFrame` **while playing only**
    and stops if events stop (B5, B10);
  - the playhead is the sounding-note tint plus a hairline, drawn in the prompt-22 overlay layer;
  - follow modes `off` / `page` / `continuous`, default `page` — a page turn, not a scroll;
  - loop is a range derived from the selection, drawn with repeat glyphs in the system margin, not a colored rectangle;
  - playback never moves the selection and selection never stops playback.
- **Inspector** wired to the live snapshot: pitch, duration as a `Fraction`, part/voice, bar:beat, and the Origin row of
  `04-provenance.md` §3 with clickable segments (selecting an occurrence works here; the held lens is prompt 24). Fields
  are read-only until prompt 25.
- **Accessibility floor** (§5) in full and tested: roving tabindex over events with the musical accessible name; live
  region for selection (polite) and transport start/stop (assertive); every action keyboard-reachable and present in
  `⌘K`; automated contrast test; reduced-motion paths; 200 % zoom without clipping.

## Target

- `apps/musa-desktop/ui`: selection store and caret rendering; `commands/map.ts` + palette + keyboard sheet; transport
  with interpolated playhead and follow modes; loop; live inspector; a11y layer.
- Tests: selection-survives-re-render (with prompt 22's anchoring); keyboard navigation over a fixture; playhead jitter
  measurement (B5); idle-CPU assertion with no rAF loop at rest (B10); `axe`-style automated accessibility scan with
  zero violations; perf assertions B3, B4, B9.

## Check

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm run check && npm run test
cd apps/musa-desktop && cargo tauri dev
# manual: navigate glass-mountain.musa with arrows only, play from selection, loop a bar,
#         confirm the playhead is smooth and the app is silent on the CPU when stopped
```

Commit as `Add score selection, keyboard navigation, and playhead`.

## Stop

- No editing commands and no note entry (prompt 25) — the inspector is read-only.
- No Origin view lens or traces (prompt 24).
- No MIDI input (prompt 33).
- No cross-voice or cross-part multi-selection; contiguous within one voice, per `03-interaction.md` §1.
