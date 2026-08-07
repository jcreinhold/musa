# 04 — Origin View

Status: **candidate**.

This is the application's signature. It is the one interaction that only musa can offer, and it is the visual
precondition for the editing story in roadmap §9.

## 1. The problem it solves

In musa, a lot of the music on the page was never typed. It was produced by expanding a motif, applying `transpose`,
`repeat`, and later `stretch` / `retrograde` / `invert`. From `glass-mountain.musa`, ten of the violin's notes come from
two occurrences of one five-note motif, one of them transposed down a fifth.

A composer working this way asks two questions constantly:

1. *Where did this note come from?*
2. *If I change it, what else changes?*

Every other program answers by making the user remember. Musa's compiler already knows exactly — `Origin` and expansion
paths, prompt 06 — and the interface's job is to show it without wrecking the engraving.

## 2. The interaction

**Hold `O` (or `⌥`) to enter Origin view. Release to leave.** It is a held lens, not a mode with state to get lost in.
A `⌥`-click on the toggle in the top margin pins it for users who cannot hold a key.

While held, over 120 ms:

- **Authored music stays exactly as it is** — full `--ink`, unmoved.
- **Generated music fades to `--plate` at 65 % opacity.** The page separates into what you wrote and what the language
  produced. Nothing moves; only ink changes.
- Each generated **run** gets a `⟨` bracket in the left margin of its system, in `--plate`, labelled with the occurrence
  in `--t-micro`: `sigh()`, `transpose down P5 ▸ sigh()`. Colour is never the only signal (`03-interaction.md` §5).
- The parts list dims voices with no generated material, so the structure of the piece is legible at a glance.
- The source drawer, if open, highlights the `use` statement and the enclosing transform blocks that produced what is
  on screen.

**Hovering a generated note while Origin view is held** draws the trace: a `1px --plate` hairline from the note, out to
the margin bracket of its occurrence, and — when the drawer is open — a matching highlight on the `motif` declaration
and the `use` statement in the source. One line, one highlight, drawn instantly; it does not animate along its path.

**Clicking a generated note while Origin view is held** selects the whole occurrence — all the notes that expansion
produced, across bars and staves. That is the unit a composer wants to operate on, and it is the selection prompt 25's
edit-definition path acts through.

## 3. The inspector's Origin row

Always present, held or not. For an authored event:

```
Origin      authored          line 31
```

For a generated event, the expansion path as a real path, in mono, each segment clickable:

```
Origin      transpose down P5 ▸ sigh() ▸ note 3        line 22
```

The path reads outside-in, in containment order: the `use sigh()` that produced these notes sits *inside* the
`transpose down P5` block, and the path says so. Clicking `sigh()` selects the occurrence. Clicking the motif name reveals its declaration in the drawer. The line number
opens the source at that line. This row is how the answer to "where did this come from" is available without holding a
key.

## 4. Editing generated music

Roadmap §9 requires that editing a generated note surfaces a real choice rather than silently mutating a cache. Origin
view is what makes that choice comprehensible instead of a startling dialog.

When an edit is issued against a generated event (prompt 25), the interface:

1. **enters Origin view automatically and holds it**, so the user can see exactly what else is affected;
2. shows an inline choice in the inspector — not a modal — with the consequence stated in counts, not in jargon:

   ```
   This note comes from sigh().

   Edit the motif       changes 2 occurrences, 10 notes
   Just this occurrence requires occurrence specialization  (prompt 34)
   ```

3. previews the affected notes with the selection halo while the choice is open;
4. on confirm, applies the command and reports what happened in the interface's own vocabulary:
   *"Edited sigh() — 2 occurrences updated."*

Until prompt 34 lands, the second option is present, disabled, and explains itself. It is never hidden — the user should
learn that the choice exists, and see the day it becomes available.

## 5. Restraint

Origin view spends the application's entire boldness budget. Therefore:

- It is the **only** use of `--plate` at high coverage anywhere in the app.
- No other lens, overlay, or "x-ray mode" is added without removing something.
- It never runs by default, never animates on its own, and never survives a release of the key (unless pinned).
- It does not change layout — no reflow, no insertion of annotations into the engraving. Ink and margin only.
