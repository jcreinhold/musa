# 05 — States and Voice

Status: **governing**.

Every screen the application can be in, and the words it uses. States that are designed late become the parts of a
program that feel unfinished, and they are disproportionately what a new user meets first.

## 1. Voice

- **Speak music, not implementation.** "Part", "voice", "bar", "motif", "occurrence" — never "node", "AST", "snapshot",
  "revision id" (except where a revision number is genuinely the information, as in the stale-playback badge).
- **Active voice, present consequence.** "Extract motif" → "Extracted `sigh`." The label of the control and the wording
  of its result are the same verb.
- **Errors do not apologize and are never vague.** They say what is wrong, where, and what would fix it. Compiler
  diagnostics are shown in the compiler's own words — the interface adds location and a way to get there, and does not
  paraphrase.
- **Empty is an invitation.** Never "No data."
- Sentence case everywhere. No exclamation marks. No emoji.

## 2. First run and empty states

| State | What is shown |
| --- | --- |
| **No piece open** | A title page for the piece that is not written yet. The leaf is present, sized as a sheet on a desk rather than stretched to the window; the app name is set in Academico in the sheet's title position; and the page's text block is **ruled with empty five-line staves** in `--rule` at the resting rastral unit (`--sp`), so the sheet reads as manuscript paper rather than as an unexplained rectangle. Beneath it on the surround, on the page's own measure and inside the page's own side margins, the ways in as a ruled list — one line each, the name at the left and the menu accelerator in mono at the right: *Open a piece* `⌘O`, *New piece* `⌘N`, *Open an example* when there is one to open. Recent pieces extend the same list when the core can supply them. Nothing but the name is placed on the sheet: on every other screen the sheet is the score and every control is margin, and the first frame is the wrong place to teach otherwise. No splash art, no hero image. |
| **New empty piece** | The leaf shows a real engraved empty system — clef, key, meter, one empty bar — because roadmap §14.8 says a new piece must be immediately playable. Caret is placed in the first voice. A single line of `--ink-muted` text under the transport: *Press a number for a duration, then a letter for a pitch.* It disappears after the first note and does not come back. |
| **Part with no voices** | The staff is drawn with a whole-bar rest and the part name greyed. Inspector offers *Add a voice*. |
| **No diagnostics** | The diagnostics pane shows nothing at all — not "0 problems". Absence is the message. |

**On the ruling, and on the arrangement.** The first version of the no-piece state took "the leaf is present but blank"
literally: a full-window-height A4 with one muted word near the top, and the actions as a row of links twenty pixels
apart on the surround below it. Both halves of that failed the same test. A sheet stretched to the height of a wide
window is not a sheet, it is a tall narrow band, and a genuinely blank one gives the eye nothing to land on; a row of
two short links at the bottom edge of that band reads as one cramped clump rather than as two choices, and it is the
full height of the window away from the only other thing on screen. The repair keeps the thesis — the paper is the only
lit surface, and no control is placed on it — and fixes the execution: the sheet is sized and the sheet is ruled, and
the list beneath shares the sheet's measure, margins and hairline weight so that page and list are one composition
rather than an object and its caption. The ruling is not ornament: it is what music paper is, it is drawn at the
engraving's own line weight and rastral unit, and it is what turns "empty is an invitation" from a rule in §1 into
something on the screen.

## 3. Loading

- **App launch → first paint**: the shell frame (margins, title, transport in a disabled state) paints immediately; the
  leaf shows a blank sheet with the correct page dimensions. No spinner. The score arrives when the worker has laid it
  out (budget: `06-performance.md`).
- **Never a full-screen loading state after launch.** Every subsequent wait happens with the previous content still on
  screen.
- If a layout exceeds 1 s (a very large score), a thin `--plate` progress hairline appears at the top edge of the leaf.
  Below 1 s nothing appears — a spinner that flashes for 300 ms is worse than no spinner.

## 4. The stale revision — the important one

Roadmap §14.7: while the source is invalid, diagnostics appear immediately, the last valid score stays visible, the last
valid playback plan stays installed, and the interface indicates that what you hear is not what you are editing.

This is a headline behavior and it gets a real design, not a badge:

- The **score does not change at all.** Not dimmed, not blurred, not overlaid. It is still correct — it is just older
  than the text.
- The **leaf's edge turns `--chalk`** (1px, the hairline that is already there), and the top margin reads, in
  `--t-small` `--chalk`:

  ```
  Showing revision 41 — the current source has 2 problems
  ```

  The revision number is included because it is genuinely the information: it tells a user who has been editing for a
  minute how far behind the page is.
- Playback keeps working, and the transport shows the same `--chalk` treatment on its position readout.
- Diagnostics appear in the source column, which **shows itself the first time** the source becomes invalid in a
  session, and thereafter respects whatever the user last chose.
- When the source compiles again, the edge returns to `--leaf-edge` and the message is removed — no success toast. A
  return to normal is not an event.

## 5. Diagnostics

- Listed in source order, each as: severity glyph, message in `--t-body`, location in `--t-value` `--ink-muted`.
- Clicking one moves the caret in the source and, when the diagnostic has a score location, flashes the corresponding
  system on the leaf once.
- The severity glyph is a shape, not just a color (`03-interaction.md` §5): a filled square for errors, hollow for
  warnings.
- Diagnostics never appear as toasts. They are not transient.

## 6. Confirmations and results

- Destructive or wide-reaching operations state their scope before they run, inline, with counts (`04-provenance.md`
  §4). Nothing musical requires a modal.
- Completed operations report in a small `--t-small` line in the top margin for 3 s, then fade: *"Exported
  glass-mountain.wav."* Undo is always the reversal path; there is no "Undo" button in the toast — `⌘Z` is the answer
  and the keyboard sheet says so.
- Long operations (WAV export of a long piece) show the same `--plate` hairline as layout, at the top of the leaf, and
  can be cancelled from the palette.

## 7. Failure

- **The core returned an error** (file unreadable, export path denied): state what failed and what to try, in the top
  margin in `--chalk`, persistent until dismissed or superseded. Never a modal alert.
- **The audio device is unavailable**: the transport disables and reads *No audio output — check your system output
  device*. Everything else in the app keeps working; a missing sound card must not block writing music.
- **The render worker died**: the last engraving stays on screen, the leaf edge goes `--chalk`, and the message offers
  *Reload the score view*. The document is never at risk — the source is on disk and in the core.
- **Unsaved work is never lost silently.** Until autosave arrives (prompt 33), a close request with unsaved edits states
  the count of edits and offers *Save*, *Discard*, *Cancel* — the only place in the application where a modal is
  correct, because the window is going away.
