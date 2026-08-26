# Format source

`musa format` owns layout: indentation, statement-per-line, blank lines, comment attachment. A hand that adjusts them is
wasting itself.

```bash
musa format first.musa           # rewrite in place
musa format first.musa --check   # fail if the file is not canonical; write nothing
musa format first.musa --diff    # print the diff; write nothing
```

Use `--check` in CI and pre-commit hooks. Use `--diff` when you want to see what the formatter would change before
letting it.

A comma-separated list — a call's arguments, a constructor's fields, a list literal — is written on one line when it
fits and one item per line when it does not. Its trailing comma follows that decision, so it is not yours to keep in
step: a list written down the page is given one whether or not you typed it, and a list on one line has it taken away.
That is the only token `musa format` writes on your behalf, and it is there so that adding an item to a stacked list is
a new line rather than an edit to the line above it. A product — `(a, b)` — is not a list in this sense: its arity is
part of its type, so it never takes one.

Everything layout cannot say — naming, repetition, redundant markings — belongs to the
[style guide](../concepts/style-guide.md), and the machine-checkable part of it is enforced by
[lints](../reference/lints.md), not the formatter.

## Insert bar assertions explicitly

`--insert-bars` is a separate semantic source action. It asks the checked score where the scoped meter proves each
measure begins, adds `|` only at direct source-item boundaries, and then formats the result. Without this flag,
`musa format` never adds a bar assertion.

```musa
voice melody {
    c4/4 d4/4 e4/4 f4/4
    g4/4 a4/4 b4/4 c5/4
}
```

```bash
musa format piece.musa --insert-bars
```

```musa
voice melody {
    | c4/4 d4/4 e4/4 f4/4
    | g4/4 a4/4 b4/4 c5/4
}
```

Preview it with `--insert-bars --diff`, or gate it without writing with `--insert-bars --check`. Musa refuses the whole
action when certainty would require changing musical material. For example, a direct two-measure use has a boundary
inside one source item:

```musa
motif long() { c4/1 d4/1 }
voice melody { use long(); }
```

It reports `a direct source item crosses a required barline and cannot be split automatically`. Musa does not open the
motif occurrence, split a note, invent a tie, or call an incomplete opening a pickup.
