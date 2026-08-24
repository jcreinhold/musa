---
id: 162ha
slug: a-literal-is-one-lexeme
status: in-progress
depends_on: [162h]
phase: 3
---

# The Phase Learns a Node That Is One Lexeme

## Task

Prompt [162h](162h-literal-parts.md) gives `c#5`, `M3` and `3/8` the parts the lexer found, in the CST, and stops at the
expansion phase's door — because the phase's `Syntax` has four shapes and none of them is *a node that is one lexeme*. A
group is written back with one space between siblings, so a composite literal grouped as one prints as `c # 5`, and
`as_expression` refuses the composer's own pitch. That is not a hypothetical: 162h left `quote at here { c5/4 }`
building a layout group of three parts, and no adapter writes one today, which is the only reason the suite is green.
Give the phase the fifth shape, and the gate that keeps it honest: a fused group's text must lex as exactly one token of
one of the three composite kinds.

**The reading side is [162hb](162hb-a-region-arrives-in-parts.md)'s**, and the Design below says why: a region's literal
cannot become a group until the adapter that reads regions can read one.

## Read

- [162h](162h-literal-parts.md)'s Design, the paragraph **"The phase sees the token it sees today"** — this prompt is
  the decision it defers, and the boundary it draws is what this one crosses. It names prompt 166 as the place the
  adapter's share lands, and 166 is `done`; [162hb](162hb-a-region-arrives-in-parts.md) is where it actually lands.
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
- `crates/musa-syntax/src/lexer.rs`'s pitch, interval and rational patterns — the reason a splice cannot stand at a
  part. Measured rather than argued: `$letter#5` lexes as `Dollar Identifier Hash Integer` and `c#$octave` as
  `Identifier Hash Dollar Identifier`, so there is no `PitchLiteral` token for 162h's splitter to be handed and no
  literal node for a splice to sit inside. A part is a node the parser mints *from a lexeme the lexer already read*,
  which is a different thing from a position the grammar admits.
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

**A quote body that holds a literal builds one, and that is what this prompt turns on.** The template walk and the
region reader ask one function what a node's delimiter is, and it answers `Fused` for the three kinds. On the template
side that repairs a live regression: since 162h, `quote at here { c5/4 }` builds a layout group of `c`, `5`, `/`, `4`
and prints `c 5 / 4`, which the parser reads as four things. Nothing in `stdlib/` quotes a literal today, so nothing
caught it; a law here does.

**Reading waits for the reader that must read it.** `read_node` keeps handing a region's composite literal over as one
token. Turning it over is one line and it was tried: 37 tests fail, because `staff.musa`'s `entering` answers `Ignoring`
for a delimiter it does not know and `group_read` drops that group's children — so every pitch in every region is
*silently dropped* and the adapter refuses the region with "this staff field says nothing". The representation change
and the adapter that reads it are one commit, and it is [162hb](162hb-a-region-arrives-in-parts.md)'s. What this prompt
leaves behind is a stated asymmetry for exactly one prompt: a quote builds a literal as a fused group and a region still
delivers one as a token. 162h left a worse one — a quote built it as a *layout* group — and this replaces it with the
shape the reader will produce.

**There is no splice at a part, and the Read section measures why.** `$letter#5` is four tokens; the lexer never offers
the parser a `PitchLiteral` to split, so there is no node for a splice to stand inside and nothing here changes that. A
literal's parts are reachable by *reading* one (162hb) and writable by naming their kinds (`syntax_token(here,
TokenKind.PitchLetter, "c")`), and assembling those into a fused group is what the gate below judges. The earlier draft
of this prompt claimed the splice followed from 162h; it does not, and a lexer change to make it follow is refused in
the Stop list.

**The token kinds an adapter may name.** 162h's part kinds are produced by the parser and not by the lexer, so
`quote/category.rs`'s drift law — "every kind the lexer can produce appears here, and no parser node kind does" — has to
say what they are. They are neither: they are tokens the parser mints from a lexer token's substrings. The law grows a
third answer and the parts become nameable, because an adapter that can read a numerator and cannot say
`TokenKind.RationalNumerator` about it has been handed half an operation. `musa_syntax::SyntaxKind::is_literal_part` is
the third answer, beside `is_composite_literal`, and a law in `musa-syntax` holds it to the splitter that mints them.

## Target

- `Delimiter::Fused`, in `ALL`, `name`, `pair`, and `named`, spelled `Delimiter::Fused` in a phase module.
- `print.rs` writes a fused group's children with no separator, and its doc comment states the two-delimiter split.
- `check_expression` gains a third refusal: a fused group whose text does not lex as exactly one token of one of the
  three composite kinds, naming the text and what it lexed as.
- `crate::quote::delimited` answers `Fused` for the three kinds, so the quote template and the region reader cannot
  disagree about what a literal is grouped as — and the template stops building the layout group 162h left it building.
- `musa_syntax::SyntaxKind::is_literal_part`, with a law in `musa-syntax` holding it to the kinds `parser/literals.rs`
  actually mints.
- `quote/category.rs`'s `TOKEN_KINDS` and its drift law admit the part kinds, with the law stating the three-way split
  it now checks; the same law in `crates/musa-compiler/tests/suite/quote_category_laws.rs` has an adapter write one.
- `docs/rules/language/11-quotation.md` §2 gains the fifth shape and the gate, written as §2 writes its other rules —
  what it is, what it is for, and what it does not re-open.
- Laws in `musa-compiler`: `quote at here { c5/4 }` builds a fused group and prints back the four characters it was
  written with, for a pitch, an interval and a rational; a built fused group that assembles `c#x5` is refused and the
  message names it; a fused group assembling `abc` is refused *because `Identifier` is not one of the three*, which is
  §2's rule restated as a test; a fused group built from the parts of a real pitch passes `as_expression`.

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

Nothing a region is read as changes, so no expansion snapshot moves, `tests/fixtures/elaboration-compatibility.txt` is
byte-identical, and every `examples/` rendering is byte-identical. A moved snapshot here is evidence that the reading
side crossed the boundary, which is [162hb](162hb-a-region-arrives-in-parts.md)'s.

```sh
cargo nextest run --run-ignored all
```

Carries the reds prompt [164](164-builtin-collapse.md)'s Check enumerates; a *new* red is this prompt's.

Commit as `Let a group be one lexeme, and check that it is`.

## Stop

- Five delimiters. Not one per literal, not a delimiter carrying a kind, not an open vocabulary. The gate says which
  lexemes are admitted; the delimiter says only how the children are written.
- No change to §2's splice rule, to hygiene, or to what a quote builds at. The gate is where a fused group is judged.
- No change to `read_node`, and therefore none to any adapter. A region still hands its literals over as tokens until
  [162hb](162hb-a-region-arrives-in-parts.md).
- No change to the lexer, and no fourth composite literal. A splice at a part would need one and is refused here.
- No new reading operation of any kind.
