---
id: 142f
slug: writable-index
status: in-progress
depends_on: [142db]
phase: 3
---

# Make an Indexed Type Writable in Source

## Task

Prompt 142d built the index stratum inside `musa-calculus` and lowered surface `T(i)` to an indexed raw type. Nothing
records which types take an index, or at what sort, so every spelling of the form is accepted and none of them means
anything:

```sh
$ cat u.musa
piece "U" {
    data Pc { Zero, Next(fewer: Pc), }
    fn f(x: Pc(12)) -> Nat { 0 }
    fn g(x: Pc(3/4)) -> Nat { 0 }
    fn h(x: Nat(1/2)) -> Nat { 0 }
}
$ musa check u.musa
u.musa: ok
```

`Pc` declares no index and takes one; it takes a whole number in one signature and an exact fraction in the next; and
`Nat`, which is the sort an index is _drawn from_, carries one itself. The index is elaborated by `infer`, so it is
whatever it happens to be rather than what the head asked for, and there is no head that asked for anything.

Give a declared type an index telescope with sorts, and check each index argument at the sort its head declares.

**Half of the defect this prompt was written from is already fixed.** As written, the reproduction was
`fn f(x: Pc12(12))` refused with `unsolved-metavariable`, and the diagnosis was that a bare numeral has no type to
infer. Both were wrong about the cause: `RawShape::Numeral` carries its counting family, so an index infers perfectly
well. The refusal came from `musa-compiler`'s copy of the grammar's type-node list, which had gone stale when
`IndexedType` joined it — so a parameter written at an indexed type was lowered as a λ with no domain, and the author
was told their parameter needed a type. Commit `87193e13` replaced the copy with `musa_syntax::ast::is_type` and the
message is gone. What survives is the half above, and it is the half that needed a declaration: checking rather than
inferring matters because a sort the head declares is the only thing a `Pc(3/4)` can be measured against.

## Read

- `docs/rules/language/02-core-calculus.md` §1.5 in full, and §2.2's literal judgments. §1.5 names three sorts — `Nat`,
  exact `Ratio`, and finite literal enums — and never says where a use site learns which one it is in; that is the hole
  this prompt fills, and if §1.5 needs a sentence to say the head declares it, repair the section first and commit the
  repair.
- §2.1's first-order matching. `fn row(pcs: List<Pc(n)>) -> Result<Row(n), RowFault>` binds `n` by appearing in a
  signature, so a use site's index may be a _variable_ as well as a literal, and the sort check has to accept both.
- `crates/musa-calculus/src/elab/infer.rs`'s `indexed_type_formation` as prompt 142da left it — the one site that builds
  an indexed term, and the site whose `self.infer(scope, index)` asks the expression a question only the head's
  declaration can answer.
- `crates/musa-calculus/src/declare.rs`, `telescope` — the existing parameter telescope, whose doc comment already says
  "parameter or index" for a mechanism that only has parameters.
- `crates/musa-calculus/src/base.rs`'s `Base::measuring`, and prompt [142d](142d-index-stratum.md)'s recorded finding
  that `Duration<C>`, `Position<C>`, and `Syntax<Cat>` carry **parameters**, not indices. That finding stands; this
  prompt does not re-open it.
- `crates/musa-compiler/src/lower/types.rs`'s `indexed_type`, which lowers the surface form correctly and says in its
  doc comment why it checks nothing.
- Commit `87193e13` and the law it added to `crates/musa-compiler/src/lower/laws.rs`,
  `an_index_written_in_a_type_reaches_the_core_as_the_type_it_forms`. It asserts that `Nat(12)` and `Ratio(3/4)` are
  accepted, which this prompt makes false: both heads are sorts and neither declares an index. The law's subject — the
  whole path from parser to formed type, and a `Ratio` literal that reaches §1.5's reader at the _registered_ base —
  survives at a head that does declare one, and rewriting it that way is part of the target.
- `crates/musa-calculus/src/convert.rs`'s `sort_of`, which already decides §1.5's question about a type value: a
  counting family is `Sort::Count` and a base registering `Measures` is `Sort::Rational`. The declaration check is that
  predicate asked one stage earlier, not a second list of admissible sorts.
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

**The sorts are §1.5's three and no others, and the code already has the predicate.** `Nat`, exact `Ratio`, and a finite
literal enum — which reach `crate::index::Sort` as two variants rather than three, because a finite enum's values are
base literals and a base says how to read one exactly as `Ratio` does. A telescope binder at any other type is refused
at the declaration, naming the binder. Admitting a fourth sort is an amendment to §1.5, with the program that needs it.

**One index, because that is what a type holds.** `Shape::Indexed` carries a single index term and the use-site grammar
reads a single expression, so a telescope here is of length zero or one and "the wrong number of arguments" is the two
refusals that leaves: a head with no telescope written with an index, and a head with one written without. A longer
telescope is a change to the term representation and to erasure, which 142d fixed and this prompt does not re-open.

**Diagnostics name the index and the type, never the solver.** "`Nat` takes no index" and "a `Row(12)` where a `Row(24)`
was expected", not "linear constraint unsatisfiable" and not "unsolved metavariable". The message this prompt exists to
delete is the one in the Task.

**Erasure is unchanged, and byte-identity is how that is checked.** `quote` already drops indices (142d). Making the
form writable adds programs that are _accepted_; it adds nothing to any stored artifact. Every snapshot, the
`elaboration-compatibility` fixture, the `musa-events` pinned digests, and `apps/musa-desktop/ui/fixtures/` move through
this prompt unchanged.

## Target

- `crates/musa-calculus/src/declare.rs`, `raw.rs`, `term.rs`: an index telescope on a declaration, with its sorts, and
  the arity a use site is checked against.
- `crates/musa-calculus/src/base.rs`: the same telescope on a registered base, because a base is the other kind of
  declaration a type head can be, and the calculus's own indexed fixture (`Row`) is one. A base declares its index
  against the registry's sorts, which is where an index at an unmeasured base type is refused.
- `crates/musa-calculus/src/elab/infer.rs`: `indexed_type_formation` checks each index argument at its declared sort
  instead of inferring it, and checks the arity.
- `crates/musa-calculus/src/refuse.rs`: the refusals — a type that takes no index, the wrong number of indices, and a
  telescope binder outside §1.5's three sorts.
- `crates/musa-syntax`: the declaration's index telescope in the grammar and the CST, if the parser does not read it
  already, with the formatter and the tree-sitter grammar held to the drift law.
- `crates/musa-compiler/src/lower/`: the declaration lowering, and the surface diagnostics for the three refusals.
- Law suites: a `.musa` fixture that writes an indexed type and checks; an indexed type at one index refused where
  another was asked for, _from source_ rather than only from the calculus registry; `Nat(12)` refused; an index written
  at the wrong sort refused; an index variable bound by a signature and solved at the call; erasure, by byte-identity.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

And the check the Task is written from, which must now refuse rather than accept: of the Task's four lines, the one
whose head declares its index at the sort written is accepted by `musa check`, and the other three are refused —
`Pc(3/4)` naming the sort `Pc` declared, `Nat(1/2)` naming `Nat`, and a bare `Pc` naming the index it is missing.

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
