# Architecture

Musa is a Rust workspace. The semantic core is Rust; the interface is a replaceable projection of it.

```text
language → compiler → { render, audio } → engine → project → { cli, lsp, desktop }
```

Dependencies point one way only. No dependency points upward.

| Crate | What it owns |
| --- | --- |
| `musa-language` | Tokens, parser, a lossless syntax tree, formatting, text edits |
| `musa-kernel` | The temporal kernel: exact rational time, typed occurrences, timeline/sequence/overlay |
| `musa-compiler` | Name resolution, units, elaboration through the kernel, score and performance snapshots |
| `musa-render` | The engraving plan; MEI, LilyPond, MusicXML, and MIDI export |
| `musa-audio` | The studio graph, DSP processors, offline rendering |
| `musa-engine` | The audio device, transport, real-time queues, MIDI input |
| `musa-project` | The session facade: documents, revisions, commands, exports |
| `musa`, `musa-lsp`, `apps/musa-desktop` | Thin shells over `musa-project` |
| `editors/tree-sitter-musa` | A tree-sitter grammar held honest by the real lexer |

## Why the shells are thin

Every shell — the CLI, the language server, the desktop app — calls `musa-project` and nothing below it. A command is
argument parsing, one session call, and printing. This is what keeps the answers identical: the editor's diagnostics,
the CLI's check, and the app's score all come from one session over one compiler.

## The boundaries are load-bearing

Types from implementation libraries never cross crate boundaries. Parser internals stay in `musa-language`, DSP
internals in `musa-audio`, device types in `musa-engine`. Public facades are narrow: `parse`, `compile`,
`render_notation`, `compile_graph`, `AudioEngine`, `ProjectSession`. A consumer that needs something the facade does not
offer is evidence the facade is missing a feature, not a reason to reach around it.

## Real-time separation

The audio callback never allocates, locks, does I/O, logs, or destroys large objects. Render plans are preallocated on
the control side and cross the boundary on lock-free queues. This is why the studio can compile a new graph while the
old one keeps playing.

The design documents in the repository — `docs/roadmap.md`, `docs/kernel/`, and `docs/governance/` — give the reasoning
behind each boundary.
