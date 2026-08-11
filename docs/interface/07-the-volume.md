# 07 — The Volume

Status: **governing** (graduated at prompt 85).

Roadmap §16 fixes what a project *is*: a `musa.toml` beside `pieces/`, `library/` and `assets/`, and the rule that the
simplest project is one file. Prompt 84 made the project the unit a composer opens. This document fixes what that looks
like — what is shown, where, in which faces, and when nothing is shown at all.

## 1. It is a volume, not a file tree

A file tree is the wrong object here, and not by a small margin. §16 fixes the shape of a project, so disclosure
triangles would model a freedom the format does not have while burying the one thing that matters: the order the pieces
go in. A file browser can tell you `02-waltz.musa` exists. It cannot tell you the piece is called *Waltz*, that it is
second, or that the library open beside it is one this piece actually imports.

A bound volume already has both devices this needs:

- a **contents page** — the running order, with what each piece is called;
- an **editorial note at the foot** — the material the pieces rest on.

That is the whole design. Everything below is how it is set.

## 2. The typographic rule

One rule carries the information design of every row in this document:

> **What the composer wrote is set in Academico. What the filesystem knows is set in mono.**

The title and the file name, two faces, one line. This is `01-visual-language.md` §3's existing assignment applied —
Academico is the score's own text face, mono is exact values and the machine's spelling — and not a new idiom. **No new
tokens.** A page that needs a colour or a size `tokens.css` does not have is a page whose design is wrong.

## 3. The contents page (`⌘0`)

On the leaf, on the page's own margins, reached by `⌘0` and by the **Contents** entry in the workspace switcher — which
is the number *before* the four workspaces because the volume comes before the piece.

- The frame's heading is the volume's name, because the frame names what is open and here what is open is the volume —
  the same rule Compose follows when it sets the piece's title there. The word *Contents* belongs to the switcher, which
  is the one place that says where you can go.
- The volume's name in the title position in `--f-score-text`; the composer beneath in `--ink-muted`.
- The running order, one ruled line per piece, on the page's own hairline: the position numeral in Academico with
  tabular figures in `--ink-faint`; the title in `--t-name` `--ink`; the file name at the right in `--f-mono`
  `--t-small` `--ink-faint`.
- The piece in hand carries `aria-current="page"` and is set in ink and underlined. **Never colour alone**
  (`03-interaction.md` §5).
- A piece with edits not on disk reads **edited** in `--t-micro` after its file name. A word, not a dot: a mark you have
  to be taught says nothing the first time it is seen.

Numbering is earned here rather than decorative. This is an album's running order, and the files are literally named
`01-` and `02-`; the numeral carries information the reader needs. Material is **not** numbered, because material has no
position.

### Material, as apparatus

Beneath the running order, after the last piece's own hairline and a full step of space — no second rule, because two
rules with nothing between them read as an empty entry — at the foot of the text block where an editorial note sits and
one type step down: **Material**. The libraries, listed the same way, and one the current piece imports reads **in
use**.

`in use` is the one fact on this page a file browser could not produce: it is the current piece's own transitive import
set, computed by the compiler. Setting material as apparatus, at apparatus size, in the apparatus position, is this
document's one deliberate risk — and a true one. A library is not a smaller kind of piece. It is a different kind of
thing: it declares, and it does not sound.

## 4. The margin

The left margin reads outside in: **Contents → Parts → Outline** — volume, piece, structure. The running order's rows
are `Outline.svelte`'s rows unchanged: right-aligned, `--ink-muted` → `--ink` on hover → `--plate` when current, the
position numeral in the slot where the outline puts a bar number, and separated from the parts below by
`margin-top: var(--s-8)` — a rest rather than a rule.

The margin lists the **pieces**. Material is reached from the contents page, because material is where you go
deliberately.

## 5. With one file, nothing appears

**A project of one shows no contents anywhere.** No page, no margin section, no entry in the switcher, no `⌘0` that
leads to a page with one line on it. A loose `.musa` file must be indistinguishable from what it was before projects
existed, and that is the restraint this design spends its budget on.

"More than one" counts pieces and material together: a single piece with one library beside it is a volume, because
there is something to choose between.

## 6. Two rules that follow from `DocumentKind`

- **Material opens in the text.** Choosing a library shows the Source workspace, because material has no page. This is a
  fact about the file, so it overrides which workspace was last asked for.
- **A file with no score is not a blank window.** A piece that has never compiled keeps the frame — its name, its
  margins, its diagnostics — and the leaf says so in a line, where the first system would be.

## 7. Turning to another piece

A piece turned away from keeps its text, its unsaved edits and its undo history, so turning back is *turning back*
rather than reopening. The one thing it gives up is the audio device: only the piece in hand may sound.

A new score means the selection is let go of (`05-states.md` §9): the selection, the loop, the marks and any pending
choice all name events in the piece that was on screen, and carried into the next one they name nothing.

## 8. The words

*Open a piece* `⌘O`, *Open a project* `⇧⌘O`, *New piece* `⌘N` — three ways in, on Launch and in the File menu. *Save
every piece* `⌘⌥S`. **Contents**, **Material**, **edited**, **in use**. Sentence case, no badges, no counts.

## 9. What this document rejects

- **No quick-open palette entry.** A piece is not a command, and the contents page and the margin list already answer
  "get me to that piece".
- **No tabs.** The contents page and the margin list are the switcher; a tab strip would be a third.
- **No file creation, renaming, deletion, or drag-reordering from the page.** It prints the running order; §16 says the
  manifest and the filesystem set it.
- **No preview, count, or duration for a piece that is not open** — every one of those needs a piece compiled that
  nobody asked to compile.
- **No recent-projects list, and no persisted last-opened project.**
