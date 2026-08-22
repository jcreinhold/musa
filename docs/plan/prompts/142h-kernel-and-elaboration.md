---
id: 142h
slug: kernel-and-elaboration
status: superseded
depends_on: [142g]
phase: 3
---

# Draw the Line Between the Kernel and the Elaborator

> **Superseded by [prompt 148](148-kernel-and-elaboration.md), *Draw the Line Between the Kernel and the Elaborator*.**
> Prompt 148 carries this prompt's whole Task, moved after prompt 147's collapse of the term language. Drawing the
> boundary across seventeen shapes and then rewriting them is the same filing done twice, and the collapse is what makes
> the file assignment obvious.
>
> This file is not executed. It stays because the ledger and the prompts above it link to it, and because prompt 148's
> Read section cites it for the argument rather than repeating it.

## Task

`musa-calculus` is 19,389 lines in thirty flat files, and two different programs live in them. One decides typing and
definitional equality on finished terms; the other turns what an author wrote into such a term. `convert.rs` sits beside
`declare.rs`, `eval.rs` beside `refuse.rs`, and nothing in the crate says which is which — every item is `pub(crate)`,
so every file may reach every other, and the direction that matters is maintained by memory.

[`lib.rs`](../../../crates/musa-calculus/src/lib.rs) already argues why the elaborator belongs in _this crate_: `Value`
stays private, and an elaborator that must check against value types would otherwise force it into the facade. That
argument is right and this prompt does not touch it. What it does not settle is that the two halves need a boundary
_inside_ the crate. Give them one: `musa_calculus::{kernel, elaboration}`, with the kernel unable to name anything the
elaborator defines.

The crate's public paths do not move. `musa-compiler` must not change by one line.

## Read

- [`crates/musa-calculus/src/lib.rs`](../../../crates/musa-calculus/src/lib.rs) — the facade, the invariants, and the
  `Value`-privacy argument this prompt preserves. Its **"What this crate does not do"** section is already the boundary
  in prose: `normalize` and `convertible` do not type-check what they are given, and typing is `check`/`infer`.
- 142g's commit, which put the kernel's errors below the elaborator's. Without it every kernel file still names
  `Refusal` and no boundary can hold.
- [`crates/musa-calculus/src/rec.rs`](../../../crates/musa-calculus/src/elaboration/rec.rs) — 633 lines that take
  `&mut Elaborator` and `&Raw`. Read it for the split described below: the recursor is one thing, elaborating a `rec`
  expression is another.
- [`crates/musa-calculus/src/base.rs`](../../../crates/musa-calculus/src/kernel/base.rs) — same shape at 1,327 lines:
  `Base`, `Builtin`, `Datum`, and `Literal` are data a term can hold; `Registry::new`'s admission checks are a
  declaration-time judgment.
- `editors/tree-sitter-musa`'s drift law and `crates/musa-syntax/tests/suite/tree_sitter_fixtures.rs` — the precedent
  for enforcing a structural rule Rust's type system cannot state, with a law suite that reads the source.
- _Philosophy of Software Design_ ch. 7 on layers that add an abstraction, and ch. 8 on where complexity should be paid.
  The kernel is the part that has to be provably right; every line that does not have to be is a line paying no rent
  there.

## Design

**Two directories, and the assignment is by what a file may see.**

`kernel/` — `term`, `value`, `eval`, `quote`, `convert`, `context`, `scope`, `level`, `budget`, `origin`, `index`,
`error`, the inductive-family machinery under `family/`, the generated recursors, and the `Base`/`Builtin`/`Datum`
_data_. Everything here works on `Term` and `Value` and raises `CoreError`.

`elaboration/` — `raw`, `elab/`, `declare`, `program`, `class`, `dictionary`, `case`, `refuse`, `show`, `meta`,
`storable`, and the registry's admission checks. Everything here reads a `Raw`, produces a `Term`, and raises `Refusal`.

**Two files split rather than move.** `rec.rs` splits: generating and applying a recursor is kernel, elaborating a
written `rec` expression is elaboration. `base.rs` splits: the base and builtin _data_ is kernel, `Registry::new`'s
structural validation is elaboration. Both splits are the same distinction the whole prompt is about, met twice.

**The boundary is a law suite, not a `mod` keyword.** Rust cannot express "a child module may not see its sibling" — a
descendant can always name `crate::`. So the rule is checked the way the tree-sitter grammar is: a test reads every file
under `kernel/` and fails if it names an item defined under `elaboration/`. Stating it as a test rather than a
convention is the entire deliverable; a directory rename with no check is filing, not architecture.

**The facade does not move.** `musa_calculus::Term`, `musa_calculus::check`, `musa_calculus::Refusal` and every other
public path stay exactly where they are — `lib.rs` re-exports across both modules. This keeps the prompt a pure internal
move, and makes "`musa-compiler` is untouched" a checkable property rather than a hope.

**The direction, stated once, in `lib.rs`.** `elaboration` may name everything in `kernel`. `kernel` may name nothing in
`elaboration`. That sentence is the crate's invariant list's new entry, and the law suite is its evidence.

## Target

- `crates/musa-calculus/src/kernel/` and `crates/musa-calculus/src/elaboration/`, with every existing file moved into
  one of them and `rec.rs`/`base.rs` split as described.
- `crates/musa-calculus/src/lib.rs`: `mod kernel; mod elaboration;`, the unchanged set of `pub use` paths, and the
  direction stated in the invariants section.
- `crates/musa-calculus/tests/suite/boundary_laws.rs`: the law that no file under `kernel/` names an item defined under
  `elaboration/`, and its negative control — a fixture the check rejects, so a green result means the check can fail.
- `AGENTS.md`'s navigation row for `crates/musa-calculus`, naming the two modules.

## Check

```sh
cargo nextest run --workspace
cargo nextest run -p musa-calculus
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

And the two checks that say this was a move and not a rewrite:

- `git diff --stat HEAD~1 -- crates/musa-compiler crates/musa-score crates/musa-project` is **empty**. The facade did
  not move, so no consumer did.
- Every `insta` snapshot, `tests/fixtures/elaboration-compatibility.txt`, the `musa-events` pinned digests, and
  `apps/musa-desktop/ui/fixtures/` are byte-identical.

## Stop

- **No new crate.** `lib.rs`'s `Value`-privacy argument settles this: the elaborator stays here.
- **No renamed public item, and no changed signature.** A consumer that compiled before compiles unchanged.
- **No behaviour change of any kind** — not a verdict, not a diagnostic, not a normal form.
- **No re-checker.** 142i, which this boundary exists to make meaningful.
- No new module beyond the two, and no third "shared" module. A file that seems to belong to both belongs to the kernel,
  and if it cannot, that is a finding to report rather than a `common/` to create.
- No change to the budget, the meter, or exhaustion accounting.
- Do not fold `musa-events` in, and do not move anything between crates. The workspace graph is 153a–153c.

Commit as `Draw the line between the kernel and the elaborator`.
