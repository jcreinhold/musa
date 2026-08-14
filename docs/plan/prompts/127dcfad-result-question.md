---
id: 127dcfad
slug: result-question
status: done
depends_on: [127dcfac]
phase: 3
---

# Let a Failure Propagate Without a Staircase

## Task

`stdlib/src/adapters/staff.musa`'s `document_read` asks six questions in a row — was the region refused, does a tie
hang, is there a head, does the clef name a clef — and each answer that is not a failure indents the next question one
frame further. The result is a staircase whose rightmost leaf carries the actual work. Every adapter operation returns
`Result`, so this shape recurs wherever more than two things can go wrong.

This prompt adds postfix `?` with exactly one meaning: propagate a `Result` failure out of the enclosing function. It is
surface elaboration to the exhaustive sum elimination that already exists. It is not a monad, not an effect, and not a
type class.

## Read

- `docs/rules/language/02-core-calculus.md` §1 and §5 — sum injections, exhaustive `match`, and the structural `Result`
  prompt 127ab added. §5's match rule is what `?` elaborates to.
- `docs/rules/language/01-surface.md` — the expression grammar this prompt extends.
- `docs/notes/research/language-design-closure/39-totality-and-structural-abstraction.md` §4.3, which fixes the single
  meaning below and refuses the general `do`/`Try`/`Monad` generalization; §6.1, which records that a second *use* of
  `Result` propagation is evidence for `?` and not for abstraction over `M : Type -> Type`; and §3.3, whose level audit
  separates failure sequencing from container protocols and structural recursion.
- Prompt [127ab](127ab-text-and-sums.md) — the one structural `Result` and its constructors.
- Prompt [127dcb](127dcb-adapter-refusal.md) — an adapter's refusal is the error half of its answer, and the node it
  points at is part of that answer. `?` must carry a refusal outward without losing what it points at.
- Peyton Jones ch. 3 §3.2 and ch. 6 — the enriched calculus and its transformations. A construct the author needs is
  added and translated away; `?` is one transformation, not a new evaluation mechanism.
- `stdlib/src/adapters/staff.musa`: `document_read` at line 1988, and `read_body` and `taken_piece` above it.
- `crates/musa-language/src/parser.rs`, `ast.rs`, `formatter.rs`, and `syntax_kind.rs`; and
  `editors/tree-sitter-musa/grammar.js` with its queries, held to the lexer by the drift law in root `AGENTS.md`.

## Design

**One meaning, stated exactly.** In a function whose inferred or explicitly written result type is `Result<B, E>`, the
expression `e?` where `e : Result<A, E>`:

- evaluates `e` exactly once;
- has type `A` and denotes the success payload when `e` succeeds; and
- makes the enclosing function's result the *same* `Err` value when `e` fails.

The error types must be identical. There is no conversion, no widening, and no `From`-like coercion: a different error
type is mapped explicitly at the call site, which keeps the set of errors a function can produce readable from its
signature.

**It elaborates to a match and a continuation.** `?` in an expression becomes the exhaustive two-arm match on `Result`,
with the failure arm returning the enclosing function's result directly and the success arm continuing with the rest of
the expression. Evaluation order is unchanged, the one evaluator runs the result, and the charge is the match's charge
plus the injections it already performs. No new core term, typing rule, reduction, normalization case, or cost-table
entry.

**Where it is allowed, and why this is not an annotation requirement.** During inference, `?` constrains the enclosing
function's result to `Result<B,E>` with the same `E` as its subject. An explicit return annotation may satisfy that
constraint, but is not required; unannotated named and anonymous functions remain under the same principal rank-1
inference as every other function. `?` is rejected only when the enclosing result cannot unify with `Result`, with a
diagnostic naming the inferred or written result type. That is not "ordinary Musa minus a feature": it is the
elaboration's typing precondition, in the same way an eliminator's arity is. A function that wants to handle a failure
rather than propagate it writes the `match` it means.

**Refused: a general `do`, `Try`, or `Monad` class.** Note 39 §6.1 is explicit that the evidence names one constructor
used twice, not one algorithm run over two constructors, so the reopening rule in `constitution.md` §9 and
`obligations.md` §10 is not met. Adopting `bind` because `Result` is a monad would be the analogy-first move that rule
forbids, and it would need higher-kinded variables the language does not have and this prompt is not permitted to add.
If a second constructor later needs propagation, it earns its own explicit operation or supplies the evidence note 39
§6.4 requires.

**Refused: exceptions, handlers, or an early-return statement.** `?` introduces no exception value, no handler, no
unwinding, and no statement form. The elaboration is local and the language stays an expression language.

**Refused: `?` on `Option`.** An `Option` has no error to carry. A caller that wants propagation converts explicitly
with a named operation, which states what the missing case means.

**The staircase is the measurement.** `document_read`'s frame depth and line count are recorded before and after, so
prompts 127dcfae and 127dcfag can attribute later improvements to the traversal change rather than to this one.

## Target

- `docs/rules/language/01-surface.md` — the postfix form, its single meaning, the identical-error-type rule, the
  inferred-or-written enclosing-result constraint, and the elaboration written out.
- `docs/rules/language/02-core-calculus.md` — a sentence in §5 recording that `?` is surface elaboration to the
  exhaustive `Result` match and adds no core term. §5 is otherwise unchanged and the prompt states that it is.
- `crates/musa-language/` — `syntax_kind.rs`, `lexer.rs`, `parser.rs`, and `ast.rs` gain the postfix form with its
  precedence fixed against application, projection, and update; `formatter.rs` lays it out.
- `crates/musa-compiler/` — the elaboration and inference constraint, without requiring an explicit return annotation;
  the diagnostic for `?` in a non-`Result` function; the diagnostic for mismatched error types, naming both; and a
  source map that points a failure at the `?` that propagated it rather than at a synthesized match.
- `editors/tree-sitter-musa/grammar.js` and `queries/` — the form and its highlighting.
- `stdlib/src/adapters/staff.musa` — `document_read` and the other `Result` staircases flatten. The refusal node and its
  text survive propagation unchanged.
- Tests in `crates/musa-language` and `crates/musa-compiler`: the subject is evaluated once, proved by the meter; a
  failure propagates the identical `Err` payload, including a refusal's node; an unannotated named function and an
  unannotated anonymous function infer through `?`; `?` in a genuinely non-`Result` function is rejected at its own
  span; two different error types are rejected with both named; `?` chained several times in one expression evaluates
  left to right and stops at the first failure; and the elaborated form is observationally the match it replaces at the
  same charge.
- A short note under `docs/notes/research/core-calculus/` recording `document_read`'s frame depth and line count before
  and after, and its entry in that directory's `README.md`.

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

Commit as `Let a failure propagate without a staircase`.

## Stop

- No `Try`, `Monad`, `Functor`, or any type class; no instance search; no higher-kinded type variable. Note 39 §6.4 owns
  the evidence that would reopen this, and it is not this prompt's to supply.
- No general `do` notation, no `bind` operator, and no second sequencing form.
- No exceptions, handlers, unwinding, effects, or early-return statement.
- No error-type conversion, widening, subtyping, or `From`-like coercion.
- No `?` on `Option` and no `?` on any other constructor.
- No requirement that a function using `?` write an explicit return annotation. Principal rank-1 inference remains the
  language-wide rule.
- No new core term, typing rule, reduction, normalization case, or cost-table entry.
- No adapter behavior change. The staff expansion produces the same items, the same diagnostics at the same spans, and
  its fixtures stay byte-identical.
