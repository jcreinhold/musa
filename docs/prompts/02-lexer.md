---
id: 02
slug: lexer
status: done
depends_on: [01]
phase: 1
---

# Lexer

## Task

Implement the `musa-language` lexer: a `logos`-based tokenizer for the `.musa` language that preserves **all** trivia
(whitespace, line and block comments) so a lossless syntax tree and a faithful formatter are possible later.

## Read

- Roadmap §7 (language shape), §10.1 (lexer requirements and the `SyntaxKind` sketch).
- The example program in §7.1 — every token class it exercises must lex.

## Design

- Add dependencies to `musa-language`: `logos`, `thiserror`, `miette` (miette is used at the CLI boundary later; the
  lexer itself returns plain error values — see Stop).
- Public surface (all in `musa_language` root, internals private):

  ```rust
  /// Lex `source` into tokens. Trivia tokens are emitted, never discarded.
  pub fn lex(source: &str) -> Lexed;

  pub struct Lexed { /* private */ }
  impl Lexed {
      pub fn tokens(&self) -> &[Token];
      /// Lexical errors with source spans; lexing always continues past an error.
      pub fn errors(&self) -> &[LexError];
  }

  pub struct Token { pub kind: SyntaxKind, pub range: TextRange }
  ```

  `TextRange`/`TextSize` come from `rowan`'s text-size re-export or a small local
  newtype; pick one and keep it consistent — the parser prompt builds on it.
- `SyntaxKind` is a `#[repr(u16)]` enum covering: trivia (`Whitespace`, `LineComment`, `BlockComment`), literals
  (`Identifier`, `Integer`, `Rational`, `String`, `PitchLiteral`, `UnitNumber` + unit suffixes
  `Hz`/`ms`/`s`/`dB`/`bpm`), punctuation (`LBrace RBrace LBracket RBracket LParen RParen Semicolon Comma Colon Arrow
  PipeForward Equals`), and the keywords the §7.1 example uses (`piece tempo meter key motif score part voice clef use
  transpose down up rest chord performance profile articulation studio patch oscillator envelope output lfo modulate bus
  assign route send master at gain mix scale bias lowpass reverb carrier` — trim to what the example and roadmap
  actually show; do not invent keywords for Phase 3 features).
- Pitch literals: a letter `a`–`g`, optional accidental (`s`/`ss` for sharp, `f`/`ff` for flat per the `gs4` example;
  natural `n`), octave digit(s) — e.g. `c5`, `gs4`, `bf3`. If this regex is too entangled with identifiers to be one
  logos token, lex it as separate tokens and note the decision in the module docs; the parser reassembles it. Do not
  silently pick the second option.
- Durations like `1/4` lex as one `Rational` token; a bare `1` lexes as `Integer`.
- Errors: unterminated strings/comments and unrecognized characters produce `LexError` with span and are skipped; lexing
  never aborts (roadmap §10.3's always-returns-a-tree policy starts here).

## Target

- `crates/musa-language/src/{lib.rs,syntax_kind.rs,lexer.rs}` (module layout is the worker's choice; the public items
  above are not).
- Unit tests: each token class, trivia round-tripping (`concat(tokens.text) == source`), pitch/duration/unit edge cases,
  error recovery continuation.

## Check

```sh
cargo nextest run -p musa-language
cargo clippy --all-targets -p musa-language -- -D warnings
cargo fmt --check
```

Commit as `Add musa-language lexer`.

## Stop

- No parser, no CST, no rowan tree building (prompt 03).
- No keyword or token for Phase 2/3 constructs (ties, slurs, `stretch`, `invert`, `harmony`, imports) unless §7.1
  already shows them.
- No `miette` diagnostic rendering — plain error data only.
