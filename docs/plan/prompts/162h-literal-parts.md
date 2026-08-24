---
id: 162h
slug: literal-parts
status: pending
depends_on: [161]
phase: 3
---

# A Literal's Parts Are Nodes, Because the Reader Already Found Them

## Task

`c#5` is one token. `M3` is one token. `3/8` is one token. The lexer's regexes found the letter, the accidental run, the
octave, the quality, the size, the numerator and the denominator on the way past, and then threw all of it away and kept
the text. So a consumer that wants a part re-derives it from the spelling — which root `AGENTS.md` names as *the* shape
of the mistake, in those words: "An adapter re-parsing `3/8` out of a token's spelling is the shape of the mistake."
`stdlib/src/adapters/staff.musa` says the same thing from the other side, in the list at the top of the file: "**A tie's
continuation is checked by spelling, not by pitch**, there being no operation that reads a pitch out of a token". Give
the three composite literals the structure the reader already computed: parts that are nodes, readable by the operations
that read nodes, and spliceable in a quote.

## Read

- Root `AGENTS.md`, **Standards**, "Hand a consumer what we already computed" — the whole bullet, and its `3/8` example,
  which is this prompt's warrant.
- `crates/musa-syntax/src/lexer.rs` lines 152–162 — the three regexes, and the comment explaining why the letter comes
  first (`b2` is B, `bb2` is B flat). The parts are *in* the regex; nothing new has to be discovered to find them.
- `crates/musa-syntax/src/syntax_kind.rs` — where `PitchLiteral` sits today as a token kind, and what a composite node
  kind looks like beside it.
- `crates/musa-syntax/src/parser/` — the sixteen sites that mention `PitchLiteral`. Read them for the distinction the
  Design turns on: almost all of them *test* the token, and only a few *write* it into the tree.
- `stdlib/src/adapters/staff.musa`, the file-head list, the tie bullet in particular; and `key_named`, which is the one
  construction that does not splice the composer's own token and says why.
- `docs/rules/language/11-quotation.md` §2's splice grammar and its rule that "A splice stands where a **whole node**
  stands and never inside one. There is no way to build the identifier `abc` out of `$a` and `bc`, because a splice is
  not string concatenation". That rule is *kept*, and the second half of this prompt is what it means once a part is a
  whole node.
- `editors/tree-sitter-musa` and the drift law that holds it to the real lexer — the grammar has to grow the same
  structure or the law fails.
- `crates/musa-syntax/src/formatter/` — a lossless CST means the printed text is unchanged, and the formatter is where
  that is proved.

## Design

**The lexer does not change.** One token, longest match, whitespace-significant: `c#5` is a pitch and `c # 5` is three
things, and that stays true because it is the lexer that decides it. A design that lexed the parts separately would have
to re-fuse them by adjacency and would get `bb2` wrong the first time someone touched it.

**The parser writes the parts.** Where a pitch, interval or rational token is bumped into the tree today, it becomes a
node of the corresponding kind whose children are the token's own substrings, each a token node with its own range and
its own kind. The node's text is the concatenation of its children, so the CST stays lossless and every printed byte is
unchanged. The parts are:

| Literal | Parts |
| --- | --- |
| `c#5`, `bb-1` | letter, accidental run (absent where there is none), octave — the octave's sign belongs to the octave |
| `M3`, `dim7`, `AA4` | quality, size |
| `3/8` | numerator, the slash, denominator |

**Testing the token stays testing the token.** `self.at(SyntaxKind::PitchLiteral)` asks about the *lexer's* token and
the lexer is unchanged, so the sixteen parser sites that test it are untouched; what changes is the handful that write
it. That is the whole reason this is tractable, and an implementation that finds itself editing all sixteen has taken
the other design by accident.

**A part is a whole node, so a splice may stand at one — and the result is re-lexed.** `quote at here { $letter#5 }` is
admitted because a letter is now a node, and §2's rule is untouched: a splice still stands where a whole node stands.
What guards it is not the splice rule but a check at the literal: the assembled spelling must lex as one token of the
kind the literal claims, and a splice that makes `c#x5` is refused, naming the part and the literal. This is the same
discipline as `check_expression` one level down — build freely, then prove the reader would have read it.

**No new reading operations.** A part is a node and the operations that read nodes read it. If the implementation wants
`syntax_pitch_letter`, the parts are not really nodes and the design is wrong.

**The tie stays as it is.** Fixing `staff.musa`'s spelling comparison is prompt [166](166-staff-rewrite.md)'s; this
prompt makes it possible and says so in the commit message.

## Target

- `PitchLiteral`, `IntervalLiteral` and `Rational` are composite CST nodes with the parts above; the lexer, the printed
  bytes, and every parser test of the token kind are unchanged.
- Splicing at a part, with the re-lex check and a refusal that names the part and the literal.
- `editors/tree-sitter-musa` grows the same structure, with corpus entries, and the drift law passes.
- Laws in `musa-syntax`: node text is the concatenation of the parts for a corpus covering `b2`, `bb2`, `bbb2`, `cn4`,
  `c#-1`, `M3`, `dim7`, `AA4`, `d2`, `3/8`, `12/16`; the formatter round-trips each unchanged; a part carries the range
  it occupies in the source; a spliced literal that would not lex is refused.
- Laws in `musa-compiler`: a pitch literal elaborates to the same term it does today, checked against the existing
  fixtures rather than new ones.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-syntax -p musa-compiler
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test -p musa-syntax -p musa-compiler --unreferenced=reject
pnpm -r test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

CST snapshots move — that is the point of the prompt and each moved snapshot must show parts and nothing else. Rendered
output, `tests/fixtures/elaboration-compatibility.txt`, and every `examples/` rendering must be byte-identical; a moved
byte there is a failure.

`cargo nextest run --run-ignored all` carries the reds prompt [164](164-builtin-collapse.md)'s Check enumerates; a *new*
red is this prompt's.

Commit as `Give a pitch, an interval and a rational the parts the lexer found`.

## Stop

- The lexer does not change. No separate tokens, no adjacency re-fusing, no whitespace-insensitive literal.
- Three literals. Not `Float`, not `String`, not `Integer`, not the key literal — a key is already three tokens
  (`staff.musa`'s note at `key_named` says so).
- No per-part reading builtin, and no `Pitch`-valued reader of a node. A part is a node; that is the deliverable.
- No change to §2's splice rule, to hygiene, or to what a quote builds at.
- No adapter rewrite. `staff.musa`'s tie comparison and `doubled.musa`'s builders are [166](166-staff-rewrite.md)'s and
  [167](167-studio-rewrite.md)'s.
