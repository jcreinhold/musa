---
id: 161
slug: one-declaration-form
status: pending
depends_on: [157]
phase: 3
---

# One Declaration Form: `enum` and `record` Become Sugar Over `data`

## Task

The surface has three ways to declare a type — `data`, `record`, `enum` — where one general form covers all three. Give
`data` index syntax at the surface (prompt 156 gave it to the core), and make `record` and `enum` desugar to it. Three
forms to one, with two spellings kept for readability rather than for meaning.

## Read

- `docs/rules/language/01-surface.md` after prompt 145's repair.
- `crates/musa-compiler/src/lower/` — the three lowering paths that become one.
- The triage of surface constructs recorded in this overhaul's design notes: which forms were "nice to have" and which
  were foundational.

## Design

**One general form, two conveniences.** `data` is the declaration. `enum` is `data` where every constructor is nullary —
kept because a scale degree list written as `data` reads worse, and refused where a constructor takes a field, so the
spelling cannot lie. `record` is `data` with one constructor and named fields — kept because projection syntax needs
field names, and refused with more than one constructor.

**Desugaring, not a second path.** Both spellings produce a `data` declaration before anything type-checks them. There
is exactly one declaration path in the lowerer afterwards, and that is the check: a bug in `enum` handling becomes
impossible because there is no `enum` handling.

**Diagnostics keep the spelling the author used.** A refusal about an `enum` says `enum`. The desugaring records which
spelling it came from, for messages only — never for typing.

**What this closes.** The user-facing complaint that started this thread: three declaration forms whose differences were
not principled. After 156 and this prompt they are one form and two readabilities, and the difference *is* principled —
arity and field names.

## Target

- `crates/musa-syntax`: index syntax in `data`; `enum` and `record` kept in the grammar.
- `crates/musa-compiler/src/lower/`: one declaration path; the two desugarings; the two refusals.
- `stdlib/`, `examples/`: unchanged where the spellings still fit; rewritten where a `record` wanted an index.
- `docs/rules/language/01-surface.md`, `docs/book/`: the one form and its two spellings.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- format --check stdlib/src examples
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Collapse the declaration forms to one`.

## Stop

- No removal of the `enum` or `record` keywords. This prompt removes a *semantics*, not a spelling.
- No module-layer work — 161.
