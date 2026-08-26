# Lint codes

The lints enforce the machine-checkable subset of the [style guide](../concepts/style-guide.md). Each fires as a warning
with its place; `musa explain <code>` prints the rule behind one.

| Code | Fires when |
| --- | --- |
| `unused-material` | A `motif` or `fragment` is declared and never used |
| `unassigned-instrument` | An `instrument` is wired but no `sound` or expert `assign` connects a part to it |
| `redundant-marking` | A tempo, meter, or key marking states the value already in force |
| `copied-bars` | Three or more identical bars appear in one voice |

One related diagnostic is an error, not a lint: a gradual tempo change with a number and a distance but no destination
does not compile (`this gradual tempo change goes nowhere`). Lints warn on what compiles; that spelling does not.

## Waiving a lint

A waiver is written in the source, directly above the construct it waives:

```musa
// musa:allow(unused-material) — kept for the B section, which is not written yet
motif answer() {
    g4/4 a4/4
}
```

`musa:allow` takes one or more comma-separated codes and suppresses exactly those codes on the construct whose leading
comment it is — nothing else, and nothing further away. A waiver with no code is no waiver. There is no project-level
switch and no configuration file: the source is canonical, and a standard that can be switched off silently is a rumour
of a standard.
