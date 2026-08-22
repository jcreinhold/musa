---
id: 149
slug: the-trusted-kernel
status: pending
depends_on: [148]
phase: 3
---

# Make the Kernel Trusted and Elaboration Untrusted

## Task

Prompt 148 gave the crate two halves. This prompt says what the split is *for*: the kernel is the trusted computing
base, elaboration is not, and the thing that makes that true is a re-checker that re-derives the type of every
elaborated term using the kernel alone. Build the minimal re-checker now — while the term language is small and before
metavariables, case trees and families arrive — and make extending it a standing obligation of every prompt that
follows.

**This is the guard for prompts 151–157.** Prompt 158 was originally the whole re-checker and sat *after* all of them,
which left five prompts' worth of new machinery running unguarded. It now extends what this prompt establishes.

## Read

- `crates/musa-calculus/src/lib.rs` — the invariants list this becomes the head of.
- `crates/musa-calculus/src/kernel/` after 148's move — everything trusted is in there and nothing else is, and
  `elaboration/case.rs`'s `Term::level_of`, which is the one kernel rule 148 had to leave outside.
- Prompt [134](134-bidirectional-elaboration.md), which specified a re-checker, called it *"the single most valuable
  invariant in the whole crate"*, and did not build it. Four sites in the crate cite a `recheck` module that has never
  existed in this repository's history.
- `~/Code/kan/crates/kernel/TRUST.md` — a trusted-computing-base document for a dependently typed language, as prior art
  for the *shape* of the document rather than its contents. Musa has no grades, no effects and no runtime, so most of
  its list does not apply; what transfers is that the boundary is written down, that the untrusted half is named, and
  that the acceptance invariants are stated where someone will read them.

## Design

**The claim being made, in one sentence.** If elaboration has a bug, the kernel rejects the artifact; only a bug in the
kernel can make musa accept an ill-typed program. That sentence is worth nothing until something enforces it, and this
prompt is that something.

**`Checked`, a type rather than a hope.** The kernel does not accept a `Term`; it accepts a `Checked`, and
`Checked::try_from(Term)` fails on any term still holding an unsolved metavariable. That is a boundary the compiler
cannot forget to check, and it is the specific defence against prompt 153's silent failure mode — a solution that
escaped its metavariable's scope, or a constraint left in the queue at the end of elaboration, becomes a refusal at a
named place instead of a term that quietly means something else. Prompt 153 has no `Meta` to reject yet; the newtype
exists here anyway, so that 153 adds one line rather than a mechanism.

**Three acceptance invariants, stated in the document and checked in the suite.**

1. *No unsolved metavariables cross the boundary.* Enforced by `Checked::try_from`.
2. *The kernel does not search.* No backtracking, no solver, no heuristic ordering — checking is structural recursion
   and conversion is `quote ∘ eval`. This is what makes kernel acceptance a property of the language rather than of the
   implementation's search order, and it is the same argument `budget.rs` already makes about resource limits.
3. *Scope discipline holds.* Every de Bruijn index in a checked term names a binder that encloses it.

The third is why this prompt exists at all in the shape it does. **Idris2 gets it for free**: `Term vars` is indexed by
its scope, so an index that escapes its binder does not typecheck in Idris2 itself. Rust cannot express that, and prompt
147 chose de Bruijn indices knowing it. The re-checker is the mitigation that decision was taken against.

**A weaker compile-time version does exist in Rust, and the decision is not to take it.** `~/Code/kan`'s
`crates/kernel/core-world` brands a `WorldView` with an invariant lifetime `'id` and refuses to promote an `OpenTerm` to
a `ScopedTerm` except through it, so terms minted under different scopes cannot be mixed *at compile time* — the
generativity trick, applied to exactly the bug this invariant is about. It does not give Idris2's guarantee (index 3 is
still not statically known to be in range) but it does statically kill the corruption case, so "Rust cannot express
this" is too strong and should not be repeated. **Musa declines it for a stated reason rather than by omission**: the
brand is a lifetime parameter on `Term`, and musa's terms are `Arc`-shared and cross into `musa-compiler`, so the
parameter would thread through the facade 148 just spent a prompt keeping stable. kan pays that cost because its terms
are arena-allocated behind a `'tcx` already and its kernel is nine crates rather than one module. Musa takes the same
*shape* one level weaker: `Checked` is that promotion with the check at run time, and the re-checker is what makes a
run-time check sufficient. If scope corruption ever shows up in practice, the brand is the escalation, and this
paragraph is where that is written down.

**`universe_of` is `Term::level_of`, moved.** The method exists already and does exactly this job — the universe a
checked type inhabits, by a structural walk. Prompt 148 filed it under `elaboration/case.rs` for one reason, recorded in
its own doc comment: `Type 1` has no universe above it, and what it said about that was a `Refusal`, which is the
elaborator's word. That reason dissolves here. The kernel is where the rule belongs, `CoreError::Malformed` is the
kernel's word for a term nobody should have built, and `case.rs`'s citation of `recheck::universe_of` — written long
before either module existed — becomes true rather than aspirational. `Term::level_of` goes; `motive_level` asks the
kernel.

Nothing about the walk changes, which is what keeps this inside the **Stop**: the same shapes answer the same sorts, and
only the error type and the module differ.

**What the minimal re-checker covers, and what it cannot yet.** After 148 the term language is seven constructors, so
the pass is small: re-derive the type of `Var`, `Named`, `Bind`, `App`, `Lit` and `Universe` against a context, using
`convert` for every equality. It cannot yet check a metavariable solution (153), a case-tree branch against an
instantiated motive (155), an index chosen by a constructor (156), or a level parameter instantiated consistently (152)
— because none of those exist. **Each of those prompts owes the extension**, and their Check sections say so rather than
leaving it to be remembered.

**Where it runs.** Behind a debug assertion on every elaborated declaration, and unconditionally in the conformance
suite. Not in release builds: it doubles checking time and its job is to catch *our* bugs, not the author's.

**It must be able to fail.** A negative control — a deliberately malformed term with an index one too large — is part of
the deliverable, for the same reason 148's boundary law has one. A re-checker nobody has seen reject anything is a
function that returns `Ok`.

## Target

- `crates/musa-calculus/TRUST.md`: the trusted half, the untrusted half, the three acceptance invariants, and the
  sentence about what a bug on each side costs. Linked from `AGENTS.md`'s navigation row.
- `crates/musa-calculus/src/kernel/checked.rs`: `Checked` and its fallible conversion.
- `crates/musa-calculus/src/kernel/recheck.rs`: the pass, and `universe_of` — which is `Term::level_of` moved into the
  kernel.
- `crates/musa-calculus/src/elaboration/declare_program.rs`: the debug-assertion call site, and the `Checked` boundary.
- The four stale `recheck` citations repaired to point at the module that now exists.
- `crates/musa-calculus/tests/suite/recheck_laws.rs`: every fixture in the suite re-checked, the scope-discipline law,
  and the negative control.
- `docs/plan/prompts/`: the Check sections of 151, 152, 153, 154, 155, 156 and 157 gain the extension obligation.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! grep -rn 'recheck' crates/musa-calculus/src | grep -v 'kernel/recheck.rs' | grep -v 'recheck::'
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

The negative control is the check that matters and it is inside the suite rather than on this list: a run where the
malformed fixture is *accepted* is a green suite over a re-checker that checks nothing.

Commit as `Make the kernel trusted and elaboration untrusted`.

## Stop

- No new typing rules. The re-checker implements the judgment the kernel already has; if it needs a rule the kernel
  lacks, the kernel is what is wrong and that is a different prompt.
- No release-build cost.
- No crate split. kan enforces its kernel boundary with cargo because its kernel is nine crates; musa's is one module,
  because `Value` must stay private and prompt 148 argued that. The newtype is what transfers, not the layout.
