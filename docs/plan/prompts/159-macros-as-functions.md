---
id: 159
slug: macros-as-functions
status: pending
depends_on: [158]
phase: 3
---

# Macros as Ordinary Total Functions Over One Evaluator

## Task

Musa's case for existing is that a composer can write music-theoretic transformations as language, so the macro layer
matters more than any other part of this overhaul. Finish it: a macro is an ordinary total function from `Syntax` to
`Syntax`, run by the same evaluator that checks everything else, with typed quotation, splicing, and hygiene, and with
its provenance preserved.

## Read

- `docs/rules/language/11-quotation.md` after 145 and 158.
- `/Users/jcreinhold/Code/Idris2/src/TTImp/Reflect.idr` and `Core/Reflect.idr` — roughly 3,200 lines of Reify/Reflect,
  which is the thing musa is *not* building. Read it to see the size of what one evaluator saves.
- `docs/rules/across-stages/` — the `Original` provenance a macro must not lose.

## Design

**One macro system, not two.** Idris2 carries syntax rewriting *and* elaborator reflection: a `%macro` runs in an `Elab`
monad, and every type that crosses between the object language and the macro language needs a Reify and a Reflect
instance. Musa needs neither, because a macro is a function over an ordinary family and the evaluator that runs it is
the one that already normalizes everything else. **This is the one place musa ends up simpler than the reference, and it
comes directly from having committed to a single theory.**

**Typed quotation, and what the type buys.** `` `{ ... } `` at category `c` has type `Syntax c`. Splicing `$x` demands
`Syntax c'` for the category the hole sits at; `$..xs` splices a `List (Syntax c')`. After 158, `c` is an index a
`match` can refine, so a macro that inspects what it was handed learns the category rather than asserting it.

**Hygiene, stated as a property rather than a mechanism.** A name introduced inside a quotation is distinct from any
name at the splice site, and a name captured from the splice site resolves there. State it as the law and let the
implementation be whatever satisfies it; the law suite is the deliverable.

**Totality is what makes this safe.** A macro is checked like any other function, so it terminates, so expansion
terminates. There is no `%macro` escape hatch and no partiality, which is why musa can afford to run macros with the
ordinary evaluator instead of a sandboxed one.

**Provenance survives expansion.** Every node a macro builds carries an `Original` that points at what the author wrote.
This is the property the Origin view depends on and it is not negotiable for the sake of a simpler expander.

## Target

- `stdlib/`: the quotation vocabulary as library functions over the `Syntax` family.
- `crates/musa-compiler/src/expand/`: expansion as evaluation, with the builtins 158 removed no longer needed.
- `crates/musa-calculus/tests/suite/`, `crates/musa-compiler/tests/suite/`: the hygiene laws, the category-refinement
  law, and a provenance law over an expanded region.
- `docs/book/`: the macro chapter rewritten against the family.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- check examples/*.musa
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Make macros ordinary total functions`.

## Stop

- No elaborator reflection, no `Elab` monad, no Reify/Reflect, no quoting of core terms.
- No macro that can fail to terminate, and no annotation that would let one.
