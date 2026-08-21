---
id: 77
slug: language-server
status: done
depends_on: [19, 26, 39, 56]
phase: 3
---

# A Language Server

## Task

Prompt 56 said it plainly: *"The structure here is what an LSP would need, which is the point, but the server is not
this prompt."* This is that prompt. Build `musa-lsp`, a language server over stdio, so the language is usable in any
editor that speaks LSP — diagnostics that teach, canonical formatting, hover that answers musically, go-to-definition
through provenance, outline symbols, certain fixes, semantic tokens, and completion.

## Read

- Roadmap §15.7 and prompt 19 — `ProjectSession` is the facade; `from_text` plus `SetSource` *is* an LSP document
  lifecycle, and last-valid-artifacts (§14.7) is exactly how a server should behave while the source is half-typed.
- Prompt 56 — diagnostics carry codes, primary and secondary labels, and certain fixes; `explain` holds the long form.
- Prompt 39 — `ScoreFacts`: every event's origin span, every occurrence's use-site and motif declaration, the outline.
- `docs/rules/desktop/03-interaction.md` §7 — the law this prompt inherits: the interface computes no musical facts. The
  server translates what the session hands it; it never derives music from text itself.
- Prompt 26 and `apps/musa-desktop/ui/src/lib/lang-musa/` — how the desktop solved highlighting on half-typed source,
  and why (the session's facts describe the last *valid* compile; highlighting cannot wait for one).

## Design

### A third shell

`crates/musa-lsp`, a thin shell beside `musa` and the desktop: the dependency direction becomes
`project → {cli, desktop, lsp}`. Repair roadmap §15 with a §15.11 entry in the same commit, naming the crate and its
dependency list: `musa-project`, `musa-syntax`, `lsp-server`, `lsp-types`.

The second edge to `musa-syntax` — which no other shell has — is deliberate and the repair must state its reason:
semantic tokens and completion must answer on *half-typed* source, which the session cannot describe (its facts are the
last valid compile's). The desktop solved this with a second tokenizer in TypeScript fed a generated vocabulary; the
server has the real lexer in-process and uses it. Formatting likewise calls `musa_syntax::format` directly and **never**
issues `ProjectCommand::Format`: a format request must not land in the session's undo history.

### Protocol crate

`lsp-server` + `lsp-types` (rust-analyzer's, MIT/Apache). Synchronous and explicit: one main loop owns the sessions,
matching "the session is single-threaded by construction" — no mutex, no runtime. `Connection::memory()` drives the
whole server in-process in tests. `tower-lsp` is rejected: it drags in tokio and demands `Send` of a session type that
holds a `cpal::Stream` in an `Option`, buying nothing.

### Shape

One public function, `serve`, over stdio; `src/main.rs` calls it. Everything else is private:

- `workspace` — URI → `ProjectSession`, full-document sync (`TextDocumentSyncKind::Full`; pieces are small and full sync
  is what the desktop's editor already does).
- `convert` — the *only* module that knows LSP's coordinate system: byte spans ↔ 0-based UTF-16 ranges, via
  `Utf16Offsets`. One place, one direction per function, law-tested on non-ASCII source.
- one handler per request kind — each a lookup into the snapshot's facts plus a `convert` call.

### Features

| Request | Backed by |
| --- | --- |
| lifecycle, `didOpen`/`didChange`/`didClose` | `from_text`, `SetSource`; diagnostics published after every change |
| `publishDiagnostics` | `snapshot().diagnostics()`: severity, code, primary label as range, secondary labels as related information |
| `formatting` | `musa_syntax::format(&parse(source))`, one whole-document edit, session untouched |
| `hover` | event at the position (origin span): spelled pitch, duration, bar:beat, key/clef, origin path; occurrences at use-sites; studio nodes |
| `definition` | `use` → motif `declaration`; generated event → `definition_span` (the statement that spells it) |
| `documentSymbol` | the outline: sections and phrases |
| `codeAction` | `Diagnostic.fixes` as quick-fix workspace edits |
| `semanticTokens/full` | `lex` → `TokenClass::of`, a fixed legend, delta-encoded; works on invalid source because the lexer is total |
| `completion` | `SPELLINGS` (keywords, units) plus named entities — motifs, parts, voices, patches — from the last valid facts |

## Target

- `crates/musa-lsp/`: `Cargo.toml` (workspace lints), `src/lib.rs` (`serve`), `src/main.rs`, private `workspace`,
  `convert`, `handlers` modules, `tests/lsp_laws.rs`.
- Roadmap §15.11 entry; `AGENTS.md` navigation row and dependency-direction line; `README.md` crate-table row.
- No changes to `musa-project` or `musa-syntax`: the facts this prompt needs, prompt 39 and 56 already shipped.

## Check

```sh
cargo nextest run -p musa-lsp
cargo clippy --all-targets -p musa-lsp -- -D warnings
cargo fmt --check
```

Behavior checks run over `Connection::memory()` against the committed fixtures: `glass-mountain.musa` publishes no
diagnostics; `broken/missing-semicolon.musa` publishes its diagnostic with its fix available as a code action; hover on
a note reports its spelled pitch and bar:beat; definition on `use sigh()` lands on the motif body; symbols list a
sectioned piece's outline; semantic tokens cover a deliberately broken document; completion offers keywords. One manual
smoke test against a real client (Neovim or VS Code's generic LSP support) before flipping status.

## Stop

- No references, rename, or `prepareRename` — prompt 78, which first gives the resolver a memory of use-sites.
- No folding ranges — prompt 79.
- No tree-sitter grammar and no editor extensions — prompts 80–82.
- No incremental sync, no semantic-token deltas, no pull diagnostics. Pieces are small; add when measured.
- No playback, MIDI entry, autosave, or exports. The server never opens the audio device.
- No second diagnostic renderer, no new diagnostic codes. The server restates; it does not diagnose.
