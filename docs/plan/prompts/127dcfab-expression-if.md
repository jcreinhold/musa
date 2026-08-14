---
id: 127dcfab
slug: expression-if
status: pending
depends_on: [127dcfaa]
phase: 3
---

# Give the Surface the Conditional the Core Already Claims

## Task

`docs/rules/language/02-core-calculus.md` §1 lists "conditionals" among the language's terms. The surface grammar has
none: `docs/rules/language/01-surface.md` never mentions `if`, and an author who wants one writes a two-arm `match` on a
boolean. That is drift between a governing page and the language it governs, and it is paid at every value decision in
the standard library — `stdlib/src/adapters/staff.musa`'s `clef_named` is four `match text_equal(…)` frames deep to ask
four questions in a row, each frame indenting the next.

This prompt adds an expression `if` to the surface, elaborating to the boolean `match` that already exists, and makes §1
say what the core actually has. It adds no core term, no typing rule, and no reduction.

## Read

- `docs/rules/language/02-core-calculus.md` §1, the sentence beginning "Terms are variables, literals, …", which names
  conditionals; §5.1, whose typing rules do not; and §5's match rule, the paragraph beginning "A match checks every arm
  under the bindings introduced by its pattern", which already gives boolean matching its meaning. These are the pages
  this prompt reconciles, and it reconciles them in this commit before any code changes —
  [`../../rules/README.md`](../../rules/README.md) calls that the ordinary way to change a page below `constitution.md`
  and `obligations.md`.
- `docs/rules/language/02-core-calculus.md` §6.2 — patterns are depth one, with no guard and no conditional equation.
  This prompt must leave that invariant exactly where it found it.
- `docs/rules/language/01-surface.md` — the expression grammar this prompt extends.
- `docs/notes/research/language-design-closure/39-totality-and-structural-abstraction.md` §4.2, which decides for an
  expression `if` and against pattern guards, and §3.3, whose level audit separates value decisions from structural
  recursion and failure sequencing.
- Peyton Jones ch. 4 §4.3 and ch. 5 — pattern-matching compilation, fall-through, and the match ordering rules. They are
  the subsystem an `if` deliberately does not buy: chapters 4–6 treat guarded equations and nested patterns as one
  machine, and this prompt takes none of it.
- Peyton Jones ch. 3 §3.2 — the enriched calculus. A construct an author needs is added to the surface and translated
  away, rather than left for the author to encode. That is the shape of this change.
- `stdlib/src/adapters/staff.musa`, `clef_named` at line 1155 and `spelling_named` below it — the staircase this prompt
  flattens.
- `crates/musa-language/src/parser.rs`, `ast.rs`, `formatter.rs`, `keywords.rs`, and `syntax_kind.rs`; and
  `editors/tree-sitter-musa/grammar.js` with its queries, which the drift law in root `AGENTS.md` holds to the real
  lexer.

## Design

**`if` is surface syntax that elaborates away.** The form is

```text
if condition { consequent } else { alternative }
```

with a mandatory `else`, both branches expressions, and `condition : bool`. It elaborates to the existing exhaustive
two-arm boolean match:

```text
match condition { true -> consequent, false -> alternative }
```

The elaboration happens where the surface becomes core, so the core term grammar, the typing rules of §5.1, the
reduction rules of §5.2, the strong-normalization measure, and the cost table are all untouched. A conditional costs
what its match costs, because it *is* that match. `else if` chains by the alternative being another `if`; there is no
separate `elif` and no dangling-else question, because `else` is required.

**§1 stops overclaiming.** The sentence listing terms is amended to say that a conditional is surface sugar for the
boolean match rather than a term of the core, so the inventory describes the calculus §5 actually defines. This is the
smaller repair of the two available: adding an `If` term to the core would duplicate the match rule, add a reduction and
a measure case, and give the language two ways to eliminate a boolean, which §5.6's own argument against redundant
eliminators forbids.

**Refused: pattern guards.** A guard on a match arm would flatten `clef_named` too, and it would do it by taking on the
pattern-compilation subsystem — fall-through between arms, exhaustiveness under guards, and the ordering semantics of
Peyton Jones chapters 4–6 — for a language whose patterns are deliberately depth one. §6.2's invariant is explicit that
a later prompt adding guards "has taken on" that machine. An expression `if` decides values without touching how
patterns are compiled, and a guard proposal must earn its own prompt.

**Refused: `cond`/`if` without `else`.** A one-armed conditional needs a unit result or an implicit failure, and this
language has neither in expression position.

**The formatter decides the layout, and the tree-sitter grammar follows the lexer.** `if` and `else` become keywords, so
they stop being usable as identifiers; the corpus is checked for collisions in this prompt rather than discovered later.

## Target

- `docs/rules/language/02-core-calculus.md` — §1's term sentence says a conditional is surface sugar for the boolean
  match; §5 is unchanged and the prompt states that it is.
- `docs/rules/language/01-surface.md` — the expression grammar gains `if`, with the mandatory `else`, the `bool`
  condition, the single result type, and the elaboration written out.
- `crates/musa-language/` — `syntax_kind.rs`, `keywords.rs`, `lexer.rs`, `parser.rs`, and `ast.rs` gain the form;
  `formatter.rs` lays it out and keeps a chained `else if` on one ladder rather than indenting each rung.
- `crates/musa-compiler/` — elaboration of the surface form to the boolean match, with the source map carrying the
  conditional's own span so a type error on a branch points at that branch and not at a synthesized match.
- `editors/tree-sitter-musa/grammar.js` and `queries/` — the form and its highlighting, held to the lexer by the drift
  law.
- `stdlib/src/adapters/staff.musa` — `clef_named`, `spelling_named`, and the other boolean staircases become `if`
  ladders. The expansion produces the same values and the fixtures are expected to be byte-identical.
- Tests in `crates/musa-language` and `crates/musa-compiler`: parse and format round-trip including a chain; both
  branches must have one type, with the diagnostic naming the branch that disagrees; a non-`bool` condition is rejected
  at its own span; `if` and `else` are rejected as identifiers with a diagnostic that says they are keywords; and the
  elaborated form is observationally the boolean match, evaluated by the one evaluator at the same charge.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Give the surface the conditional the core already claims`.

## Stop

- No guards on match arms, no guarded function equations, no `where`. **Design** says why, and §6.2's invariant is the
  boundary.
- No new core term, typing rule, reduction, normalization case, or cost-table entry. If the elaboration cannot be done
  without one, that is a finding to report, not a rule to add in passing.
- No boolean operators, no `&&`/`||`, no negation sugar. Whether the language has short-circuiting boolean connectives
  is a separate question about its expression vocabulary, and answering it here would answer this prompt twice.
- No `match` change of any kind — not arm ordering, not exhaustiveness, not the wildcard.
- No unit type and no one-armed conditional.
- No adapter behavior change. The staff expansion produces the same items and its fixtures stay byte-identical.
