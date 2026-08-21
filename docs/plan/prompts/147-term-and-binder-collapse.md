---
id: 147
slug: term-and-binder-collapse
status: pending
depends_on: [145]
phase: 3
---

# Collapse Seventeen Shapes to Seven Terms, Three Binders, and Three Case Nodes

## Task

`Shape` has seventeen variants. Seven of them are one variant wearing different hats, and one of them — `Base`, holding
an `Arc<BaseDeclaration>` with a kind and host rules — is a *context entry smuggled into the term language*. Collapse
the term language to `Var`, `Named`, `Meta`, `Bind`, `App`, `Lit`, `Universe`, move reduction behaviour into the
context, and fix the naming while the file is open. **Mechanical: no semantics change and no program's acceptance
moves.**

## Read

- `crates/musa-calculus/src/term.rs` — the seventeen, and the doc comments that already say which ones overlap.
- `crates/musa-calculus/src/{eval,quote,convert}.rs` — the three traversals that shrink.
- `/Users/jcreinhold/Code/Idris2/src/Core/TT/Term.idr` — twelve constructors; musa keeps seven, and the five it drops
  are dropped for reasons prompt 143 records.

## Design

**The target, exactly.**

```rust
enum Term {
    Var      { index: Index },
    Named    { name: Name, role: Role },
    Meta     { source: MetaSource, ty: Term, scope: Vec<Term> },
    Bind     { name: Name, binder: Binder, body: Term },
    App      { head: Term, arg: Term },
    Lit      (Constant),
    Universe (Sort),
}

enum Binder   { Lam { filling, ty }, Pi { filling, ty }, Let { ty, value } }
enum Filling  { Written, Inferred }
enum Role     { Function, Constructor, TypeConstructor }
enum Constant { Payload(..), Numeral { family, count } }

enum Definition { Undeclared, Compiled(CaseTree), Constructor{..},
                  TypeConstructor{..}, Base{..}, Builtin(..) }
enum CaseTree   { Split { on, alternatives }, Answer(Term), Impossible }
```

`CaseTree` is declared here and stays a stub with one `Answer` arm until prompt 154 fills it; declaring it now is what
lets `Definition` be written once.

**Where each of the seventeen goes.** `Lam`/`Pi`/`Let` → `Bind` + `Binder`. `Const`/`Def`/`Base`/`Builtin` → `Named` +
`Definition`. `Lit`/`Numeral` → `Lit(Constant)`. `RecordType`/`Record`/`Project` stay for now — 156 moves them, and
moving them here would need families, which do not exist yet. `Indexed` stays for now — 150 deletes it, separately,
because that deletion has an argument attached and this one must not. `Hole` → `Meta`. `Var`/`App`/`Universe` unchanged.

**Why `Bind` saves nothing and is still right.** Three constructors become one plus a three-way tag: no net saving in
variants. The saving is that "go under a binder" is written once instead of three times, across `eval`, `quote`, `zonk`,
and the shrink traversals — which is exactly where the crate's binder bugs would live.

**The naming, decided.** `Hole` → `Meta`: `Hole` is surface syntax and `EventsHole` already takes the word at the
surface; `MetaSource` already exists in the crate, so the codebase had agreed and the term language had not. `PiInfo` →
`Filling`, with `Written`/`Inferred`, because the question it answers is who writes the argument. `Index` keeps its
name; `DbIndex` is rejected — the `Db` reads as *database*, and the pair `Index`/`Level` already says which side of the
index/level duality each is on. `DbLevel` and `Depth` merge into one `Level`: they are a position and a count into the
same environment, and carrying both invites using one where the other is meant. `Sort` takes the universe word so
`Level` means one thing; 151 gives `Sort` its contents.

**`Constant` keeps its two arms and the reason is a rule, not taste.** Two rules depend on the split: §3 compares a
numeral *as a number*, and ι decrements on a numeral, which is what lets `Nat`'s eliminator fire without unfolding a
tower of `Succ`.

## Target

- `crates/musa-calculus/src/term.rs`: the seven, the three binders, `Filling`, `Role`, `Constant`, `Index`, `Level`,
  `Sort` (two-point, unchanged in behaviour until 151), and `Definition`/`CaseTree`.
- `crates/musa-calculus/src/{value,eval,quote,convert,elab/}`: the traversals, rewritten against the new shape.
- `crates/musa-calculus/src/base.rs`: `BaseDeclaration` becomes a `Definition::Base`, reached through the context.
- No change to `crates/musa-compiler` beyond what `raw.rs` lowering requires; the facade does not move.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
git diff --stat crates/musa-calculus     # must be net negative
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

**The oracle is exact acceptance.** This prompt changes no program's verdict. `--run-ignored all` must show the same
failure list, byte for byte, that prompt 165's Check records — same tests, same messages, same counts. A message that
changes wording is acceptable only where a type name in it changed; a *count* that changes means a semantics change
crept in and the prompt is not done.

Commit as `Collapse the term language to seven constructors`.

## Stop

- No `Indexed` deletion (150), no records-as-data (156), no case trees beyond the stub (154), no metavariable solving
  (152), no universe change (151).
- No new diagnostics. Wording repairs only where a renamed type appears in a message.
