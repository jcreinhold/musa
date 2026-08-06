# AGENTS.md — musa

Musa is a notation-first music language and workbench: a `.musa` source language, a provenance-preserving compiler,
MEI/LilyPond/MusicXML/MIDI export, a small built-in DSP studio, and a Tauri/Svelte desktop score editor. Rust workspace;
the semantic core is Rust, the UI is a replaceable projection.

## The two documents that govern this repo

1. **`docs/initial-design-roadmap.md`** — the architecture. Semantic layers, crate ownership, language design, DSP
   rules, and what is explicitly rejected or deferred. Read the cited sections before changing anything structural.
2. **`docs/prompts/`** — the work plan. 26 numbered feature prompts with their own README defining prompt anatomy and
   execution rules. Implementation happens by executing prompts in dependency order (see the `prompt-stack` skill).

If code and these documents disagree, either the code is wrong or the document needs a deliberate repair — never let
them drift silently.

## Navigation

| Path | What lives there |
| --- | --- |
| `crates/musa-language` | tokens, lexer, parser, lossless CST, formatter, text edits |
| `crates/musa-compiler` | resolution, units, compositional model, expansion, score/performance snapshots |
| `crates/musa-render` | NotationPlan, MEI, LilyPond, MusicXML, MIDI export |
| `crates/musa-audio` | studio graph spec→render-plan compiler, processors, offline rendering |
| `crates/musa-engine` | CPAL stream, transport, real-time queues, MIDI input |
| `crates/musa-project` | ProjectSession facade: documents, revisions, commands, exports |
| `crates/musa-cli` | thin CLI over musa-project |
| `apps/musa-desktop` | thin Tauri shell + Svelte UI over musa-project |
| `examples/` | `.musa` fixtures — executable specifications, not demos |
| `docs/prompts/` | numbered implementation prompts + README |

Dependency direction is one-way: language → compiler → {render, audio} → engine → project → {cli, desktop}. No
dependency points upward.

## Commands

```sh
cargo build --workspace
cargo nextest run [-p <crate>]            # fall back to cargo test if nextest missing
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo deny check                          # if cargo-deny installed
```

All four must be green before committing. The workspace lints in `Cargo.toml` are strict on purpose: fix the code, do
not allow-list lints.

## Standards

- **Deep modules.** Public facades are narrow (`parse`, `compile`, `render_notation`, `compile_graph`, `AudioEngine`,
  `ProjectSession`); pass types, Rowan internals, DSP internals, and CPAL types never cross crate boundaries. No public
  item without a caller. Doc-comment the public API and its invariants before implementing it.
- **Layer separation.** Roadmap §2's table is law: written pitch ≠ MIDI number, notated duration ≠ performed duration,
  voice ≠ mixer track, part ≠ synthesizer, dynamic marking ≠ decibels, motif definition ≠ its expansions, source ≠
  widget state.
- **The source is canonical.** Score and studio UIs are structured editors of `.musa` text. No second editable AST, no
  mutable expanded cache.
- **Real-time rules.** The audio callback never allocates, locks, does I/O, logs, or destroys large objects. Plans are
  preallocated on the control side and cross the boundary on `rtrb` queues.
- **Exact time.** Musical time is rational (`num-rational`); floats appear only at the performance/DSP edge.
- **Dependencies.** New crates must come from the roadmap §15 dependency lists and be added in the prompt that needs
  them. FundSP/CPAL/Rowan types stay private to their crate.

## Working conventions

- One prompt = one commit (or one small squashed set), using the commit message the prompt names. Flip the prompt's
  `status` frontmatter in that commit.
- A prompt is done only when its **Check** section passes.
- If a prompt's design proves wrong, repair the prompt file first, commit the repair, then implement.
- Examples in `examples/` are regression fixtures; keep them compiling and rendering.
