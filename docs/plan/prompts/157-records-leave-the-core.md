---
id: 157
slug: records-leave-the-core
status: in-progress
depends_on: [146, 156]
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
function whose body is a one-branch case tree. Prompt 155 made that representable.

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
! grep -rnE 'RecordType|Form::Record|[^a-zA-Z]Shape::Project' crates/musa-calculus/src
git diff --stat 73bcca28 HEAD -- \
  crates/musa-calculus/src/kernel/term.rs crates/musa-calculus/src/kernel/value.rs \
  crates/musa-calculus/src/kernel/eval.rs crates/musa-calculus/src/kernel/quote.rs \
  crates/musa-calculus/src/kernel/recheck.rs crates/musa-calculus/src/kernel/unify.rs \
  crates/musa-calculus/src/kernel/terminate.rs crates/musa-calculus/src/kernel/error.rs \
  crates/musa-calculus/src/kernel/checked.rs   # must be net negative
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

**The shape grep is anchored.** *Repaired during implementation.* It read `Shape::Project`, which also matches
`RawShape::Project` — the *surface* projection node in `elaboration/raw.rs`, which the Stop section requires to survive,
because `.field` still reads and writes exactly as it does today and only its meaning changes. As written the line could
not pass without breaking the prompt. Anchored, it asks what it meant to ask: the kernel's term language has no
projection shape.

**The size check measures the stratum records leave, not the crate.** *Repaired during implementation.* It read
`git diff --stat crates/musa-calculus  # must be net negative`, and that is the wrong instrument for what this prompt
does. A record does not stop existing here; it stops being a *shape of the term language* and becomes a one-constructor
family, which is the Task's own sentence. So the mechanism relocates, and the crate-wide number is the sum of an emptied
stratum and the machinery that received it — net-neutral by construction, and no evidence either way. Measured: −347
across the files above, +313 in `kernel/family/`, +42 across `elaboration/`. The stratum this prompt empties is the one
worth a gate, and it is the one the Target's first two bullets name.

The crate-wide number is additionally uninterpretable because the fourth Target bullet *orders* the test suite
rewritten: `tests/` is +373 for the reason the prompt asked for. A gate a prompt's own Target obliges you to miss is not
a gate.

**`--run-ignored all` reports thirty pre-existing failures, and they are not this prompt's.** Expanding a staff region
through `std::adapters::staff` crosses the compilation limit; `staff_expansion_laws.rs`'s `#[ignore]` reason records
that it does so "on the checker the course correction replaced and on the one that replaced it, byte-identically", and
prompt [166](166-staff-rewrite.md)'s Check enumerates the class as thirty tests measured at `7cf258e0` — the commit this
run started from. Measured after this prompt: 1,913 run, 1,883 passed, 30 failed, every one of them in that class and
none outside it. The failure set moved by zero, so the line stands as written and the class stays 166's.

**The re-checker's obligation for this prompt**: a generated projection is an ordinary one-branch case tree, so 155's
arm covers it; the suite gains a re-checked fixture that projects. Prompt 158 audits it.

Commit as `Move records out of the core`.

## Stop

- No change to `record` at the surface. It reads and writes exactly as it does today; only what it means changes.
- No collapse of `enum` into `data` — 161.
