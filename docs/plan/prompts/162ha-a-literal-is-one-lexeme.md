---
id: 162ha
slug: a-literal-is-one-lexeme
status: pending
depends_on: [162h]
phase: 3
---

# The Phase Learns a Node That Is One Lexeme

## Task

Prompt [162h](162h-literal-parts.md) gives `c#5`, `M3` and `3/8` the parts the lexer found, in the CST, and stops at the
expansion phase's door — because the phase's `Syntax` has four shapes and none of them is *a node that is one lexeme*. A
group is written back with one space between siblings, so a composite literal read as a group prints as `c # 5`, and
`as_expression` refuses the composer's own pitch. Give the phase the fifth shape, and the gate that keeps it honest: a
fused group's text must lex as exactly one token of one of the three composite kinds. Then an adapter can read a
rational's numerator apart from its denominator, and `staff.musa`'s own note — that `time 4/4` is unwritable because
`4/4` and `1/1` arrive identical — stops being true.

## Read

- [162h](162h-literal-parts.md)'s Design, the paragraph **"The phase sees the token it sees today"** — this prompt is
  the decision it defers, and the boundary it draws is what this one crosses.
- `crates/musa-compiler/src/quote/category.rs` — `Delimiter`, its four cases, `ALL`, `name`, `pair`, and `named`. The
  vocabulary a fifth case joins, and the three places that have to agree.
- `crates/musa-compiler/src/quote/build.rs`'s `delimited` and `read.rs`'s `read_node` — where a CST node becomes a group
  and which delimiter it gets.
- `crates/musa-compiler/src/quote/print.rs`'s `write_syntax` and its doc comment: "The spacing is uniform, one space
  between siblings, because nothing reads this text for its shape." That sentence is true of a whitespace-insensitive
  grammar and false of these three literals, which is the whole reason this prompt exists.
- `crates/musa-compiler/src/quote/gate.rs` — `check_expression` and `NotAnExpression`. The gate this prompt adds a third
  question to, and the shape a refusal takes: a case per mistake, so a transformer author is told which one they made.
- `docs/rules/language/11-quotation.md` §2, the splice rule — "A splice stands where a **whole node** stands and never
  inside one. There is no way to build the identifier `abc` out of `$a` and `bc`, because a splice is not string
  concatenation." **That rule is kept, and the Design below is the argument that it survives a fused group.** §2 is also
  where the fifth shape is written down.
- `stdlib/src/adapters/staff.musa`'s file-head, the paragraph beginning "For a related reason `time (4, 4)` is two
  numbers rather than `4/4`" — the standing evidence, in the adapter's own words.
- `crates/musa-compiler/src/quote/tests.rs`'s `a_group_that_names_no_real_delimiter_does_not_check` and
  `crates/musa-compiler/src/registry/rules.rs`'s `SyntaxOp::DelimiterEqual` — the two places the closed vocabulary is
  checked and compared.
- Peyton Jones ch. 3 §3.1 — the argument for adding a construct to the vocabulary rather than special-casing the
  translator, and the discipline that goes with it: for each construct, say what it looks like and say what it means. A
  printer that special-cased three node kinds would be the other choice, and it would put the fact in the code that
  writes the text rather than in the value being written.

## Design

**Say what it looks like.** `Delimiter::Fused` is a fifth case: its pair is empty, like `Layout`'s, and its children are
written back with **nothing** between them. `Layout` and `Fused` are the two delimiters that open and close nothing, and
they differ in exactly one thing — whether the children are separate words.

**Say what it means, and the meaning is a gate.** A fused group is admitted only when its text lexes as exactly one
token, and that token's kind is one of `PitchLiteral`, `IntervalLiteral`, `Rational`. `check_expression` asks it, and
the refusal names the group's assembled text and what it lexed as instead. Everything else about the phase is unchanged:
a transformer builds freely and the gate says whether the reader would have read it, which is the discipline the gate
already runs on output paths and binders.

**§2's rule survives, and this is the argument.** A fused group of `$a` and `bc` assembles to `abc`, which lexes as one
token — and its kind is `Identifier`, which is not one of the three, so the gate refuses it. Hygiene's guarantee is that
an adapter cannot build a name out of pieces, and that guarantee is untouched: the three admitted kinds carry no binding
and no scope. The rule that changes is not §2's; it is that "one node, one lexeme" was previously unrepresentable and is
now representable for exactly the three literals whose lexeme has parts.

**Reading is symmetric.** `read_node` reads a composite literal node as a `Fused` group over its part tokens, replacing
162h's flattening comment. An adapter then folds into a pitch and meets a letter, an accidental and an octave as
ordinary token children, with ordinary paths — no reading operation, exactly as 162h's Design requires.

**Splicing at a part follows, and needs nothing new.** `quote at here { $letter#5 }` parses as a pitch whose letter is a
splice, because 162h made the letter a whole node and §2's rule was always "a splice stands where a whole node stands".
What guards it is the gate above: a splice that makes `c#x5` is refused, naming the literal and what it lexed as.

**The token kinds an adapter may name.** 162h's part kinds are produced by the parser and not by the lexer, so
`quote/category.rs`'s drift law — "every kind the lexer can produce appears here, and no parser node kind does" — has to
say what they are. They are neither: they are tokens the parser mints from a lexer token's substrings. The law grows a
third answer and the parts become nameable, because an adapter that can read a numerator and cannot say
`TokenKind::RationalNumerator` about it has been handed half an operation.

**Measure the evidence rather than assert it.** The commit message carries a `time 4/4` region read by the staff
adapter's own reading path, showing the numerator and denominator arriving apart. Rewriting `staff.musa` to accept it is
prompt [166](166-staff-rewrite.md)'s; showing that it now can is this prompt's.

## Target

- `Delimiter::Fused`, in `ALL`, `name`, `pair`, and `named`, spelled `Delimiter::Fused` in a phase module.
- `print.rs` writes a fused group's children with no separator, and its doc comment states the two-delimiter split.
- `check_expression` gains a third refusal: a fused group whose text does not lex as exactly one token of one of the
  three composite kinds, naming the text and what it lexed as.
- `read_node` reads the three composite literal nodes as fused groups, and 162h's flattening comment comes out.
- `quote/category.rs`'s `TOKEN_KINDS` and its drift law admit the part kinds, with the law stating the three-way split
  it now checks.
- `docs/rules/language/11-quotation.md` §2 gains the fifth shape and the gate, written as §2 writes its other rules —
  what it is, what it is for, and what it does not re-open.
- Laws in `musa-compiler`: a composer's pitch spliced through `as_expression` round-trips unchanged; a fused group
  assembling `c#x5` is refused and the message names it; a fused group assembling `abc` is refused *because `Identifier`
  is not one of the three*, which is §2's rule restated as a test; a numerator and a denominator are read apart from one
  another by an ordinary fold; `quote at here { $letter#5 }` builds the pitch it looks like.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-compiler -p musa-syntax
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Expansion snapshots move where a composite literal is inside a region, and each moved one must show a fused group and
nothing else. `tests/fixtures/elaboration-compatibility.txt` and every `examples/` rendering must be byte-identical.

```sh
cargo nextest run --run-ignored all
```

Carries the reds prompt [164](164-builtin-collapse.md)'s Check enumerates; a *new* red is this prompt's.

Commit as `Let a group be one lexeme, and check that it is`.

## Stop

- Five delimiters. Not one per literal, not a delimiter carrying a kind, not an open vocabulary. The gate says which
  lexemes are admitted; the delimiter says only how the children are written.
- No change to §2's splice rule, to hygiene, or to what a quote builds at. The gate is where a fused group is judged.
- No new reading operation. A part is a token child and the fold reads it, which is [162h](162h-literal-parts.md)'s
  deliverable and this prompt's obligation to leave intact.
- No adapter rewrite. `staff.musa`'s `time (4, 4)` and its tie comparison are [166](166-staff-rewrite.md)'s.
- No change to the lexer, and no fourth composite literal.
