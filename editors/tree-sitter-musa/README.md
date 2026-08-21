# tree-sitter-musa

A [tree-sitter](https://tree-sitter.github.io) grammar for [musa](https://github.com/jcreinhold/musa), the
notation-first music language — plus the query files editors consume (highlighting, folding, indentation, locals,
outline, tags).

The authoritative parser is the hand-written one in `crates/musa-syntax`; this grammar is a second reader, and it owns
no vocabulary. Two laws keep it honest, both run by `npm test`:

- **The corpus** (`test/corpus/`, one entry per `examples/*.musa`) pins the tree shape, via `tree-sitter test`.
- **The drift law** (`test/compare-tokens.js`) compares the grammar's leaves against the *real* lexer's token stream for
  every compilable fixture — committed in `test/tokens/` by `crates/musa-syntax/tests/suite/tree_sitter_fixtures.rs` —
  and checks the grammar agrees with the real parser about which broken fixtures are syntactically broken
  (`test/broken.json`). Refresh the committed data with `UPDATE_FIXTURES=1 cargo test -p musa-syntax --test suite`.

Every rule in `grammar.js` names the function in `crates/musa-syntax/src/parser.rs` it traces to; node names mirror
`syntax_kind.rs`.

```sh
npm test              # generate, corpus, and the drift law
tree-sitter parse ../../examples/glass-mountain.musa
```
