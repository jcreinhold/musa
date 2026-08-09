---
id: 80
slug: tree-sitter-grammar
status: pending
depends_on: [3]
phase: 3
---

# A Tree-Sitter Grammar

## Task

Build `tree-sitter-musa` at `editors/tree-sitter-musa/`: a tree-sitter grammar for the surface language, with the
query files editors consume (highlighting, folding, indentation, outline, tags) and a corpus that pins the grammar to
the real lexer and parser. Editors like Zed embed tree-sitter for structure; this is the artifact prompts 81 and 82
stand on, and it is useful to no one if it can quietly drift from the language it claims to describe.

## Read

- `crates/musa-language/src/parser.rs`, `lexer.rs`, and `syntax_kind.rs` — the authoritative parser. **Every grammar
  rule traces back to a specific function here.** Tree-sitter grammars describe concrete syntax trees; do not invent
  node shapes from examples, and do not guess at disambiguation the hand parser resolves structurally.
- Roadmap §7 — the language design: explicit semicolons and braces, units as syntax, rational durations. These are
  what make the grammar tractable, and the corpus is where they stay true.
- `apps/musa-desktop/ui/src/lib/lang-musa/` and `crates/musa-project/tests/ui_fixtures_generators.rs` — the standing
  answer to drift: a second reader owns no vocabulary, and a generator test writes expectations from the real
  implementation so a stale copy fails loudly.

## Design

### Layout

The standard grammar project: `grammar.js`, generated `src/`, `queries/{highlights,folds,indents,locals,outline,
tags}.scm`, `test/corpus/*.txt`, `tree-sitter.json`, `package.json`. An external scanner (`src/scanner.c`) only if
the hand parser proves something context-sensitive — read the parser first; the bet is that §7's explicitness makes
one unnecessary, and an unneeded scanner is complexity sold as rigor.

Node names and fields mirror `syntax_kind.rs` where the tree shapes agree, so the query files read in the language's
own vocabulary and prompt 82's queries need no translation table. Highlight captures mirror `TokenClass`.

### The drift law

A generator test in `musa-language` (the `ui_fixtures_generators` pattern, `UPDATE_FIXTURES=1` to refresh) writes the
real lexer's token stream for every `examples/*.musa` and every compilable fixture into the grammar's test data. A
corpus-side test compares the tree-sitter parse's tokens against them, token for token. A grammar that disagrees with
the lexer about a single token fails CI, not the composer.

Error recovery is tested, not hoped for: the `examples/broken/` fixtures must parse with `ERROR` nodes and must not
crash the parser — editors run the grammar on every keystroke, mid-word.

## Target

- `editors/tree-sitter-musa/`: the grammar project above, with a corpus entry per example.
- `crates/musa-language/tests/`: the fixture generator and its committed output.
- `AGENTS.md` and `README.md`: one row each.

## Check

```sh
cd editors/tree-sitter-musa && tree-sitter generate && tree-sitter test
cargo nextest run -p musa-language
cargo clippy --all-targets -p musa-language -- -D warnings && cargo fmt --check
```

Behavior: every example parses without `ERROR`; every broken fixture parses *with* `ERROR` and without a panic; the
token-for-token comparison against the real lexer is green.

## Stop

- No editor extensions — prompts 81 and 82 consume this grammar; this prompt ships no client.
- No queries beyond the listed set; injections, textobjects, and overrides wait for an editor that needs them.
- No replacing `musa-language`'s parser. The hand parser stays authoritative (roadmap §15.2); tree-sitter is a second
  reader held honest by the drift law, not a second source of truth.
- No registry publishing (crates.io, npm) — the grammar is consumed in-repo and by the sibling extension repos.
