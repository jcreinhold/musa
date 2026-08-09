---
id: 82
slug: zed-extension
status: done
depends_on: [77, 80]
phase: 3
---

# The Zed Extension

## Task

Build `zed-musa`: the Zed extension that embeds the prompt 80 grammar for structure and highlighting, and starts the
prompt 77 server for everything semantic. Zed's extension model makes the division explicit — tree-sitter answers
"what shape is this text" on every keystroke, the language server answers "what does it mean" — which is the same
division this repository drew between `musa-language` and `musa-project`.

## Read

- Prompt 80 — the grammar and its node names; every query file here is written against those nodes, so this prompt
  depends on it for more than convenience.
- Prompt 77 — the server and its fixed semantic-token legend.
- Zed's extension model: `extension.toml` is the manifest; the extension itself is a Rust `cdylib` compiled to
  `wasm32-wasip2` against `zed_extension_api`, sandboxed; query files live under `languages/musa/`.

## Design

### Layout

A sibling repository `zed-musa` (Zed installs extensions as their own repositories):

- `extension.toml` — `schema_version = 1`; a `[grammars.musa]` entry pointing at `editors/tree-sitter-musa` in this
  repository by `repository` and pinned `rev`; a `[language_servers.musa-lsp]` entry binding the server to the
  language.
- `Cargo.toml` (`crate-type = ["cdylib"]`, an empty `[workspace]` table to stay out of any parent workspace) and
  `src/lib.rs` — the `Extension` trait implementation. Its one real job is `language_server_command`: resolve
  `musa-lsp` from the extension's settings, then `PATH`, else an error that names what to install. The WASM sandbox
  cannot reach the network or the filesystem beyond its worktree; there is nothing else honest for it to do.
- `languages/musa/` — `config.toml` (comments, brackets, indentation: §7's explicit braces make this declarative)
  and the query files: `highlights.scm` (captures mirroring `TokenClass`), `folds.scm` (braced blocks and comment
  runs — prompt 79's rule, stated in tree-sitter's tongue), `outline.scm`, `brackets.scm`, `indents.scm`,
  `locals.scm` (declaration and reference scopes, so Zed's local rename and highlighting agree with prompt 78's
  resolver), `tags.scm`.
- `semantic_token_rules.json` — mapping the server's legend onto the grammar's captures, so the two highlighting
  layers refine rather than fight.

### The honesty rule, restated

The query files target the grammar's committed node names and nothing else. If a query wants a node the grammar does
not have, the grammar prompt is repaired first — a query that pattern-matches text the CST already structured is the
tokenizer.ts mistake one layer down.

## Target

- The `zed-musa` repository: manifest, `src/lib.rs`, the `languages/musa/` query set, `README.md` with dev-install
  instructions (`zed: install dev extension`, build `musa-lsp`).

## Check

```sh
cargo build --manifest-path <zed-musa>/Cargo.toml --target wasm32-wasip2
```

Behavior, by hand before flipping status: dev-install in Zed, open `glass-mountain.musa` — highlighted and outlined
from the grammar, folds on parts and motifs; diagnostics, hover, definition, and quick fixes arrive from the server;
the outline panel lists the piece's sections; uninstalling the server binary leaves structure and highlighting
standing.

## Stop

- No themes, snippets, slash commands, or context servers.
- No publishing to the Zed extension registry.
- No grammar changes "while in there": the grammar is prompt 80's artifact, pinned by `rev`.
- No duplicating the server's semantics in queries. Structure is the grammar's; meaning is the server's.
