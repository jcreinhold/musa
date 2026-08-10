---
id: 112
slug: braced-function-bodies
status: done
depends_on: [110, 111]
phase: 3
---

# A Function Body Is a Block Expression

## Task

Replace `fn f(x: τ) -> υ = e;` with `fn f(x: τ) -> υ { e }`. The braces are a block expression — `{ e }` elaborates to
`e` and holds exactly one expression — so a function body is delimited the way every other body in the language is,
without adding a statement form, a `let`-in-block, a `return`, or a second sequencing rule.

## Read

- `docs/language-correction.md` §6, which governs this prompt, and §1 fault 6 for why the first draft of that document
  argued the opposite and why the argument was wrong.
- `docs/language/02-core-calculus.md` §5, whose fragment gains one derived form; §5's normalization proof must be shown
  to be undisturbed, not merely asserted to be.
- `docs/language/01-surface.md`'s `fn` production and the formatter rules that render it.
- Prompt 109 for the migration-diagnostic shape a syntax change takes here, including its applicable fix.
- Prompt 80 for the tree-sitter drift law.
- Prompt 93's frozen compatibility baseline and `docs/language/README.md`'s graduation criterion 2.

## Design

`fn` is the only declaration in the language whose body is not braced. `piece`, `library`, `voice`, `motif`,
`fragment`, `signature`, `structure`, and `music` all write their contents between braces; `fn` alone writes `= e;`.
The inconsistency was defended as forced by the absence of a statement language, and that defense confuses a block with
a statement sequence. A block is a delimiter. Rust's function body is a block *expression* whose value is its trailing
expression, and that is exactly the shape adopted here.

The block form is deliberately minimal, and its minimality is the whole safety argument:

```
block := "{" expr "}"
fn-decl := "fn" IDENT params "->" type block
```

A block holds **one** expression. Two expressions in a block is a parse error whose message says so and names the rule,
rather than a silent sequence. There is no `let` inside a block, no `return`, no early exit, and no statement form of
any kind. `⟦{ e }⟧ = ⟦e⟧` is a definitional expansion, so `02-core-calculus.md` §5's preservation, progress,
determinism, and strong-normalization results carry over with no new case — and §5 must say that in writing, because a
derived form that is only *claimed* to be conservative is exactly the fault prompt 108 was written to end.

The block is admitted as a general expression form, not as a special case of `fn`. `{ e }` is grammatical wherever an
expression is, because a rule with one exception is two rules, and because `match` arms and `if` branches read better
with it available. It gains no meaning there: parenthesizing an expression in braces does nothing.

Two consequences look like objections and are not. A `music`-valued function reads
`fn triad(register: frame) -> music { music { … } }`; the doubled brace is honest, since the outer delimits the
function and the inner is a `music` value, and they are two different things that happen to abut. And a `match` body
loses its trailing semicolon, which was the one place the old form read well — inside braces it reads better, next to
every other `match` in the language.

This prompt runs before the theory block resumes because it touches every function in `stdlib/`, and the standard
library grows with every prompt after it. Migrating fifty functions now is cheaper than migrating two hundred later.

The old spelling becomes a hard error with a located applicable fix, on prompt 109's precedent. The fix is mechanical —
`= e;` becomes `{ e }` — which is what makes a hard error affordable for a break this wide.

## Target

- The `block` expression form in the lexer's existing braces, the parser, a CST node kind, and the formatter; the `fn`
  production requiring one.
- The one-expression rule as a diagnostic with a snapshot test: two expressions in a block names the rule and does not
  silently sequence.
- The migration diagnostic for `fn … = e;`, with a located applicable fix and a snapshot test.
- Every `fn` rewritten across `examples/`, `stdlib/`, and the test corpora — including the `.musa` fixture strings
  embedded in Rust tests, which the corpus tools do not reach.
- `docs/language/02-core-calculus.md` §5 gains the derived form and the sentence discharging it; `01-surface.md`'s `fn`
  production and its formatter rules are repaired.
- Formatter rendering plus its idempotence and round-trip laws: a braced body on one line when it fits, indented when
  it does not.
- `editors/tree-sitter-musa` grammar, corpus, and queries updated, with the lexer drift law green.
- LSP completion, semantic tokens, signature help, and the generated desktop highlight fixtures.
- Prompt 93's frozen compatibility baseline updated for this deliberate break, with the break *named* in the baseline
  rather than absorbed into it.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-lsp -p musa-project
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-lsp -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/tonal-construction.musa
cargo run -p musa -- check examples/neo-riemannian.musa
cargo run -p musa -- format examples/tonal-construction.musa --check
cargo run -p musa -- render examples/canon.musa --to musicxml -o /tmp/canon.musicxml
npm --prefix editors/tree-sitter-musa test
```

Commit as `Give a function body braces`.

## Stop

- No statement language. No `let` in a block, no `return`, no `;`-separated sequence, no early exit, and no block with
  two expressions.
- No anonymous functions, lambdas, or closures; `02-core-calculus.md` §5's "a checked named `fn` supplies the lambda"
  is unchanged, and this prompt only changes how one is delimited.
- No change to `let` declarations, signature members, or any other `=`; this is about `fn` alone.
- Do not accept both spellings, and do not add a compatibility flag or edition mechanism to allow the old one.
- No change to type syntax, parameter defaults, or the arrow type.
