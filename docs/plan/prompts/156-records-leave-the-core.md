---
id: 156
slug: records-leave-the-core
status: pending
depends_on: [146, 155]
phase: 3
---

# Move Records Out of the Core

## Task

`RecordType`, `Record` and `Project` are three of the seventeen shapes and they describe a one-constructor inductive
type with generated projections. Delete them from the term language: `record` becomes surface sugar for a
one-constructor family, and a projection becomes a generated function. Three constructors to zero.

## Read

- `crates/musa-calculus/src/kernel/{term,value,quote,convert}.rs` — the three shapes and the η rule at `quote`.
- `docs/plan/prompts/145-repair-the-dependent-rules.md`, the paragraph headed *Record η loses its only client*.

## Design

**Why this was blocked and no longer is.** Records-as-data costs record η, and `10-traits.md` stated coherence as
"unique up to conversion… decided by η at `quote`". Coherence was a property of the trait system; prompt 146 deleted the
trait system, so nothing requires η and the deletion is free. **This is the reasoning to re-read before starting**: a
reviewer who does not know it will read this prompt as removing a conversion rule that something depends on.

**η may still be kept, and cheaply.** A one-constructor family admits η at `quote` by the same rule Π does, and prompt
144's specification allows it. Keep it: it costs one arm in the type-directed reader and it means `r` and
`Mk (r.a) (r.b)` stay convertible, which several stdlib rewrites would otherwise notice. The point of this prompt is
that η is now a *choice about a family*, not a third pair of term constructors.

**Projections are generated functions, not a term former.** `r.field` elaborates to an application of a generated
function whose body is a one-branch case tree. Prompt 154 made that representable.

**Field order stops being load-bearing.** `10-traits.md` §2 keyed instance resolution on the first parameter, which is
why `algebra.musa` says "the carrier comes first". With no resolution, field order is a style rule and prompt 145 moved
it to the style guide.

## Target

- `crates/musa-calculus/src/kernel/term.rs`, `value.rs`: `RecordType`, `Record`, `Project` removed.
- `crates/musa-calculus/src/kernel/quote.rs`: η for one-constructor families, replacing record η.
- `crates/musa-compiler/src/lower/`: `record` desugared to `data`; `.field` to a generated projection.
- `crates/musa-calculus/tests/suite/`: the record law suite rewritten against the family, with the η law kept.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! grep -rn 'RecordType\|Form::Record\|Shape::Project' crates/musa-calculus/src
git diff --stat crates/musa-calculus    # must be net negative
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Move records out of the core`.

## Stop

- No change to `record` at the surface. It reads and writes exactly as it does today; only what it means changes.
- No collapse of `enum` into `data` — 160.
