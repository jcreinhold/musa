---
id: 142f
slug: writable-index
status: pending
depends_on: [142db]
phase: 3
---

# Make an Indexed Type Writable in Source

## Task

Prompt 142d built the index stratum inside `musa-calculus` and lowered surface `T(i)` to an indexed raw type. No `.musa`
source can write one. Two things are missing, and they are one feature:

```sh
$ cat t.musa
piece "T" { fn f(x: Pc12(12)) -> Nat { 0 } }
$ musa check t.musa
unsolved-metavariable
  × this cannot be given a type on its own; write the type it should have
   ╭─[t.musa:1:24]
 1 │ piece "T" { fn f(x: Pc12(12)) -> Nat { 0 } }
   ·                     ─────┬────
```

The index expression is elaborated by `infer`, and a bare numeral has no type to infer — §2.2's literal judgments run
against a type the checker supplies. So *every* well-formed indexed type spelled in source is refused, including the
four `02-core-calculus.md` §1.5 uses as its own examples. And because nothing records which types take an index, the
form is also unrestricted in the other direction: `Nat(12)` elaborates exactly as far as `Pc12(12)` does, and would be
accepted if the first defect were fixed on its own.

Give a declared type an index telescope with sorts, and check each index argument at the sort its head declares.

## Read

- `docs/rules/language/02-core-calculus.md` §1.5 in full, and §2.2's literal judgments. §1.5 names three sorts — `Nat`,
  exact `Ratio`, and finite literal enums — and never says where a use site learns which one it is in; that is the hole
  this prompt fills, and if §1.5 needs a sentence to say the head declares it, repair the section first and commit the
  repair.
- §2.1's first-order matching. `fn row(pcs: List<Pc(n)>) -> Result<Row(n), RowFault>` binds `n` by appearing in a
  signature, so a use site's index may be a *variable* as well as a literal, and the sort check has to accept both.
- `crates/musa-calculus/src/elab/infer.rs`'s `indexed_type_formation` as prompt 142da left it — the one site that builds
  an indexed term, and the site whose `self.infer(scope, index)` is the defect.
- `crates/musa-calculus/src/declare.rs`, `telescope` — the existing parameter telescope, whose doc comment already says
  "parameter or index" for a mechanism that only has parameters.
- `crates/musa-calculus/src/base.rs`'s `Base::measuring`, and prompt [142d](142d-index-stratum.md)'s recorded finding
  that `Duration<C>`, `Position<C>`, and `Syntax<Cat>` carry **parameters**, not indices. That finding stands; this
  prompt does not re-open it.
- `crates/musa-compiler/src/lower/types.rs`'s `indexed_type`, which lowers the surface form correctly and says in its
  doc comment why it checks nothing.
- `docs/plan/prompts/143-builtin-collapse.md`, which is the first consumer: `Pc(n)`, `PcSet(n)`, and `Row(n)` cannot be
  declared until this lands.

## Design

**An index is checked, never inferred, and the sort comes from the head.** This is the bidirectional rule the rest of
the elaborator already follows and the one place that broke it: a literal has no type of its own, so `12` in `Pc(12)` is
checked at `Nat` because `Pc`'s declaration says its index is a `Nat`. Inferring it instead asks the expression a
question only the declaration can answer, which is why the current code cannot answer it.

**Index arity and sorts are declared, in the spelling §1.5 already uses at the use site.** Parentheses for indices,
angle brackets for parameters:

```musa
data Pc(n: Nat) { … }
data Bar(m: Ratio) { … }
```

A type with no index telescope applied to an index argument is refused naming the type; so is the wrong number of
arguments. That is what makes `Nat(12)` an error rather than a second spelling of `Nat`, and it is the check that gives
the erasure argument its teeth — a form nothing declares is a form nothing can be erased from.

**The sorts are §1.5's three and no others.** `Nat`, exact `Ratio`, and a finite literal enum. A telescope binder of any
other type is refused at the declaration, naming the binder and listing the three. Admitting a fourth sort is an
amendment to §1.5, with the program that needs it.

**Diagnostics name the index and the type, never the solver.** "`Nat` takes no index" and "a `Row(12)` where a `Row(24)`
was expected", not "linear constraint unsatisfiable" and not "unsolved metavariable". The message this prompt exists to
delete is the one in the Task.

**Erasure is unchanged, and byte-identity is how that is checked.** `quote` already drops indices (142d). Making the
form writable adds programs that are *accepted*; it adds nothing to any stored artifact. Every snapshot, the
`elaboration-compatibility` fixture, the `musa-events` pinned digests, and `apps/musa-desktop/ui/fixtures/` move through
this prompt unchanged.

## Target

- `crates/musa-calculus/src/declare.rs`, `raw.rs`, `term.rs`: an index telescope on a declaration, with its sorts, and
  the arity a use site is checked against.
- `crates/musa-calculus/src/elab/infer.rs`: `indexed_type_formation` checks each index argument at its declared sort
  instead of inferring it, and checks the arity.
- `crates/musa-calculus/src/refuse.rs`: the refusals — a type that takes no index, the wrong number of indices, and a
  telescope binder outside §1.5's three sorts.
- `crates/musa-syntax`: the declaration's index telescope in the grammar and the CST, if the parser does not read it
  already, with the formatter and the tree-sitter grammar held to the drift law.
- `crates/musa-compiler/src/lower/`: the declaration lowering, and the surface diagnostics for the three refusals.
- Law suites: a `.musa` fixture that writes an indexed type and checks; `Row(12)` against `Row(24)` refused *from
  source* rather than only from the calculus registry; `Nat(12)` refused; an index variable bound by a signature and
  solved at the call; erasure, by byte-identity.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

And the check the Task is written from, which must now pass rather than fail: a `.musa` file declaring an indexed type
and writing it in a signature is accepted by `musa check`, and `Nat(12)` in the same file is refused with a message
naming `Nat`.

Byte-identity is load-bearing: every `insta` snapshot, `tests/fixtures/elaboration-compatibility.txt`, the `musa-events`
pinned digests, and `apps/musa-desktop/ui/fixtures/` are unchanged.

## Stop

- No index-level functions, no existentials, no proof obligations discharged by search. 142d's Stop, unchanged.
- No inductive-family indices. `family/` is not touched.
- No fourth sort.
- No change to `Duration<C>`, `Position<C>`, `Syntax<Cat>`, or `EventTrack<time>`. They carry parameters, and 142d
  recorded why.
- No `Pc(n)`, `PcSet(n)`, or `Row(n)`, and no collapse of the seventeen modulus-12 builtins. That is 143, and this
  prompt is what unblocks it.
- No general Presburger procedure. The fragment 142d implemented is what the corpus generates.

Commit as `Make an indexed type writable in source`.
