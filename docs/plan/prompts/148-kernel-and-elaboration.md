---
id: 148
slug: kernel-and-elaboration
status: pending
depends_on: [147]
phase: 3
---

# Draw the Line Between the Kernel and the Elaborator

## Task

`musa-calculus` holds two programs in one flat directory. One decides typing and definitional equality on finished
terms; the other turns what an author wrote into such a term. Every item is `pub(crate)`, so every file may reach every
other, and the direction that matters is maintained by memory. Give them a boundary *inside* the crate:
`musa_calculus::{kernel, elaboration}`, with the kernel unable to name anything the elaborator defines.

**This prompt supersedes [`142h`](142h-kernel-and-elaboration.md)**, which specified the same split against the term
language prompt 147 replaced. It runs *after* the collapse rather than before it: drawing the line across seventeen
shapes and then rewriting them is the same filing done twice, and the collapse is what makes the assignment obvious.

The crate's public paths do not move. `musa-compiler` must not change by one line.

## Read

- [`142h-kernel-and-elaboration.md`](142h-kernel-and-elaboration.md) — the argument, in full. This prompt is that one
  with a new file list.
- `crates/musa-calculus/src/lib.rs` — why the elaborator belongs in *this* crate at all (`Value` stays private), which
  this prompt does not touch.

## Design

**Two directories, and the assignment is by what a file may see.**

`kernel/` — `term`, `value`, `eval`, `quote`, `convert`, `context`, `scope`, `sort`, `budget`, `origin`, `error`, the
inductive-family machinery under `family/`, and the `Base`/`Builtin`/`Datum` *data*. Everything here works on `Term` and
`Value` and raises `CoreError`.

`elaboration/` — `raw`, `elab/`, `declare`, `program`, `case`, `refuse`, `show`, `meta`, `storable`, and the registry's
admission checks. Everything here reads a `Raw`, produces a `Term`, and raises `Refusal`.

**Three differences from 142h's list, each caused by an earlier prompt.** `class` and `dictionary` are absent — 146
deleted them. `index` is absent — 151 deletes it, and this prompt runs after. `level` is `sort`, per 147's naming.

**Two files split rather than move.** `rec.rs` splits: generating and applying an eliminator is kernel, elaborating a
written `rec` expression is elaboration. `base.rs` splits: the base and builtin *data* is kernel, `Registry::new`'s
structural validation is elaboration.

**The boundary is a law suite, not a `mod` keyword.** Rust cannot express "a child module may not see its sibling" — a
descendant can always name `crate::`. So the rule is checked the way the tree-sitter grammar is: a test reads every file
under `kernel/` and fails if it names an item defined under `elaboration/`, with a negative control so that a green
result means the check can fail. Stating it as a test rather than a convention is the entire deliverable.

**The facade does not move.** `musa_calculus::Term`, `::check`, `::Refusal` and every other public path stay where they
are; `lib.rs` re-exports across both modules. That makes "`musa-compiler` is untouched" checkable rather than hoped.

## Target

- `crates/musa-calculus/src/kernel/` and `src/elaboration/`, with every file moved into one and `rec.rs`/`base.rs` split
  as described.
- `crates/musa-calculus/src/lib.rs`: `mod kernel; mod elaboration;`, the unchanged `pub use` set, and the direction
  stated in the invariants list.
- `crates/musa-calculus/tests/suite/boundary_laws.rs`: the law and its negative control.
- `AGENTS.md`'s navigation row for `crates/musa-calculus`.
- `142h-kernel-and-elaboration.md`: `status: superseded` and a banner naming this prompt.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
git diff --stat crates/musa-compiler crates/musa-notation crates/musa-project   # must be empty
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

The `git diff` line is the prompt's real check: an internal move that changed a dependent changed the facade.

Commit as `Draw the line between the kernel and the elaborator`.

## Stop

- No behaviour change, no renamed public path, no new diagnostic.
- No further module splitting inside `kernel/` or `elaboration/`. One boundary, checked.
