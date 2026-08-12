---
id: 52
slug: linked-reading
status: done
depends_on: [24, 25, 26]
phase: 2
---

# Linked Reading: Which Note Is Which

## Task

Make the correspondence between a note on the page and the statement that wrote it answerable by *looking*, in both
directions, without clicking anything. Point at a note and the source says which line spelled it; point at a line and
the page says which notes it produced. Prompt 26 linked caret to selection — a committed, sticky, semantic act. This
links attention, which is what reading a score actually is.

## Read

- `docs/rules/desktop/04-provenance.md` — Origin view is the *held* answer to "where did this come from". This prompt is
  the *unheld* one, and the two must not look alike or the lens stops meaning anything.
- `docs/rules/desktop/01-visual-language.md` §2 (two hues, each with one meaning) and §6 (three animations, and this is
  not one of them); `03-interaction.md` §1–§2 (selection is not hover), §6 (the accessibility floor).
- `docs/rules/desktop/06-performance.md` — hover is a pointer-rate event and must be frame-local.
- Prompt 24's `originSpans`/`sourceSpans` and occurrence model; prompt 26's `SourcePane`, marks, and `eventsForSpan`;
  the snapshot's `OriginFacts.definitionSpan`, which already carries the statement that *spells* an event as distinct
  from the one that *places* it.

## Design

### The one idea

**The two views share one focus.** Whatever the pointer is over — a notehead on the page, a line in the source — is the
focus, and the focus is marked in *both* views at once, in the same weight, in the same frame. There is one focus, not a
score hover and a source hover that happen to agree.

### It must not look like selection, and must not look like the lens

Three things can now mark the same note, so each gets a distinct *shape* rather than a distinct colour — the discipline
`03-interaction.md` §5 already uses for error and warning glyphs:

| State | On the page | In the source |
| --- | --- | --- |
| **Focus** (hover; transient) | a `1px --plate` hairline directly under the notehead, `1sp` wide | the line number turns `--plate` |
| **Selection** (clicked; sticky) | the filled `--plate-wash` halo, unchanged | the `--plate-wash` mark behind the statement, unchanged |
| **Origin view** (held) | re-inked generated material and the trace, unchanged | the wash on the `motif` and the `use`, unchanged |

Two hues survive. A hairline is not a halo and a coloured line number is not a wash, so all three can be true at once
and still be read apart.

### What the focus resolves to

This is the part that is musa's and not a generic editor's: **one statement is usually more than one note, and one note
usually comes from two statements.** Pretending otherwise is what makes the current app confusing.

- **Focus a note that was authored.** Its one statement is marked. Symmetric and boring, which is correct.
- **Focus a note that was generated.** *Both* places are marked: the statement inside the `motif` that spells it
  (`definitionSpan`) and the `use` that placed it (`span`). They are distinguishable — the definition gets the hairline
  treatment, the `use` gets its line number marked — because they answer two different questions and a composer who
  cannot tell them apart will edit the wrong one.
- **And the siblings.** All the other notes that same definition spelled, in every occurrence, get the hairline too, at
  the same weight. "Which note corresponds to this line" has a plural answer, and showing one of six is a lie the
  editing choice (`04-provenance.md` §4) later has to correct with a number.
- **Focus a line in the source.** Every note it produced gets the hairline. Nothing else changes — no scroll, no
  selection, no re-engraving.

### Rules

- **Frame-local, no round trip.** Every mapping needed is already in the snapshot: event → `origin.span` and
  `origin.definitionSpan`, and the reverse by containment (`eventsForSpan`). Build both indexes once per snapshot, not
  per pointer move. Budget: focus resolves within one frame at the largest fixture in `examples/`.
- **No motion.** The hairline appears and disappears. `01-visual-language.md` §6 allows three animations; this is not
  one of them.
- **No scrolling.** Focus never moves either view. A hover that scrolls the source out from under the pointer is a trap.
- **Keyboard parity.** Focus follows the keyboard too: the note the arrow keys land on, and the line the caret is on,
  are the focus. That is what makes this reachable without a pointer, and it means the feature is *on* for someone who
  never hovers at all.
- **Nothing on an empty focus.** Pointer over blank paper marks nothing; the last focus does not linger.

### The affordance for people who do not know to hover

A feature that only exists on hover is a feature most people never find. Two quiet, permanent signals:

- The inspector's `ORIGIN` row already prints `sigh() ▸ note 1  line 19` and already links the line number. It gains the
  sibling count when there is one — `line 19 · 2 notes` — so the plural answer is visible before anyone hovers.
- In the source column's gutter, a line that produced music on the *currently visible page* gets a `--rule` tick beside
  its number. Not a hue and not a count: just the difference between "this line makes sound" and "this line is
  scaffolding", which is the shape of a musa file at a glance.

### Pointing at text means pointing at a line

The first cut resolved a source hover to the character offset under the pointer and asked the index which spans
contained it. It is wrong in the ordinary case: a statement ends where its semicolon does, the rest of the line is
blank, and a pointer resting anywhere in that blank reports the line's end — which no span contains, so the page marks
nothing. Worse, it makes the answer depend on where in the line the pointer happens to be, which is not something a
reader is thinking about.

So the editor reports the *line* it is over — `posAtCoords` then `lineAt`, both of them CodeMirror answering questions
about its own document — and the index answers by overlap rather than containment. A line is what a reader points at.
Nothing musical is derived in the frontend either way (`03-interaction.md` §7).

## Target

- `apps/musa-desktop/ui`:
  - `src/lib/state/focus.svelte.ts` — the shared focus: what is focused, and the two indexes that resolve it in both
    directions. Built from the snapshot, invalidated with it.
  - `Overlay.svelte` — the focus hairline, beside the existing halo.
  - `SourcePane.svelte` / `SourceEditor.svelte` — the focused line number, and the gutter tick for lines with music on
    the visible page.
  - `Inspector.svelte` — the sibling count on the origin row.
- No Rust changes. `definitionSpan` and the occurrence model already carry everything; if something is missing, it is a
  fact the core must add, not one the frontend may compute (`03-interaction.md` §7).
- Tests:
  - unit: the two indexes on `glass-mountain.musa` — an authored note resolves to one statement, a generated note to
    two, and a definition line resolves to every note it spelled across both occurrences.
  - Playwright: hover a generated note and assert both marks and the sibling hairlines; hover the `motif` body line and
    assert the notes on the page; arrow to a note and assert the focus follows the keyboard; assert focus never changes
    the selection, never scrolls either view, and never survives leaving the leaf.
  - a golden of the focused state at 1440, both themes.

## Found along the way

- **A mark under a note has to ask for the notehead.** An event's box is the notehead, the stem, the flag, and any
  ledger lines, so a hairline under *that* lands two staff spaces below a stem-down quarter. `headsFor` in
  `score/geometry.ts` is `boxesFor` restricted to `g.notehead`, falling back to the whole element for rests and anything
  else the engraver draws without a head.
- **The sibling count belongs to the trailing, not to the path.** `line 19 · 2 notes` reads as one fact about where the
  note came from. Put in the row's body it would read as a second control, and it is not one.

## Check

```sh
cd apps/musa-desktop/ui && npm run check && npm run test
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Commit as `Link reading between the score and the source`.

## Stop

- No hover on anything but notes and source lines — not slurs, not dynamics, not barlines, not part names.
- No tooltips or popovers of any kind. The answer is a mark in the other view, never a floating box.
- No change to the selection model, the Origin lens, or the caret.
- No new hue, and no reuse of `--chalk`.
