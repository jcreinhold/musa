---
id: 156
slug: indexed-families
status: done
depends_on: [155]
phase: 3
---

# Let a Constructor Choose Its Index

## Task

`data` declares parameters and a constructor may not choose an index. Lift that: `data Vect : Nat -> Type -> Type` with
`Nil : Vect 0 a` and `Cons : a -> Vect n a -> Vect (n+1) a`, positivity extended to indices, and a `match` that *learns*
the index from the constructor it matched. This is the mechanism the four deleted workarounds were each approximating.

## Read

- `docs/rules/language/02-core-calculus.md` §1.1 after prompt 144's rewrite — parameters and indices, distinguished.
- `/Users/jcreinhold/Code/Idris2/src/TTImp/ProcessData.idr` — the declaration side, including the positivity check.
- `docs/notes/research/language-design-closure/52-the-musical-algebra.md` §1 — the indexed musical domains this is for.

## Design

**Parameters and indices are different and the difference is one rule.** A parameter is fixed across all constructors of
a declaration; an index is chosen per constructor. Everything else follows: a parameter may be abstracted once at the
declaration, an index may not.

**Positivity extends to indices.** A family may not occur in a negative position of a constructor's argument type, and
that check now has to look through index expressions as well as through argument types.

**What becomes writable, and each is a thing the corpus wanted.**

- `Pc : Nat -> Type` with the modulus an index — which is what prompt 164's seventeen builtins collapse into.
- `Equal : {a : Type} -> a -> a -> Type` with `Refl : Equal x x`. `rewrite` is sugar over its eliminator. Prompt 143
  admitted this as a *consequence of the theory*, not as a proof assistant; the apparatus stays refused.
- `Syntax : Cat -> Type`, which prompt 159 uses to delete a compiler builtin and the language's only coercion rule.
- `Bar : Duration -> Type` and the rest of note 52's domains, which the index stratum could describe and not check.

**Large elimination arrives with this prompt, and it moves the budget.** A dependent motive may compute a *type* from a
value, so a user's own definition can run while a type is being checked. §3 already names this and the meter is already
the backstop, but the numbers move here. Prompt 165's pressure class must be re-measured **after** this prompt, not
before, and 165's Check says so.

**Eliminators are still generated, and now they are the folds.** Prompt 146 deleted `Iterable` on the argument that a
family's fold *is* its eliminator. This is the prompt that makes that true for indexed families as well, which is why
146's Check does not wait for it: the two `Iterable` instances were `List` and `Option`, both plain.

## Target

- `crates/musa-syntax`, `crates/musa-compiler/src/lower/`: index syntax in `data`.
- `crates/musa-calculus/src/kernel/family/`: indices in the group representation; positivity through indices.
- `crates/musa-calculus/src/elaboration/case.rs`: index unification at a split, producing `Impossible` where the indices
  cannot agree.
- `stdlib/`: `Equal`/`Refl` as library code, and the note 52 domains re-declared as families.
- `crates/musa-calculus/tests/suite/family_laws.rs`: `Vect` head/tail total; a `match` that refines; a negative
  positivity control.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- check examples/*.musa
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

**The re-checker's obligation for this prompt**: an `Impossible` branch must be re-derived — the index unification that
ruled it out re-run, not trusted — and a constructor's chosen indices re-checked against the family's signature. Prompt
158 audits it.

Commit as `Let a constructor choose its index`.

## Stop

- No tactics, no proof search, no hint database, no interactive holes as a workflow. `Equal` being declarable does not
  make this a proof assistant and prompt 143 drew that line.
- No builtin collapse (164), no `Syntax` rework (159), no records-as-data (157).
- No budget change. Measuring is 164's; this prompt only records the numbers it moved.
