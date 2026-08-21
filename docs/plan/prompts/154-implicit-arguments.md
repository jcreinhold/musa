---
id: 154
slug: implicit-arguments
status: pending
depends_on: [153]
phase: 3
---

# Make Implicit Arguments Real and Writable

## Task

`Filling` exists in the term language and elaboration does not read it. Make it read it: an `Inferred` Π binder is
solved at application by a metavariable rather than demanded from the author, an implicit binder may be *written* at a
declaration, and an argument may be supplied by name where inference cannot reach it. Without this, dependent types are
formally present and ergonomically unusable.

## Read

- `crates/musa-calculus/src/kernel/term.rs`, `Filling` — the two arms 147 named.
- `/Users/jcreinhold/Code/Idris2/src/TTImp/Elab/App.idr` — insertion at application, and where it stops.
- `docs/rules/language/01-surface.md` after prompt 145's repair — the surface syntax this implements.

## Design

**Insertion is at application and at the end of checking, and nowhere else.** When a function's type is an `Inferred` Π
and the next written argument is not that argument, insert a metavariable. When a checked term's type is an `Inferred` Π
and the expected type is not, insert a λ. Two rules; everything else follows.

**Where insertion stops is the whole design.** It stops when the head is being applied to *no* further arguments and the
expected type is itself an `Inferred` Π — otherwise `id` used as a value would eta-expand forever. This is the rule
Idris2 states as `expandAmbigName`'s companion and it is the one place a naive implementation loops.

**Named implicits, because inference genuinely cannot always reach.** `f { A = Nat } x`. The surface already has the
brace form for type parameters; this generalizes it, and prompt 145 wrote it into `01-surface.md`.

**No `auto` implicits and no proof search.** Idris2's third `PiInfo` arm runs a search procedure to fill an argument.
Prompt 143 refused it: it is the trait system's dispatch problem re-entering through the type language, and 146 deleted
that system on measured evidence. `Filling` has two arms and gains no third.

**The stdlib is the measurement.** After this prompt, the level parameters prompt 152 generalizes are *inferred* at
every use site rather than written. If they are not, inference is not working and the prompt is not done — that is a
sharper check than any synthetic test, and it is why 151 comes first.

## Target

- `crates/musa-calculus/src/elaboration/elab/`: insertion at application, insertion at check, the stopping rule, and
  named implicit resolution.
- `crates/musa-syntax`, `crates/musa-compiler/src/lower/`: written implicit binders and `{ name = value }` arguments.
- `stdlib/`: the signatures that currently spell a type parameter explicitly, made implicit where inference reaches.
- `crates/musa-calculus/tests/suite/implicit_laws.rs`: the stopping rule as a test that would loop without it.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- format --check stdlib/src
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

**The re-checker's obligation for this prompt**: an inserted implicit argument is an ordinary application in the
finished term, so the existing arm covers it — but the suite must contain a fixture whose implicits were *inserted*
rather than written, re-checked. Prompt 158 audits it.

Commit as `Make implicit arguments real and writable`.

## Stop

- No `auto`, no instance arguments, no search.
- No change to how explicit arguments elaborate.
