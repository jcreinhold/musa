---
id: 162a
slug: module-privacy-for-source
status: done
depends_on: [162]
phase: 3
---

# Give a Source File a Module, So `private` Means Something

## Task

`private` at a file's root is a marker the compiler carries and nothing enforces. The kernel's rule is right and its
laws pass; the compiler never tells it which module a definition was written in, so every source declaration is "written
nowhere in particular" and hides from nobody. Wire it: each file a document elaborates from gets a `ModuleId`, the
context stands in the document's own, and a `private` name in an imported file is refused when another file writes it.
This is what prompt 162 deleted the `signature` layer *for* — sealing by listing became sealing by marking, and this
prompt is where the marking starts biting.

## Read

- `crates/musa-calculus/src/kernel/visibility.rs` — the whole rule, in eight lines. `(Private, None, _) => true` is the
  arm every source definition takes today, and the doc above it says why: a declaration with no module "was written
  nowhere in particular and hides from nobody."
- `crates/musa-compiler/src/document.rs` — `top_level`, the one constructor of a source definition, which writes
  `module: None`; `elaborate`, which builds one `Cx` `in_module(prelude::SOURCE)` for the whole document; and `Source`,
  which already knows which file each declaration came out of.
- `crates/musa-compiler/src/prelude.rs` §`PHASE` and §`SOURCE` — the two ids that exist, and why `0` is reserved for a
  context standing nowhere.
- [`136a`](136a-module-visibility.md) and [`141n`](141n-top-level-program.md), which built the mechanism this prompt
  supplies an argument to. Neither was wrong; neither was asked for the compiler half.
- [`58-the-module-layer-and-its-make.md`](../../notes/research/language-design-closure/58-the-module-layer-and-its-make.md)
  §8 — the measurement, taken while 162 was being implemented.
- `docs/rules/language/01-surface.md` §1.3, which is what `private` is supposed to mean.

## Design

**One id per file, not per declaration and not per package.** `01-surface.md` §1.3 scopes `private` to "the module that
declares it", and a Musa module is a file (`mod tonal;` names `tonal.musa`). So the unit is the source file, and the ids
a document mints are private to that document's elaboration — nothing persists them, nothing compares two documents'
ids, and `SemanticHash` never sees one.

**A document's root and its piece are two `Source`s of one file.** `crate::elaborate`'s `declaring` builds a `Source`
for the root and another for the piece node inside it, so an id keyed on the `Source` would make a piece's voices unable
to see the root's `private` declarations. The id is keyed on the *file* — `Source::from`'s path, or the document's own
name when there is none.

**The viewer is the document's own file, and there is exactly one viewer.** `elaborate` builds one `Cx` for the whole
document, and every declaration in it — the piece's and its imports' — is checked in that context. That is right: the
question `private` answers is "may *this file* name it", and this file is the one that is being compiled.

**Prelude and phase declarations keep `module: None`.** They are written nowhere in particular and are meant to hide
from nobody. Only definitions read out of a `.musa` file get an id.

**Aliased imports are the same rule, said once.** `import std::scale as scale;` files names as `scale.name`, and the
alias is a spelling rather than a scope: a `private` declaration in that file is refused under its qualified name for
the same reason it is refused under its bare one.

**What this does not decide.** Whether a *package* is a privacy boundary as well as a file — whether
`stdlib/src/scale.musa` may name `stdlib/src/voicing.musa`'s private declarations — is a real question and not this
prompt's. File-scoped is the reading `01-surface.md` §1.3 states; if the standard library wants package-scoped, that is
an amendment with its own evidence.

## Target

- `crates/musa-compiler/src/document.rs`: a per-file `ModuleId` minted during `elaborate`, carried on every
  `RawTopLevel` and every `RawData` the document declares, with `Cx` standing in the id of the file being compiled.
  Doc-commented with the sentence of §1.3 it implements and with why a document's root and piece share one id.
- `crates/musa-compiler/src/prelude.rs`: the reserved-id block extended so a source file's ids cannot collide with
  `PHASE` or `SOURCE`, stated once where the two constants are.
- `crates/musa-compiler/src/lower/refusals.rs`: `Refusal::Private` restated as a diagnostic that names the file the
  declaration was written in, so the reader is told where the name lives rather than only that it is out of reach.
- Laws in `crates/musa-compiler/tests/suite/import_laws.rs`: a `private let` in an imported library is refused where an
  importer names it, and the refusal names the file; the same for a `private` case of a public `data` family — the type
  crosses the import and the constructors do not, which is sealing restated over the form 161 left standing; a `private`
  declaration is an ordinary name inside its own file; a piece's voice may name its own file's `private` declarations;
  and an aliased import hides the same names the bare one does.
- `stdlib/src/context.musa`: its three `private` members proved private — the file that 162 rewrote is the corpus this
  prompt is measured on.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- check stdlib/src/context.musa
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

`--run-ignored all` carries the 30 staff-budget failures prompt 166 owns; the count must not move.

Commit as `Make private mean private across files`.

## Stop

- No package-level privacy, and no `pub(package)`-like third visibility. Two words, one boundary.
- No persistence of a `ModuleId` past one document's elaboration, and none in any hash, snapshot, or export.
- No re-opening of `signature`, `structure`, or sealing by listing. The layer is gone; this is the marking that replaced
  it.
- No change to `01-surface.md` §1.3. If the file boundary turns out to be the wrong one, that is an amendment and not an
  implementation detail.
