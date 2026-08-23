---
id: 159
slug: delete-the-coercion-rule
status: done
depends_on: [156]
phase: 3
---

# Delete the Language's Only Coercion Rule

## Task

The category-forgetting rule — *a position of category `TokenTree` accepts a value of any category* — is implemented as
`Accepts` in `base.rs`, fired from `elab/check.rs` and deliberately kept out of conversion. **Delete it**, and make
forgetting a total function an author writes: `forget : Syntax Expr -> Syntax TokenTree`, which already exists as a
δ-builtin and is reachable from nowhere.

## Read

- `docs/rules/language/11-quotation.md` §1 as this prompt repaired it, and
  [`55-cat-stays-a-base-type.md`](../../notes/research/language-design-closure/55-cat-stays-a-base-type.md) for the
  measurement behind the repair.
- `crates/musa-calculus/src/kernel/base.rs`, `pub type Accepts` — and its doc comment, which already names the problem:
  *"conversion is symmetric, so a rule living there would admit the reverse direction too."*
- `crates/musa-calculus/src/elaboration/elab/check.rs`, `carried` and `at_a_literal_index`.
- `crates/musa-compiler/src/registry/rules.rs`'s `forgets`, `FORGOTTEN` and `FORGET`, and
  `crates/musa-compiler/src/phase/ownership.rs` — the table a name has to be in before an adapter can write it.

## Design

**`Accepts` is subtyping and prompt 143 refused it.** It names a δ-builtin taking the found type to the expected one —
an inserted coercion, in Luo's sense, scoped to base-type index literals. It cannot live in conversion because
conversion is symmetric and a rule that certifies both directions certifies nothing. **The resolution is to delete it,
not to generalize it**: the carrier it inserts already exists with the right signature, and what has kept it out of
reach is one missing row in `SYNTAX_OWNERSHIP`.

**After this prompt, one sentence is true with no exception:** no rule accepts a program that conversion would reject.
Prompt 144's specification states it; this is the prompt that earns it.

**The scope repair, recorded.** *Repaired during implementation, before a line of it was written.* The Task read "Make
`Syntax : Cat -> Type` a real family; `as_expression` becomes an ordinary function; and `Accepts` goes", and two of the
three were not what they looked like.

`as_expression` **is already** an ordinary function — row `SyntaxOp::AsExpression` in `SYNTAX_OWNERSHIP`, at
`Syntax TokenTree -> Option (Syntax Expr)`, since prompt 142. The Target's "removed from the registry" would have
deleted a working operation.

And `Cat` cannot become a family here. Two mechanisms hold it: a δ-rule is a bare `fn` pointer and cannot write down a
family's constructor, which the fourteen syntax rules need because a `Syntax` literal carries its own type; and the
registered base types are a process-global keyed by name, out of which `Syntax`'s kind is written. Both are argued in
the code by name as D3 decisions. The family therefore costs a new `Datum` arm in the core and a registry rewrite —
and buys nothing, because its three claimed benefits are respectively already had, deliverable without it, and
unwritable (a `match` on a syntax value, which §1 elsewhere says has no eliminator). `11-quotation.md` §1 is repaired
in this prompt's commit and note 55 records the measurement.

What survives is the half that was always the point, and it is the half the title names.

## Target

- `crates/musa-calculus/src/kernel/base.rs`: `Accepts`, `Base::accepting`, `Base::accepts` and the `accepts` field
  removed. *The Target also read "and the `index: Option<Binder>` field" — there is no such field in `Base` and there
  is none anywhere under `crates/musa-calculus/src`; it belongs to a shape of `Base` that no longer exists.*
- `crates/musa-calculus/src/elaboration/elab/check.rs`: `carried` and `at_a_literal_index` removed, and the `Switch`
  call site with them. *One call site, not four: §1's "four call sites inside `elab/check.rs`" counted the rule's
  clauses rather than its callers.*
- `crates/musa-calculus/src/lib.rs`, `kernel/error.rs`: the `Accepts` re-export and `Malformed::UnregisteredCarrier`
  removed — the latter is raised only by the deleted site and cannot be reached once it is gone.
- `crates/musa-compiler/src/registry.rs`, `registry/rules.rs`: the `accepting(rules::forgets)` decoration and
  `rules::forgets` removed; the carrier stays and becomes a registered, ownable operation.
- `crates/musa-compiler/src/phase/ownership.rs`: `forget` as a `SyntaxOp` row with its type, so an adapter can write it.
- `stdlib/src/adapters/`, `examples/`, and the compiler's own fixtures: `forget` written where the coercion used to
  fire. Measured, by disabling the rule and running the suite: **26 tests**, every one reporting
  `expected TokenTree, found Expr`.
- `crates/musa-calculus/tests/suite/conversion_laws.rs`: the law that acceptance and conversion agree, with the negative
  control that `Syntax Expr` is not accepted where `Syntax TokenTree` is expected without `forget`.
- `docs/rules/language/11-quotation.md` §1, `01-surface.md`, `02-core-calculus.md` §5, and
  `docs/notes/research/language-design-closure/55-cat-stays-a-base-type.md`: the repair and its record.

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

Commit as `Delete the language's only coercion rule`.

## Stop

- No general coercion mechanism, declared or otherwise. Prompt 143 refused it and this prompt is the reason it can be
  refused without cost.
- **No `Datum` arm and no registry re-plumbing.** They are what `Cat`-as-a-family would cost, and note 55 states the
  condition on which that is worth re-opening — an eliminator for `Syntax`, which is not this prompt and not 160.
- No macro rework — 160.
