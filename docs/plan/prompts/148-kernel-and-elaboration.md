---
id: 148
slug: kernel-and-elaboration
status: pending
depends_on: [147a]
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

**Two directories, and the assignment is by what a file may see.** A file is kernel when it works on `Term` and `Value`
and raises `CoreError`; it is elaboration when it reads a `Raw` or raises `Refusal`. That criterion decides every file
below, including the ones 142h filed the other way.

`kernel/` — `term`, `value`, `eval`, `quote`, `context`, `scope`, `sort`, `budget`, `origin`, `error`, `list`, `room`,
`visibility`, `index`, `meta`, the inductive-family machinery under `family/`, the `Base`/`Builtin`/`Datum` *data*, and
the `Def`/`Defined`/`Program` *data*.

`elaboration/` — `raw`, `elab/`, `declare`, `case`, `rec`, `refuse`, `show`, `namespace`, `storable`, `convert`,
`declare_program`, and the registry's admission checks.

**Where this list differs from 142h's, and why.** Each difference is read off the code rather than off the plan.

- `class` and `dictionary` are absent — 146 deleted them. `level` is `sort`, per 147's naming.
- **`index` is present.** 142h expected 151 to have deleted it; 151 runs *after* this prompt, not before. It raises
  `CoreError` and names nothing above it, so it is kernel, and 151 deletes it from there.
- **`meta` is kernel, not elaboration.** `Shape::Meta` and `Form::Meta` hold a `crate::meta::Meta`: a kernel that could
  not name one could not define a term. `MetaSource` is the elaborator's word about *why* a meta exists and rides along
  as data, the same way an `Origin` does.
- **`convert` is elaboration, not kernel.** `Conversion` is one walk that does three things at once — decides
  definitional equality, solves metavariables, and reports `Refusal::Mismatch` with a `PathStep` path. Splitting that
  walk is a rewrite and the **Stop** below forbids one, so the file moves whole, and the criterion sends it up: it names
  `Refusal` on forty lines. Nothing under `kernel/` names `crate::convert` today, so the move costs nothing. The kernel
  does not thereby lose the ability to decide equality: prompt 149's re-checker states its conversion as `quote ∘ eval`
  compared by `Term`'s own `PartialEq`, which needs no part of this file.
- **`rec` does not split.** 142h's kernel half of it — generating and applying an eliminator — is in `family/` and has
  been since 147. What is left of `rec.rs` is the raw-syntax rewrite that turns a recursive call into a reference to an
  induction hypothesis, which is elaboration whole.
- **`program` splits instead.** `Defined`, `Program`, `Def`, and `one` are data that `Shape::Named`'s resolution,
  `Head::Def`, and `Cx` all hold; `declare_program`, its dependency graph, and its ordering are elaboration.
- `list`, `room`, and `visibility` are named by both halves, so 142h's tie-break applies and they are kernel.
  `namespace` is read only by `elab/`, `case`, and `program`'s declaration half, so it is elaboration.

**Two files split rather than move.** `base.rs` splits: the base and builtin *data* is kernel, `Registry::new`'s
structural validation is elaboration. `program.rs` splits as described above.

**The boundary is a law suite, not a `mod` keyword.** Rust cannot express "a child module may not see its sibling" — a
descendant can always name `crate::`. So the rule is checked the way the tree-sitter grammar is: a test reads every file
under `kernel/` and fails if it names an item defined under `elaboration/`, with a negative control so that a green
result means the check can fail. Stating it as a test rather than a convention is the entire deliverable.

**The facade does not move.** `musa_calculus::Term`, `::check`, `::Refusal` and every other public path stay where they
are; `lib.rs` re-exports across both modules. That makes "`musa-compiler` is untouched" checkable rather than hoped.

## Target

- `crates/musa-calculus/src/kernel/` and `src/elaboration/`, with every file moved into one and `base.rs`/`program.rs`
  split as described.
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
