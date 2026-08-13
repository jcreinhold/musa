---
id: 127aa
slug: kinded-inference
status: pending
depends_on: [127a]
phase: 3
---

# Infer Types, With Two Classes of Type Variable

> **Governed by the event-track and machine core installed by prompts 127a–127i.** First of the five prompts that
> replace the source checker and evaluator; the chain is 127aa, 127ab, 127ac, 127ad, 127b.

## Task

Replace the monomorphic checker in `musa-compiler` with rank-1 Hindley–Milner inference over the type set that exists
today. Make parameter and result annotations optional wherever the program determines them, and give every type variable
one of two kinds: ordinary, ranging over any value type, or data, ranging over storable data only.

## Read

- `docs/rules/language/02-core-calculus.md` §1 and §1.1, as repaired before this prompt — the type grammar, the storable
  data rule, and the inference discipline.
- Research `05-selected-calculus.md` §2 and §2.1, which both the rule and this prompt derive from.
- `crates/musa-compiler/src/core.rs` — `Type` (`enum Type`), `declared_type`, `signature_type`, `function_type`, the
  `Shape`/`Base`/`Family` primitive registry, and the seven hand-checked `Eliminator` arms.
- Prompts 94–96 and 108, and the higher-order call tests they installed.
- Peyton Jones, *The Implementation of Functional Programming Languages*, chapters 3 and 6, as cited by the research.

## Design

Keep the type set exactly as it is today — the musical base types, `unit`, `bool`, `nat`, `ratio`, products, `option`,
`list`, functions, and `Music`. This prompt adds inference, not types; `text`, sums, `Result`, and nominal data arrive
at 127ab and 127ac, and `EventTrack`/`Machine` at 127c and 127d.

Add type variables and schemes. A variable carries its kind: an ordinary variable may stand for any value type, a data
variable only for storable data. Unification never binds a data variable to a function type or to a container holding
one at any depth. The check is structural over the type, not a surface-syntax rule, and it is a side condition on
ordinary unification — not subtyping, overloading, or a source-visible type class.

Generalize at `let` and at a declaration; instantiate at a use. Compute principal types. An annotation remains accepted
everywhere it is written today and remains required only where separate checking or an abstract public signature needs
one; `function_type` must stop returning `None` merely because a parameter or result type was omitted.

Retire the seven hand-checked `Eliminator` arms in favour of declared schemes wherever the scheme states the same
constraint the arm was checking by hand. Keep an arm only where the operation's type genuinely is not a rank-1 scheme,
and say in a doc comment which one and why. The `Delta` registry's arrow-free premise stays true by construction.

Diagnostics report inferred types in the same plain spelling the annotations use. An unresolved variable at a point
where the program does not determine the type is a located error naming what to annotate, never a silent default.

Keep `Type`, schemes, substitutions, the unifier, and evaluator values private to `musa-compiler`.

## Target

- Kinded rank-1 Hindley–Milner inference in `musa-compiler`: type variables, schemes, unification with the data-kind
  side condition, generalization at `let` and declarations, instantiation at uses.
- Optional annotations wherever inference determines the type, with the existing corpus unchanged in meaning.
- Property tests for principal types and for substitution/unification soundness; compile-fail tests for a data variable
  unified with a function type, and for a genuinely ambiguous type with no annotation.
- Updated hover text and diagnostics showing inferred types in plain form.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
find examples stdlib -name '*.musa' -print0 | xargs -0 -n1 cargo run -q -p musa -- check
```

Commit as `Infer source types with two kinds of type variable`.

## Stop

- No new type: no `text`, sum, `Result`, nominal data, `EventTrack`, or `Machine` in this prompt.
- No deletion of partial calls or default parameters; that is 127ad, and the corpus must still compile here.
- No general recursion, overloading, subtyping, higher-rank type, or dependent type.
- No public Rust mirror of `Type`, a scheme, the substitution, or an evaluator value.
