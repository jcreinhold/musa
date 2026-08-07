---
id: 25
slug: score-editing
status: done
depends_on: [24]
phase: 1.5
---

# Structured Score Editing

## Task

Implement the provenance-aware edit pipeline: semantic score commands (`InsertNote`, `ChangePitch`, `ChangeDuration`,
`ExtractMotif`) that the project layer resolves — through each event's `Origin` — into transactional source text edits,
plus the keyboard-first note entry that issues them. This is the roadmap's distinctive editing story: the source stays
canonical, and editing generated music surfaces a real choice instead of silently mutating a cache.

## Read

- Roadmap §9 (editing transformed music: edit-definition vs detach/specialize — **note**: occurrence specialization
  syntax `use sigh() with {...}` is implemented in prompt 34; this prompt must offer the choice and perform
  edit-definition now, and diagnose "specialization requires prompt 34" cleanly otherwise), §11 (`EditCommand` enum,
  project resolves to text transactions), §14.5 (keyboard-first entry), §14.6 (the six-step command flow).
- `docs/interface/04-provenance.md` §4 — the editing choice, its wording, its counts, and the rule that it is inline
  rather than modal; `03-interaction.md` §1, §3 (the caret and the keyboard map this extends); `05-states.md` §1, §6
  (voice; how results are reported).
- Prompt 04's `apply_edits`/`TextEdit`, prompt 05/06's `Origin` and expansion paths, prompt 19's `ProjectCommand`,
  prompt 24's occurrence selection.

## Design

- `musa-language` gains the syntax-aware edit computation (it owns text-edit utilities, §15.2): given a CST, a source
  span (or event origin), and an intent (insert note at position, change pitch token, change duration token), compute
  `Vec<TextEdit>`. Insertion must place the new statement at the correct sequential position inside a voice block with
  correct indentation; changing pitch/duration is a token replacement. Extract-motif wraps the selected statements into
  a new `motif` declaration and replaces them with `use name();`.
- `musa-project`: extend `ProjectCommand` with `EditScore(EditCommand)` using §11's enum (`InsertNote`,
  `ChangePitch { mode: GeneratedEditMode }`, `ChangeDuration`, `ExtractMotif`). `GeneratedEditMode` at this prompt:
  `EditDefinition` works; `Specialize` returns a structured "not yet supported" error variant (prompt 34 removes it).
  The flow is §14.6 exactly: resolve provenance → authored vs generated → compute text edits → apply transactionally →
  recompile → return updated snapshot.
- **Impact counts.** A command against a generated event returns, before applying, how many occurrences and events an
  `EditDefinition` would change — `04-provenance.md` §4 states the consequence in counts, and the frontend must not
  compute them. Add this to the command's result type.
- Transactional: if the computed edits produce invalid source, the whole command fails and the session is unchanged
  (the stale-revision behavior covers the UI).
- **GUI** (Compose workspace), extending prompt 23's map rather than inventing a second one:
  - number keys pick duration (whole→1, half→2, quarter→4, eighth→8, sixteenth→6, thirty-second→3), `.` toggles dotted;
    letter keys `c d e f g a b` enter pitch at the caret; `↑`/`↓` with a modifier adjust octave and accidental; `r`
    inserts a rest (see the repairs below). The active duration is shown in the top margin as the SMuFL glyph itself,
    not as a word.
  - The same shortcuts change a selected event rather than inserting, per `03-interaction.md` §1's selection kinds.
  - Every action issues an `EditScore` command; the frontend never mutates a local model (§14.2).
  - **Editing generated music** follows `04-provenance.md` §4 exactly: Origin view enters and holds automatically, the
    affected events are previewed with the selection halo, the inline choice states the counts, the disabled
    `Specialize` option explains itself, and the result is reported in musical words — *"Edited sigh() — 2 occurrences
    updated."*
  - Extract motif: select a range, name it inline (not a dialog), source updates visibly in the drawer.
  - After each command the score re-renders through prompt 22's anchored path; the caret and selection follow.
- Undo/redo already exists (prompt 19's revisions); wire `⌘Z`/`⇧⌘Z` and register them in the command map.

### Repairs made while implementing

Four deliberate deviations, each repaired in the governing document before the code was written:

- **Entry is a mode, toggled with `N`.** The keyboard map of `03-interaction.md` §3 already spends the unmodified
  letters — `F` follows, `L` loops, `O` is the lens — so a bare `f` cannot also be the note F. The mode is never
  invisible: the duration glyph sits in the top margin while it is on. The bindings are now written into §3 as their
  own table.
- **`r` inserts a rest, not `Space`.** `Space` is play, and a transport key that stopped playback from inside a mode
  would be worse than one more letter to learn.
- **No tie shortcut.** The language has no tie construct until prompt 27, and a key that spells nothing is worse than
  a key that is not there yet. `~` arrives with the constructs it would write.
- **`ChangeDuration` carries a `GeneratedEditMode` too.** Roadmap §11 gave it only to `ChangePitch`, but renotating a
  generated note changes every occurrence exactly as respelling one does, and §9 forbids making that choice silently.
  The roadmap snippet is repaired.

Two facts the implementation settled, rather than deviations:

- **The impact rule is one line.** `Origin` gains a `definition_span` — the statement that literally spells the event,
  which for a generated note is inside the `motif` body — so an edit-definition edit is the *same* token replacement
  as an authored one, and the events that change together are exactly those sharing a `definition_span`.
- **`04-provenance.md` §4's worked example said "changes 2 occurrences, 10 notes"**; the honest count for a one-note
  edit is 2, and the document is repaired. A screen that overstates its own consequence teaches a composer to stop
  reading it.

## Target

- `musa-language`: edit-computation API (one deep entry point preferred over four shallow ones).
- `musa-project`: `EditScore` command path with provenance resolution, impact counts, and transactional apply.
- `apps/musa-desktop/ui`: keyboard entry, editable inspector fields, the generated-edit choice, extract motif.
- Tests: edit computation snapshots (insert into empty/nonempty voice, change pitch of an authored note, change pitch of
  a motif-generated note with `EditDefinition`, extract motif); transactional failure test; project round-trips (apply →
  snapshot changed → undo → restored); impact-count test on `glass-mountain.musa`; Playwright: enter a four-note melody
  by keyboard, edit a generated note through the choice, undo.

## Check

```sh
cargo nextest run -p musa-language -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm run check && npm run test
cd apps/musa-desktop && cargo tauri dev
# manual: enter a melody by keyboard only, change a generated note and read the counts, extract a motif, undo
```

Commit as `Add provenance-aware score editing`.

## Stop

- No `use sigh() with {...}` specialization (prompt 34) — the error and the disabled option must be clean and explain
  themselves.
- No mouse drag editing (§14.5 rejects it as primary; do not add it at all here).
- No MIDI keyboard input (prompt 33).
- No slur/tie/tuplet **notation** entry beyond the tie shortcut (prompt 27's constructs do not exist yet).
