---
id: 85
slug: the-contents-page
status: done
depends_on: [84, 59, 76]
phase: 2
---

# The Contents Page

## Task

Give the project a page. Prompt 84 made the project the unit and left the interface still showing one piece with no
way to reach another; this prompt prints the running order — as the front matter of a bound volume, on the leaf, in
the score's own text face — and puts a quiet copy of it at the top of the left margin so a composer can turn to
another piece without leaving the one they are writing. It also fixes the window that is currently blank: a file with
no score renders nothing at all today, and material has no score by definition.

## Read

- `docs/interface/01-visual-language.md` §3 — the four faces and their four jobs. This prompt extends one existing
  assignment and invents nothing: Academico is the score's own text face, mono is exact values and the machine's
  spelling.
- `docs/interface/05-states.md` §2 — the no-piece state, and the argument behind it: the sheet is sized and ruled,
  the ways in share the sheet's measure and margins, and nothing but the name is placed on the paper. The contents
  page is that composition with real content in it.
- `apps/musa-desktop/ui/src/screens/Outline.svelte` — the left-margin list idiom, exactly: right-aligned buttons,
  `--ink-muted` → `--ink` on hover → `--plate` when current, a coordinate in the trailing slot, and separation from
  the list above by `margin-top: var(--s-8)` — "a rest rather than a rule".
- Prompt 59 — the one registry: a command's words and its binding come from `registry.rs`, and
  `tests/unit/commands.test.ts` is what stops the menu and the palette from disagreeing.
- Prompt 84 — `ContentsFacts` and `EntryFacts`, and the rule that every string in them was decided in Rust.

## Design

### It is a volume, not a file tree

A file tree is the wrong object. §16 fixes the shape of a project — a manifest, `pieces/`, `library/` — so disclosure
triangles would model a freedom the format does not have while burying the one thing that matters, which is the order
the pieces go in. A printed volume already has the two devices this needs: a **contents page** with a running order,
and an **editorial note at the foot** listing the shared material. Both say something a file browser cannot.

One typographic rule carries the information design of every row in this prompt:

> **What the composer wrote is set in Academico. What the filesystem knows is set in mono.**

Title and file name, two faces, one line. No new tokens; `tokens.css` is not touched by this prompt.

### Contents (`⌘0`)

On the leaf, on the page's own margins. The volume's name in the title position in `--f-score-text`, the composer
beneath in `--ink-muted`. Then the running order, one ruled line per piece: the position numeral in Academico with
tabular figures in `--ink-faint`, the title in `--t-name` `--ink`, the file name at the right in `--f-mono`
`--t-small` `--ink-faint`. The piece in hand carries `aria-current="page"` and is set in ink and underlined — never
colour alone (`03-interaction.md` §5). A piece with edits not on disk reads **edited** in `--t-micro` after its file
name: a word in the application's own vocabulary, because a dot is a thing you have to be taught.

Beneath, after one hairline, at the foot of the text block where an editorial note sits and one type step down:
**Material**. The libraries, listed the same way, and one the current piece imports reads **in use**.

Material is set as apparatus, at apparatus size, in the apparatus position, because that is what it is: a library
declares and does not sound. That is this prompt's one deliberate risk, and everything around it reuses an existing
idiom unchanged.

### The margin

The left margin becomes **Contents → Parts → Outline** — volume, piece, structure, read from the outside in. The
rows are `Outline.svelte`'s rows, with the position numeral in the slot where the outline puts a bar number.

**With one entry, nothing appears.** No section in the margin, no `⌘0`, no menu item that leads to a page with one
line on it. A loose `.musa` file must be indistinguishable from what it is today, and that is the restraint this
prompt spends its budget on.

### Two rules that fall out of prompt 84's `DocumentKind`

- **Material opens in the text.** Choosing a library shows the Source workspace, because material has no page.
- **A file with no score is not a blank window.** `Compose.svelte` guards its entire template on `{#if snapshot &&
  score}` with no `{:else}`, so a document that parses and yields no score renders *nothing* — no title, no
  transport, no diagnostics. With `kind` on the snapshot the routing is decidable: material goes to Source, and a
  piece that has never compiled keeps the frame and shows its diagnostics.

### Words

*Open a project* `⇧⌘O`, beside *Open a piece* `⌘O` on Launch and in the File menu. *Save all*. **Contents**,
**Material**, **edited**, **in use**. Sentence case, no badges, no counts.

## Target

- `docs/interface/07-the-volume.md`, **governing**: the two-face rule, the contents page, the margin section, the
  hide-when-one rule, material-opens-in-the-text, and the words above. `00-*.md`'s index gains it.
- `apps/musa-desktop/ui/src/screens/Contents.svelte` and `screens/RunningOrder.svelte`, both driven by
  `snapshot.contents` and neither composing a string.
- `Screen` gains `"contents"`; `Workspaces.svelte`'s list and `App.svelte`'s render chain follow.
- `Compose.svelte`'s missing `{:else}`; `Launch.svelte`'s third way in; `commands/map.ts` following `registry.rs`.
- `apps/musa-desktop/ui/tests/screens/shell.ts`: the second album piece as a seed, `show_piece`, `saveAll`.
- `docs/interface/06-performance.md` gains **B12**, and `tests/screens/perf.spec.ts` asserts it.
- `apps/musa-desktop/ui/tests/screens/contents.spec.ts`.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
UPDATE_UI_FIXTURES=1 cargo test -p musa-project -p musa-desktop   # fixtures + commands.json
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit && npx playwright test
```

`contents.spec.ts` asserts: a project of one shows no contents anywhere and offers no `⌘0`; the album's running order
is the manifest's, with titles from the pieces and file names beside them; choosing a piece draws it and lets the
selection go (`05-states.md` §9's rule — event identity is a position in a score); edits survive a turn away and back
and the row says **edited**; a library row opens the Source workspace; **in use** marks exactly the current piece's
imports.

**B12** — choosing a piece already opened this session → its page drawn, ≤ 400 ms, previous page visible throughout.
Measured apart from B11 because that is a re-reading of one score and this is a different score; a piece opened for
the first time is B7's cold number by construction and is not asserted here.

## Stop

- No `⌘P` quick-open in the palette. A piece is not a command, and two ways to reach one is one too many.
- No tabs. The contents page and the margin list are the switcher; a tab strip would be a third.
- No file creation, renaming, deletion, or drag-reordering from the page. It prints the running order; §16 says the
  manifest and the filesystem set it.
- No `assets/` browser, no preview of a piece that is not open, no counts or durations in the running order — every
  one of those needs a piece compiled that nobody asked to compile.
- No recent-projects list on Launch, and no persisted last-opened project.
- No new design tokens. If this page needs a colour or a size that `tokens.css` does not have, the design is wrong.
