# Work with events files

The event-track has a text format of its own, separate from the musician-facing language. Two commands cover it.

## Print a piece as events text

```bash
musa events first.musa
musa events first.musa --normalized
```

The first prints the piece's elaboration in the events interchange syntax. `--normalized` prints the canonical normal
form: two pieces that mean the same music normalize to the same text, which makes the form useful for diffing and for
semantic hashing.

## Check an events file

A standalone `.musa.events` file begins with a version header and contains one closed event-track term:

```text
% musa-events-3
events "example" {
  composition main : EventTrack[WrittenTime, ScoreFact] =
    track 1/2 {
      occurrence "voice 0 0 note c4 1/2 [0:4]" from 0 to 1/2;
    };
}
```

Parse, check, and evaluate one:

```bash
musa events --check example.musa.events
```

An events file has no imports, no functions, and no free variables. It is the interchange form a second implementation
can read — not a format composers write by hand. [The event-track](../concepts/event-track.md) explains what the terms
mean; the [events interchange format](../reference/events-format.md) reference gives the grammar.
