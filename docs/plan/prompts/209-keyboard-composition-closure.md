---
id: 209
slug: keyboard-composition-closure
status: pending
depends_on: [206, 208]
phase: 2
---

# Retire Step Entry and Close Keyboard Composition

## Task

Prove the complete Listen → Capture/Keep that → Review → Accept workflow and group-revision commands against real
keyboard performances, accessibility, latency, source authority, and transcription quality. Then delete MIDI/computer
step entry, its mode/state, and its fixed chord heuristic so the desktop teaches one coherent workflow.

## Read

- Prompts 201–208 and every completion report/repair; historical prompts 25 and 33; prompt 193's language conformance
  method and prompt 191's performance method.
- Governing desktop interaction/state/frame-budget rules as amended by prompt 201, roadmap §14.5, book
  tutorials/how-tos, command map, generated IPC, current entry tests, and code-map.
- Prompt 203's corpus/thresholds and optional exact-pinned external evaluation procedure. Re-run the representative
  human takes; generated jitter alone cannot close an interaction designed for musicians.

## Design

Build a conformance matrix for: source unchanged during Listen/Capture/Keep that/Review; complete timestamp/pedal
evidence; known/free/tapped clock; straight/compound/tuplet/syncopated/asymmetric/unmeasured rhythm; chords/arpeggios;
one/multiple/ crossing voices; spelling; group duration/pitch edits; generated-source impact; candidate locality;
acceptance formatting/ provenance/autosave; exact undo; device/stale/error recovery; audition parity; privacy bounds;
and every declared refusal.

Measure real workflows rather than isolated functions: plug in to first sound, Capture to first proposal, Keep that to
proposal, ambiguity choice to stable engraving, group edit to preview, Accept to audible/engraved revision, and undo.
Record p50/p95/max, peak memory, search states, correction operations per phrase, top-K intended-candidate recall, and
callback violations. A transcription that meets note F1 but requires excessive corrections is red.

Once the matrix is green, delete `NoteEntry`, `MidiEntry`, the 40 ms `EntryBuffer`, the Notes/`N` mode, active duration/
dot/octave/accidental state, MIDI-to-`InsertNote`, entry-only polling, entry glyph, and their snapshots/tests. Preserve
the general semantic insert/edit commands where source, pointer, capture acceptance, or other callers still use them.
Numbers with a score selection remain prompt 206's group-duration commands; with no selection they do nothing. Exact
note entry is taught in the source editor.

The final score keyboard map uses **Capture** and **Keep that** labels/bindings established by prompt 201, not recycled
entry terminology. A keyboard auditions whenever safely connected; capture is the only armed state and is visually and
accessibly unmistakable. Remove every doc sentence that tells a musician to select a duration and then play/type a
pitch, including the new-piece invitation.

Write Diátaxis documentation: a first captured phrase tutorial; how-tos for known-tempo capture, free capture with tap
anchors, Keep that, chord/voice corrections, selecting notes and setting duration/transposing, device/latency problems;
an explanation of performed versus written time and pedal; and references for supported MIDI evidence, transcription
policy, bounds, alternatives, commands, and losses. State plainly that audio transcription and performance recording
sessions are outside scope.

## Target

- Complete conformance, performance, correction-count, and accessibility reports with real/synthetic corpus results and
  host/device/toolchain record.
- Focused fixes within prompts 202–208's boundaries until the matrix is green; repair a governing/public boundary first
  if evidence requires it.
- Complete removal of step-entry code, state, IPC, UI, tests, rules/roadmap/book language, and dead dependencies while
  preserving autosave and semantic editing.
- Final book, code-map, prompt overview, command/help generation, and desktop screenshot updates for the one workflow.

## Check

```sh
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
make docs-check
cargo deny check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
cd apps/musa-desktop/ui && npx playwright test --project=screens
python3 scripts/renumber-prompts.py audit
```

Commit as `Replace step entry with capture and transcription`.

## Stop

- No audio transcription, waveform/recording/take browser, comping, overdub timeline, model service, or DAW feature.
- No keeping dead entry code behind a preference or compatibility alias; history remains in prompt 33 and the research
  note.
- No claim of universal transcription, hidden fallback, or green matrix cell without linked evidence.
