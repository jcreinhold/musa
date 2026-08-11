# AGENTS.md — musa

Musa is a notation-first music language and workbench: a `.musa` source language, a provenance-preserving compiler,
MEI/LilyPond/MusicXML/MIDI export, a small built-in DSP studio, and a Tauri/Svelte desktop score editor. Rust workspace;
the semantic core is Rust, the UI is a replaceable projection.

## The documents that govern this repo

1. **`docs/initial-design-roadmap.md`** — the architecture. Semantic layers, crate ownership, language design, DSP
   rules, and what is explicitly rejected or deferred. Read the cited sections before changing anything structural.
2. **`docs/course-correction.md`** — the semantic course correction: a small temporal kernel (ambient exact rational
   time, typed occurrences, `timeline`/`sequence`/`overlay`) is the ontology; the surface language elaborates into it.
   Where it and the roadmap disagree, the course correction wins. Its authoritative elaboration is **`docs/kernel/`**,
   the governing kernel specification. §36 is its one amendment, made at prompt 126.
3. **`docs/core-boundary.md`** — what the core is a calculus *of*: occurrences of any canonical payload over exact
   rational time, `ScoreFact` being one payload and the performance gesture another. Signals stay outside the core
   because a signal is coinductive and a signal graph has no extent; the prepared render plan is what crosses, under the
   law `R1`. It sits under the course correction and governs over `docs/language/`. Its §"What this forbids" is
   deliberately hard to re-open: doing so means amending it and course correction §36 together.
4. **`docs/interface/`** — the desktop interface specification: visual language, engraving quality bar, interaction and
   selection model, Origin view, states and voice, performance budgets. Roadmap §14 fixes the app's *architecture*;
   `docs/interface/` fixes everything §14 leaves open, and §14's wireframe is not a visual spec. Governing since prompt
   26.
5. **`docs/language-correction.md`** — the language correction: the musical domains are a *proved* conservative
   extension rather than an accumulating fragment, the standard library is a package with a real module tree, and
   `import` and `use` are two words because they were always two statements. It governs over **`docs/language/`** (the
   elaboration-language specification; candidate until prompt 146 graduates it) the way the course correction governs
   over the roadmap.
6. **`docs/prompts/`** — the numbered work plan, currently 155 prompts through rank 153, with its README defining prompt
   anatomy and execution rules. Implementation happens in dependency order (see the `prompt-stack` skill).

If code and these documents disagree, either the code is wrong or the document needs a deliberate repair — never let
them drift silently.

## Navigation

| Path | What lives there |
| --- | --- |
| `crates/musa-language` | tokens, lexer, parser, lossless CST, formatter, text edits |
| `crates/musa-kernel` | finite temporal kernel: exact time, typed occurrences, timeline/sequence/overlay, normalization |
| `crates/musa-compiler` | resolution, units, elaboration through the kernel, score/performance snapshots |
| `crates/musa-render` | NotationPlan, MEI, LilyPond, MusicXML, MIDI export |
| `crates/musa-audio` | studio graph spec→render-plan compiler, processors, offline rendering |
| `crates/musa-engine` | CPAL stream, transport, real-time queues, MIDI input |
| `crates/musa-project` | ProjectSession facade: documents, revisions, commands, exports |
| `crates/musa` | thin CLI over musa-project, installed as the `musa` binary |
| `crates/musa-lsp` | thin language server (LSP) over musa-project + musa-language |
| `crates/musa-wasm` | wasm-bindgen shell: musa source → MEI for `@musa/web` |
| `apps/musa-desktop` | thin Tauri shell + Svelte UI over musa-project |
| `packages/musa-engrave` | shared worker engraver: Verovio behind the `Engraver` interface |
| `packages/musa-web` | `@musa/web` — typeset musa scores in the browser |
| `editors/tree-sitter-musa` | tree-sitter grammar + editor queries, held to the real lexer by the drift law |
| `stdlib/` | the standard library as a real package (`musa.toml` + `src/`) |
| `examples/` | `.musa` fixtures — executable specifications, not demos |
| `docs/kernel/` | the temporal-kernel specification (governing) |
| `docs/core-boundary.md` | what the core is a calculus of, and what that forbids (governing) |
| `docs/interface/` | the desktop interface specification (governing) |
| `docs/language/` | the elaboration-language specification (candidate until prompt 146) |
| `docs/prompts/` | numbered implementation prompts + README |

Dependency direction is one-way: language → compiler → {render, audio} → engine → project → {cli, lsp, desktop}, with
`musa-kernel` a leaf that `musa-compiler` (and later consumers) depend on, and `musa-lsp` the one shell that also
depends on `musa-language` (highlighting and completion answer on half-typed source, which the session's facts cannot
describe — roadmap §15.11). `musa-wasm` is a fourth shell, over compiler + render, and `packages/*` sits below it in
TypeScript. No dependency points upward.

## Commands

```sh
cargo build --workspace
cargo nextest run [-p <crate>]            # fall back to cargo test if nextest missing
cargo nextest run --run-ignored all       # adds the slow tests; minutes, not seconds
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
taplo fmt --check                         # TOML
mdwright fmt-check                        # Markdown
cargo deny check                          # if cargo-deny installed
```

All of these must be green before committing. The workspace lints in `Cargo.toml` are strict on purpose: fix the code,
do not allow-list lints.

**Two build systems.** `cargo build --workspace` covers `crates/*` and the Tauri shell only. `packages/*` and
`apps/musa-desktop/ui` are a pnpm workspace: `pnpm -r check` and `pnpm -r test` there. Touching one side does not check
the other.

**Slow tests carry `#[ignore]` and say so in their name**, so the default suite asks for them by name. Marking one
requires a doc comment arguing it: what the test protects, what still covers that contract in the fast suite, and what
breadth is deferred. A slow test without that note should have been made fast.

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
- **`.musa` style.** `docs/style-guide.md` owns what the formatter cannot say — naming, and spellings that are correct
  and still mislead the player. The lint pass enforces its machine-checkable subset; each rule names its diagnostic.
- **Dependencies.** New crates must come from the roadmap §15 dependency lists and be added in the prompt that needs
  them. FundSP/CPAL/Rowan types stay private to their crate.

## Working conventions

- One prompt = one commit (or one small squashed set), using the commit message the prompt names. Flip the prompt's
  `status` frontmatter in that commit.
- A prompt is done only when its **Check** section passes.
- If a prompt's design proves wrong, repair the prompt file first, commit the repair, then implement.
- Examples in `examples/` are regression fixtures; keep them compiling and rendering.
