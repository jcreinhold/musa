---
id: 53
slug: pointer-editing
status: done
depends_on: [25, 26, 27, 52]
phase: 2
---

# Pointer Editing: the Score Writes the Source

## Task

Make the score an input surface as well as a projection: a pointer gesture on the page issues the same `EditScore`
command the keyboard already issues, and the source text changes in front of you while the gesture is live. Three
gestures — drag a notehead vertically to respell it, drag its right edge to renotate it, click an empty staff step to
write a note there — and nothing else.

This is the second direction of the relationship prompt 26 built one way. Prompt 26 made the text and the page two views
of one document by linking caret to selection; this prompt makes the *page* a way to write the *text*.

## Read

- Roadmap §14.5 (why dragging is not the primary interaction — the six ambiguities this prompt answers by construction),
  §14.6 (the command flow this reuses unchanged), §11 (`EditCommand`), §9 (editing generated music).
- `docs/rules/desktop/03-interaction.md` §1 (the caret is a first-class state), §2 (the pointer table this repairs), §3
  (the entry-mode bindings whose ladder the duration drag snaps to), §6 (the accessibility floor).
- `docs/rules/desktop/04-provenance.md` §4 — the generated-edit choice, reused verbatim. A pointer edit against
  generated music is the same choice with the same counts and the same words.
- Prompt 25's `EditScore` path, impact counts, and `GeneratedEditMode`; prompt 26's `SourcePane`, provenance marks, and
  span linking; prompt 22's `boxesFor`/page coordinates in `src/lib/score/geometry.ts`, which already resolve to the
  page's own coordinate system and therefore to staff spaces at any zoom.

## Design

### The rule that makes this safe

Roadmap §14.5 rejects dragging because it is ambiguous about voice ownership, accidentals, duration, ties, tuplets, and
generated occurrences. Every one of those is an ambiguity about *what the gesture meant*. Musa can refuse the question
instead of guessing at it, because it has something a notation editor does not: every engraved event carries an `Origin`
with a `definition_span`, so a gesture can name the exact token that produced it.

**A pointer edit replaces one token with one value. It never moves a statement, never reorders a voice, and never
invents a construct.** Each ambiguity is then answered by construction, not by heuristic:

| §14.5's ambiguity | Why it cannot arise here |
| --- | --- |
| Voice ownership | The statement stays exactly where it is in the source. A gesture cannot move a note between voices, because voices are blocks of text and no gesture edits block structure. |
| Accidentals | The gesture writes a **diatonic step**; the token's accidental is carried through unchanged and the key signature does the rest. `⌥`-drag cycles the accidental instead, and does not touch the step. Neither gesture ever guesses. |
| Duration | Vertical drag does not write a duration token. Duration has its own gesture on its own axis. |
| Ties and tuplets | Both are constructs spanning more than one statement (prompt 27). A gesture that edits one token cannot create, destroy, or reshape one. A drag whose result would need one refuses and says which construct it would need. |
| Generated occurrences | Routed into `04-provenance.md` §4's existing choice, unchanged: Origin view enters, the affected events are haloed, the counts are stated, and the result is reported in musical words. |

The gesture set is therefore *smaller* than what a mouse could express, on purpose. Dragging remains what §14.5 says it
is — not the primary interaction — and the keyboard remains the way to write music quickly.

### The gestures

Resolved by axis and by target, after a **4px threshold**. Under the threshold the gesture is a click and still means
"select" (§2), so nothing about the existing pointer model changes for anyone who does not drag.

1. **Vertical drag on a notehead → `ChangePitch`.** Snapped to staff steps: the nearest half staff space is the nearest
   diatonic step, which the page coordinates already give exactly. `⌥`-drag cycles the accidental of the note under the
   pointer instead (`♭ ♮ ♯`), leaving the step alone.
2. **Drag the right edge of a note → `ChangeDuration`.** Snapped to the ladder the number keys already spell — whole,
   half, quarter, eighth, sixteenth, thirty-second, each with its dotted form — and to nothing between them. The hit
   target is the last `1sp` of the note's advance, cursor `ew-resize`.
3. **Click an empty staff step inside the active voice → `InsertNote`** at the caret, with the active duration, at the
   step clicked. This is the caret-first entry of §1 with the step supplied by the pointer rather than by a letter key.
   Only while entry is armed (`N`), so a stray click on empty paper never writes music.

**Horizontal drag is unchanged: range selection.** Musical position in musa is statement order plus duration, not a
coordinate, so a note dragged sideways would have to guess between changing its own duration, changing its
predecessor's, and inserting a rest. It is not a gesture with an answer, so it is not a gesture.

### The signature: the drag writes the text

While a pointer edit is live, and before anything is committed:

- the source column marks the **exact token** the gesture will replace, and shows the candidate value in its place, in
  `--plate` — the hue that already means *derived, or live* (`01-visual-language.md` §2);
- the score draws the candidate note at the candidate step, in the same hue, over the unmoved engraving;
- releasing commits — one `EditScore` command, one revision, one undo step;
- `Esc`, or releasing back at the origin, cancels and leaves the document untouched.

Nothing is written on press. Nothing is written per-pixel. The composer watches the text they are about to write, in the
file it will be written into, and can back out of it.

If the source column is not showing, the candidate value is printed beside the pointer instead — `f♯5`, `1/8.` — set in
the mono face, so the gesture still names what it will write rather than merely showing where it will land.

### What this must not become

- **No second edit path.** Every gesture issues the `EditScore` command prompt 25 built (§14.2). The frontend computes a
  *step and a token id*, never text.
- **No optimistic mutation.** The candidate is a drawn overlay, not a change to the engraving or to the document. The
  score re-renders through prompt 22's anchored path when the snapshot returns.
- **No drag-only capability.** WCAG 2.5.7: everything reachable by drag is already reachable by key — `⌥↑`/`⌥↓` respell,
  the number keys renotate, the letter keys insert. This prompt adds no capability, only a second way to reach one.
- **No motion.** `01-visual-language.md` §6 allows three animations and this is not one of them. The candidate appears
  and disappears; it does not ease.

## Repairs required before implementing

Both are governing documents, so both are repaired and committed *before* any code (README execution rule 5).

- **`docs/rules/desktop/03-interaction.md` §2**, whose table currently reads `Drag on the leaf | Range selection
  **only**. Dragging never moves a note.` The sentence is right about what it forbids and wrong as a general rule: no
  gesture here moves a note, but two of them retype one. Replace the row with the three gestures above plus the
  unchanged horizontal behaviour, and add the token rule as the paragraph that governs them.
- **Roadmap §14.5**, whose list of ambiguities is the reason dragging was rejected. It stays rejected *as the primary
  interaction*; add the clause that a gesture scoped to a single token is not the thing the section rejects, and cite
  the table above rather than restating it.

If either repair proves unwritable — if the argument does not survive being written into the governing document — this
prompt is wrong and should be deleted rather than implemented.

### The geometry is relative, not absolute

The Target below asks for `page coordinate → (staff, step)`. Implementing it that way would mean reading the clef, the
key signature, and the staff's own position out of Verovio's SVG — which is the frontend deciding what pitch a place on
the page is, and so exactly what `03-interaction.md` §7 forbids.

The gesture does not need it. Every gesture starts *on a note the core already spelled*, so the only thing the pointer
has to say is **how far it went**. A diatonic step is half a staff space on every clef, in every key, at every zoom, so
`stepsFor(dy, staffSpace)` is the whole of it, and `shiftStep` is letter-and-octave arithmetic on the token the source
already contains — the same kind `state/compose.ts` does for the letter keys. No clef is read and no pitch is guessed.

The one gesture with no note under it — the empty-step click — measures from the nearest note on the same staff, which
the core also spelled. That is the same relative rule, and it is also how a reader would answer the question.

### The label spells the source, not the score

The Design says the candidate is printed as `f♯5`. It is printed as `fs5`: the text the gesture will write, in the
language's own spelling. A label that says `f♯5` while the source column would say `fs5` is two vocabularies for one
value, and the whole point of the signature is that the composer is watching the text they are about to write.

## Target

- `musa-syntax`: nothing new. `ChangePitch`, `ChangeDuration`, and `InsertNote` already compute the edits.
- `musa-project`: nothing new, unless the impact-count path needs to answer *before* a command is applied for the live
  preview — in which case a query that returns the candidate text edits without applying them, so the frontend can show
  the token it will replace. One entry point, not four.
- `apps/musa-desktop/ui`:
  - `src/lib/score/steps.ts` — page coordinate → `(staff, step)`, the inverse of what `geometry.ts` already does
    forward; the one piece of real geometry this prompt adds.
  - a pointer-gesture state machine beside the existing selection state: idle → pressed → (select | pitch | duration),
    resolved by axis and target after the threshold.
  - the candidate overlay in `Overlay.svelte`, and the candidate token mark in `SourcePane.svelte`.
- Tests:
  - unit: coordinate → step at three zoom levels and on both clefs; the duration ladder's snapping, including dotted;
    the state machine's axis resolution, including the under-threshold click.
  - Playwright: drag a notehead up two steps and read the changed source; drag it back and read the original; `Esc`
    mid-drag and assert the document is untouched; drag a *generated* note and assert the §4 choice appears with its
    counts; drag a note's edge from `1/2` to `1/4`; click an empty step with entry armed and assert one statement was
    written; drag horizontally and assert a range selection, not an edit.
  - accessibility: the axe scan with a gesture in flight, and an assertion that every gesture's keyboard equivalent
    exists in the command map.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm run check && npm run test
cd apps/musa-desktop && cargo tauri dev
# manual: drag a note of `glass-mountain.musa` up a third with the source column open, and watch the
# pitch token change under the pointer before the mouse is released; press Esc and confirm nothing was written
```

Commit as `Add pointer editing to the score`.

## Found along the way

- **The impact query answers the preview.** `EditImpact` gained `writes: Vec<CandidateEdit>` — the text edits the
  command *would* make, from the same `intent_of` / `compute_edits` path that applies them. One entry point, as the
  Target asked: the query that decides whether an edit needs §4's choice is the query that says which token it replaces.
- **The duration handle is a third of the note, not a staff space.** A whole note is barely wider than one staff space,
  so a fixed `1sp` handle swallowed its middle and every drag on it became a renotation. The handle is
  `min(1sp, width / 3)`, which leaves every note a middle to respell.
- **The blank staff had to be made pointable.** An SVG group is hit only where something is drawn, so the space between
  two staff lines — the one place a new note goes — was the one place a click could not land. `g.staff` gets
  `pointer-events: bounding-box`, the same answer prompt 51 gave for hollow noteheads.
- **`⌥`-drag and Origin view share a modifier.** Holding `⌥` is the Origin lens (§2 of `04-provenance.md`), and a press
  on *generated* music while the lens is held already means "select this expansion" — so on generated notes the lens
  wins and the accidental drag does not start. Authored notes drag normally, and `⌥⇧↑` / `⌥⇧↓` reach the accidental
  everywhere, so nothing is unreachable. Left as it is: the lens is the older meaning and the more common one.
- **The range drag had no key.** WCAG 2.5.7 is a rule about every gesture, and the horizontal drag — which this prompt
  keeps rather than adds — had no keyboard equivalent. `⇧←` / `⇧→` (`score.extend.previous`, `score.extend.next`) grow
  the selection from its anchor, and the parity test in `tests/unit/commands.test.ts` is what found the gap.

## Stop

- No dragging notes horizontally, between staves, or between voices.
- No dragging slurs, hairpins, beams, stems, barlines, or any other engraved object. This prompt edits *notes*.
- No drag-to-reorder in the parts list or the outline.
- No page-layout dragging of any kind: engraving is Verovio's and the page has no adjustable objects
  (`02-engraving.md`).
- No new `EditCommand` variants. If a gesture wants one, the gesture is out of scope.
