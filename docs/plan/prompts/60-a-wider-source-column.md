---
id: 60
slug: a-wider-source-column
status: done
depends_on: [26, 55, 59]
phase: 2
---

# A Wider Source Column: the Seam Becomes a Control

## Task

The hairline between the source column and the page is the most obvious control in the window and does nothing. Make it
a real one: drag it to widen the column, drag it back to narrow it, double-click it to return to the measure. The width
the composer chooses is theirs and comes back next launch.

Nothing about the *default* changes. The column still opens at the source measure, and it still never resizes itself.

## Read

- `docs/rules/desktop/01-visual-language.md` §7 (the source column, whose "not draggable" sentence this prompt repairs)
  and §8 (the source measure — why `48ch`, and why not a fraction of the window).
- `docs/rules/desktop/03-interaction.md` §2 (the 4px threshold, and the rule that every drag has a key first) and §5
  (the accessibility floor).
- Prompt 55's `Preferences` — the one preference mechanism this app has, which the width joins.

## Design

### A default the composer may override is not a default that moved

§8's argument for `48ch` is about what the language's lines actually measure, and it is an argument about where the
column should *start*. The sentence that followed it — that the column "is not draggable, because a pane that resized
itself would move the page under the reader on every keystroke" — answers a different question. A pane that grows with
its text is the application having an opinion every time a line gets longer; a pane the composer drags once is the
composer overriding a default. The first is what the reason forbids and stays forbidden. The second is what a composer
with a 32-inch display and a studio chain 93 characters long reasonably wants, and there was no way to ask for it.

So the repair to §7 keeps the reason and narrows what it forbids. The measure is what the column opens at and what
double-clicking returns it to; it is no longer what the column is stuck at.

### The seam is a separator, and the ARIA pattern is the pattern

One component, `Seam.svelte`, used by both screens that show text — Compose and the Source workspace — because
`01-visual-language.md` §7 and §8 describe one seam and there should be one of it.

It is `role="separator"` with `aria-orientation="vertical"`, `tabindex="0"`, and an accessible name that says what it
sizes. `aria-valuenow` is the column's measured width in pixels, so what a screen reader announces is what is on screen
rather than what was asked for.

| Gesture | Result |
| --- | --- |
| Drag the seam | Widen or narrow the source column |
| Double-click the seam | Back to the source measure |
| `←` `→` (seam focused) | Narrow / widen by 16px |
| `⇧←` `⇧→` (seam focused) | Narrow / widen by 96px |
| `Home` `End` (seam focused) | The narrowest / the widest the body allows |

Arrows are what makes the drag legal under WCAG 2.5.7 (`03-interaction.md` §2: a pointer gesture adds a second way to
reach a capability, never a capability). The seam is chrome and is only reachable by `Tab`, so its unmodified arrows do
not collide with the score's — the scope rule in §3 holds without an exception.

The seam takes no space of its own. It is a 9px transparent strip laid over the column's existing `--rule` hairline, and
it paints that hairline in the inspector's three weights: `--rule` at rest, `--ink-muted` on hover, `--plate` on focus
(§7). No handle, no grip dots, no widening bar — the hairline already is the affordance, and this is the whole of its
feedback.

### Two guards, and only one of them governs an ask

- **The layout's cap**, `--source-cap`, unchanged and still in CSS: `min(measure, cap)`. It exists because the *default*
  is the column asking for room nobody granted it, and on a window too small to spare that room the page must win.
- **The composer's bound**, three fifths of the body, measured from the body element: no page should ever be a minority
  of the screen. It is applied where the drag lands *and* at every render, because the window resizes without anybody
  dragging and a column widened on a large display would otherwise leave nothing on a laptop.

A composer who drags the seam has granted the room the cap was protecting, so an explicit width **replaces** the capped
expression rather than being clamped by it — `width: var(--asked, min(measure, cap))`. What is stored is the ask itself,
never the bounded result, which is what lets a width chosen on a large display come back whole.

This is the one thing the implementation had to be taught. Clamping the ask by the cap as well looked conservative and
was simply a broken control: the cap reserves the two margins and a stage wide enough for an upright leaf, so on a
1440px window it stops at 580px and the seam refuses to move past it for no reason the composer can see.

### The width is a preference

`musa.source-width`, beside `musa.theme`, `musa.text-size`, and `musa.vim`, restored by the same `start()` — one
preference mechanism, as `preferences.svelte.ts` says. `null` means the measure, and double-clicking the seam stores
`null` rather than the measure's pixel value, so a column left at the default follows the measure when the type size
changes instead of freezing at the pixels the measure happened to be.

It does not appear in the Settings sheet. The sheet is for preferences with no other home; this one is set by dragging
the thing it is about, which is a better control than a number field would be.

## Deliver

- `docs/rules/desktop/01-visual-language.md` — §7's source-column paragraph repaired, as above. Committed first.
- `apps/musa-desktop/ui`:
  - `src/lib/ui/Seam.svelte` — the separator: drag, double-click, keys, and the three-weight hairline.
  - `src/lib/ui/SourcePane.svelte` — takes a width and renders `min(asked, cap)`; the measure is the fallback.
  - `src/screens/Compose.svelte`, `src/screens/Source.svelte` — the seam, between the column and what it borders.
  - `src/lib/session/preferences.svelte.ts` — `sourceWidth`, `widenSource`, `resetSource`, restored in `start()`.
- Tests:
  - unit: the width is clamped to the bound at both ends; `resetSource` stores nothing rather than a number; a stored
    width that is not a number is ignored, the way a stored text size that is not a step already is.
  - Playwright: dragging the seam widens the column; double-clicking returns it to the measure; the arrow keys move it
    without the pointer; the width outlives a reload; the seam is announced as a separator with a value, and the axe
    scan is clean.

## Check

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm run check && npm run test
```

Commit as `Let the composer widen the source column`.

## Found along the way

- **The seam lives inside the column, not between the halves.** A separator standing between two landmarks is page
  content belonging to neither, and axe says so — `region`, "all page content should be contained by landmarks". So
  `SourcePane` grows its own seam on its right edge, absolutely positioned over the hairline that was already there. The
  layout is unchanged, the component tree is simpler, and the screens pass callbacks rather than placing a sibling.
- **A `$effect` that reads what it writes is an effect Svelte drops.** An early version measured with
  `width = pane ? … : width`, which made the effect depend on `width` and silently stopped updating — the separator
  announced a number from before the drag. Splitting the write from the read fixed it; measuring by `bind:clientWidth`
  and handing the number down removed the need for either.
- **`ResizeObserver` and `requestAnimationFrame` do not run in a hidden tab.** Half an hour went into a phantom
  "`ResizeObserver` stops delivering in this app" — it was the preview browser being backgrounded. Worth knowing before
  the next person concludes the same thing: check `document.visibilityState` first.

## Stop

- No second draggable seam. The margins are not panels and do not resize (§7).
- No width in the project file. This is the app's state, not the document's (`05-states.md`).
- No auto-sizing, ever: not to the longest line, not on open, not after a format. That is the thing §8's reason forbids,
  and this prompt does not touch it.
- No collapse-by-drag. `⌘'` hides the column and the drag has a floor; a seam dragged to zero is a column a composer
  cannot find again.
