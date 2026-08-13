# Work with kernel files

The temporal kernel has a text format of its own, separate from the musician-facing language. Two commands cover it.

## Print a piece as kernel text

```bash
musa kernel first.musa
musa kernel first.musa --normalized
```

The first prints the piece's elaboration in the kernel interchange syntax. `--normalized` prints the canonical normal
form: two pieces that mean the same music normalize to the same text, which makes the form useful for diffing and for
semantic hashing.

## Check a kernel file

A standalone `.musa.kernel` file begins with a version header and contains one closed kernel term:

```text
% musa-kernel-2
kernel "example" {
  composition main : EventTrack[WrittenTime, ScoreFact] =
    track 1/2 {
      occurrence "voice 0 0 note c4 1/2 [0:4]" from 0 to 1/2;
    };
}
```

Parse, check, and evaluate one:

```bash
musa kernel --check example.musa.kernel
```

A kernel file has no imports, no functions, and no free variables. It is the interchange form a second implementation
can read — not a format composers write by hand. [The temporal kernel](../concepts/temporal-kernel.md) explains what the
terms mean; the [kernel interchange format](../reference/kernel-format.md) reference gives the grammar.
