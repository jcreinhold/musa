---
id: 55
slug: reading-preferences
status: done
depends_on: [26]
phase: 2
---

# How the Composer Reads and Types

## Task

Two things the app currently decides on the composer's behalf: how large its text is, and how its text editor behaves.
The frame and the source column are set at a fixed 13 px, which is a decision made for whoever's eyesight chose it; the
source column is modeless, which is a decision made for everyone who is not a vim user. Both become choices, both
persist, and neither touches the score.

## Read

- `docs/interface/01-visual-language.md` §3 (the scale, and the four faces it is built from), §7 (the responsive rules
  the scale has to survive), §8 (the source measure — stated in `ch`, so it scales with the type and the arithmetic
  still holds).
- `docs/interface/02-engraving.md` §5 — zoom is a re-layout of the **score**. Text size is the **frame**. An app with
  two zooms that fight is an app where neither is trusted.
- `docs/interface/03-interaction.md` §3 (bindings), §5 (the accessibility floor, which this is partly how musa meets),
  §6 (the palette).
- `docs/interface/05-states.md` — a preference is a state of the *app*, not of the document. It never appears in the
  file, in a revision, or in the undo history.
- `session/theme.svelte.ts` — the preference mechanism that already exists. There should not be a second one.

## Design

### 1. Text size

One root multiplier, four steps:

| Step | `--type-scale` |
| --- | --- |
| Small | 0.85 |
| Normal | 1 |
| Large | 1.15 |
| Larger | 1.3 |

Every `--t-*-size` token becomes `calc(<its px> * var(--type-scale))`. Nothing else changes: the steps stay fixed and
discrete (§3 — "chrome does not use sizes between them"), and the ratios between them are preserved, so Large is the
same typographic system read from further away rather than a different one.

- **The frame and the source column, not the score.** The score's size is zoom, it is a re-layout, and it already has a
  control, a keybinding, and a spec section. A text-size preference that also grew the staves would be a second zoom
  that disagrees with the first.
- **§7's responsive rules survive it** because the source measure is stated in `ch`: larger type makes a wider column,
  and the page-first cap already decides who yields when the window cannot hold both. Larger type at 1100 px simply
  reaches the 300 px floor sooner, which is the behaviour the rule was written for.
- **Persisted the way the theme is** — `localStorage`, stamped on the root, restored on start. One preference mechanism,
  extended; not a second one beside it.
- Commands `view.text.larger`, `view.text.smaller`, `view.text.reset`, in the View group, in the palette, in the native
  menu, on `CmdOrCtrl+Alt+=`, `CmdOrCtrl+Alt+-`, `CmdOrCtrl+Alt+0` — deliberately *not* `⌘=`/`⌘-`, which are the score's
  zoom and must stay the score's zoom.

### 2. Vim mode in the source column

`@replit/codemirror-vim`, off by default, persisted with the text size, toggled from the View menu and the palette.

Nothing about the language, the score, the command model, or the document changes: the keymap lives inside the one
CodeMirror instance and nowhere else. What makes this worth its dependency is that the source column is a real text
editor and musa's thesis is that the source is canonical — an editor a vim user cannot type in makes the canonical
artifact the one that is least pleasant to touch.

**The mode says which mode it is in.** The package's own status line is turned on — `--NORMAL--`, `--INSERT--`, and the
line `:` is typed into — restyled to the tokens like every other piece of chrome. A modal editor that does not say what
mode it is in is the one thing worse than a modeless one, and `:w` needs somewhere to be typed; this is the package's
default surface and nothing beyond it. It appears only while vim mode is on, so the source column a composer who never
asked for vim reads is unchanged.

**The three conflicts, named, because each of them is a way to get this wrong.**

- **`Esc`.** The app's `Esc` clears the selection, or — with nothing selected — puts the source column away. In vim it is
  the most-pressed key there is, and an editor that cannot leave insert mode is not an editor. So: while vim mode is on
  *and* the source column has focus, `Esc` is vim's. Everywhere else it is unchanged. That is the one binding the mode
  moves, and `03-interaction.md` §3 gains the row.
- **`u` and `⌘Z` must be the same undo.** `SourceEditor` deliberately keeps no history of its own, because ⌘Z is the
  project's undo over revisions and two stacks over one document disagree about what the document is. Vim's `u` and
  `⌃r` are therefore rebound to `session.undo()` and `session.redo()`. A vim mode that quietly reintroduced a second
  history would be a worse bug than not having vim mode.
- **`:w` saves the project.** One save, one path, the same command the menu runs. `:q` does nothing: closing a document
  from inside its text is a footgun, and the window already has a close.

The score-scope bindings need no new rule — `scopeFor` already keeps `n`, `f`, and the arrows from firing while a text
field has focus, which is exactly what vim needs.

### 3. What is not a preference

No font choice, no colour customization, no theme beyond the light/dark that exists, no layout options, no configurable
keymap, no settings window. The two preferences here are here because each answers a fixed decision the app was making
badly for somebody; a preferences surface that grows past what can be answered that way is a design that gave up.

## Target

- `apps/musa-desktop/ui`:
  - `src/lib/session/preferences.svelte.ts` — text size and vim mode, shaped like `ThemeChoice`, persisted and restored
    together.
  - `src/lib/design/tokens.css` — every `--t-*-size` through `--type-scale`.
  - `src/lib/ui/SourceEditor.svelte` — the vim compartment, `Esc` scoping, `u`/`⌃r`/`:w` bound to the session.
  - `src/lib/commands/map.ts` and `src-tauri/src/registry.rs` — four commands (three sizes, one vim toggle).
  - `package.json` — `@replit/codemirror-vim`.
- Tests:
  - unit: the preference persists, restores, and clamps to the four steps; an unknown stored value falls back to Normal.
  - Playwright: a step changes the frame's type and leaves the engraving alone; vim mode takes `i` then `Esc` without
    the source column closing; `u` moves the *project* revision back; `:w` reaches the project's save; the mode and the
    size survive a reload; with vim off, `Esc` still puts the column away.

    "Leaves the engraving alone" is asserted as a **ratio** — one staff's height over its page's — and not as a pixel
    height, because a larger frame legitimately leaves the leaf less room and the page is fitted to what is left. That
    is the same thing a window resize does. What must not change is the engraving inside the page, and a staff taking a
    larger share of it is exactly what a second zoom would look like.
  - a golden of the frame at Larger, 1440, both themes.
- Docs: `01-visual-language.md` §3 gains the scale and its steps; `03-interaction.md` §3 gains the four commands and the
  `Esc` row; `05-states.md` gains the one-line statement that preferences are app state and never reach the document.

## Check

```sh
cd apps/musa-desktop/ui && npm run check && npm run test
cargo nextest run -p musa-desktop
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Commit as `Let the composer choose the text size and the editor`.

## Stop

- The score's size stays zoom's business. No preference touches the rastral unit.
- No settings window, no preferences pane, no per-document settings.
- No second undo history, in vim mode or out of it.
- No emacs mode, no configurable keymap, no vim plugin surface beyond the package's own defaults.
- No font, colour, or layout customization.
