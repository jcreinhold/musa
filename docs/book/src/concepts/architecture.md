# Architecture

Musa is a Rust workspace. The semantic core is Rust; the interface is a replaceable projection of it.

```text
language → compiler → { render, audio } → engine → project → { cli, lsp, desktop }
```

Dependencies point one way only. No dependency points upward.

| Crate | What it owns |
| --- | --- |
| `musa-syntax` | Tokens, parser, a lossless syntax tree, formatting, text edits |
| `musa-events` | The event-track: exact rational time, typed occurrences, track/follow/together |
| `musa-calculus` | The dependent term core, kernel rechecker, and bidirectional elaboration boundary |
| `musa-score` | Musical values, score snapshots, exact performed gestures, diagnostics, provenance, and analysis |
| `musa-compiler` | Name resolution, imports, units, expansion, and elaboration through the event track |
| `musa-notation` | The engraving plan; MEI, LilyPond, MusicXML, and MIDI export |
| `musa-dsp` | Checked scheduling, opaque one-frame audio preparation, DSP processors, offline rendering |
| `musa-playback` | The audio device, transport, real-time queues, MIDI input |
| `musa-project` | The session facade: documents, revisions, commands, exports |
| `musa`, `musa-lsp`, `apps/musa-desktop` | Thin shells over `musa-project` |
| `editors/tree-sitter-musa` | A tree-sitter grammar held honest by the real lexer |

## Why the shells are thin

Every shell — the CLI, the language server, the desktop app — calls `musa-project` and nothing below it. A command is
argument parsing, one session call, and printing. This is what keeps the answers identical: the editor's diagnostics,
the CLI's check, and the app's score all come from one session over one compiler.

## The boundaries are load-bearing

Types from implementation libraries never cross crate boundaries. Parser internals stay in `musa-syntax`; dependent
terms, values, evaluation, quotation, and unification stay in `musa-calculus`; DSP internals stay in `musa-dsp`; and
device types stay in `musa-playback`. Public facades are narrow: `parse`, `compile`, `render_notation`, `prepare_audio`,
`AudioEngine`, `ProjectSession`. A consumer that needs something the facade does not offer is evidence the facade is
missing a feature, not a reason to reach around it.

## Real-time separation

The audio callback never allocates, locks, does I/O, logs, or destroys large objects. Prepared audio state is allocated
on the control side and cross the boundary on lock-free queues. This is why the studio can prepare a new machine while
the old one keeps playing.

## What the core cutover now guarantees

The production runtime path is checked source → exact event track → exact performed gestures → checked frame schedule →
opaque prepared audio → repeated one-frame step. MIDI reads the exact gestures directly and chooses ticks only at its
edge; it does not create an audio schedule. Offline and live audio repeat the same step, so host buffer size is not part
of musical meaning.

The conformance result is conditional on registered builtin/primitive contracts and canonical payload encodings. It does
not prove a real-time deadline, decide behavioral equality of arbitrary machines, choose a musical theory, or make
performance and transcription unique. Those limits are part of the result, not work hidden by a green test suite.

The design documents in the repository — `docs/plan/roadmap.md`, `docs/rules/events/`, and `docs/rules/` — give the
reasoning behind each boundary.
