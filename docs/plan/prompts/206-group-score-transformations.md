---
id: 206
slug: group-score-transformations
status: pending
depends_on: [201, 205c]
phase: 2
---

# Revise Groups of Notes with One Musical Command

## Task

Generalize provenance-aware score editing from one event at a time to one atomic operation over an explicit selection.
Make common review and composition corrections—set every selected duration to an eighth, scale rhythm, move pitches,
transpose by a written interval, or respell—faster than editing tokens individually while retaining exact source impact.

## Read

- Prompt 201's amended selection/edit contract; prompts 23–25, 34, 53, 56–58, 78, 100, 103, 131, 169, and 200;
  `docs/rules/desktop/03-interaction.md` and `04-provenance.md`.
- Current `EditCommand`, `EditImpact`, syntax-aware edit computation, origin/definition spans, generated specialization,
  selection geometry, undo/autosave, and transactional compile path.
- Open Music Theory chapters 009–012 for the distinction among setting each duration, scaling a passage, and renotating
  one total span; chapters 016 and 100 for diatonic movement versus interval transposition.

## Design

One project operation accepts a nonempty, revision-stamped, de-duplicated set of `EventId`s and one intent:

- `SetEachDuration(d)`—every applicable selected note/rest/chord is written as exact duration `d`;
- `ScaleDurations(r)`—multiply each selected written duration by positive exact `r`;
- `MoveDiatonically(steps)`—change written steps while carrying accidentals;
- `ShiftAccidentals(steps)`—change alterations while carrying letters; or
- `TransposeBy(interval)`—apply the source's spelled interval action to every selected pitch/chord member.

Do not collapse these into “change notes.” Setting every value to `1/8` is not fitting a phrase into its old span;
scaling is not setting; diatonic staff movement is not chromatic/interval transposition. A future fit-to-span command
needs its own musical contract and is outside this prompt.

Preflight resolves every event through provenance, computes the minimal set of unique source replacements, detects
duplicate definition spans and conflicting requested replacements, compiles the candidate source, and returns an
immutable preview: affected events, unchanged inapplicable events, grouped origin consequences, source edits, bar/meter
effects, and resulting diagnostics. Applying consumes the preview identity at the same project revision or returns
stale; it is one transaction and one undo entry.

Mixed selections are honest. Duration intents apply to notes, chords, and rests. Pitch intents leave rests unchanged and
state the count before application; they never silently drop an error on a note/chord they cannot transform. Chord
members transform together. Ties, tuplets, bars, and generated occurrences are validated as structures, not rewritten as
disconnected tokens. If exact source would need a new tie/tuplet or violate a bar, preview shows the refusal and the
smallest next action rather than committing invalid source.

Generated material preserves prompt 24's choices, now grouped by definition/occurrence. An edit-definition transform is
applied once per unique definition and reports every affected occurrence. Specialization is offered only where the
existing override semantics can express the complete selection; no partial batch quietly changes the definition.

The score UI makes selection commands available without an entry mode. With score focus and a selection, `1 2 4 8 6 3`
set durations directly; existing diatonic/accidental arrows operate on the whole selection; **Transpose selection** asks
for a source interval inline through the palette/inspector. Pointer rectangle or explicit command may collect rendered
event identities across voices as prompt 201 allows; the project validates membership and ordering. With no selection,
number and pitch-edit keys do nothing and never start an implicit mode.

## Target

- One deep project group-edit plan/apply facade, syntax edit support, generated-impact grouping, and versioned IPC
  types.
- Multi-event/cross-voice selection where required, with keyboard/pointer/accessibility paths and generated TypeScript.
- Live engraved/source preview using the exact candidate edits, affected/unchanged counts, diagnostics, Accept/Cancel,
  stale revision handling, and one-step undo.
- Laws for order/de-duplication, atomicity, locality, exact duration/interval action, chord/rest applicability,
  bars/ties/ tuplets, generated definitions/specializations, conflicting origins, Unicode spans, and invalid/stale
  refusal.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-project -p musa-desktop
cargo clippy --all-targets -p musa-syntax -p musa-project -p musa-desktop -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
cd apps/musa-desktop/ui && npx playwright test --project=chromium
```

Commit as `Transform selected music in one source transaction`.

## Stop

- No transcription Review UI or proposal acceptance; prompt 207 consumes this operation.
- No fit-to-span, arbitrary note dragging, voice reassignment, free-form piano roll, or mutable expanded score.
- No frontend duration/pitch/provenance computation and no partial application after a preflight refusal.
