# 03 — Interaction

Status: **governing**.

Roadmap §14.5 fixes the policy: keyboard-first, deterministic, mouse for selection and inspection. This document fixes
the model that policy needs to be real.

## 1. Selection model

One model, used by the score, the parts list, the inspector, and (from prompt 26) the source.

```ts
type Selection =
  | { kind: 'none' }
  | { kind: 'caret';  part: PartId; voice: VoiceId; before: EventId | 'end' }
  | { kind: 'event';  events: EventId[] }        // one, or a contiguous range
  | { kind: 'range';  part: PartId; voice: VoiceId; from: EventId; to: EventId }
```

- **The caret is a first-class state.** Entry happens at a caret, not "after the selected note" — otherwise inserting at
  the start of a voice, or into an empty voice, has no representation. The caret renders as a `2sp` vertical hairline in
  `--plate` at the insertion point.
- **Selection is by `EventId`**, never by index or by DOM node. It survives re-engraving (`02-engraving.md` §6).
- **Active part and voice** are part of selection, not a separate mode. Selecting a note sets them.
- Multi-select is contiguous within one voice for now. Cross-voice selection is deferred until an operation needs it.

## 2. Pointer

| Gesture | Result |
| --- | --- |
| Click a note | Select it; set active part/voice; inspector fills |
| Click between notes | Place the caret |
| Shift-click | Extend to a range within the same voice |
| Click a part or voice in the left margin | Set active part/voice; caret to that voice's start |
| Click empty leaf | Clear to `none` |
| Double-click a note | Focus the inspector's first editable field |
| Drag on the leaf | Range selection **only**. Dragging never moves a note. |

Pointer feedback is immediate and local: selection is applied by the frontend from the clicked element's `xml:id` in the
same frame (**≤ 16 ms**, no round trip). The Rust round trip only *enriches* the inspector — pitch spelling, exact
duration, origin path — and may land a frame or two later. This is the one place the frontend acts before the core
answers, and it is safe because it changes nothing semantic.

## 3. Keyboard

Musa is a keyboard instrument. The full map lives in one file and is rendered into the help sheet from that file, so
documentation cannot drift from bindings.

**Navigation** (always available)

| Key | Action |
| --- | --- |
| `←` `→` | Previous / next event in the active voice |
| `↑` `↓` | Previous / next voice in the active part (staff order) |
| `⌥←` `⌥→` | Previous / next bar |
| `Home` `End` | First / last event in the voice |
| `Tab` | Next part |
| `Esc` | Clear selection; if none, hide the source column |

**Transport**

| Key | Action |
| --- | --- |
| `Space` | Play / pause |
| `⇧Space` | Play from the selection |
| `Return` | Stop and return to the start |
| `L` | Toggle loop over the selected range |
| `F` | Toggle follow (playhead scrolls the page) |

**View**

| Key | Action |
| --- | --- |
| `O` (hold) or `⌥` (hold) | Origin view (`04-provenance.md`) |
| `⌘1` `⌘2` `⌘3` `⌘4` | Compose / Sound / Mix / Source workspace |
| `⌘−` `⌘=` `⌘0` | Zoom out / in / reset |
| `⌘'` | Show or hide the source column |
| `⌘⇧F` | Format the source |
| `⌘K` | Command palette |
| `?` | Keyboard sheet |

All four workspaces exist as of prompt 31; before that `⌘2` and `⌘3` were bound to nothing and the switcher showed only
the two that were real, because a tab that opens an empty room is a promise the application cannot keep.

**Modified keys belong to the application; unmodified ones belong to the score.** Everything without `⌘` or `⌥` — the
arrows, `Space`, `F`, `L`, `⇧O` — fires only when the score pane has focus, so typing `f` in the source is an `f` and
not a follow toggle. Scope is read off the binding rather than declared per command, which is what keeps the rule from
being remembered one command at a time.

**Entry** (prompt 25 — only while entry is on)

Entry is a mode, and it has to be: the navigation map above already owns the unmodified letters, so a bare `f` cannot be
both follow and the note F. `N` says "the letters are notes now", which is the key every notation editor a musician has
used binds it to. The mode is never invisible — the duration the next note would take is drawn as its glyph in the top
margin for as long as entry is on, so the state is legible without colour (§5).

| Key | Action |
| --- | --- |
| `N` | Turn note entry on or off |
| `1` `2` `4` `8` `6` `3` | Duration: whole, half, quarter, eighth, sixteenth, thirty-second |
| `.` | Dotted or not |
| `c` `d` `e` `f` `g` `a` `b` | Write that pitch — or, with an event selected, respell it in its own octave |
| `r` | Write a rest |
| `⌘↑` `⌘↓` | Octave up / down for the notes that follow |
| `⇧↑` `⇧↓` | Sharp / natural / flat for the notes that follow |
| `Esc` | Leave entry (before it clears the selection) |

A duration key with an event selected renotates that event rather than only setting what comes next: §1's rule is that
the same gesture changes a selection where there is one and writes at the caret where there is not.

`r` and not `Space`, because `Space` plays, and a transport key that stopped playing inside a mode would be worse than a
second letter to learn. `16` and `32` take the nearest free digits — `6` and `3` — because they do not fit on one key.
Ties have no binding: the language has no tie construct until prompt 27, and a key that spells nothing is worse than a
key that is not there yet.

**Focus.** There is one focus ring: `2px --plate`, `2px` offset, `3px` radius, and it is always visible on keyboard
focus — never suppressed. Focus order is: top margin → source column → parts list → score → inspector. The score pane is
a single tab stop; arrow keys navigate *within* it (a roving tabindex over events), because 4000 tab stops is not
accessibility.

## 4. Transport and playhead

- Transport state comes from the engine via events, never from a frontend timer. The frontend interpolates *between*
  position events for smoothness (`01-visual-language.md` §6) but never predicts past the last one by more than one
  event interval; if events stop, the playhead stops.
- **Follow modes**: `off` (page stays put), `page` (the page turns when the playhead leaves the viewport — a page turn,
  not a continuous scroll), `continuous` (the page scrolls under a fixed playhead; only available in continuous layout).
  Default `page`.
- **Position readout**: `bar:beat` per `01-visual-language.md` §3, plus elapsed time in `m:ss` at `--t-small`. Both are
  tabular so digits do not jitter.
- **Loop** is a range on the timeline derived from the current selection; it renders on the leaf as a `--plate` bracket
  in the margin of the affected systems, using the repeat-sign glyphs, not a colored rectangle.
- Starting playback never moves the selection, and moving the selection never stops playback.

## 5. Accessibility floor

Not a later pass. Each item is a check in the prompt that introduces the surface.

- **Every event is focusable and named.** The accessible name is a musical sentence, built from the snapshot: *"A4
  quarter, Violin, lead, bar 3 beat 2, generated from sigh."* Blind and low-vision musicians exist, and a score reader
  that speaks musical facts is better than one that speaks `path element 4712`.
- **A live region** announces selection changes (polite) and transport changes (assertive on start/stop only, never per
  position event).
- **No pointer-only actions anywhere.** Every operation in the app has a keyboard path and appears in `⌘K`.
- **Contrast** per `01-visual-language.md` §2, verified by a test over the token file.
- **`prefers-reduced-motion`** honored per `01-visual-language.md` §6.
- **Zoom to 200 %** and OS text scaling do not clip chrome; the leaf reflows.
- Color is never the only signal: generated material in Origin view is also marked with a `⟨` bracket in the left margin
  of its system, and diagnostics carry a glyph as well as `--chalk`.

## 6. The command palette

`⌘K` opens a leaf-styled palette listing every command with its binding, sourced from the same map as the help sheet. It
is the discovery surface, which is what lets the chrome stay free of icon toolbars (`00-thesis.md` §5). Commands are
named as they resolve: **Extract motif**, **Transpose selection**, **Export WAV** — verbs, sentence case, matching the
toast that confirms them.

## 7. What the frontend may compute

Exhaustive list. Anything not here must come from `ProjectSnapshot`.

- Which element the pointer is over, and the resulting `EventId`.
- Scroll, zoom, page window, focus, hover, whether the source column is showing.
- Interpolated playhead position between engine events.
- Selection membership within a range whose endpoints came from the core.

The frontend does not compute bar numbers, beat positions, durations, pitch names, tie relationships, or anything about
provenance. If one of those is needed and absent, the snapshot gains a field.

### 7.1 The unit a span is measured in

A span is the one number the frontend and the core both index the *same* text with, so they have to agree on how that
text is measured, and by default they do not. Rust measures a string in **bytes** — `musa-language` is built on Rowan
and `text-size`, whose ranges are byte ranges, and `musa-compiler`, `musa-project`, and `musa-cli`'s `miette` labels all
carry that measure through. JavaScript measures a string in **UTF-16 code units**, and so does CodeMirror: a document
position, a decoration range, a lint range, and `selection.main.head` are all counted in them.

While the source is ASCII the two measures are the same number, which is why this was invisible until a piece had an em
dash in it. They diverge at the first character above U+007F, and the divergence accumulates: an em dash is three bytes
and one code unit, so every span after it is two too large read as a JavaScript index — a mark two characters off, a
caret in the middle of the wrong word, a gutter diagnostic underlining the wrong token. A flat sign costs two; a
character outside the basic plane costs two the other way.

**The contract: the wire is in UTF-16 code units, and Rust is in bytes.** The line is drawn at serialization, not inside
either language:

- Every span in a serialized snapshot — diagnostics, `OriginFacts`, `OccurrenceFacts`, `OutlineFacts`, studio facts, and
  anything of that shape added later — is a UTF-16 code-unit offset. `ProjectSnapshot::to_wire` is the only way to
  produce that snapshot, and `ProjectSnapshot` does not implement `Serialize`, so there is no second path that could
  skip the translation.
- Every span in `musa-project`'s Rust API stays a byte offset, because that is what applies an edit to a Rust string and
  what points a terminal diagnostic at the right column.
- A span the frontend sends *back* — a `TextEditDto`, built from a diagnostic it was shown — is in UTF-16 code units
  too, and the shell restates it in bytes with `Utf16Offsets::to_bytes` before the session applies it.
- The generated fixtures the UI is tested against go through the same translation, so what a test sees is what the
  running application sends. `examples/unicode-fixture.musa` is the piece that makes the difference observable; the
  highlighting and linking tests both run against it, and both fail if a span arrives in bytes.

The frontend therefore never converts an offset and never needs to know that bytes exist. A frontend that finds itself
counting code units has been handed the wrong number.
