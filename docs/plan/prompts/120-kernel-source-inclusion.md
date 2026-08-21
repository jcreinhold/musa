---
id: 120
slug: kernel-source-inclusion
status: done
depends_on: [86, 92, 94]
phase: 3
---

# Every Kernel File Is a Musa Document

## Task

Make the subset claim true at the source-document boundary: every syntactically valid `.musa.kernel` file is accepted
unchanged as a Musa document, and every registered payload type produces the same checked term and denotation through
the unified route as direct kernel parsing. Support check/format/open/LSP/editor workflows without copying the kernel
grammar or pretending an unknown payload type has Musa score meaning.

## Read

- `docs/rules/kernel/{01-grammar,02-static-semantics,07-backend-contract,10-term-calculus}.md` and prompt 86.
- `docs/rules/language/00-semantics.md` kernel subset law and `01-surface.md` document alternatives.
- Current `DocumentKind`, `SourceDocument`, `musa kernel`, kernel text parser/printer, language parser, LSP lifecycle,
  tree-sitter grammar, project opening, and desktop file handling.

## Design

The first-line `% musa-kernel-1` header selects the kernel-document alternative. `musa-kernel` remains the one owner of
the interchange grammar and checked term; `musa-syntax` owns a lossless document wrapper/dispatch, comments, edits, and
diagnostics without reimplementing term semantics. Tree-sitter recognizes the same top-level alternative and is held to
committed kernel fixtures by a drift test.

State/test two laws separately:

1. **Syntactic inclusion:** every kernel corpus file is a valid unified Musa document byte-for-byte.
2. **Typed semantic preservation:** for a registered payload adapter `A`, unified parsing/checking yields the same
   `Term<A>` and evaluated timeline as `musa_kernel::parse::<A>`.

For unknown payload names, preserve, highlight, and format the syntactically valid file but refuse evaluation/export
with an unsupported-payload diagnostic. `ScoreFact` whole-score documents may contain context facts and compile to the
ordinary projections/backends. A kernel document has exactly one composition result; it is not rewritten into a surface
library or plural declaration set.

## Target

- Unified document dispatch in language/compiler/project/CLI; canonical kernel formatting delegates to the kernel
  printer while preserving required header/version behavior.
- LSP diagnostics, formatting, semantic tokens, symbols, and hover appropriate to kernel documents.
- Tree-sitter and desktop/sibling editor support for `.musa.kernel` as Musa.
- `crates/musa-compiler/tests/suite/kernel_subset_laws.rs`: the full corpus, term/denotation equality, unknown payload,
  version refusal, comments/roundtrip, and whole-score `ScoreFact` export.
- File associations and project opening for `.musa.kernel` without changing ordinary `.musa` defaults.

## Check

```sh
cargo nextest run -p musa-kernel -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-kernel -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
for f in examples/kernel/*.musa.kernel; do cargo run -q -p musa -- check "$f"; done
cd editors/tree-sitter-musa && tree-sitter test
cd apps/musa-desktop/ui && npm test
```

Commit as `Accept kernel files as Musa documents`.

## Stop

- No local quotation/antiquotation — prompt 121.
- No payload grammar or musical fact knowledge in `musa-kernel`.
- No second term parser, normalization rule, or kernel meaning in `musa-syntax`/tree-sitter.
- No silent conversion from unknown payload text to `ScoreFact` and no dropping facts a backend does not understand.
