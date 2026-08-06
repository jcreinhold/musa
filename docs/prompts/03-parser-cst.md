---
id: 03
slug: parser-cst
status: pending
depends_on: [02]
phase: 1
---

# Parser and Lossless CST

## Task

Build the hand-written recursive-descent parser and `rowan` lossless concrete syntax
tree for the core `.musa` grammar: `piece` with `tempo`/`meter`/`key`, `score` with
`part`/`voice`/`clef`, note/rest/chord statements with rational durations. The parser
always returns a tree, keeps every token and comment, and recovers from errors at
statement and declaration boundaries.

## Read

- Roadmap §7.1 (the example program is the grammar spec for this prompt), §10.2–§10.5
  (parser architecture, error recovery, typed wrappers), §17.1 (snapshot testing).
- The lexer from prompt 02.

## Design

- Add dependencies to `musa-language`: `rowan`, `insta` (dev).
- Architecture follows §10.3 exactly: the parser emits events
  (`StartNode`/`Token`/`FinishNode`/`Error`), a second pass builds the Rowan green
  tree. Parser code never constructs Rowan nodes directly.
- Recovery sets (§10.4): `;`, `}`, and the next declaration keyword (`piece`, `part`,
  `voice`, `motif`, `patch`, `bus`, `score`, `performance`, `studio`). Malformed
  regions become `ERROR` nodes; parsing continues.
- Grammar scope for this prompt (only):
  - `piece "name" { ... }` with `tempo <dur> = <int>;`, `meter <int>/<int>;`,
    `key <pitch-class> (major|minor);`;
  - `score { part <ident> { clef (treble|bass|alto|tenor); voice <ident> { ... } } }`;
  - voice items: `<pitch> <dur>;`, `rest <dur>;`, `chord [<pitch>, ...] <dur>;`.
  - `motif`, `use`, `transpose`, `repeat` parse as **stub nodes** (recognized and
    tree'd, flagged with an "unsupported in this compiler version" parse-level note
    node or deferred entirely to prompt 06 — your choice, but state it in the prompt
    repair if you deviate). Preferred: parse them fully now (the grammar is small)
    so prompt 06 is compiler-only.
- Public surface:

  ```rust
  /// Parse `source`; always returns a tree, even for invalid input.
  pub fn parse(source: &str) -> ParsedDocument;

  pub struct ParsedDocument { /* private */ }
  impl ParsedDocument {
      pub fn syntax(&self) -> SyntaxNode;      // rowan root
      pub fn errors(&self) -> &[SyntaxError];  // lex + parse errors, with spans
  }
  ```

  plus typed wrappers (§10.5): `PieceDecl`, `ScoreDecl`, `PartDecl`, `VoiceDecl`,
  `NoteStmt`, `RestStmt`, `ChordStmt` — thin `struct X(SyntaxNode)` casts with
  accessors; they cache nothing.
- The `ParsedDocument` facade hides Logos and the event machinery; `SyntaxNode` may be
  a Rowan type at the API boundary (intentional, §15.2) but green-node construction
  details stay private.
- Create `examples/glass-mountain.musa` reduced to this prompt's grammar (drop the
  `motif`/`performance`/`studio` blocks if you chose not to parse them; keep the
  two-part texture). Create `examples/invention.musa` (monophonic, one part).

## Target

- `musa-language`: event-based parser, Rowan tree builder, typed wrappers, `parse`.
- `insta` snapshot tests: CST shape for both examples, diagnostic output for a set of
  malformed inputs (missing `;`, unclosed `{`, bad duration, unknown keyword), recovery
  continuing after each error class.
- Trivia round-trip property: `parse(source).syntax().text() == source` for generated
  inputs.

## Check

```sh
cargo nextest run -p musa-language
cargo clippy --all-targets -p musa-language -- -D warnings
cargo fmt --check
cargo insta review        # snapshots reviewed and accepted
```

Commit as `Add lossless parser and CST for core grammar`.

## Stop

- No formatter (prompt 04), no semantic analysis or name resolution (prompt 05).
- No motif expansion logic — parsing declarations is the most you may do here.
- No Pratt parser for signal expressions; `studio` blocks are not in scope.
- Do not add `chumsky` or any parser combinator/generator dependency (roadmap §10.2).
