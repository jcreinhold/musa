---
id: 16
slug: score-editing
status: pending
depends_on: [15]
phase: 1.5
---

# Structured Score Editing

## Task

Implement the provenance-aware edit pipeline: semantic score commands (`InsertNote`, `ChangePitch`, `ChangeDuration`,
`ExtractMotif`) that the project layer resolves — through each event's `Origin` — into transactional source text edits,
plus the GUI's keyboard-first note entry that issues them. This is the roadmap's distinctive editing story: the source
stays canonical, and editing generated music surfaces a real choice instead of silently mutating a cache.

## Read

- Roadmap §9 (editing transformed music: edit-definition vs detach/specialize — **note**: occurrence specialization
  syntax `use sigh() with {...}` is implemented in prompt 24; this prompt must offer the choice and perform
  edit-definition now, diagnose "specialization requires prompt 24" cleanly otherwise), §11 (`EditCommand` enum, project
  resolves to text transactions), §14.5 (keyboard-first entry; mouse is for selection), §14.6 (the six-step command
  flow).
- Prompt 04's `apply_edits`/`TextEdit`, prompt 05/06's `Origin` and expansion paths, prompt 14's `ProjectCommand`.

## Design

- `musa-language` gains the syntax-aware edit computation (it owns text-edit utilities, §15.2): given a CST, a source
  span (or event origin), and an intent (insert note at position, change pitch token, change duration token), compute
  `Vec<TextEdit>`. Insertion must place the new statement at the correct sequential position inside a voice block with
  correct indentation; changing pitch/duration is a token replacement. Extract-motif wraps the selected statements into
  a new `motif` declaration and replaces them with `use name();`.
- `musa-project`: extend `ProjectCommand` with `EditScore(EditCommand)` using §11's enum (`InsertNote`,
  `ChangePitch { mode: GeneratedEditMode }`, `ChangeDuration`, `ExtractMotif`). `GeneratedEditMode` at this prompt:
  `EditDefinition` works; `Specialize` returns a structured "not yet supported" error variant (prompt 24 removes it).
  The flow is §14.6 exactly: resolve provenance → authored vs generated → compute text edits → apply transactionally →
  recompile → return updated snapshot.
- Transactional: if the computed edits produce invalid source, the whole command fails and the session is unchanged
  (last-valid behavior covers the UI).
- GUI (Compose workspace):
  - Selection model: active part/voice, selected event, cursor position between events.
  - Keyboard entry per §14.5: number keys pick duration (whole→1, half→2, quarter→4, eighth→8, sixteenth→6,
    thirty-second→3), letter keys or MIDI-less keyboard map pick pitch (c d e f g a b with arrow-key octave/accidental
    adjust), space inserts a rest, arrows navigate events, `.` toggles dotted duration. Every action issues the
    corresponding `EditScore` command — never a local mutation (§14.2).
  - Duration/pitch change on a selected event via the same shortcuts.
  - Extract motif: select a range of events, name the motif, command issues; source updates visibly in the source
    drawer.
  - After each command the score re-renders from the new snapshot; selection follows the event id where possible.
- Undo/redo already works (prompt 14's revisions); wire keyboard shortcuts.

## Target

- `musa-language`: edit-computation API (`compute_insert_note`, `compute_change_pitch`, `compute_change_duration`,
  `compute_extract_motif` — names may vary; one deep entry point preferred).
- `musa-project`: `EditScore` command path with provenance resolution and transactional apply.
- `apps/musa-desktop`: keyboard-first entry + selection + inspector editing.
- Tests: edit computation snapshot tests (insert into empty/nonempty voice, change pitch of authored note, change pitch
  of motif-generated note with `EditDefinition`, extract motif); transactional failure test; project-level round-trips
  (apply edit → snapshot pitch changed → undo → restored). GUI: extend the smoke checklist.

## Check

```sh
cargo nextest run -p musa-language -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop && cargo tauri dev   # manual: enter a melody by keyboard, change a note, extract a motif, undo
```

Commit as `Add provenance-aware score editing`.

## Stop

- No `use sigh() with {...}` specialization (prompt 24) — the error must be clean.
- No mouse drag editing (§14.5 rejects it as primary; do not add it at all here).
- No MIDI keyboard input (prompt 23).
- No slur/tie/tuplet entry (prompt 17 constructs don't exist yet).
