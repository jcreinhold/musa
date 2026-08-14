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
