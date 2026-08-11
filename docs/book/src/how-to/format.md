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

Everything layout cannot say — naming, repetition, redundant markings — belongs to the
[style guide](../concepts/style-guide.md), and the machine-checkable part of it is enforced by
[lints](../reference/lints.md), not the formatter.
