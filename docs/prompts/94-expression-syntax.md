---
id: 94
slug: expression-syntax
status: pending
depends_on: [92, 93]
phase: 3
---

# One Expression and Type Grammar Everywhere

## Task

Add the governing core expression/type syntax to the lossless Musa grammar and every syntax consumer: typed `let`,
named `fn`, application, products, lists, options, case/fold forms, and `music` blocks. Format it canonically, recover
while half typed, and update tree-sitter, keyword documentation, semantic-token classification, and editor grammars in
the same prompt so there is never a period in which two tools recognize different Musa languages.

Here, “case/fold forms” means `01-surface.md`'s exhaustive `match` syntax plus ordinary calls to the three named fold
primitives. Do not invent a `fold` statement or special call grammar. Product/list/option values and patterns use the
exact spellings now recorded in that governing file.

## Read

- `docs/language/01-surface.md` and `02-core-calculus.md`; implement their chosen spellings exactly.
- `musa-language` lexer/parser/CST/formatter; prompts 77, 80–82, 84, and 87–90 for the drift, keyword-doc, and
  readable formatting laws.
- The sibling workspaces `../vscode-musa` and `../zed-musa`; their generated artifacts consume, rather than redefine,
  Musa vocabulary.

## Design

Expressions have an explicit precedence table in `01-surface.md` and one parser implementation. Blocks returning
`music` keep the notation-first line grammar: a musician writing notes does not need `emit`, commas, or an AST-shaped
builder. General expressions use ordinary function-call punctuation. Public function parameters and results are
annotated; locals may infer. `motif` and `fragment` remain syntax nodes for later desugaring, not parser aliases that
lose their role.

All new nodes have typed wrappers and round-trip through the formatter. Recovery cases include a missing result type,
half-written call, unmatched type arrow, incomplete match arm, and a note line adjacent to a general expression. The
real lexer/token stream remains authoritative; tree-sitter's drift corpus and the desktop/LSP classification tables are
regenerated from it. Compiler behavior for these nodes is a stable `unsupported-language-stage` diagnostic until
prompt 95, never silent omission or a parser error pretending the syntax is invalid.

## Target

- `musa-language`: tokens, `SyntaxKind`s, parser, typed AST wrappers, formatter, edit-safe spans, keyword docs.
- `editors/tree-sitter-musa`: grammar, highlights, folds, indents, locals, outline/tags, recovery corpus, regenerated
  token comparison data.
- `musa-lsp` and desktop language support: semantic token/category vocabulary for new syntax on invalid documents.
- `../vscode-musa` and `../zed-musa`: regenerated syntax/query assets only; no semantic feature yet.
- `crates/musa-language/tests/expression_syntax_laws.rs`: CST snapshots, parse/format/parse, idempotence, trivia
  preservation, and recovery cases.

## Check

```sh
cargo nextest run -p musa-language -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-lsp -- -D warnings
cargo fmt --check
cd editors/tree-sitter-musa && tree-sitter generate && tree-sitter test
node editors/tree-sitter-musa/test/compare-tokens.js
cd apps/musa-desktop/ui && npm test
git -C ../vscode-musa diff --check
git -C ../zed-musa diff --check
```

Commit Musa and each sibling repository intentionally; record the commit ids in the prompt's repair notes. Commit the
Musa change as `Add the elaboration expression grammar`.

## Stop

- No type checking or evaluation — prompt 95.
- No pitch arithmetic, scale, chord, template/module, assertion, or kernel-quotation syntax; their owning prompts add
  the smallest additional grammar.
- No anonymous lambda syntax unless `01-surface.md` chose and justified it; named functions already support the block.
- No independent TextMate or tree-sitter vocabulary list.
