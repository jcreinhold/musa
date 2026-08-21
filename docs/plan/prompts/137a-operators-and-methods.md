---
id: 137a
slug: operators-and-methods
status: done
depends_on: [137]
phase: 3
---

# Route Operators and Methods Through Traits

## Task

Give 137's dictionaries their spelling. `trait`, `impl`, and `where` in the surface language — grammar, CST, formatter,
highlighting, completion, tree-sitter and its drift test — plus operator syntax for `== < + - * /` and indexing,
method-call syntax and `Type::item` paths, and the exact-receiver lookup that resolves them. Naming rules for traits,
methods, record fields, and enum constructors in `docs/rules/style-guide.md`, each with the `lint.rs` diagnostic that
reports it.

## Read

- `docs/rules/language/10-traits.md` §5 and §6 in full — the operator table, exact-receiver lookup, and the refused
  list. §5 and §6 make both of them *surface* spellings for terms `Raw` can already hold: `x == y` is `Eq.equal x y` and
  `x.m(y)` is a name and an application, so the elaborator never learns an operator table and no new `Raw` shape is
  needed for either.
- Prompt 137's `crates/musa-calculus/src/{class.rs, dictionary.rs}` — the tables this prompt queries. Exact-receiver
  lookup is `Classes::method` plus the same resolution 137 already runs; if this prompt finds itself writing a second
  lookup, that is a repair of 137 rather than work here.
- `crates/musa-language/src/{lexer.rs, parser.rs}` — the token set, and the reason the operators are addable. The lexer
  has no `==`, `+`, or `*` today; `-`, `/`, `<`, and `>` exist and are consumed **only** by music statements (durations,
  negative rationals, type arguments, hairpins), never by `expr()`. So infix operators enter the expression grammar
  without an unresolved ambiguity — but the music statements are the test that proves it, and a fixture for each of the
  four belongs in this prompt.
- `crates/musa-compiler/src/core/mod.rs`'s `BUILTIN_OWNERSHIP` (117 source operations) and `SYNTAX_OWNERSHIP` (14 phase
  operations). The operator table has to cover the arithmetic, comparison, and text entries; prompt 143 is the prompt
  that deletes them, and this prompt is where their replacements must actually exist.
- `docs/rules/style-guide.md` and `crates/musa-compiler/src/lint.rs` — this is the prompt that adds naming rules for
  traits, methods, record fields, and enum constructors, each with the diagnostic that reports it. Adding the rules in
  130 would have been a rule with no code behind it.
- `docs/plan/prompts/136a-module-visibility.md` — the precedent for the surface half of a two-prompt mechanism, down to
  the shape of the tree-sitter work and the drift test.

## Design

**Operators and methods are spellings, not semantics.** The parser desugars `a + b` to an application of a qualified
name and `x.m(y)` to a projection-free application, so what reaches `musa-calculus` is what a hand-written
`Add.add(a, b)` would have reached it as, and `a + b` and `Add.add(a, b)` are convertible because they are the same
term. This is what keeps the operator table out of the elaborator, and it is testable directly: the law is a
convertibility check, not a snapshot of a desugaring.

**Failing operations keep their failing shape.** `ratio_sub` returns `Result` today, and its operator form returns
`Result` tomorrow. No partial operator, no panicking division, no silent saturation. A total language that grows one
partial operator has stopped being one.

**Exact-receiver lookup, enforced negatively.** An *operator* resolves when its head is known or a `where` constraint
supplies the dictionary — that is `01-surface.md` §1.5, and it works because `a + b` is `Add.add(a, b)`, which names its
trait. *Method syntax* is stricter: a value of a generic parameter `A` never acquires a method from anywhere, `where`
clause or not, because finding one would mean scanning every trait in scope (`10-traits.md` §6 and §9's eighth row). The
test that matters is the negative one, and it is two tests: a generic function that writes `a + b` on an unconstrained
`A` fails with a message telling the author to write the constraint, and one that writes `x.add(y)` on any `A` fails
with a message telling the author to write `Add.add(x, y)`. Those are worth more than the ten positive ones.

**`Duration::of(n)`, not return-type-directed overloading.** A type namespace is a path the author writes, so the
elaborator never chooses an instance from the type a call is checked against. `Type::item` therefore parses as a path
and resolves as a qualified name, with no new resolution rule.

**Precedence is fixed and small, and there is no way to add to it.** One table in the parser covering comparison,
additive, multiplicative, and indexing, with no user-defined symbols and no sections — the mechanism that makes an
operator table extensible is the mechanism that makes a program's parse depend on its imports. The table is
`01-surface.md` §1's six levels exactly, so `step`, `up`, and `down` sit *between* additive and comparison rather than
above or below all of it. There is no `>`: `10-traits.md` §5 gives `Ord` one method and `01-surface.md` §1's `binary-op`
lists six symbols, and `>` after a note is the accent mark.

**A note's pitch is not an arithmetic expression.** In a music statement `/` and `-` are the duration's own syntax, so
`c5 up 2 /4` is a transposed note lasting a quarter and not a division. The note statement therefore reads its pitch at
`01-surface.md` §1's levels 1, 4, and 5, and an author who means arithmetic writes the parentheses that say so. This is
the concrete form of the Read section's claim that the music statements are what proves the operators enter without an
ambiguity — they enter the *expression* grammar, which is a different production from a note.

**`private` on a `trait` or an `impl`.** 136a's grammar admits the marker and `01-surface.md` §1.3 explicitly deferred
its meaning to this prompt: a hidden `impl` is a coherence question, because the same expression elaborating to two
different dictionaries in two modules is exactly what §2 forbids. Settle it here — the marker hides the *name*, never
the instance, so a private `impl` is still in the one global table and still refuses a duplicate — and record the
sentence in `01-surface.md` §1.3 in place of the deferral.

**Laws.** Operators desugar to the method, so `a + b` and `Add.add(a, b)` are convertible. Method lookup refuses on a
generic parameter, naming the constraint to write. Every music statement that consumes `-`, `/`, `<`, or `>` still
parses as it did, by fixture. Parsing round-trips losslessly and formatting is idempotent for every new form. The
tree-sitter grammar agrees with the real lexer under the drift law. Each style rule reports its named diagnostic.

## Target

- `musa-language`: the `trait`, `impl`, and `where` keywords, their grammar and CST positions, operator tokens and
  precedence, method-call and `Type::item` paths, formatter layout, highlighting, and completion.
- `editors/tree-sitter-musa`: grammar, queries, and corpus, with the drift test green.
- `musa-calculus`: exact-receiver method resolution over 137's tables, and the refusal for a method on an unconstrained
  type.
- New `Code` variants with `musa explain` text for: ambiguous instance, and a method on an unconstrained type.
- `docs/rules/style-guide.md` naming rules and their `lint.rs` diagnostics.
- `docs/rules/language/01-surface.md` §1.3 — replace the `trait`/`impl` deferral with the rule.
- `crates/musa-language/tests/suite/` and `crates/musa-calculus/tests/suite/operator_laws.rs`, plus the compile-fail
  suite.
- `docs/plan/code-map/` rows.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-language -p musa-calculus -p musa-compiler
cargo nextest run --workspace
cargo clippy --all-targets -p musa-language -p musa-calculus -p musa-compiler -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Route operators and methods through traits`.

## Stop

- No change to 137's tables, resolution order, or coherence rules. If the surface needs one, that is a repair of 137.
- No user-defined operator symbols, no operator sections, no precedence declarations.
- No return-type-directed overloading and no auto-deref.
- No method resolution on a generic parameter — the negative test is the point of the feature.
- No deletion from `BUILTIN_OWNERSHIP`; prompt 143.
- No `musa-compiler` checker wire-up beyond the `Code` table and `lint.rs`, and no `stdlib/` or `examples/` migration.
  Prompt 142.
- No `Syntax<Cat>`, no quotation, no collection library.
