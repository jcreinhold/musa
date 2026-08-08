# 01 — Visual Language

Status: **governing**.

Every value here is a design token. Tokens live in one file (`apps/musa-desktop/ui/src/lib/design/tokens.css`) as CSS
custom properties and are the only source of color, type, spacing, and duration in the application. A literal hex value,
font name, or pixel size anywhere else in the UI is a bug.

## 1. Where the palette comes from

Music was engraved on metal plates for three centuries: a plate of pewter or copper, punched and cut, inked, and pulled
onto rag paper. That workshop supplies the whole palette, and it supplies it honestly rather than as pastiche.

- The paper of a good modern edition is **not cream**. It is a bright, faintly warm white. Cream is what aged paper and
  stock photography look like.
- Engraver's ink is **not black**. It is a dense blue-black that reads warmer and deeper than `#000` at small sizes and
  avoids the harsh edge pure black gives to hairlines on an LCD.
- The plates oxidize. Copper goes **verdigris**; that green-teal is the one saturated color in the workshop, and it is
  the color musa uses for the one thing musa uniquely knows: **provenance**.
- The proofreader marks corrections on the pull in **red chalk** (sanguine). That is the only other hue, reserved
  strictly for diagnostics and the stale-revision state.

Two hues total, each with a single fixed meaning. A third hue is a design change, not a feature.

## 2. Color tokens

### Light

```css
--leaf:        #FCFCFA;  /* the score sheet — the only lit surface */
--leaf-edge:   #E2E0DA;  /* 1px hairline at the sheet boundary */
--surround:    #E8E6E1;  /* matte margin; every non-score surface */
--surround-in: #DEDCD6;  /* recessed margin (the source column's well) — used sparingly */
--ink:         #14161C;  /* notation glyphs, primary text */
--ink-muted:   #5C5F68;  /* secondary text, labels, staff-adjacent chrome */
--ink-faint:   #8E9099;  /* tertiary: hints, disabled, placeholder */
--rule:        #C9C6C0;  /* hairlines: staff-bracket, focus underline */
--plate:       #0E6B63;  /* verdigris — provenance, generated material, live */
--plate-wash:  #0E6B631F;/* generated-material fill in Origin view */
--chalk:       #A83A2B;  /* sanguine — diagnostics, stale revision */
--chalk-wash:  #A83A2B14;
```

### Dark

Dark mode is not an inversion. The leaf becomes a **graphite sheet** — the page still reads as a distinct, slightly
lifted object; it does not dissolve into the background.

```css
--leaf:        #1B1D21;
--leaf-edge:   #2C2F35;
--surround:    #101114;
--surround-in: #0A0B0D;
--ink:         #E9E6DF;  /* bone */
--ink-muted:   #9B9DA5;
--ink-faint:   #6A6C74;
--rule:        #3A3D44;
--plate:       #45B3A6;
--plate-wash:  #45B3A626;
--chalk:       #D9705E;
--chalk-wash:  #D9705E1F;
```

### Rules

- **`--ink` and `--leaf` are the notation's colors.** Verovio output is plumbed through `currentColor` (see
  `02-engraving.md` §3) so that switching theme re-inks the score rather than filtering it. Never apply a CSS
  `filter: invert()` to a score. Ever.
- **Contrast.** `--ink` and `--ink-muted` on `--surround` and on `--leaf` must hold ≥ 4.5:1; `--plate` and `--chalk`
  must hold ≥ 3:1 against both surfaces, because they carry meaning on their own. `--rule` is exempt from the 3:1 floor
  and holds ≥ 1.3:1 instead: it is a decorative hairline that matches the engraving's own line weights, and a rule dark
  enough for 3:1 against paper is a rule twice the weight of the staff lines beside it — the accessibility rule for
  non-text contrast applies to graphics that convey information, and a hairline that separates does not. Every one of
  these is checked by an automated test over the token file, not by eye.
- **`--ink-faint` is not a text color.** It is for disabled controls, placeholders, and decorative separators — things a
  reader is meant to skip. Anything a reader is meant to *read*, including the `--t-micro` labels above inspector and
  readout values, takes `--ink-muted` and therefore holds the 4.5:1 floor. A label small enough to need a magnifier is
  not a quiet label, it is an unreadable one; the automated accessibility scan (prompt 23) asserts this on the running
  screen rather than on the token file.
- **`--plate` means one thing:** derived from something else, or currently live. Generated material, the Origin trace,
  the focus ring, the source caret, the playhead hairline, the "playing" state. Nothing else. The caret belongs to this
  list on both counts — it is the focus ring of the text and the playhead of the keyboard — and it takes the hue rather
  than `--ink` for a practical reason as well: a caret set in the colour of the body text around it is a two-pixel
  hairline that has to be hunted for, which on a full screen of source is the difference between knowing where you are
  editing and guessing.
- **`--chalk` means one thing:** something the compiler is telling you is wrong, or that what you are hearing is not
  what you are looking at. Nothing else. It never appears as a brand accent.
- **Printing** uses the light tokens with `--surround` dropped entirely; the leaf becomes the page.

## 3. Typography

Four families, four roles, all OFL, all bundled — no network font requests, ever (roadmap §14.8: zero setup, and the app
must be fully functional offline).

| Role | Family | Why this one |
| --- | --- | --- |
| Notation | **Bravura** (SMuFL) | The SMuFL reference font; what Verovio engraves with. Not negotiable. |
| Display / score text | **Academico** | It *is* the score's text font — the face Verovio sets titles, tempo marks, and expressions in. The application's display face is borrowed from the page rather than imposed on it, so a piece title in the app frame and the same title on the printed page are the same object. |
| UI | **Instrument Sans** | Slightly condensed, tall x-height, crisp at 12–14px, unfussy. Quiet enough to keep the chrome unlit and the page loud. |
| Mono | **Recursive Mono Linear** (`MONO 1, CASL 0, CRSV 0`) | Musa source, event ids, expansion paths, exact values. Real tabular figures, unmistakable `0`/`O` and `1`/`l`, no code ligatures — musa's `/` is a fraction bar, not an operator to be prettified. |

**The Academico risk, stated plainly.** A serif display face on a paper-toned surface is one step away from the generic
"warm cream + high-contrast serif" look. Academico survives the test on three grounds: it is a sturdy bookface, not a
high-contrast display serif; the surround is a cool matte gray, not cream; and the choice is derived from the subject
(the score's own typography) rather than picked for mood. If Academico ever appears at display size on the surround
rather than on the leaf, that justification has been lost — pull it back.

### Scale

Fixed steps. Chrome does not use sizes between them.

```css
--t-micro: 10px/1.4  Instrument Sans 500, +0.08em tracking, uppercase  /* field labels only */
--t-small: 12px/1.5  Instrument Sans 400                               /* secondary chrome */
--t-body:  13px/1.5  Instrument Sans 400                               /* chrome default */
--t-value: 13px/1.5  Recursive Mono Linear 400, tabular                /* every exact value */
--t-name:  15px/1.3  Academico 400                                     /* part names, section names */
--t-title: 24px/1.2  Academico 400                                     /* piece title, empty states */
--t-large: 34px/1.0  Recursive Mono Linear 300, tabular                /* transport bar number */
```

### The scale is read through one multiplier

Every size above is stated as `calc(<its px> * var(--type-scale, 1))`, and the composer chooses the multiplier from four
steps: **Small** 0.85, **Normal** 1, **Large** 1.15, **Larger** 1.3. The steps stay fixed and discrete and the ratios
between them are preserved, so Large is this same typographic system read from further away rather than a different one.

It is the **frame's** type, and never the score's. The score's size is zoom, which is a re-layout with its own control,
its own binding, and `02-engraving.md` §5; a text-size preference that also grew the staves would be a second zoom that
disagrees with the first. A larger frame leaves the leaf less room and the page is fitted to what is left — the same
thing that happens when the window is resized, and not a change to the engraving.

The source measure is stated in `ch` (§8), so it scales with the type and its arithmetic still holds: larger type simply
reaches §7's 300px floor at a wider window, which is the behaviour that rule was written for.

### Two typographic rules that carry meaning

1. **Exact time is always set as a real fraction, never as a decimal.** `7/8`, not `0.875`. A `<Fraction>` primitive
   sets numerator and denominator around a true diagonal bar in mono, sized to sit on the text baseline. Musa's time is
   rational; the interface says so on every screen it appears.
2. **Positions are `bar:beat` with the bar dominant.** `4:2` is set with the bar in `--t-large` and the beat one step
   down and `--ink-muted`, because a reader scans for the bar. A bare tick counter never appears in the UI.

## 4. Space

Chrome uses a 4px scale: `4 8 12 16 24 32 48 64`. Nothing else.

Anything drawn **on the leaf** — selection halos, the playhead, Origin traces — is measured in **staff spaces**
(`--sp`), the rastral unit Verovio laid the page out with, exposed by the render layer. A selection halo is `0.6sp`, not
`5px`, so it stays correct at every zoom and every staff size. This is the one place the interface adopts the score's
own measurement system, and it is the reason overlays never look pasted on.

## 5. Elevation and shape

- **One shadow in the application**, on the leaf: `0 1px 2px #0000000A, 0 8px 24px #00000010` (light);
  `0 1px 2px #00000059, 0 10px 30px #00000040` (dark). Nothing else is elevated. Popovers and the command palette use
  the leaf treatment (leaf fill + `--leaf-edge` hairline + the leaf shadow) because they are, conceptually, small
  sheets.
- **Radius:** `0` on the leaf and on anything that reads as paper. `3px` on interactive chrome affordances (a focused
  field's hit area, a toggle). No pill shapes, no capsule buttons.
- **Hairlines are 1 physical device pixel**, not 1 CSS pixel — `--rule` strokes are drawn at `1/dpr` where the platform
  allows, so the parts bracket and staff-adjacent rules match the engraving's line weights instead of doubling them.

## 6. Motion

Motion exists for exactly three reasons. Anything else is removed.

| Moment | Spec |
| --- | --- |
| **Score re-engraving** | The new SVG cross-fades over the old at **90 ms, `ease-out`**, with scroll anchored to a stable element id. The page appears to correct itself, like ink settling. It must never blank, flash white, or jump. This is a correctness requirement, not a flourish — see `06-performance.md`. |
| **Playhead** | Interpolated on `requestAnimationFrame` between the engine's ~10 Hz position events, `linear`, no easing. A playhead that steps is worse than no playhead. |
| **Origin view** | 120 ms cross-fade in and out. The trace line draws instantly; it does not animate along its path. |

Durations: `--m-fast: 90ms`, `--m-base: 120ms`, `--m-slow: 200ms` (showing or hiding the source column only). Easing:
`--e-out: cubic-bezier(0.2, 0, 0, 1)`.

Under `prefers-reduced-motion: reduce`: the cross-fade becomes an instant swap (scroll anchoring still applies), the
playhead advances discretely per position event, and Origin view toggles instantly. No feature is withheld — only the
interpolation.

## 7. Layout: the leaf and the margin

The Compose workspace, as an arrangement of the elements roadmap §14.3 requires — not as boxes.

```
   ┌ surround ─────────────────────────────────────────────────────────────┐
   │                                                                       │
   │  Glass Mountain            ▶  ■  𝄆        4:2      ♩= 72     A minor  │   top margin, no bar, no borders
   │  ·······································································│
   │ Source      Hide │                                                    │
   │                  │   ┌──────────────────────────────┐                 │
   │ score {          │   │                              │  Pitch          │
   │   part violin {  │◄─┐│    the leaf: engraved        │  A4             │   right margin:
   │     voice lead { │  ││    Verovio SVG, one          │                 │   typographic rows,
   │       use sigh();│  ││    soft shadow, hairline     │  Duration       │   no input boxes
   │     }            │  ││    edge                      │  1/4            │
   │   }              │  ││                              │                 │
   │ }                │  ││                              │  Origin         │
   │                  │  │└──────────────────────────────┘  sigh() ▸ 3     │
   │ ■ 2 problems     │  └── parts list, bracketed, right-aligned to the leaf
   └───────────────────────────────────────────────────────────────────────┘
     the source column: one measure, full height, shown on ⌘'
```

- **Top margin** (48px minimum, one row where one row holds it): piece title in Academico at the left, with the
  workspace switcher beside it; transport, position, tempo, and key at the right. No background fill, no bottom border.
  The leaf's shadow provides the separation. It is a band of rows rather than a bar — see below.
- **Left margin**: the parts list, right-aligned toward the leaf and connected by a **real staff bracket** drawn in
  `--rule` — the same brace/bracket grouping the score itself uses at the left edge of every system. Part names in
  Academico; voice names one step down in `--ink-muted`. The list is a miniature of the score's own left margin, which
  is why it needs no header and no box.
- **Right margin**: the inspector, as a run of rows — label in `--t-micro`, value in `--t-value` or `--t-name`. Editable
  values carry a hairline underline in three weights — `--rule` at rest, `--ink-muted` on hover, `--plate` on focus —
  and values that only report carry none, which is what makes the mark a signal rather than decoration. There are no
  field boxes at rest, and the rest weight is the reason there need not be: a hairline is not a box, and a control
  nobody can see is not an affordance. With nothing selected the inspector's rows are *the piece* — every statement its
  header can carry, the unnamed ones included, which is how a composer discovers that a piece can name an arranger at
  all (prompt 54). A field is as wide as its value and no wider — the hairline has to sit under the value it belongs to
  — and **a value too long for the column wraps**, the way the band takes another row rather than clipping. `© 2026.
  Licensed CC BY-` is not a shorter statement of the same thing, and a field that shows it is lying about what the
  document says.
- **Source column**: source and diagnostics, at the *left edge*, full height, on `--surround-in` behind one `--rule`
  hairline — the same two materials and the same seam §8 gives the Source workspace. Closed by default in Compose; shown
  with `⌘'`, from the View menu, or from the palette. Its width is the **source measure** (§8), fixed: it does not grow
  with its text and it is not draggable, because a pane that resized itself would move the page under the reader on
  every keystroke. The source scrolls inside it. Problems list beneath it, in the same column.

**The top margin is a band of rows, not a bar.** 48px is its minimum, not its height. What it carries has grown past
what one row holds at the window's own minimum width — the transport, note entry, the Origin pin, the position and the
score's facts, the view mode, the zoom, and whatever the interface currently has to say — and a fixed-height row does
not clip what will not fit, it prints it on top of what is already there. A piece title struck through a key signature
is a worse header than a header two lines tall. So the band wraps, by these rules:

- **Groups break whole.** The workspace switcher, the transport, the view toggle, the zoom, and the position readout
  each move to the next row entire. "Zoom out · 100 % · Zoom in" split across two rows is not a zoom control any more.
- **The title gives way first.** It is the one thing in the band whose length nobody controls, so it sets on one line
  and truncates. A control never gives way to a long name.
- **The controls keep the right edge**, on a row of their own exactly as on a shared one, so they are found in the same
  place at every width.
- **The seam falls between identity and controls.** Where two rows are needed, the first says where you are — title,
  save state, workspace — and the second what you can do there. Where the interface has something to say, its line takes
  a row of its own rather than competing with either.

This is also what makes 200 % browser zoom reflow instead of collide, which WCAG asks for and a fixed-height header
cannot give.

**Why the source is a column and not a drawer.** It was a drawer: a band across the bottom, a fixed third of the window
tall. Two things were wrong with that, and both were visible the moment it was opened on a real piece.

The first is shape. Musa source is a *narrow, tall* thing — half its lines are under 24 characters and 99 in 100 are
under 52, because the language is a tree of short statements. A drawer is a *wide, short* box. Putting one in the other
left a thin column of text at the left and roughly a thousand pixels of nothing beside it, while showing ten lines of a
sixty-line file. The pane was simultaneously too wide to fill and too short to read.

The second is which dimension it spent. The leaf is a portrait page, so in page view the binding constraint is height —
and a bottom drawer takes exactly that, while the surround to the left and right of the leaf sits empty. Moving the same
source to a column trades the dimension the page does not need for the one it does: at 1440 × 900 the page is *larger*
beside a column of text than it was above a band of it, and the text shows three times as many lines.

So the source takes width, which Compose has spare, and gives back height, which it does not — and it lands in the same
place, on the same material, behind the same hairline as in the Source workspace. `⌘1` and `⌘4` become one layout at two
proportions rather than two layouts: what `⌘4` does is clear the score's margins away and let the page have the room,
not rearrange the screen.

Responsive behavior:

- Below 1100px the inspector collapses to a toggled overlay in the *right margin only*.
- Below 1100px the position readout stops printing the score's facts — tempo, key, and meter. They belong to the piece
  and are engraved at the head of the page in the score's own hand, while the position belongs to where you are working
  and is printed nowhere else. This is the band's **only** drop, and what it drops is *reporting the page already does*
  — never a control. A control that disappears at a window size or a zoom level is a control someone cannot reach; when
  there is not room, the band takes another row instead.
- Below 840px the parts list collapses to a single active-part readout.
- The source column keeps its measure while the window can hold it, the two margins, and a leaf wide enough to be an
  upright page. Below that — roughly 1250px — the column gives up characters rather than the page giving up its shape,
  down to a floor of 300px, which is still wider than nine lines in ten. The page is the subject of this workspace; when
  something has to yield, it is the text.
- Where the margins stop being margins and become sections above and below the leaf, the source becomes the first of
  them and takes the full width: a window that narrow genuinely is a stack.
- The leaf never shrinks below a legible staff size — the window scrolls instead.

## 8. Layout: the Source workspace

Roadmap §14.4's second workspace, and the one place the text is the subject rather than the evidence. Text left, page
right — the same order and the same seam as the source column in Compose (§7). What this workspace changes is not the
arrangement but the proportion: the score's margins retire and the page takes everything the text does not.

**The source measure.** The text column is set to a measure, not to a fraction of the window. `48ch` of Recursive Mono
Linear, plus the line-number gutter and the pane's own padding — about 455px at the resting type size.

The number is the language's, not a preference. Across every file in `examples/`, ignoring comments: half of all lines
are 23 characters or shorter, nineteen in twenty fit in 37, and the longest is a studio chain at 93. `48ch` clears the
nineteen-in-twenty case with eleven characters of headroom, and still lets the longest ordinary lines reach for the edge
— which is what stops a column of type from reading as a column of margin. The few that run past it are comment prose
and that one chain; they scroll, as they would in any editor.

A fraction cannot do this. `5fr / 7fr` gave the text 660px on a 1440px window — 200px of which no line ever reached, a
band of empty paper down the right of every file — and would have starved it on a 900px one. A measure is right at every
width, and what varies is the page, which is the thing that can actually use the room.

- The text sits on `--surround-in` and the page on `--surround`, so the two halves read as two materials rather than two
  panels. One `--rule` hairline between them; no card, no shadow, no tab strip.
- **The source is set, not dumped** (§3): Recursive Mono Linear, no ligatures, `1.65` line height, four-space indent —
  the formatter's own, so a file the editor indented and a file the formatter wrote are the same file.
- **Ink weight does the work.** Keywords in `--ink` at 500; pitches and durations in `--ink`; identifiers, numbers,
  units, strings, comments and punctuation in `--ink-muted`, with strings, units and comments italic. `use` is the one
  accent — `--plate`, because it is the generated-material site, and the same hue means the same thing in the text as on
  the page (`04-provenance.md` §2). Nothing else is coloured.
- **Provenance is `--plate-wash` behind the text**, the same wash the halo uses — and inside the mark the text returns
  to `--ink`. The hue is the ground there, so setting the type in it as well is the same colour twice, and it reads at
  3.9:1. Marked text is the text the screen is about; the darkest ink is what it wants anyway.
- **Diagnostics** are the compiler's own: a `--chalk` underline on the span, the message on hover, and the list beneath
  the text, each entry a place to go rather than a notification (`05-states.md` §5).
- **Problems** appear beneath the source or not at all. With none, the pane shows nothing — not "0 problems".
