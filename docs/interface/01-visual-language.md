# 01 — Visual Language

Status: **candidate**.

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
--surround-in: #DEDCD6;  /* recessed margin (drawer well) — used sparingly */
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
  the focus ring, the playhead hairline, the "playing" state. Nothing else.
- **`--chalk` means one thing:** something the compiler is telling you is wrong, or that what you are hearing is not what
  you are looking at. Nothing else. It never appears as a brand accent.
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

**The Academico risk, stated plainly.** A serif display face on a paper-toned surface is one step away from the
generic "warm cream + high-contrast serif" look. Academico survives the test on three grounds: it is a sturdy bookface,
not a high-contrast display serif; the surround is a cool matte gray, not cream; and the choice is derived from the
subject (the score's own typography) rather than picked for mood. If Academico ever appears at display size on the
surround rather than on the leaf, that justification has been lost — pull it back.

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
  the leaf treatment (leaf fill + `--leaf-edge` hairline + the leaf shadow) because they are, conceptually, small sheets.
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

Durations: `--m-fast: 90ms`, `--m-base: 120ms`, `--m-slow: 200ms` (drawer open only). Easing: `--e-out:
cubic-bezier(0.2, 0, 0, 1)`.

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
   │                    ┌───────────────────────────────────┐              │
   │            Violin ─┐│                                   │  Pitch      │
   │                    ││        the leaf: engraved         │  A4         │   right margin:
   │             lead  ─┤│        Verovio SVG, one           │             │   typographic rows,
   │                    ││        soft shadow, hairline      │  Duration   │   no input boxes
   │           Strings ─┐│        edge                       │  1/4        │
   │            upper  ─┤│                                   │             │
   │             bass  ─┘│                                   │  Origin     │
   │                    └───────────────────────────────────┘  sigh() ▸ 3  │
   │                                                                       │
   │  ╌╌╌╌╌ source / diagnostics drawer: slides up, pushes the leaf ╌╌╌╌╌╌ │
   └───────────────────────────────────────────────────────────────────────┘
```

- **Top margin** (48px): piece title in Academico at the left; transport, position, tempo, and key at the right. No
  background fill, no bottom border. The leaf's shadow provides the separation.
- **Left margin**: the parts list, right-aligned toward the leaf and connected by a **real staff bracket** drawn in
  `--rule` — the same brace/bracket grouping the score itself uses at the left edge of every system. Part names in
  Academico; voice names one step down in `--ink-muted`. The list is a miniature of the score's own left margin, which
  is why it needs no header and no box.
- **Right margin**: the inspector, as a run of rows — label in `--t-micro`, value in `--t-value` or `--t-name`. Editable
  values show a `--rule` underline on hover and a `--plate` underline on focus. There are no field boxes at rest.
- **Bottom drawer**: source and diagnostics. It slides up from the surround and **pushes the leaf**; it never overlaps
  it. Closed by default in Compose.

Responsive behavior: below 1100px the inspector collapses to a toggled overlay in the *right margin only*; below 840px
the parts list collapses to a single active-part readout. The leaf never shrinks below a legible staff size — the window
scrolls instead.
