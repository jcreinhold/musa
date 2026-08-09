# musa

A notation-first music language and workbench: you write a piece as text, and musa compiles it into a score you can
engrave, a performance you can hear, and files other programs can read.

The source is the only document. The desktop app is a structured editor for that text — not a second place where music
lives — so anything the interface can do, the language can say, and anything the language says, the interface shows.

```musa
piece "twinkle" {
    tempo 1/4 = 104;
    meter 4/4;
    key c major;

    score {
        part piano {
            voice melody {
                c4/4 c4/4 g4/4 g4/4 a4/4 a4/4 g4/2
                rest/1
            }
        }
    }
}
```

A piece can also describe its own sound. Patches are signal chains, modulation is a typed connection to a named
parameter, and the mix is written the way it is heard:

```musa
studio {
    patch glass_pad {
        oscillator(sine)
            |> envelope(adsr(attack: 30 ms, decay: 1.8 s, sustain: 0.65, release: 3.5 s))
            |> lowpass(cutoff: 1400 Hz, q: 0.7)
            |> output;
    }

    lfo = oscillator(sine, frequency: 0.08 Hz) |> scale(250 Hz) |> bias(1400 Hz);
    modulate lfo -> glass_pad.lowpass.cutoff;

    assign violin -> glass_pad;
    route violin -> master;
}
```

## Getting started

You need a [Rust](https://rustup.rs) toolchain and [Node](https://nodejs.org) 20+.

```bash
make setup && make desktop
```

`make` on its own lists every task. The ones you will use most:

| Command | What it does |
| --- | --- |
| `make desktop` | Start the score editor (Tauri shell, hot-reloading UI) |
| `make ui` | Start the UI alone in a browser against a stubbed backend |
| `make check-file FILE=…` | Compile one `.musa` file and print its diagnostics |
| `make render FILE=… TO=…` | Render to `wav`, `midi`, `mei`, `lilypond`, `musicxml`, `performance`, or `plan` |
| `make play FILE=…` | Play a piece through the audio engine |
| `make verify` | Every gate CI runs: format, clippy, tests, types, licences |

The CLI underneath is `musa`:

```bash
cargo run -p musa-cli -- render examples/glass-mountain.musa --to lilypond -o mountain.ly
```

For editor integration there is a language server: build it once (`cargo build -p musa-lsp`) and point any
LSP-speaking editor at the `musa-lsp` binary over stdio. It speaks the session's vocabulary — diagnostics with
their certain fixes, hover that answers musically, go-to-definition through provenance, outline symbols, canonical
formatting, semantic tokens, and completion — and computes nothing of its own.

`examples/` holds the pieces the test suite compiles on every run. They are executable specifications rather than demos
— `glass-mountain.musa` exercises motifs, transposition, and the studio; `tuplet-fixture.musa` and
`profile-fixture.musa` pin down timing and dynamics.

## How it fits together

Musa is a Rust workspace. The semantic core is Rust; the interface is a replaceable projection of it.

```
language → compiler → { render, audio } → engine → project → { cli, lsp, desktop }
```

| Crate | What it owns |
| --- | --- |
| `musa-language` | tokens, parser, a lossless syntax tree, formatting, text edits |
| `musa-kernel` | the temporal kernel: exact rational time, typed occurrences, timeline/sequence/overlay |
| `musa-compiler` | name resolution, units, elaboration through the kernel, score and performance snapshots |
| `musa-render` | engraving plan, MEI, LilyPond, MusicXML, MIDI |
| `musa-audio` | the studio graph, DSP processors, offline rendering |
| `musa-engine` | audio device, transport, real-time queues, MIDI input |
| `musa-project` | the session facade: documents, revisions, commands, exports |
| `musa-cli`, `apps/musa-desktop`, `musa-lsp` | thin shells over `musa-project` |
| `editors/tree-sitter-musa` | tree-sitter grammar and editor queries; a second reader held honest by the lexer |

Dependencies point one way only, and the boundaries are load-bearing: written pitch is not a MIDI number, notated
duration is not performed duration, a voice is not a mixer track, a dynamic marking is not a number of decibels. Time is
exact rational arithmetic everywhere except the audio edge, and the audio callback never allocates or locks.

## Where the design lives

Three documents govern this repository, and the code is expected to agree with them:

- [`docs/initial-design-roadmap.md`](docs/initial-design-roadmap.md) — the architecture: layers, crate ownership,
  language design, DSP rules, and what is deliberately rejected or deferred.
- [`docs/course-correction.md`](docs/course-correction.md) — the semantic correction that makes a small temporal kernel
  the ontology and the surface language an elaboration into it. Its specification is [`docs/kernel/`](docs/kernel/).
  Where it and the roadmap disagree, it wins.
- [`docs/prompts/`](docs/prompts/) — the work plan: numbered implementation prompts executed in dependency order, each
  one commit with its own acceptance check.

[`docs/interface/`](docs/interface/) covers the desktop app's design: its states, its performance budgets, and the
reasoning behind the score editor.

## Status

Early, and honest about it. The language, compiler, kernel, notation and audio export, the studio, and the score editor
are implemented and tested; 30 of the 36 planned prompts are done. What remains is listed in `docs/prompts/` with
`status: pending` — effects and mixing, MusicXML export, MIDI entry, transforms, annotations and harmony, and imports
with continuous curves.

Contributions should follow the same rules the prompts do: `make verify` is green, one prompt is one commit, and if the
code and the design documents disagree, one of them gets repaired rather than left to drift.

Licensed under Apache-2.0. See [LICENSE](LICENSE).
