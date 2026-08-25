---
id: 166a
slug: equality-for-declared-types
status: pending
depends_on: [166]
phase: 3
---

> **Repaired after prompts 143 and 146 retired traits.** An earlier version asked whether a declared type received an
> `Eq` instance and cited coherence and dictionary elaboration. Neither mechanism exists. The live question is whether
> the type's namespace contains `equal`, because `x == y` is the ordinary named call `equal(x, y)` selected from the
> definitions in scope by the type of `x`.

# Give Declared Types an Equality, or Decide They Do Not Get One

## Task

The compiler supplies seven namespace definitions named `equal`: the five closed by literal patterns (`Text`, `Ratio`,
`Duration<WrittenTime>`, `Pitch`, `Interval`), plus `Position<WrittenTime>` and `Nat`, which prompt 164 admitted for
measured callers. Prompt 166's rewritten library also declares its own finite types and already compares several of them
through functions named `same_place`, `same_operation`, `same_class`, `same_set`, and `same_row`. Decide, on that
corpus, whether a declared type's equality is generated structurally, written as an ordinary namespace function, or left
as an explicit `match`, and implement the answer.

## Read

- `crates/musa-compiler/src/prelude.rs`, `methods` and `equalities`, including both ownership comments. They state why
  the compiler owns the seven definitions and why `Bool` remains absent. This prompt does not move that boundary.
- `docs/rules/language/01-surface.md` §1.4 and §1.5. Traits are gone; an `impl T` block is a namespace, and `==` is
  surface syntax for an ordinary definition named `equal`, selected by type-directed disambiguation.
- Prompt [164](164-builtin-collapse.md), especially its repaired statement that there is no `Eq`, instance table, or
  dictionary, and the `Nat.equal` implementation it installed.
- Prompt [166](166-staff-rewrite.md)'s corpus. Count before deciding: how many declared types it contains, which values
  a program actually compares, their structural depth, and which existing `same_*` bodies are merely fieldwise folds.
- `stdlib/src/cyclic.musa`, `stdlib/src/post_tonal/pcset.musa`, and `stdlib/src/post_tonal/serial.musa`, where the live
  declared-value comparisons are written and passed as first-class functions.
- `~/Code/Idris2/libs/base/Decidable/Equality.idr`. Idris writes structural decidable equalities by hand; its instance
  machinery is not Musa's, but the bodies show the cost of the written route without pretending derivation is free.
- The Haskell 2010 Report §11.1. If generation wins, its structural rule is the precedent: reject unequal constructor
  tags, then compare fields from left to right, and generate only when every field has an equality.

## Design

**Count first, decide second.** All three outcomes remain admissible. The count is evidence, not decoration.

- **Generated.** Every eligible declared type receives a namespace definition `T.equal`; constructor tags are compared
  first and fields left to right. A separately written `T.equal` is a duplicate declaration and is refused by the
  ordinary namespace rule. This is one dedicated equality facility, not a general `deriving` mechanism.
- **Written.** The compiler generates nothing. An author writes
  `impl T { fn equal(left: T, right: T) -> Bool { ... } }`; `==`, `.equal`, and `T::equal` are then three surface
  spellings of that one definition. Existing semantic comparisons migrate only where the corpus uses them.
- **Neither.** Declared types compare by explicit `match` or by domain-named predicates. `==` works only for heads with
  an `equal` definition already in scope.

Many types or deeply nested fieldwise comparisons favour generation. Few shallow comparisons, especially comparisons
that intentionally project through a type's invariant, favour written definitions. No declared-value comparisons favour
neither.

If generation wins, it must answer three refusals explicitly:

- A function field is not equatable; the diagnostic names that field.
- An indexed family's index is erased at quotation and does not participate in value equality. Values are compared at
  the already-known common indexed type, never across two indices.
- A registered base field with no compiler-owned `equal` definition stops generation rather than inventing equality.

Whichever route wins, the compiler-owned seven do not move. A written namespace definition is not an instance: there is
no coherence rule, orphan rule, dictionary, named-instance escape hatch, or overlap to design.

## Target

- The count and decision recorded in `docs/notes/research/language-design-closure/` and linked from the code map:
  declared types in the rewritten library, types whose values are compared, their depth, and the handwritten bodies
  generated or retained.
- `crates/musa-compiler/`: implementation only if generation wins. Written equality uses the existing namespace and
  operator machinery; neither adds no compiler code.
- `stdlib/`: the namespace definitions or explicit comparisons the corpus actually needs, and no others.
- If generation wins, fixtures for the function-field, indexed-family, and unregistered-base refusals.
- Laws that every compared declared type reaches the chosen route through `==` when that route supplies `equal`, and
  that a type with no such definition is refused rather than given an implicit fallback.

## Check

```sh
cargo nextest run -p musa-compiler
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

The decision's own check is that the corpus exercises it: every declared type the rewritten library compares uses the
chosen route, and no second comparison definition is introduced under another spelling.

## Stop

- No trait, instance, dictionary, coherence rule, named instance, priority, overlap, or default-method mechanism.
- No `Ord`, `Hash`, `Show`, and no general `deriving` mechanism. If generation wins, it generates equality only.
- No change to the compiler-owned seven, and no new literal pattern.
- No propositional equality or `DecEq`, and no proof that `equal` agrees with conversion.
- No change to `==` lowering or type-directed disambiguation.
- Do not generate equality for registered base types. The host owns their private representations and capabilities.

Commit as `Decide how a declared type gets an equality`.
