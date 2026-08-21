---
id: 149
slug: errors-below-refusals
status: pending
depends_on: [148]
phase: 3
---

# Put the Kernel's Errors Below the Elaborator's

## Task

`error.rs` says of `CoreError::Refused` that it carries a sentence and a place "because this module is below that one:
`ElabError`'s conversion is where the two meet, and is the only lift." That is the right architecture and it is not the
one in the code: one `Refusal` enum braids two claims that are not the same claim — *the kernel could not decide this
conversion* and *the author wrote a name that is not in scope* — and the files that decide conversion reach straight for
it. Separate them.

**This prompt supersedes [`142g`](142g-errors-below-refusals.md)**, which specified the same split when `Refusal` had
seventy variants. It runs after 146, 147 and 148, which between them delete the trait variants, the shape variants, and
the ambiguity about which file is on which side of the line. The split is the same; the list is shorter and the boundary
it follows is now a real one.

## Read

- [`142g-errors-below-refusals.md`](142g-errors-below-refusals.md) — the argument, in full.
- `crates/musa-calculus/src/kernel/error.rs` and `src/elaboration/refuse.rs` after 148's move.

## Design

**Which variants go down, and the test for it.** A variant belongs to `CoreError` when a caller can reach it *with no
program in hand* — by handing the kernel two terms, a sort, or a registry. That is: the conversion mismatch with its
`PathStep`/`Mismatch` payload, the universe-arithmetic refusal, and the registry's structural checks. Everything whose
sentence names something the author *wrote* stays a `Refusal`. After 148 this test has a mechanical form: a variant
raised from a file under `kernel/` is a `CoreError`, and the boundary law suite already forbids the other direction.

**One lift, and it is the one already documented.** `ElabError: From<CoreError>` is the only crossing, and it is where
the place and the written spelling are attached. The kernel does not know how to spell a type the author's way.

**No diagnostic changes, and that is checkable.** Every surface code, sentence, help line and span is byte-identical
afterwards; moved variants keep their tags, now supplied by the lift. This is the whole safety argument: a refactor that
also reworded a diagnostic could not be reviewed.

**`Mismatch` is shared, not duplicated.** It describes two terms that did not agree, which is a kernel fact, so it moves
down whole and `Refusal` names the lifted `CoreError` rather than a second copy.

**One variant class is new and belongs at the bottom.** Prompt 152's unification failures — *these two terms have no
solution*, *this constraint is still blocked* — are kernel facts with no program in hand, and this prompt's rule assigns
them without a special case. Say so, so 152 does not invent a third enum.

## Target

- `crates/musa-calculus/src/kernel/error.rs`: `CoreError` gains the conversion, sort-arithmetic, registry and
  unification variants.
- `crates/musa-calculus/src/elaboration/refuse.rs`: `Refusal` keeps only what names written syntax; the `From` lift is
  the single crossing.
- `crates/musa-calculus/tests/suite/`: the coverage gate updated to the two enums, with its count preserved.
- `142g-errors-below-refusals.md`: `status: superseded` and a banner naming this prompt.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

`--unreferenced=reject` with every snapshot unchanged is the diagnostic-identity check. One changed snapshot means a
message moved, which this prompt forbids.

Commit as `Put the kernel's errors below the elaborator's`.

## Stop

- No reworded diagnostics, no new codes, no merged variants.
- No error-handling change in `musa-compiler`; the lift keeps the facade's type the same.
