---
id: 141c
slug: structural-eliminators
status: done
depends_on: [141b]
phase: 3
---

# Give the Core Its Structural Eliminators

## Task

Prompt 141b gave `musa-calculus` §5.8's δ-builtins — first-order operations over base types, reduced by a
`fn(&[&Literal]) -> Option<Literal>`. §5.8 names **four** families, and the second one, *structural eliminators*, is
"the generated recursors and the derived traversals over them". The generated recursors are prompt 135's ι-rule and are
done. The derived traversals are not: eleven compiler-owned operations take a *function* argument, and a δ-rule cannot
express one — `Literal` is a closed value of a base type, and a λ is neither.

Three of the eleven cannot become library code either, and that is what makes this a prerequisite rather than a cleanup.
`recurse_syntax`, `run_syntax_step`, and `syntax_fold_from_leaves` traverse a `Syntax`, which is a *base type* precisely
because the reader's node representation is hidden. A base type has no constructors, so it has no recursor, so there is
nothing for a library traversal to recurse on. Give the core the mechanism, so that prompt 142 has a `recurse_syntax` to
migrate `stdlib/src/adapters/staff.musa` onto.

## Read

- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §5.8, on what a structural
  eliminator is and what separates it from a δ-builtin: the δ conditions D1–D4 are stated over *δ*-builtins only, and
  the arrow-free rule is D1's, not the family's. Then §5.9, which fixes the phase operations as a second registry, and
  §1.4's ι-rule, which is the shape this prompt copies.
- [`141b`](141b-base-types-and-builtins.md) in full — `Base`, `Literal`, `Builtin`, `Family`, `Registry`, and
  `eval.rs`'s `delta`, which is where the new rule fires beside the old one.
- `crates/musa-calculus/src/family.rs`'s `iota` — the existing structural eliminator, and the proof that a rule which
  answers a *term* built from its arguments is enough. A recursor reduces to `method(fields…, hypotheses…)`; nothing in
  it inspects a value that quotation could not have produced.
- `crates/musa-compiler/src/core.rs`'s `Eliminator` (eight entries), `SyntaxOp` (three `PhaseFamily::Fold` entries), and
  `eval_syntax` — the eleven operations, what each is applied to, and which of them prompt 142 must keep working because
  `stdlib/` calls it.
- `stdlib/src/adapters/staff.musa:1932`, `:1957`, `:2234`, and `stdlib/src/adapters/doubled.musa:107` — the calls, with
  their `fn (kid, later) { … }` arguments written out. These are the programs this prompt exists to keep compiling.
- Peyton Jones ch. 6 on the enriched lambda calculus: the enrichment is a set of constants *with their reduction rules*,
  and a rule is a rewrite from a redex to a term. Ch. 4 §4.2 on how a case expression over a structured type becomes an
  application of the type's own eliminator.

## Design

**A structural rule answers a term, not a literal, and that is the whole mechanism.** A δ-rule is
`fn(&[&Literal]) -> Option<Literal>` because a δ-builtin computes a value from values. A structural eliminator does not
compute a value; it *rewrites* — `recurse_syntax(node, algebra)` becomes `algebra` applied to what the node holds,
exactly as `Nat.elim P z s (succ k)` becomes `s k (Nat.elim P z s k)`. So the rule is

```rust
pub type Rewrite = fn(&[&Term]) -> Option<Term>;
```

and the core evaluates whatever comes back. No callback into the evaluator, no handle on a `Value`, and roadmap §15.12's
privacy boundary is untouched — the rule sees terms, which are already public, and builds terms, which the host can
already build. `fn` rather than a closure for D3's reason, unchanged: a rewrite that captured host state would make
reduction depend on which compiler ran it.

**It fires on its target, not on its arguments.** δ fires when *every* argument is a literal, because that is when a
first-order function has everything it needs. ι fires when the *target* is a constructor, because the other arguments
are methods and a method is a function. A structural eliminator is the second shape: it declares which argument is its
target, and it fires when that argument has reduced to a literal — the other arguments may be λs, neutrals, or anything
else, and are passed through untouched. `Builtin::structural(name, ty, target, rewrite)` is where the target index is
written down, and it is written down rather than inferred because "the first argument" is true of the eleven today and
is not a law.

**Firing where δ fires.** Both live in `eval::apply`'s `Form::Neutral` arm, after `family::iota`, for 141b's reason: an
application is the first moment a rule can know its last argument arrived. The new rule is tried when the δ rule does
not apply, and the two are disjoint by construction because a `Builtin` carries one or the other.

**The rewrite may need the argument as a value, and must not have it.** `recurse_syntax` reads the node's children out
of an opaque payload — which it can, because the target arrives as a `Shape::Lit` and `Literal::payload` is public — and
it builds an application of `algebra`, which it has as a `Term`. What it may *not* do is ask what `algebra` reduces to,
and the type is what prevents it: a `Term` is syntax, and there is no evaluator in scope. A traversal that needed to
force a function argument would be asking for a strictness the calculus does not have, and is a finding rather than an
amendment.

**Registration checks what it can see, and what it can see is different here.** D1's arrow-free rule is a *δ* condition
and must not be applied to this family — a structural eliminator's signature contains an arrow by definition, and a
registry that refused one would refuse the family it is registering. What registration does check is that a structural
builtin's target index is within its arity, and that its target's type is a base type of this registry: a rewrite whose
target is a declared family would be a second ι-rule for a type that already has one, which is the second path the
audits keep looking for.

**The eight collection eliminators are not in scope.** `NatFold`, `ListFoldFromStart`, `ListFoldFromEnd`, `OptionFold`,
`Map`, `Filter`, `Range`, and `Repeat` are traversals over `Nat`, `List`, and `Option` — types prompt 141 proved are
ordinary declarations, with `map`, `filter`, and `collect` as derived methods over one fold body. They become library
code in prompt 142's migration and leave the registry in 143. This prompt gives the mechanism the three that *cannot*
need, and says so rather than registering eleven and pretending eight of them earned it.

**Proved by a program, not by the compiler.** As in 141b: a worked structural eliminator in the test suite over a base
type of the suite's own — a tree whose payload holds children — with the laws stated over it. `musa-compiler` does not
change, and prompt 142 is still the one cutover.

## Target

- `crates/musa-calculus/src/base.rs`: `Rewrite`, `Builtin::structural`, the target index on `BuiltinDeclaration`, and
  `Builtin::rewrite`, all doc-commented with their invariants before the implementation. `Builtin::new` keeps its
  meaning and gains a doc line saying it builds a δ-builtin.
- `crates/musa-calculus/src/eval.rs`: the structural arm beside `delta`, firing on a literal target and passing every
  other argument through as the term it was.
- Registration checks: a target index within arity, and a target type headed by a registered [`Base`]; D1's arrow-free
  check narrowed to δ-builtins so that a structural signature is not refused for having the arrow it must have.
- Two new `Refusal` variants with their `musa explain` codes in `crates/musa-compiler/src/diagnose.rs`: a target index
  outside the signature, and a structural eliminator whose target is not a base type.
- `crates/musa-calculus/tests/suite/base_laws.rs`: the worked structural eliminator and its laws — it fires when its
  target is a literal and not before, a function argument is passed through unevaluated, the rewrite's result is
  evaluated by the core rather than by the host, a neutral target leaves a neutral, budget charging, and the two
  registration refusals.
- `docs/plan/code-map/` rows for `musa-calculus`, extending 141b's row rather than adding a second one.
- No change to `musa-compiler`'s checker, no `stdlib/` or `examples/` change, and no builtin moved out of
  `BUILTIN_OWNERSHIP`.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo clippy --all-targets -p musa-calculus -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Give the core its structural eliminators`.

## Stop

- No collection eliminator registered in `musa-calculus`, and no `List`, `Nat`, or `Option` in its `src/`. Those are
  library types; prompt 141 already proved they are writable.
- No wiring, no move of `BUILTIN_OWNERSHIP`, and no `musa-compiler` checker change. Prompt 142 owns the cutover.
- No registry collapse and no builtin deleted. Prompt 143.
- No callback into the evaluator, no `Value` in a public signature, and no interior mutability. The rule takes terms and
  answers a term; anything wider re-opens roadmap §15.12.
- No amendment to §5.8. This prompt implements its second family; a disagreement is a finding to record.
- No strictness on a function argument. If a traversal needs one forced, that is evidence about the traversal.
