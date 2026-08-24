---
id: 162g
slug: value-literals-as-syntax
status: pending
depends_on: [161]
phase: 3
---

# A Computed Number Can Be Written Into a Quote

## Task

An adapter that has computed a `Nat` and wants it to appear in the syntax it builds has two ways to do it, and both are
bad. Inside a quote it must enumerate: `stdlib/src/adapters/staff.musa`'s `dot_count` is four `quote at here { … }`
bodies holding `0`, `1`, `2` and `3`, and it stops at three because someone decided how many dots a note gets. Outside a
quote it must drop to the token builders and hand-allocate provenance: `stdlib/src/adapters/doubled.musa` writes
`syntax_token(syntax_built(here, 8, 0), TokenKind::Integer, "2")` and carries thirteen role integers, `0` through `12`,
that mean nothing and must not collide. Add the two operations that close it — a `Nat` and a `Text` each become the one
expression node that denotes them — and make `11-quotation.md` §2's "there is no operation from `Text` to `Syntax c`"
say precisely what it means.

## Read

- `docs/rules/language/11-quotation.md` §2, and in particular its closing **"What a quote is not"** paragraph: "It is
  not `eval`: … there is no operation from `Text` to `Syntax c`. It is not a procedural macro over a token stream: the
  body is parsed, so an adapter cannot assemble syntax the grammar does not admit." Both refusals are right and this
  prompt keeps both; what it changes is the sentence, which currently forbids by shape what it means to forbid by
  *capability*.
- `docs/rules/language/11-quotation.md` §1 and §3 — the `Cat` index and the provenance the elaborator computes. §3 is
  why the new operations take a `NodePath` and why an adapter should never be writing `syntax_built(here, 8, 0)` by
  hand.
- `stdlib/src/adapters/staff.musa`'s `dot_count` (four quotes for four numbers) and `stdlib/src/adapters/doubled.musa`
  lines 138–157 (the thirteen hand-allocated roles). The two are the same gap seen from either side of the quote
  boundary, and the prompt is not justified without both.
- `crates/musa-compiler/src/phase/ownership.rs` — `syntax_number`, `syntax_token`, `syntax_identifier`, `syntax_group`,
  `syntax_built`. The reading direction is complete and the writing direction stops at `TokenTree`; the ownership table
  is where the two new entries go, with what a library could not express.
- `crates/musa-compiler/src/quote.rs` — `check_expression`, the gate every expansion answer passes, and what it means
  for a built node to be an expression.
- `crates/musa-syntax/src/lexer.rs` — how an integer and a string literal are spelled, which is what the escaping
  obligation below is stated against.
- Prompt [164](164-builtin-collapse.md)'s Design — spellings live on carriers. These two are declared as builtins here
  because 164 has not run; 164's Stop forbids adding one, so this prompt lands first or the entries are 164's to place.

## Design

**The value is the node, so nothing is parsed.** `syntax_numeral(here, n)` is one integer-token expression node spelling
`n`; `syntax_text(here, t)` is one string-literal expression node whose contents are `t`. Neither reads its argument as
source: there is no grammar consulted, no lexer run over the value, and no way for the argument to decide what *kind* of
node comes out. That is the difference between this and `eval`, and it is a difference in capability rather than in type
— which is why §2's sentence has to be rewritten rather than merely excepted.

**The compiler writes the escape, and that is the load-bearing half.** A `Text` holding a quote mark, a backslash, or a
newline becomes a string literal that reads back as exactly that text. An adapter never composes the spelling, so there
is no injection to get wrong; the round-trip law states it. Without this the operation *would* be `eval` with extra
steps, and this paragraph is the reason it is not.

**Both answer `Syntax<Expr>`.** Construction is fixed at `Expr` (§2), a `TokenTree` position takes the value through the
`forget` written at the site, and neither operation gets a category argument. They are ordinary splice arguments:
`quote at here { NoteValue($numeral, ${ syntax_numeral(here, dots) }) }`.

**Provenance is the elaborator's, exactly as a quote's is.** They take a `NodePath` and the node they build is *built*
provenance derived from it (§3), by the same rule a quote body's nodes get theirs. No role integer crosses the boundary.
`doubled.musa`'s thirteen are a symptom, and a design that left an adapter allocating even one would have missed what
the evidence was saying.

**No `Ratio`.** `ratio_literal` already answers `Option<Text>` because the grammar has no literal below zero, and an
operation that answers `Option<Syntax<Expr>>` would put a refusal path at every splice site. A negative exact span is
built from the numerals it is made of, and a prompt that wants a one-node ratio literal should first argue the grammar
should have one.

## Target

- `syntax_numeral(here: NodePath, value: Nat) -> Syntax<Expr>` and `syntax_text(here: NodePath, value: Text) ->
  Syntax<Expr>`, registered, with their `BUILTIN_OWNERSHIP` rows and the hidden information each names.
- `docs/rules/language/11-quotation.md` §2's "What a quote is not" paragraph rewritten: what stays forbidden is an
  operation that *parses* a text, and the two literal constructions are named as what is admitted and why they are not
  that.
- Laws in `musa-compiler`: `syntax_number(forget(syntax_numeral(here, n))) = n/1` for a spread of `n` including zero and
  a large value; a `Text` holding a quote, a backslash, a newline and a `$` round-trips through `syntax_text` and the
  reader; both nodes pass `check_expression`; both carry built provenance derived from the `here` they were given; a
  spliced numeral and a written one elaborate to the same term.
- `dot_count` in `stdlib/src/adapters/staff.musa` becomes the one-line function it is, with the twenty-five
  `staff_expansion_laws` unedited and passing — that is the evidence, and the commit message carries the before/after.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-compiler -p musa-syntax
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test -p musa-compiler --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

```sh
cargo nextest run --run-ignored all -E 'test(staff_expansion)'
```

Twenty-five run, twenty-five pass, none of them edited.

`cargo nextest run --run-ignored all` and `cargo insta test --workspace` carry the reds prompt
[164](164-builtin-collapse.md)'s Check enumerates; a *new* red is this prompt's.

Commit as `Let a computed number and a computed text be written as syntax`.

## Stop

- No operation that parses. No `Text -> Syntax` that reads the text as source, no `eval`, no token-stream splicing, no
  identifier built from a text — `01-surface.md`'s hygiene rule and §2's "a splice stands where a whole node stands"
  both depend on it and neither is re-opened here.
- No `Ratio`, no `Pitch`, no `Interval`, no third literal of any kind. Two operations.
- No new category, no change to `Cat`, no change to `forget`.
- No sweep of the adapters. `dot_count` is the evidence; `doubled.musa`'s thirteen roles come out when
  [166](166-staff-rewrite.md) and [167](167-studio-rewrite.md) rewrite what builds them.
- No change to how a quote body is parsed or to `check_expression`'s gate.
