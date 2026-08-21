---
id: 150
slug: delete-the-index-stratum
status: pending
depends_on: [147]
phase: 3
---

# Delete the Index Stratum and Restore Read-Back Equality

## Task

Delete the `Indexed` shape, the `Form::Indexed` value, `index.rs` (453 lines of linear-form solver), and the erasure
special case in `quote`. Re-register `Pc` and `Row` as ordinary parameterized types, so `Pc(12)` becomes `Pc 12` — a
type constructor applied to a `Nat`, decided by ordinary NbE. Nothing is put in their place.

## Read

- `docs/plan/prompts/142c-index-amendment.md` and [`142d`](142d-index-stratum.md) — what was admitted, and the count.
- `crates/musa-calculus/src/convert.rs`, the `Form::Indexed` arm, and its comment: "reading them back would print one
  word twice, because erasure is what quotation does."
- `crates/musa-calculus/src/quote.rs`, `Form::Indexed { ty, .. } => read_type(meter, reading, ty)` — the index dropped.
- `crates/musa-calculus/src/base.rs`, `kind: Term` — the machinery that already does what the stratum was added for.

## Design

**The count is discharged by machinery already present.** 142c cited 17 of 121 builtins hardcoded to modulus 12. A
`Base` already carries `kind: Term`, so a base type may take parameters — `Nat : Type 0` and `Syntax : Type 0 → Type 0`
are both base types today. `Pc : Nat → Type 0` needs nothing new. The 17 builtins collapse at prompt 163, which is why
that prompt's dependency moves here.

**The solver's one unique buy is unused.** Index *arithmetic* — `Bar(p+q)`, a bar whose length is the sum of its parts —
is the only thing a Presburger solver does that application and NbE do not. **Zero committed `.musa` files use it**, and
142c's own Design already conceded that `Bar(m)` is checkable only when the durations are static, which is the case
application handles.

**This is the prompt that makes §3 true.** Today §3 says `A ≡ B iff quote(A) = quote(B)` while §1.5 erases indices at
`quote`; together they say `Pc(12) ≡ Pc(24)`. The code disagrees with the spec by comparing `Form::Indexed` structurally
on *values*, before quoting — a conversion rule read-back cannot decide. With no erasing wrapper, the equation holds as
written and `convert.rs` loses an arm rather than gaining one. **After this prompt, `convert.rs` must have no
type-specific arms at all**, and that is checkable by reading it.

**Erasure needed no replacement.** §1.5's erasure protected byte-identity of stored artifacts. Identity digests event
tracks, bindings, seed and options — never a core term — so no artifact contains a type and nothing was protected.
Prompt 144 already wrote this down; this prompt is where it is relied on.

## Target

- `crates/musa-calculus/src/kernel/term.rs`, `value.rs`: `Indexed` and `Form::Indexed` removed.
- `crates/musa-calculus/src/kernel/index.rs`: deleted.
- `crates/musa-calculus/src/kernel/{quote,convert}.rs`: the erasure case and the structural arm removed.
- `crates/musa-compiler/src/phase/types.rs`: `Pc12`/`Row12` re-registered as `Pc`/`Row` applied to a numeral. The
  seventeen builtins stay seventeen here; 163 collapses them.
- `stdlib/`, `examples/`: `Pc(12)` → `Pc 12` and the same for `Row`, wherever written.
- `crates/musa-calculus/tests/suite/`: the index law suite deleted; a conversion law added stating that `Pc 12` and
  `Pc 24` are not convertible *and* that read-back distinguishes them.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! test -f crates/musa-calculus/src/kernel/index.rs
! grep -rn 'Indexed' crates/musa-calculus/src
git diff --stat crates/musa-calculus    # must be net negative by ~600 lines
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Delete the index stratum`.

## Stop

- No collapse of the seventeen builtins (163), no universe change (151), no families (155).
- No new solver of any kind. If an index turns out to need arithmetic, that is a family with a `Nat` index and prompt
  155's business — not a decision procedure beside conversion.
