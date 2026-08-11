# Set up an editor

Two editor integrations exist, and they answer different questions.

## The language server

`musa-lsp` speaks LSP over stdio. Build it once:

```bash
cargo build -p musa-lsp
```

Then point any LSP-speaking editor at the `musa-lsp` binary. It provides:

- diagnostics with their certain fixes;
- hover that answers musically;
- go-to-definition through provenance;
- outline symbols;
- canonical formatting;
- semantic tokens and completion.

The server answers from the project's own session. It computes nothing of its own, so what the editor shows is what
`musa check` would say.

## Tree-sitter

`editors/tree-sitter-musa` holds a tree-sitter grammar and editor queries (highlighting, outline, text objects) for
editors that prefer a local grammar — Zed, Neovim, Helix. It is a second reader of the language, held honest by a test
that runs the real lexer against it, so it cannot drift from the language musa compiles.

Use both: tree-sitter highlights before the server answers, and the server answers what highlighting cannot know.
