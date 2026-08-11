# Check and fix a piece

`musa check` compiles a piece and prints every problem with its place:

```bash
musa check first.musa
musa check examples/*.musa        # several files in one run
```

Diagnostics are errors or warnings. Warnings carry *certain fixes* — repairs the compiler knows are safe. Apply them in
place:

```bash
musa check first.musa --fix
```

Every diagnostic has a code. Ask for the rule behind one:

```bash
musa explain redundant-marking
```

The lint codes and their waivers are listed under [Lint codes](../reference/lints.md).

A piece can have more than one reading of its performance (a `.performance` sidecar next to the source). Pick which one
to compile:

```bash
musa check first.musa --seed 3
```
