---
id: 158
slug: syntax-as-a-family
status: pending
depends_on: [155]
phase: 3
---

# Make `Syntax` a Family, and Delete the Language's Only Coercion Rule

## Task

`Syntax<Cat>` is a base type with a hardcoded category argument, `as_expression` is a compiler builtin, and the
category-forgetting rule — *a position of category `TokenTree` accepts a value of any category* — is implemented as
`Accepts` in `base.rs`, fired from four sites in `elab/check.rs` and deliberately kept out of conversion. Make `Syntax :
Cat -> Type` a real family; `as_expression` becomes an ordinary function; **and `Accepts` goes**, because forgetting
becomes a total function anybody can call.

## Read

- `docs/rules/language/11-quotation.md` §1 after prompt 145's repair.
- `crates/musa-calculus/src/kernel/base.rs:143`, `pub type Accepts` — and its doc comment, which already names the
  problem: *"conversion is symmetric, so a rule living there would admit the reverse direction too."*
- `crates/musa-calculus/src/elaboration/elab/check.rs`, the four `Accepts` call sites.

## Design

**The refusal that expires.** `11-quotation.md` §1 refused to make `Syntax` a family because "a family would pull the
indexed-family machine into the core for one type." Prompt 155 put that machine in the core for the music domains, so
the cost is now zero.

**What a family buys, beyond tidiness.** Matching a syntax value *refines its category*, so `as_expression` stops being
a builtin that answers a decision the type system cannot see and becomes a function returning a value whose type says
which branch it is in. That is a strictly better tool for the macro layer, which is the layer this language is actually
for.

**`Accepts` is subtyping and prompt 143 refused it.** It names a δ-builtin taking the found type to the expected one —
an inserted coercion, in Luo's sense, scoped to base-type index literals. It cannot live in conversion because
conversion is symmetric and a rule that certifies both directions certifies nothing. **The resolution is to delete it,
not to generalize it**: with `Syntax : Cat -> Type`, forgetting is

```
forget : Syntax c -> Syntax TokenTree
```

an ordinary total function written at the splice site. The measured cost of going explicit is four internal call sites
here and two hand-written projections in the stdlib (`triad_chord`), against which the gain is that `≡` has no companion
rule and `convert.rs` decides acceptance alone.

**After this prompt, one sentence is true with no exception:** no rule accepts a program that conversion would reject.
Prompt 144's specification states it; this is the prompt that earns it.

## Target

- `stdlib/`: `Syntax` declared as a family; `forget`, `as_expression`, and the category decisions as library functions.
- `crates/musa-calculus/src/kernel/base.rs`: `Accepts` and the `index: Option<Binder>` field removed.
- `crates/musa-calculus/src/elaboration/elab/check.rs`: the four call sites removed.
- `crates/musa-compiler`: the `as_expression` builtin removed from the registry.
- `stdlib/src/adapters/`, `examples/`: `forget` written where the coercion used to fire.
- `crates/musa-calculus/tests/suite/conversion_laws.rs`: the law that acceptance and conversion agree, with the negative
  control that `Syntax Expr` is not accepted where `Syntax TokenTree` is expected without `forget`.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! grep -rn 'Accepts' crates/musa-calculus/src
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

The `grep` is the deliverable: musa's one subtyping rule is gone.

Commit as `Make Syntax a family and delete the coercion rule`.

## Stop

- No general coercion mechanism, declared or otherwise. Prompt 143 refused it and this prompt is the reason it can be
  refused without cost.
- No macro rework — 159.
