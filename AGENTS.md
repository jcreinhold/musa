# AGENTS.md — musa

Musa is a notation-first music language and workbench: a `.musa` source language, a provenance-preserving compiler,
MEI/LilyPond/MusicXML/MIDI export, a small built-in DSP studio, and a Tauri/Svelte desktop score editor. Rust workspace;
the semantic core is Rust, the UI is a replaceable projection.

## The documents that govern this repo

Read [`docs/README.md`](docs/README.md) first: it maps the four directories and states the precedence ladder in full.
`docs/rules/` governs, `docs/plan/` directs, `docs/book/` teaches, `docs/notes/` records. Within `docs/rules/`:

1. **`docs/rules/constitution.md`** and **`docs/rules/obligations.md`** — the core decisions and what follows. Source
   authority, plural theory-owned presentations, exact ambient time, the finite-process/running-signal distinction,
   typed derivation coherence, and versioned exact identity. Removing one changes what Musa is; amend deliberately. §7
   and §4 say what the core is a calculus *of*: occurrences of any canonical payload over exact rational time,
   `ScoreFact` being one payload and the performance gesture another. Signals stay outside the core because a signal is
   coinductive and a process graph has no musical extent. A complete semantic preparation result crosses under exact
   `R1`; the private process IR has its own formal tick semantics. What these forbid is hard to re-open — only through
   `docs/rules/README.md`'s amendment procedure.
2. **`docs/rules/events/`** — the governing event-track specification: a small event track (ambient exact rational time,
   typed occurrences, `timeline`/`sequence`/`overlay`) is the ontology, and the surface language elaborates into it.
   Where it and the roadmap disagree on semantic architecture, the event track wins.
3. **`docs/rules/across-stages/`** — the cross-stage formal specification, owning presentation, pass, process, and
   identity semantics. It refines **`docs/plan/roadmap.md`**, which still owns the broad crate/product plan.
4. **`docs/rules/desktop/`** — the desktop interface specification: visual language, engraving quality bar, interaction
   and selection model, Origin view, states and voice, performance budgets. Roadmap §14 fixes the app's *architecture*;
   `docs/rules/desktop/` fixes everything §14 leaves open, and §14's wireframe is not a visual spec. Governing since
   prompt 26.
5. **`docs/rules/language/`** — the elaboration-language specification: the musical domains are a *proved* conservative
   extension rather than an accumulating fragment, the standard library is a package with a real module tree, and
   `import` and `use` are two words because they were always two statements. Candidate until prompt 193 graduates it, so
   everything above it in `docs/README.md`'s precedence ladder wins where they differ.

Under `docs/plan/`: **`prompts/`** is the numbered work plan, currently 301 prompts through rank 200, with its README
defining prompt anatomy and execution rules — implementation happens in dependency order (see the `prompt-stack` skill).
Prompts 127a–127e and 171–174, including the inserted 127aa–127ad, 127ca, and 127da–127dd repairs, are the clean-break
core-calculus cutover; 127e was superseded by prompt 142, and 127a amends the governing boundary before code implements
the replacement. Prompts **128–170** are the language pass: a dependent core with bidirectional elaboration, indexed
families, records, modules, typed quotation, and pattern-unification discipline, admitted by an amendment at 128 on the
evidence of `stdlib/src/adapters/staff.musa` and closed by the measured rewrites of that adapter and the studio adapter.
**`code-map/`** reports which crate implements which stage and what is implemented, partial, or absent; it describes
code and decides nothing.

If code and a governing document disagree, either the code is wrong or the document needs a deliberate repair — never
let them drift silently.

## Navigation

| Path | What lives there |
| --- | --- |
| `crates/musa-syntax` | tokens, lexer, parser, lossless CST, formatter, text edits |
| `crates/musa-events` | finite event-track: exact time, typed occurrences, timeline/sequence/overlay, normalization |
| `crates/musa-calculus` | the dependently typed core calculus a checked term lives in: `kernel/` decides typing and equality (NbE, inductive families), `elaboration/` reads what an author wrote — one way, checked by a law, and the trusted half is named in [`TRUST.md`](crates/musa-calculus/TRUST.md) |
| `crates/musa-score` | the musical values: pitch, chords, scales, exact time, marks, score/performance snapshots, provenance, diagnostics, analysis |
| `crates/musa-compiler` | resolution, units, imports, expansion, elaboration through the event track — the passes that compute those values |
| `crates/musa-notation` | NotationPlan, MEI, LilyPond, MusicXML, MIDI export |
| `crates/musa-dsp` | studio graph spec→render-plan compiler, processors, offline rendering |
| `crates/musa-playback` | CPAL stream, transport, real-time queues, MIDI input |
| `crates/musa-project` | ProjectSession facade: documents, revisions, commands, exports |
| `crates/musa` | thin CLI over musa-project, installed as the `musa` binary |
| `crates/musa-lsp` | thin language server (LSP) over musa-project + musa-syntax |
| `crates/musa-wasm` | wasm-bindgen shell: musa source → MEI for `@musa/web` |
| `apps/musa-desktop` | thin Tauri shell + Svelte UI over musa-project |
| `packages/musa-engrave` | shared worker engraver: Verovio behind the `Engraver` interface |
| `packages/musa-web` | `@musa/web` — typeset musa scores in the browser |
| `editors/tree-sitter-musa` | tree-sitter grammar + editor queries, held to the real lexer by the drift law |
| `stdlib/` | the standard library as a real package (`musa.toml` + `src/`) |
| `examples/` | `.musa` fixtures — executable specifications, not demos |
| `docs/README.md` | the map of the four directories and the precedence ladder — read this first |
| `docs/rules/` | **governing.** constitution, obligations, and the per-stage specifications |
| `docs/rules/across-stages/` | cross-stage presentations, derivations, process semantics, identity |
| `docs/rules/events/` | the event-track specification |
| `docs/rules/desktop/` | the desktop interface specification |
| `docs/rules/language/` | the elaboration-language specification (candidate until prompt 193) |
| `docs/rules/style-guide.md` | `.musa` naming and spelling; the lint pass cites it by section |
| `docs/plan/` | **directive.** roadmap, numbered prompts, and the spec-to-code map |
| `docs/book/` | **teaching.** tutorials, guide, how-to, explanation, reference |
| `docs/notes/` | **governs nothing.** `research/` decision records, `toolchain/` machine traps |

Dependency direction is one-way: language → score → compiler → audio → engine → project → {cli, lsp, desktop}, with
`musa-notation` sitting on `musa-score` alone, and with `musa-calculus` and `musa-events` two leaves that
`musa-compiler` (and later consumers) depend on, and `musa-lsp` the one shell that also depends on `musa-syntax`
(highlighting and completion answer on half-typed source, which the session's facts cannot describe — roadmap §15.11).
`musa-wasm` is a fourth shell, over compiler + render, and `packages/*` sits below it in TypeScript. No dependency
points upward.

## Commands

```sh
cargo build --workspace
cargo nextest run [-p <crate>]            # fall back to cargo test if nextest missing
cargo nextest run --run-ignored all       # adds the slow tests; minutes, not seconds
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check                            # Rust, TOML, Markdown, and the UI, all four
make lint-ui                              # ESLint over the UI
make docs-check                           # docs/ links, teaching examples, mdbook build
cargo deny check                          # if cargo-deny installed
```

All of these must be green before committing. The workspace lints in `Cargo.toml` are strict on purpose: fix the code,
do not allow-list lints.

**Two build systems.** `cargo build --workspace` covers `crates/*` and the Tauri shell only. `packages/*` and
`apps/musa-desktop/ui` are a pnpm workspace: `pnpm -r check` and `pnpm -r test` there. Touching one side does not check
the other. `make fmt` writes both sides — `cargo fmt`, `taplo`, and Prettier — so run it rather than `pnpm run format`
alone.

**Generated files are never formatted.** `apps/musa-desktop/ui/src/lib/session/generated/` and
`apps/musa-desktop/ui/fixtures/` are written by generator tests and compared byte for byte; `.prettierignore` excludes
both. Change the generator and regenerate, never the file — see
[`docs/notes/toolchain/generated-files.md`](docs/notes/toolchain/generated-files.md).

**Integration tests live in `crates/<crate>/tests/suite/`, one module per file**, declared in
`crates/<crate>/tests/suite/main.rs`, so each crate builds one test binary rather than one per file. A file directly
under `tests/` becomes its own target and its own link of the workspace; 103 of them made a clean build twice as slow
and left 17,000 stale object files per incremental rebuild. Adding a test file means adding its `mod` line. Paths in
`include_str!` are relative to the file, so they carry the extra `../`; insta snapshots are prefixed `suite__`. See
[`docs/notes/toolchain/slow-test-suite.md`](docs/notes/toolchain/slow-test-suite.md).

**Slow tests carry `#[ignore]` and say so in their name**, so the default suite asks for them by name. Marking one
requires a doc comment arguing it: what the test protects, what still covers that contract in the fast suite, and what
breadth is deferred. A slow test without that note should have been made fast.

**When a gate misbehaves rather than fails**, check [`docs/notes/toolchain/`](docs/notes/toolchain/README.md) before
debugging the code — it collects the toolchain and environment traps that make a green change look broken.

**If the suite seems to hang on macOS at 0% CPU**, check `ls -1 target/debug/deps | wc -l`. Object files accumulate
without bound there, and past a few hundred thousand entries every process launch in that directory stalls for tens of
seconds. `cargo clean` fixes it; see
[`docs/notes/toolchain/slow-test-suite.md`](docs/notes/toolchain/slow-test-suite.md).

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
- **No sublanguage by subtraction.** Any language we hand a user — an adapter phase, a template dialect, a config
  grammar — is built by *enriching* a core, never by removing modules, local definitions, or data declarations from the
  source language and calling the remainder a phase. Peyton Jones ch. 3 enriches the calculus for precisely this reason:
  a language for programmers needs abstractions and local definitions. A layer that is "ordinary Musa minus features" is
  the same abstraction one level down (Ousterhout ch. 7), and every convenience it drops is paid by every author who
  writes in it rather than once by us (ch. 8).
- **Hand a consumer what we already computed.** If a stage has already lexed, resolved, or measured something, expose it
  rather than making the next stage re-derive it from text. An adapter re-parsing `3/8` out of a token's spelling is the
  shape of the mistake.
- **`.musa` style.** `docs/rules/style-guide.md` owns what the formatter cannot say — naming, and spellings that are
  correct and still mislead the player. The lint pass enforces its machine-checkable subset; each rule names its
  diagnostic.
- **Dependencies.** New crates must come from the roadmap §15 dependency lists and be added in the prompt that needs
  them. FundSP/CPAL/Rowan types stay private to their crate.

## Working conventions

- One prompt = one commit (or one small squashed set), using the commit message the prompt names. Flip the prompt's
  `status` frontmatter in that commit.
- A prompt is done only when its **Check** section passes.
- If a prompt's design proves wrong, repair the prompt file first, commit the repair, then implement.
- Examples in `examples/` are regression fixtures; keep them compiling and rendering.
