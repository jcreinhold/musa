---
id: 127ca
slug: builtin-ownership-registry
status: done
depends_on: [127c]
phase: 3
---

# Give Compiler-Owned Operations Their One Name

## Task

Discharge the clean-break ledger row prompt 127b left resolving: rename `PrimitiveOwnership` to `BuiltinOwnership`,
merge the two ownership tables into the one registry §5.8 describes, and stop the word *primitive* from naming a
compiler-owned operation anywhere in the workspace. This frees the name for prompt 127d's registered units.

## Read

- [`../clean-break-ledger.md`](../clean-break-ledger.md) §2, the row assigned to 127b, and §7's discharge rule.
- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) — the paragraph beginning "Two
  words, two meanings", and §5.8's four families and its statement of the registry's law suite.
- [`../../rules/across-stages/03-machine-calculus.md`](../../rules/across-stages/03-machine-calculus.md) §2, for what a
  *registered primitive* will mean at prompt 127d.
- `crates/musa-compiler/src/core/mod.rs`: `PrimitiveOwnership`, `PRIMITIVE_OWNERSHIP`, `BUILTIN_OWNERSHIP`, `Family`,
  `Eliminator`, and the ownership law suite at the end of the file.
- `crates/musa-language/src/types.rs` and its three consumers.

## Design

This prompt renames and merges. It adds no operation, no family member that 127d owns, and no behaviour.

**One word per meaning.** `02-core-calculus.md` already fixes the vocabulary and this prompt only makes the code say it:
a **primitive** is a registered unit whose implementation the language does not own; a **builtin** is an operation the
compiler owns and can reason about. Both current ownership tables hold compiler-owned operations, so both are builtins,
and the collision that made this look undecidable is an artifact of the old naming rather than a real distinction.

**One registry.** §5.8 states the law over a single table — "every builtin is classified exactly once" — and the current
law suite already reaches across both tables to check it. Merge them:

- `struct PrimitiveOwnership<T>` becomes `struct BuiltinOwnership<T>`;
- the 71-entry `Primitive` enum and the 8-entry `Builtin` enum become one `Builtin` enum;
- `PRIMITIVE_OWNERSHIP` and `BUILTIN_OWNERSHIP` become one `BUILTIN_OWNERSHIP: [BuiltinOwnership<Builtin>; 79]`;
- `eval_primitive`, `primitive_named`, and `primitive_application` take the matching `builtin` spellings;
- the two lookup sites collapse into one, and the law suite reads one table rather than chaining two.

**One family rename, and no new family.** `Family::Music` is already documented as §5.7's family; rename it
`Family::Track` so the four families of §5.8 are spelled as §5.8 spells them. `Family::Machine` is prompt 127d's to add,
with the operations that populate it. Do not add an empty variant here.

**Base types are base types.** The same governing paragraph says base types "are called *base types*, never primitives",
so `musa_language::PRIMITIVE_TYPES` and its four consumers become `BASE_TYPES`. This is beyond the ledger row's literal
text, which covers operations; it is in scope because a prompt whose purpose is to leave the word one meaning cannot
leave a second one exported from a public facade.

**What keeps the word.** `docs/rules/kernel/` and `crates/musa-kernel` use *primitive* in its ordinary English sense of
*irreducible*, which `02-core-calculus.md` explicitly permits where no registered unit is in scope; leave those. Leave
`num_enum::IntoPrimitive` and `num_enum::FromPrimitive` in `syntax_kind.rs`, which are a dependency's derive names and
not ours to spell. Correct the two `syntax_kind.rs` doc comments that call a base type a primitive.

Update the ledger row's replacement column only if the merged registry makes its text inaccurate; the row already names
`BuiltinOwnership<Builtin>` as the replacement, so it should need no change.

## Target

- One `BuiltinOwnership<T>`, one `Builtin` enum, one `BUILTIN_OWNERSHIP` table, and one lookup path in
  `crates/musa-compiler/src/core/mod.rs`.
- `Family::{Delta, Eliminator, Track}`, with the doc comment on each citing the §5.8 family it names.
- The ownership law suite reading one table, with its classified-exactly-once and family-count assertions restated over
  79 entries rather than 71 plus 8.
- `musa_language::BASE_TYPES` and its consumers in `musa-compiler` and `musa-lsp`.
- Renamed tests, including `every_primitive_is_reachable_from_the_spelling_it_replaced`,
  `every_first_order_primitive_is_total_on_its_declared_domain`, `finite_primitives_agree_with_small_reference_folds`,
  and `the_schema_libraries_add_no_compiler_primitive`. No old test name survives as an alias
  ([`../clean-break-ledger.md`](../clean-break-ledger.md) §5's reason applies here too).

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! rg -n 'PrimitiveOwnership|PRIMITIVE_OWNERSHIP|PRIMITIVE_TYPES|eval_primitive|primitive_named|primitive_application|Primitive::' crates
! rg -n 'Family::Music' crates
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Repair prompt 127b: rename compiler-owned operations to builtins`.

## Stop

- No `Family::Machine`, no `Primitive<K,A,B>`, no registered-unit type, and no primitive descriptor table — all of that
  is prompt 127d's, and adding a placeholder here would put the name back into two meanings from the other side.
- No new builtin, no removed builtin, no changed signature, and no changed evaluation. The 79 entries after this prompt
  are the 71 and the 8 before it.
- No rename inside `musa-kernel` or `docs/rules/kernel/`, where the word carries its ordinary sense.
- No alias, no deprecated re-export, and no `#[doc(hidden)]` bridge for either old name.
