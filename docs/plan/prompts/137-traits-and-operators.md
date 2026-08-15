---
id: 137
slug: traits-and-operators
status: pending
depends_on: [136]
phase: 3
---

# Add Traits, Dictionaries, Operators, and Methods

## Task

Implement the trait system prompt 130 specified: trait and impl declarations, coherence and orphan checking, dictionary
elaboration, `where` constraints, operator syntax routed through traits, inherent methods and type namespaces under
exact-receiver lookup, and the `Storable` constraint that retires the `d` type-variable class. This is the mechanism the
131 compiler builtins collapse into, and the last piece of the language before metaprogramming.

## Read

- `docs/rules/language/10-traits.md` in full — coherence, the orphan rule, instance lookup and its termination measure,
  local-beats-global, the operator table, exact-receiver lookup, and the refused list. This prompt implements that
  document and decides nothing it left open without a repair.
- Prompt 132's trial, for what it found about coherence in practice. A trait design that was never written against a
  program is a design; that note is the evidence.
- `crates/musa-compiler/src/core.rs`'s `BUILTIN_OWNERSHIP` (117 source operations) and `SYNTAX_OWNERSHIP` (14 phase
  operations). The operator table has to cover the arithmetic, comparison, and text entries; prompt 143 is the prompt
  that deletes them, and this prompt is where their replacements must actually exist.
- `docs/rules/language/02-core-calculus.md` §1.1 as rewritten by 129 — the `Storable` constraint, and the rule that its
  instances are generated from the declaration group and never written by hand. That is the single most important
  coherence property in the system: a user-writable `Storable` instance is a hole in the kernel payload boundary.
- `crates/musa-compiler/src/infer.rs`'s `d`-variable handling — the machinery being replaced, and the failure messages
  it produces today, which the constraint form must match or beat.
- `docs/rules/style-guide.md` and `crates/musa-compiler/src/lint.rs` — this is the prompt that adds naming rules for
  traits, methods, record fields, and enum constructors, each with the diagnostic that reports it.

## Design

**Dictionaries are records, and that is the whole implementation.** A trait elaborates to a dependent record type; an
impl elaborates to a value of it; a constrained function takes it as an extra argument; a use site is resolved by lookup
and applied. Because 133 made records primitive with η, two dictionaries for the same instance are convertible without a
projection chain, which is what makes coherence checkable rather than merely asserted.

**Resolution is a lookup, and the code must be structured so that it cannot become a search.** One table keyed by
(trait, head constructor). No overlap: a second impl for the same key is an error at the *declaration*, naming both. No
orphans: an impl lives with its trait or with its head type, checked at the package boundary. No defaulting. Ambiguity
is an error naming the candidates. Instance heads carry a decreasing measure, checked at declaration, so context
reduction terminates — and it is checked at the declaration rather than at the use, because an error at the use site
blames the wrong author.

**Local beats global.** A dictionary bound by an enclosing `where` is preferred to a global instance for the same head.
This is what makes a generic function's behaviour a consequence of its own signature, and it is a one-line rule with a
large payoff in comprehensibility.

**Exact-receiver lookup, enforced negatively.** `x.add(y)` resolves when `x`'s concrete type is known or a `where`
constraint supplies the dictionary. A value of a generic parameter `A` never acquires a method from anywhere. The test
that matters is the negative one: a generic function that calls `.add` on an unconstrained `A` must fail with a message
telling the author to write the constraint, and that test is worth more than the ten positive ones.

**`Eq` and `Id` are different words for different things, in the error messages too.** `==` is `Eq`'s method and returns
a `bool`; `=` is 129's identity type and is introduced by `refl`. `DecEq` returns a proof or a refutation. A message
that says "equality" without saying which one is a message that will be misread, so the diagnostics name the trait or
the type.

**Failing operations keep their failing shape.** `ratio_sub` returns `Result` today, and its operator form returns
`Result` tomorrow. No partial operator, no panicking division, no silent saturation. A total language that grows one
partial operator has stopped being one.

**`Storable` retires the `d` variable class.** Instances are generated from the declaration group by the same structural
check §1.1 already specifies — no source `impl Storable` is accepted, ever, and that refusal has its own test. What
changes for an author is that the constraint appears in signatures they read, and that its failures are ordinary
instance errors naming the field that is not storable.

**Naming rules land here because their diagnostics do.** Traits, methods, record fields, and enum constructors get style
rules in `docs/rules/style-guide.md`, each naming its lint diagnostic, and `lint.rs` reports them. Adding the rules in
130 would have been a rule with no code behind it.

**Laws.** Coherence: at most one instance per key, tested by the rejection. Orphan refusal. Dictionary elaboration is
sound — a resolved use is convertible to the dictionary's field. Local beats global, tested by a program where they
differ. Instance matching terminates on the pathological head. Operators desugar to the method, so `a + b` and
`Add::add(a, b)` are convertible. Method lookup refuses on a generic parameter. `Storable` cannot be implemented by
hand. And the negative suite carries one case per refusal in `10-traits.md`.

## Target

- `musa-language`: grammar, CST, formatter, highlighting for `trait`, `impl`, `where`, method-call and `Type::item`
  paths, and operator forms; tree-sitter and its drift test.
- `musa-core`: trait and impl elaboration to records, the instance table with coherence, orphan, and termination
  checking, `where` handling, local-beats-global, exact-receiver method resolution, and the generated `Storable`
  instances.
- New `Code` variants with `musa explain` text for: duplicate instance, orphan impl, ambiguous instance, unresolved
  instance, method on an unconstrained type, non-terminating instance head, and hand-written `Storable`.
- `docs/rules/style-guide.md` naming rules and their `lint.rs` diagnostics.
- `crates/musa-core/tests/suite/{trait_laws.rs, coherence_laws.rs, operator_laws.rs}` plus the compile-fail suite.
- `docs/plan/code-map/` rows.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-language -p musa-core -p musa-compiler
cargo nextest run --workspace
cargo clippy --all-targets -p musa-language -p musa-core -p musa-compiler -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Add traits, dictionaries, operators, and methods`.

## Stop

- No instance search, no overlapping instances, no specialization, no defaulting, no return-type-directed overloading,
  no auto-deref, no blanket impls, no functional dependencies, no associated-type resolution beyond what `10-traits.md`
  states, and no method resolution on a generic parameter. Every one of these turns the lookup into a search.
- No user-written `Storable` instance, behind any spelling.
- No deletion from `BUILTIN_OWNERSHIP`. Prompt 143 collapses the registry, after there is something to collapse it into
  and a migration that uses it.
- No `musa-compiler` wire-up and no `stdlib/` migration. Prompt 142.
- No `Syntax<Cat>`, no quotation, no collection library.
- No user-defined operator symbols and no operator sections.
