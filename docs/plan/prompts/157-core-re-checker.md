---
id: 157
slug: core-re-checker
status: pending
depends_on: [156]
phase: 3
---

# Build the Re-Checker the Crate Already Cites

## Task

Prompt [134](134-bidirectional-elaboration.md) specified an independent re-checker for elaborated terms, called it *"the
single most valuable invariant in the whole crate"*, and listed it as a deliverable. It does not exist, and several
sites in `musa-calculus` speak of it as though it does — including an intra-doc link to a module that has never existed
in this repository's history. Build it.

**This prompt supersedes [`142i`](142i-core-re-checker.md)**, which specified the same tool against the core prompts
147–156 replaced. It runs *after* those prompts rather than before them, because a re-checker for a term language about
to be rewritten would be written twice, and because it is now guarding something much larger.

## Read

- [`142i-core-re-checker.md`](142i-core-re-checker.md) — the argument and the four citing sites.
- `crates/musa-calculus/src/lib.rs` — the invariant list this becomes an entry in.

## Design

**What it is.** A function that takes a `Term` and a context and re-derives its type using the kernel alone — no
elaborator, no metavariables, no unification, no `Raw`. If elaboration produced it, the kernel must accept it.

**Why it matters more now than at 142i.** Idris2 keeps terms well-scoped *by construction*: `Term vars` is indexed by
its scope, so a de Bruijn index that escapes its binder does not typecheck in Idris2 itself. **Rust cannot express
that**, and prompt 147's decision to use de Bruijn indices in terms and levels in values was taken knowing it. The
re-checker is the mitigation, and it was named as such when that decision was made. After 152 and 154 there is far more
that can go silently wrong: a solution that mentions a variable outside its metavariable's scope, a case tree branch
checked at the wrong instantiated motive, a level parameter instantiated inconsistently.

**Where it runs.** Behind a debug assertion on every elaborated declaration, and unconditionally in the conformance
suite. Not in release builds of the compiler: it doubles checking time and its job is to catch *our* bugs, not the
author's.

**It must be able to fail.** A negative control — a deliberately malformed term the re-checker rejects — is part of the
deliverable, for the same reason 148's boundary law has one.

## Target

- `crates/musa-calculus/src/kernel/recheck.rs`: the pass, `universe_of`, and the scope check.
- `crates/musa-calculus/src/elaboration/program.rs`: the debug-assertion call site.
- The four stale citations repaired to point at the module that now exists.
- `crates/musa-calculus/tests/suite/recheck_laws.rs`: every fixture in the suite re-checked, plus the negative control.
- `142i-core-re-checker.md`: `status: superseded` and a banner naming this prompt.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
! grep -rn 'recheck' crates/musa-calculus/src | grep -v 'kernel/recheck.rs'  | grep -v 'recheck::'
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Build the core re-checker`.

## Stop

- No new typing rules. The re-checker implements the same judgment the kernel already has; if it needs a rule the kernel
  lacks, the kernel is what is wrong.
- No release-build cost.
