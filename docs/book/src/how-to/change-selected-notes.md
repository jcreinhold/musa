# Change notes you have selected

Numbers, arrows, and intervals act on a selection. There is no "next note" state hiding behind them: with nothing
selected, these keys do nothing at all.

## Select

| To select | Do |
| --- | --- |
| One note | Click it |
| A range | Drag, or `⇧←` / `⇧→` |
| Another voice | `↑` / `↓` |
| Notes across voices | Lasso them |

The selection is always visible and always announced. `Esc` clears it.

## Set a duration

With a selection and the score focused, press a number:

| Key | Duration |
| --- | --- |
| `1` | whole |
| `2` | half |
| `4` | quarter |
| `8` | eighth |
| `6` | sixteenth |
| `3` | thirty-second |

This *sets*, it does not scale: `4` says "these are quarter notes". Multiplying the rhythm that is already there by an
exact ratio is a separate command in the compiler, because it is a different musical question — not a mode a number key
could be in.

## Move pitches

| Keys | What it does |
| --- | --- |
| `⌥↑` / `⌥↓` | Move the notehead one diatonic step, leaving the sign alone |
| `⌥⇧↑` / `⌥⇧↓` | Move the sign, leaving the notehead alone |
| `T` | Transpose by an interval you type, in the language's own spelling — `P5`, `up A4` |

Respelling and transposing are two commands because they are two different marks on the page. An interval the language
cannot read comes back as a refusal you can read, not a field that silently does nothing.

## Read the preview before it happens

Every one of these commands previews first, in the inspector:

```text
transpose the selection by P5.
6 notes change, and 2 notes are left alone.
melody, bar 3: c4/4 g4/4 → g4/4 d5/4.
```

The counts and the bars are the compiler's answer about the exact source it would write. **Accept** commits that and
nothing else; **Cancel** writes nothing.

## When the selection is generated music

If any selected note came from expanding a motif, the preview says which motif and what it would write instead, across
how many occurrences. You then choose:

- **Accept** — change the motif, and therefore every occurrence; or
- **Just these occurrences** — write the change onto the calls, leaving the motif alone.

A call that runs more than once cannot be specialized, and the app says so rather than changing more than you asked.

## One command, one undo

Whatever the command changed, `⌘Z` puts it back in one step. Mixed selections — notes and rests, tuplets, ties, several
voices — are disclosed in the preview rather than partly changed.
