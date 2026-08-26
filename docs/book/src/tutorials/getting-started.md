# Getting started

This lesson takes you from a fresh checkout to a rendered score and a played piece. It assumes a
[Rust](https://rustup.rs) toolchain and [Node](https://nodejs.org) 20 or later.

## Install

```bash
git clone https://github.com/jcreinhold/musa.git
cd musa
make setup
```

`make setup` installs everything the repository needs. `make` on its own lists every task.

## Write a first piece

Create `first.musa`:

```musa
piece "first" {
    tempo 1/4 = 104;
    meter 4/4;
    key c major;

    score {
        part piano {
            voice melody {
                | c4/4 c4/4 g4/4 g4/4
                | a4/4 a4/4 g4/2
                | rest/1
            }
        }
    }
}
```

Read the notes aloud: `c4/4` is middle C for a quarter note, `g4/2` the G above it for a half, and `rest/1` a whole
measure of rest. `|` begins a checked measure. Durations are exact fractions of a whole note. [First
pieces](../guide/first-pieces.md) takes this apart properly; this lesson only gets it playing.

## Check it

```bash
make check-file FILE=first.musa
```

A clean compile reports the file as `ok`. A mistake prints a diagnostic at its source location; try deleting a `/4` and
run the check again.

## Render it

```bash
make render FILE=first.musa TO=lilypond OUT=first.ly
```

The targets are `mei`, `lilypond`, `musicxml`, `midi`, and `wav`. The same piece renders to all of them because the
backends share one semantic core.

## Hear it

```bash
make play FILE=first.musa
```

Playback runs through the built-in audio engine. Without a `studio` block the parts sound with a default instrument;
[Write for the studio](../how-to/studio.md) shows how to give a piece its own sound.

## Where to go next

- [First pieces](../guide/first-pieces.md) — notes, bars, voices, parts, and reusable phrases, worked through against
  the repository's own fixtures. Start here; the four chapters after it build on it.
- [The language](../reference/language.md) — the full surface syntax.
- `examples/` in the repository — pieces the test suite compiles on every run. `glass-mountain.musa` exercises motifs,
  transposition, and the studio; `annotated.musa` shows phrases, sections, and chord symbols.
- [The event-track](../concepts/event-track.md) — what your piece means once it compiles.
