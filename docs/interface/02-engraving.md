# 02 — Engraving

Status: **governing**.

The score is what the user looks at for hours. Verovio is capable of genuinely good engraving; the difference between
good and bad output is almost entirely in how it is driven, themed, scaled, and re-rendered. This document is that
"how". It is the reason prompt 22 exists as its own prompt rather than as a bullet in the shell prompt.

## 1. Ownership

Rust owns MEI (prompt 13, `musa-render`). The frontend owns nothing musical. The render layer converts
`ProjectSnapshot.mei` → SVG and hands back a page index and an id map. It is a projection, not a model.

The render layer's public surface to the rest of the UI is deliberately narrow:

```ts
interface Engraver {
  load(mei: string, revision: number): Promise<Layout>;   // page count, staff-space unit, id index
  page(n: number, opts: PageOptions): Promise<PageSvg>;   // one page of SVG
  relayout(opts: LayoutOptions): Promise<Layout>;         // zoom / breaks / staff size change
  locate(eventId: string): Promise<PagePoint | null>;     // for scroll-to and playhead anchoring
}
```

No component outside this module touches a Verovio toolkit, an MEI string, or a raw SVG string.

## 2. The worker rule

**Verovio never runs on the main thread.** The toolkit is instantiated once inside a dedicated Web Worker and kept
alive; `loadData` is called once per revision, `renderToSVG` per page. Layout of a 100-bar orchestral score takes
hundreds of milliseconds — on the main thread that is a frozen window, a dropped playhead, and dropped keystrokes.

Consequences that must be designed for, not discovered:

- Every request carries a **generation token** (revision + layout epoch). Results whose token is stale are dropped
  silently. Without this, a fast typist sees pages arrive out of order.
- The worker is **cancel-aware**: a new `load` supersedes in-flight page renders.
- The WASM binary is bundled, not fetched. Cold start is paid once at app launch, behind the shell's own first paint.

## 3. Theming: `currentColor`, never `filter`

Both themes need real re-inking, and Origin view needs to re-ink *part* of the page without touching the rest.

Verovio 6 emits no per-element `fill`, but it does set `color="black"` on the `.definition-scale` element and lets
everything inherit from there. That single attribute is what defeats theming. The render layer runs a **documented
sanitizer** over emitted SVG that:

1. **removes** `color="black"` from `.definition-scale`, so the CSS cascade governs;
2. removes the `font-family="Times, serif"` fallback, so score text takes Academico from the token system;
3. strips `xlink` (use `svgRemoveXlink`);
4. preserves every `xml:id` verbatim — the id map is prompt 13's contract with the editor and must survive intact;
5. leaves genuinely semantic colors (if any are ever introduced by the MEI) alone, matched by an explicit allowlist.

The CSS side matters as much as the sanitizer:

```css
#score       { color: var(--ink); }
#score g     { fill: currentColor; stroke: currentColor; }  /* re-resolved per group */
```

`currentColor` on an inherited property resolves **where it is declared** and inherits as a resolved color. Declaring it
once on the root would freeze every glyph at the root's ink and make Origin view impossible; declaring it per `g` lets
any group that sets its own `color` — a generated-note group under the lens — re-ink correctly. Verified in
`prototype.html`.

`filter: invert()` on a score is forbidden. It ruins glyph weight, breaks colored annotation, and prints wrong.

## 4. Layout options

Defaults, fixed here so they are a decision rather than an accident:

```
font:              "Bravura"
breaks:            "auto"          // "encoded" once the language expresses system breaks; "none" for continuous view
pageWidth/Height:  derived from the leaf's box at the current zoom, in Verovio units
adjustPageHeight:  true in continuous view, false in page view
svgViewBox:        true            // the leaf scales without rasterizing
svgRemoveXlink:    true
svgHtml5:          false
justifyVertically: true
unit:              11              // rastral: half a staff space in 1/10 mm. Verovio's default 9 is an A4
                                   // engraving size and is too small to read on a display; 11 is the
                                   // on-screen default, and staff size stays a separate preference (§5)
spacingStaff:      8               // slightly open — musa scores are read on screen, not printed at A4
spacingSystem:     10
spacingNonLinear:  0.55
spacingLinear:     0.25
header:            "auto"          // the page head Verovio draws from the MEI `<meiHead>`: title and subtitle
                                   // centred, composer and arranger to the right, a running head after page 1
footer:            "encoded"       // only the `<pgFoot>` the MEI backend writes — the piece's copyright line.
                                   // Verovio's automatic footer is its own credit and is not a fact about
                                   // this piece, so a piece that claims no copyright gets no footer at all
mnumInterval:      0               // one measure number at the head of each system — Verovio counts a repeat
                                   // interval, so 0 is per-system and any n > 0 is a number every n bars
```

**The page is an edition, not a run of staves** (prompt 51). The head, the foot, the measure numbers, the part labels
in `<staffDef>`, and the final thin-thick barline are all things musa *names* and Verovio *places*. Nothing above says
where any of them sits; the moment musa answers that, musa owns page layout forever.

**Page view is the default** — a leaf, per the thesis. Continuous (single system, horizontal scroll) is an explicit
mode, valuable while writing a single line, and it is what the playhead-follow mode prefers.

## 5. Zoom

Zoom is a **re-layout**, not a CSS transform. A transform scales hairlines and beams nonlinearly and destroys the
optical weight that makes engraving readable; it is the single most common reason score apps look like PDF viewers.

- Discrete steps: 50 / 75 / 90 / 100 / 125 / 150 / 200 %. Keyboard `⌘−` / `⌘=` / `⌘0`.
- During a continuous gesture (trackpad pinch), a transient CSS transform gives immediate feedback; when the gesture
  settles (120 ms idle) the layer performs a true re-layout at the nearest step and drops the transform.
- Staff size (rastral) is a separate, remembered preference from zoom. Zoom changes how much of the page you see; staff
  size changes the engraving.

## 6. Re-render without disruption

This is the quality floor that separates a tool a composer can work in from one they abandon. When the source changes:

1. The old SVG **stays on screen** and stays interactive.
2. The new layout is computed in the worker.
3. Scroll is anchored to a **stable element id** — the id of the first event visible at the top of the viewport before
   the re-render — not to a pixel offset. After the swap, that event is restored to the same viewport position. Re-flow
   caused by an inserted note must not teleport the reader.
4. Selection is restored by `EventId`, not by index. If the selected event no longer exists, selection moves to its
   nearest surviving neighbor in the same voice and the inspector says so.
5. The swap is a 90 ms cross-fade (`01-visual-language.md` §6). No white frame, no layout collapse, no scrollbar jump.
6. If the new source does not compile, **nothing in the score pane changes at all** — the last valid engraving stays
   exactly as it was, and the stale-revision state is expressed in the margins (`05-states.md` §4).

## 7. Long scores

Pages are rendered on demand: the current page ± 1, driven by an `IntersectionObserver` over page placeholders whose
dimensions come from the layout result. Off-window pages are removed from the DOM and their SVG dropped. A 300-page
score must not hold 300 SVG trees.

Placeholders reserve exact space, so the scrollbar is honest from the first layout and does not grow as pages arrive.

## 8. Overlays

Selection, hover, playhead, and Origin traces are drawn in a **single SVG overlay layer** positioned over the page, in
the page's own coordinate system, measured in staff spaces (`--sp`). They are never drawn by mutating Verovio's SVG —
mutating it fights the next re-render and corrupts the id map.

- **Selection**: a `0.6sp` rounded halo in `--plate` at 18% fill, plus a `1px` `--plate` stroke.
- **Hover**: the same halo at 8% fill, no stroke. Hover never moves anything.
- **Playhead**: a full-system hairline in `--ink` at 24%, plus the currently sounding notes tinted `--plate`. The
  sounding-note tint is the primary signal — that is what a musician actually reads — and the hairline is for precision.
- **Origin trace**: see `04-provenance.md`.

## 9. Regression testing

Engraving quality without a regression net decays within three prompts. The net:

- **Structure snapshots** (`insta`-style, in the UI test suite): the sanitized SVG for each fixture page, so an option
  change or sanitizer regression is visible in a diff.
- **Raster goldens**: rasterize fixture pages (via `resvg`) at 2× and compare to committed PNGs with a small per-pixel
  tolerance. Fixtures: `glass-mountain.musa` (multi-part, motif expansion), `counterpoint.musa` (dense two- voice),
  `twinkle.musa` (single line — the small-score case where bad spacing is most visible).
- **Both themes** are rasterized; the dark golden proves `currentColor` plumbing rather than a filter.
- **Stability test**: apply an edit to a fixture, re-render, and assert the anchor event's viewport y-offset moved by
  less than 2 px and that no frame between the two renders was empty.
