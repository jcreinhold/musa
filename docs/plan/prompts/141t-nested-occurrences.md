---
id: 141t
slug: nested-occurrences
status: done
depends_on: [135, 141]
phase: 3
---

# A Family May Hold a List of Itself

## Task

`02-core-calculus.md` §1.1 states exactly one positivity prohibition: "A recursive occurrence may not appear to the left
of an arrow at any depth." [`declare.rs`](../../../crates/musa-calculus/src/elaboration/declare.rs)'s `occurrence`
refuses a second thing the section never mentions — a recursive occurrence *nested inside another family's argument*. So

```musa
data StaffRead {
    Sung,
    Body(items: List<StaffRead>),
}
```

is refused with "`StaffRead` occurs in `Body` where a recursive occurrence is not allowed", and
[`family.rs`](../../../crates/musa-calculus/src/kernel/family/mod.rs)'s module doc records the narrowing deliberately,
offering "written as a mutual declaration instead" as the repair.

The repair does not hold up. Writing the mutual declaration means hand-rolling a second list family per containing type
and losing `map`, `filter`, `List`'s instances, and the list literal — which is the language limitation prompt 141
existed to close, reappearing one layer down. Prompt 132's paper trial writes the shape twice, and prompt 139's
acceptance fixtures compile it: [`tests/fixtures/staff-dispatch.musa`](../../../tests/fixtures/staff-dispatch.musa)
emits `Body([$..children])`, whose whole point is the sequence splice into a list literal, and both staff fixtures are
red for this reason and no other.

Code and a governing document disagree, so one of them is wrong. Here it is the code: widen `declare.rs` to §1.1's
actual rule.

## Read

- [`02-core-calculus.md`](../../rules/language/02-core-calculus.md) §1.1 (the positivity sentence this prompt
  implements, and the generated recursor it promises), §1.2 (storability, which a nested field must still satisfy), §2.4
  (the well-founded measure, which is what a fold *through* a nested field would have to answer to), §5.8 (a
  conservative extension is argued, not assumed).
- [Prompt 135, inductive families](135-inductive-families.md) — the declaration checker and the recursor it generates.
  This prompt widens what 135 accepts; it does not change what 135 does with what it already accepted.
- [Prompt 141, collections](141-collections.md) — `List` and the literal, which is what a nested occurrence is nested
  *in* and why hand-rolling around it is a loss rather than a spelling.
- [`141s-numeral-representation.md`](141s-numeral-representation.md) — the precedent for a `musa-calculus` prompt
  landing inside 142's migration, and its **Check** section's argument for why the compiler suite is not the gate.
- Peyton Jones ch. 4 §4.1–§4.2 — what a structured-type declaration means and which occurrences a fixed point admits.
  Ch. 4's account is the reason the prohibition is about *arrows* and not about depth: a negative occurrence admits a
  fixed point that diverges, and an occurrence under a positive functor does not.

## Design

### What is being widened, precisely

A field type is currently classified three ways: it is a direct recursive occurrence `N p⃗ i⃗`, it mentions no family in
the group, or it is refused. This adds a fourth: it mentions a family in the group, but only at **strictly positive
parameter positions of already-declared families**, never to the left of an arrow. `List<StaffRead>` and
`Option<List<StaffRead>>` are that; `(StaffRead -> Nat)` and `Result<Nat, StaffRead -> Text>` are not, and stay refused
by the same sentence that refused them before.

### Polarity is a property of a declaration, computed once

Whether `List<X>` is positive in `X` is a question about `List`, not about the field. So [`Group`] gains one flag per
parameter, computed in `declare_data` after the constructors are built and stored beside them — the same arrangement
[`Constructor::recursive`](../../../crates/musa-calculus/src/kernel/family/mod.rs) already has, and for its reason: the
positivity check and every later reader must be the same list rather than two derivations that can disagree.

A parameter is positive when, in every constructor field of every family in its group, each occurrence of it is either
the field type itself, an argument at a positive parameter of an applied family, or an argument of a family *in this
same group*. The last clause is the fixpoint, and taking it optimistically is what makes the analysis terminate:
`Cons : A -> List A -> List A` passes `A` through a recursive occurrence of the very family whose polarity is being
computed, and refusing to answer until it is answered is the only alternative.

`Shape::Const` carries `Arc<Group>`, so the flags are reachable from the field type with no name table and no second
pass — which is also why this is a lookup rather than an analysis rerun per field.

### A nested field is not a recursive field

[`Constructor::recursive`](../../../crates/musa-calculus/src/kernel/family/mod.rs) stays the direct occurrences only, so
the generated recursor gives **no induction hypothesis** for a nested field. That is a real weakening and it is the
decision, not an omission:

- §1.1 promises the recursor exists and is the only eliminator. It does not promise a hypothesis per field, and a
  recursor with fewer hypotheses is weaker rather than unsound: the family is still the least fixed point of a strictly
  positive functor, which is the whole of what consistency (148) rests on.
- The alternative is a generated `All P xs` predicate per nested family plus the functorial map that builds one, which
  is a second declaration mechanism and a prompt of its own.
- `family.rs`'s standing invariant — "an induction hypothesis is an application rather than a synthesized closure, so ι
  never builds syntax" — is preserved *exactly* by giving nested fields none. Building one is the thing that would break
  it.

What this buys and what it does not: a nested family can be **constructed**, and a `match` on it **binds** the children
as an ordinary `List<StaffRead>` (`case.rs` splits the column at `Body` and the field is just a field). What cannot yet
be written is a fold that recurses *into* the children, because the only route is mutual recursion through `List`'s own
eliminator and §2.4's measure has no reason to believe an element of `items` is smaller than `Body(items)`. That is a
real gap, it is the next prompt in this direction, and this prompt **records it with a law** rather than leaving a
reader to discover it — see **Target**.

### Storability and the machine boundary are unaffected

§1.2 asks whether a field's type is storable, which is a question about `List<StaffRead>` as a type and not about where
`StaffRead` appears in it. A container of storable data is storable; a container of arrows is not, and the arrow rule is
untouched. Nothing about the machine ports, the event track boundary, or the δ readback changes, because none of them
asks about positivity.

### The refusal keeps its name and gains its reason

`Refusal::NonPositive` still fires, at strictly the arrow case now. Its message names the occurrence, which is where the
edit is; the help should say what is actually wrong — an occurrence left of an arrow — rather than the blanket sentence
it says today, because "a recursive occurrence is not allowed" was true of two different faults and is now true of one.

## Target

- `Group` carries one positivity flag per parameter, computed at `declare_data` from the constructors it just built,
  with the group's own families taken optimistically.
- `occurrence` accepts a field whose recursive occurrences all sit at positive parameter positions, and refuses the
  arrow case with a help that names it.
- `Constructor::recursive` is unchanged: direct occurrences only, so `counting()`, ι, and `case.rs` see exactly what
  they see today.
- **The construction law**: `data StaffRead { Sung, Body(items: List<StaffRead>) }` declares, `Body([Sung, Sung])`
  checks, and a `match` on it binds `items` at `List<StaffRead>`.
- **The nesting-depth law**: `Option<List<StaffRead>>` is accepted and `Result<Nat, List<StaffRead>>` is accepted, so
  the rule is about position rather than about one level.
- **The negative laws**: `Bad(f: StaffRead -> Nat)` is refused, and so is `Bad(f: List<StaffRead -> Nat>)` — the arrow
  under a positive parameter is still an arrow. Each with its negative program, as prompt 135's refusals have.
- **The recorded-gap law**: a function that folds into a nested field is refused, with the refusal asserted as the
  finding it is — so that the day §2.4's measure learns to see through a container, the law changes and says so.
- `stdlib/`, `examples/`, and every `.musa` fixture are untouched; the two staff fixtures in
  [`crates/musa-compiler/tests/suite/`](../../../crates/musa-compiler/tests/suite/) compile with no edit, which is the
  observation this prompt is for.
- `02-core-calculus.md` §1.1 gains one sentence stating that a nested occurrence is admitted and carries no induction
  hypothesis, and the `docs/plan/code-map/` rows for the changed files.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus
cargo nextest run -p musa-compiler -E 'test(staff_construction_fixture) or test(staff_dispatch_fixture)'
cargo clippy --all-targets -p musa-calculus -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

`musa-compiler`'s full suite and its clippy run are not gates here, for the reason
[`141s`](141s-numeral-representation.md)'s **Check** states and measures: this prompt lands inside 142's migration, that
suite is red for migration reasons this prompt neither causes nor can fix, and `-D warnings` is red on `core.rs`'s
superseded checking paths that 142's own Target deletes. The four staff-fixture tests are named above because they are
what this prompt is *for*; the whole corpus answers at 142's Check.

Commit as `A family may hold a list of itself`.

## Stop

- **No induction hypothesis for a nested field.** The `All` predicate and the functorial map that would build one are a
  separate prompt; this one records their absence as a law.
- **No change to `Constructor::recursive`, `counting()`, ι, or `case.rs`.** If widening positivity needs any of them to
  move, the design is wrong and this is a repair.
- **No corpus migration.** Not one `.musa` file or snapshot moves. The staff fixtures compile because the core widened,
  and if either still needs an edit, that edit is 142's.
- **No change to §1.2's storability rule** and no change to what may cross the machine or event track boundary.
- **No relaxation of the arrow rule.** An occurrence left of an arrow stays refused at any depth, under any container.
