---
id: 151
slug: universe-levels
status: pending
depends_on: [150]
phase: 3
---

# Replace the Two Fixed Universes with a Non-Cumulative Polymorphic Hierarchy

## Task

`level.rs` is sixty lines encoding "two universes, fixed," and `refuse.rs` has a diagnostic that reads *"this
declaration needs a universe above `Type 1`, and there are two."* Replace it with a real hierarchy: level expressions
`0 | u | l+1 | max l l'`, universe parameters generalized per declaration, level unification local to a declaration, and
**no cumulativity**. Authors never write a level; the ceiling is gone.

## Read

- `crates/musa-calculus/src/kernel/sort.rs` (was `level.rs`) — in particular its own note that a normalized
  `max(k, ℓ₁+k₁, …)` with level metavariables *used to exist here* and was deleted by the course correction on an audit
  that found no universe-polymorphic declaration. This prompt restores that representation with a reason.
- `crates/musa-calculus/src/elaboration/refuse.rs`, the `needs a universe above Type 1` variant — the ceiling, as a
  message.
- Lean 4, `src/kernel/level.cpp` — the level algebra and its normalization. Agda, `Agda.Primitive` — the same idea with
  `Level` reified as a type, which this prompt does **not** follow.

## Design

### Why the ceiling has to go

Two universes are enough for a language whose types are all `Type 0`. They stop being enough the moment a *record holds
a type*, because that record lands at `Type 1` and anything generic over it wants `Type 2`. Prompt 146 turns every
structure into a record, and prompt 161 turns every `signature` into one — so the stdlib is about to be full of exactly
the construct the ceiling forbids. Waiting for the wall to be hit would mean discovering it in the middle of 161 with
two prompts already committed against it.

### Sound? Yes — and non-cumulative is the *safer* of the two options

The question the design has to answer is whether dropping cumulativity costs soundness. It does not, and the reason is
that **cumulativity and the hierarchy are orthogonal**. Three separate things travel under one word:

|  | What it is | Who does it |
| --- | --- | --- |
| **Hierarchy** | `Type 0 : Type 1 : Type 2 : …`, predicative | everyone |
| **Cumulativity** | a *subtyping* rule: `A : Type i` implies `A : Type j` for `i ≤ j` | Coq |
| **Polymorphism** | a definition abstracts over levels: `id : {u} → (A : Type u) → A → A` | Agda, Lean, Coq ≥8.5 |

A predicative, non-cumulative hierarchy with universe polymorphism is the standard Martin-Löf presentation and is what
**Agda and Lean 4 both do**. Agda offers cumulativity only behind an experimental flag; Lean 4 has none. Coq is the
outlier, and its cumulativity is also the source of its hardest metatheoretic work — cumulative inductive types needed
real restrictions to stay sound. The non-cumulative theory is a *subtheory* of the cumulative one: it accepts strictly
fewer programs, so it cannot be unsound where the other is sound.

What non-cumulativity actually costs is ergonomic and is paid in one place: you cannot pass a `Type 0` thing where a
`Type 1` is wanted, so anything meant to work at more than one level must be *written* polymorphically instead. That is
the Agda/Lean idiom and it is why polymorphism is part of this prompt rather than a later one — **cumulativity and
polymorphism are alternative answers to the same ergonomic problem, and polymorphism is the one that does not make
conversion directional.**

And directional conversion is exactly what prompt 143 refused. Cumulativity *is* subtyping: `≼` rather than `≡`,
variance-flipping in Π domains, and no most-general solution when a metavariable sits under a `≤` constraint — which is
Agda's stated reason for avoiding it. Admitting it here would re-open, for one inequality, the property the whole
overhaul exists to buy.

### The design, concretely

**Level expressions, four formers and no more.**

```rust
enum Sort { Zero, Param(SortId), Succ(Box<Sort>), Max(Box<Sort>, Box<Sort>) }
```

**No `imax`.** Lean needs `imax` so that `∀ x : A, B` lands in `Prop` when `B : Prop` — impredicative `Prop`. Musa has
no `Prop` and no impredicativity, so Π takes the plain `max` of its parts and the fourth former does not exist. This is
a real simplification and it should be stated in the specification, because a reader coming from Lean will look for it.

**Normalization.** Every expression normalizes to `max(k, u₁+k₁, …, uₙ+kₙ)` — a constant and a set of
parameter-plus-offset terms, deduplicated, with dominated terms dropped. Comparison is on the normal form. This is the
representation `sort.rs`'s own comment says used to be here, and it is about 150 lines.

**Generalization per declaration, not a global graph.** Coq needs a global constraint graph because *typical ambiguity*
lets one anonymous `Type` be constrained from anywhere in the development. Lean does not, because level parameters are
generalized at the declaration boundary and constraints are solved inside it. Musa follows Lean: no global graph, no
cross-declaration universe constraints, no `Set` of inequalities that can go inconsistent halfway through a build. This
is the smaller machine and it is the one that composes with separate compilation.

**Inference, so nobody writes a level.** A declaration's unsolved level metavariables are generalized into universe
parameters, in written order, and a use site instantiates them. `Type` with no argument elaborates to `Type ?u`. Surface
syntax for an explicit level exists but is not documented as ordinary usage.

### The two risks, named rather than discovered

**`max` unification is not unitary.** `max ?u ?v = 3` has several solutions. Agda and Lean both handle this by
postponing the constraint and defaulting at generalization; do the same, and make the *defaulting* rule explicit in the
specification rather than emergent from solver order — a program whose acceptance depends on constraint ordering is a
program two compilers disagree about, which is `budget.rs`'s argument applied to levels.

**Conversion must still not compare with `<`.** `sort.rs`'s current doc says levels are compared with `==`, never with
`<`. That sentence survives this prompt unchanged and is the check that cumulativity did not creep in: conversion
compares normal forms for equality; only *formation* rules take a `max`.

## Target

- `crates/musa-calculus/src/kernel/sort.rs`: the four formers, normalization, equality, and `max`. No `<`.
- `crates/musa-calculus/src/kernel/term.rs`: `Universe(Sort)`.
- `crates/musa-calculus/src/elaboration/`: level metavariables, generalization at the declaration boundary,
  instantiation at use, and the documented defaulting rule.
- `crates/musa-calculus/src/elaboration/refuse.rs`: the `needs a universe above Type 1` variant **deleted**, and a new
  refusal for a genuinely inconsistent level constraint.
- `docs/rules/language/02-core-calculus.md` §1: the hierarchy, the three-column table above, the refusal of cumulativity
  with its reason, and the absence of `imax` with its reason.
- `crates/musa-calculus/tests/suite/sort_laws.rs`: normalization is idempotent; equality is decidable; `Type u` is not
  convertible with `Type (u+1)`; a polymorphic identity instantiates at two different levels in one program.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! grep -rn 'needs a universe above' crates/
! grep -rn 'imax' crates/musa-calculus/src
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

The first `grep` is the deliverable: the ceiling diagnostic is gone because the ceiling is. The second is the check that
the Lean former nobody needs did not arrive with the rest of the algebra.

Commit as `Replace the two fixed universes with a polymorphic hierarchy`.

## Stop

- **No cumulativity.** Not a `Type 0 ≤ Type 1` special case, not "just for records."
- No `Level : Type` in the object language. Agda reifies levels and pays for it with `Setω`; musa keeps them a separate
  syntactic sort, which is Lean's choice and the reason Lean has no `Setω` problem.
- No global universe constraint graph, and no typical ambiguity across declarations.
- No universe *checking* of the standard library beyond what the suite already compiles. Whether the stdlib actually
  needs three levels is prompt 161's measurement, not this prompt's promise.
