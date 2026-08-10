# CLI

`musa` is a thin command line over the project session. Build it with `cargo build -p musa`, or run it as
`cargo run -p musa -- <command>`.

## Commands

```text
musa check <file.musa>… [--fix]         every problem in a piece, with its place
    --fix                                  apply warnings' certain fixes in place
musa explain <code>                     the rule behind a diagnostic code
musa format [<file|folder>…] [--check]  format in place; no path means here
    --check                              fail instead of writing
    --diff                               print the diff instead of writing
musa render <file.musa> --to <target>   mei | lilypond | musicxml | midi | wav
    --to plan | performance              the debug dumps, to stdout
    --mode score | performance           for --to midi (default: score)
    -o <path>                            where to write it (`-` for stdout)
musa play <file.musa> [--loop]          live playback through the audio engine
musa kernel <file.musa> [--normalized]  print the piece as kernel interchange text
musa kernel --check <file.musa.kernel>  parse, check, and evaluate kernel text
--seed <n>  on check, render and kernel: which performance to compile
```

## Notes

- `format` takes files and folders, and with no path at all it formats the folder it was run in — the way `ruff format`,
  `black` and `cargo fmt` work. A folder is walked whole; hidden folders, `target`, and symbolic links are passed over.
  A file that does not parse is reported and left exactly as it was, because the formatter would otherwise write a guess
  over text its author is in the middle of.
- Text render targets accept `-o -` for stdout; binary targets (`midi`, `wav`) require `-o <path>`.

## `.musaignore`

Some files are shaped the way they are on purpose — a generator writes them and a test compares them byte for byte, or a
diagnostic's snapshot pins their byte positions. A `.musaignore` names what a `musa format` walk passes over. The
nearest one at or above the folder being walked governs it, and it wins outright rather than merging with the lists
above it, so the answer to "is this file excluded?" is in one file. What it passed over is counted in the run's summary
rather than skipped in silence.

```text
# comments and blank lines are skipped
tests/fixtures/         a path, and everything under it
*.generated.musa        `*` matches inside one path segment
examples/**/draft.musa  `**` matches across separators
build                   with no `/`, a name matched at any depth
```

Two stated departures from `.gitignore`: a trailing `/` reads as documentation and is not a folders-only assertion, and
there is no negation. What matches a folder matches everything in it.

The list governs what a walk *finds*, which is where a file gets rewritten by accident. A file named on the command line
is formatted whatever the list says.
- `--seed` selects a performance reading when a piece carries more than one. It applies to `check`, `render`, and
  `kernel` — the three commands that compile.
- Exit codes follow the convention: success is silent, failure prints diagnostics to stderr.

## Make targets

The repository Makefile names the common invocations:

| Command | Equivalent |
| --- | --- |
| `make check-file FILE=…` | `musa check` |
| `make render FILE=… TO=… OUT=…` | `musa render --to … -o …` |
| `make play FILE=…` | `musa play` |
| `make verify` | Every gate CI runs: format, clippy, tests, types, licences |
