---
id: 120
slug: events-source-inclusion
status: done
depends_on: [86, 92, 94]
phase: 3
---

# Every Kernel File Is a Musa Document

## Task

Make the subset claim true at the source-document boundary: every syntactically valid `.musa.events` file is accepted
unchanged as a Musa document, and every registered payload type produces the same checked term and denotation through
the unified route as direct event track parsing. Support check/format/open/LSP/editor workflows without copying the
event track grammar or pretending an unknown payload type has Musa score meaning.

## Read

- `docs/rules/events/{01-grammar,02-static-semantics,07-backend-contract,10-term-calculus}.md` and prompt 86.
- `docs/rules/language/00-semantics.md` events subset law and `01-surface.md` document alternatives.
- Current `DocumentKind`, `SourceDocument`, `musa events`, events text parser/printer, language parser, LSP lifecycle,
  tree-sitter grammar, project opening, and desktop file handling.

## Design

The first-line `% musa-events-1` header selects the event-track-document alternative. `musa-events` remains the one
owner of the interchange grammar and checked term; `musa-syntax` owns a lossless document wrapper/dispatch, comments,
edits, and diagnostics without reimplementing term semantics. Tree-sitter recognizes the same top-level alternative and
is held to committed events fixtures by a drift test.

State/test two laws separately:

1. **Syntactic inclusion:** every events corpus file is a valid unified Musa document byte-for-byte.
2. **Typed semantic preservation:** for a registered payload adapter `A`, unified parsing/checking yields the same
   `Term<A>` and evaluated timeline as `musa_events::parse::<A>`.

For unknown payload names, preserve, highlight, and format the syntactically valid file but refuse evaluation/export
with an unsupported-payload diagnostic. `ScoreFact` whole-score documents may contain context facts and compile to the
ordinary projections/backends. An events document has exactly one composition result; it is not rewritten into a surface
library or plural declaration set.

## Target

- Unified document dispatch in language/compiler/project/CLI; canonical event track formatting delegates to the event
  track printer while preserving required header/version behavior.
- LSP diagnostics, formatting, semantic tokens, symbols, and hover appropriate to events documents.
- Tree-sitter and desktop/sibling editor support for `.musa.events` as Musa.
- `crates/musa-compiler/tests/suite/events_subset_laws.rs`: the full corpus, term/denotation equality, unknown payload,
  version refusal, comments/roundtrip, and whole-score `ScoreFact` export.
- File associations and project opening for `.musa.events` without changing ordinary `.musa` defaults.

## Check

```sh
cargo nextest run -p musa-events -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-events -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
for f in examples/events/*.musa.events; do cargo run -q -p musa -- check "$f"; done
cd editors/tree-sitter-musa && tree-sitter test
cd apps/musa-desktop/ui && npm test
```

Commit as `Accept events files as Musa documents`.

## Stop

- No local quotation/antiquotation — prompt 121.
- No payload grammar or musical fact knowledge in `musa-events`.
- No second term parser, normalization rule, or event track meaning in `musa-syntax`/tree-sitter.
- No silent conversion from unknown payload text to `ScoreFact` and no dropping facts a backend does not understand.
