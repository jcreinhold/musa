---
id: 135
slug: inductive-families
status: pending
depends_on: [134]
phase: 3
---

# Add Inductive Families, Dependent Match, and the Termination Checker

## Task

Give `musa-core` parameterized and indexed `data` declarations with strict positivity, generated dependent recursors,
dependent `match` compiled through case trees with coverage checking, and the checked well-founded termination rule that
prompt 128 kept totality for. After this prompt the core is complete: everything above it is library code.

Also **level metavariables**, which prompt 134 deferred to here: `data Vec (A : Type l)` is the first declaration that
cannot write its own levels, so this is the prompt that has to solve them.

## Read

- `docs/rules/language/02-core-calculus.md` §1 (families, parameters versus indices), §5's positivity, coverage, and
  termination obligations, and §6.2 as prompt 129 replaced it — flat patterns are gone and the replacement is argued
  there, so this prompt implements case-tree compilation rather than re-deciding it.
- Prompt [129](129-dependent-core-spec.md)'s K paragraph and prompt 132's finding on it. **If the trial found K
  unnecessary, it is not implemented here**, and this prompt's first job is to check which answer it recorded.
- `crates/musa-compiler/src/core.rs`'s existing finite-data machinery and
  `crates/musa-compiler/tests/suite/finite_data_laws.rs` — the current positivity rule and the laws that hold today. The
  new check is strictly stronger; the old laws should still pass when restated over the new declarations, and any that
  cannot is a finding worth recording.
- `crates/musa-compiler/tests/suite/literal_pattern_laws.rs` and prompt [127dca](127dca-text-patterns-match.md) —
  literal patterns already exist and have laws. Case-tree compilation has to keep them, and a literal is the one pattern
  that does not decompose into constructors.
- Peyton Jones ch. 4 and ch. 5 in full — structured types and the semantics of pattern matching, including the
  match-compilation algorithm and the treatment of overlapping and missing cases. This is the chapter this prompt is
  written from; cite it in the implementation's doc comments where the algorithm follows it and say where it does not,
  because a dependent case tree refines the *type* as it goes and ch. 5's does not.
- `docs/notes/research/language-design-closure/39-totality-and-structural-abstraction.md` §12.3 — the rule that a
  termination relaxation needs a recorded failing program. The measure form implemented here is the one 129 specified;
  anything wider is a separate amendment.

## Design

**Parameters and indices are different, and the code should not blur them.** A parameter is fixed across the whole
declaration and appears uniformly in every constructor's result; an index varies per constructor. The distinction
decides what the generated recursor's motive quantifies over, so getting it wrong produces an eliminator that
type-checks and proves nothing useful. State it once, in the declaration's doc comment, with `Vec A n` as the example —
`A` a parameter, `n` an index — since that is the type prompt 141 needs.

**Strict positivity is checked on the declaration group**, including through function-argument positions and through
mutual recursion, before any constructor type is admitted. The failure message names the offending occurrence and the
constructor it sits in, because "not strictly positive" without a location is the least actionable error a type checker
can produce. Consistency depends on this check, so it is conservative by construction: a declaration it cannot see
through is refused, not admitted.

**The recursor is generated; `match` is compiled to it.** The core's one elimination form stays the dependent recursor.
Surface `match` elaborates to a case tree — split on a scrutinee, refine the goal type by the constructor's indices,
recurse — and the case tree emits recursor applications. Coverage is decided while building the tree, which is what
makes "coverage" and "compilation" the same pass rather than two that can disagree. Unreachable branches are reported,
not silently dropped: a branch the tree can prove unreachable is usually an author's mistaken mental model, and telling
them is worth more than the branch.

**Index unification is where the sharp edges are.** Splitting refines indices, which needs constructor injectivity and —
if 129's K survived the trial — uniqueness of identity proofs. When a split forces two indices to be equal and they
cannot be, the branch is *impossible* and is discharged rather than requiring a body. When the elaborator cannot decide
either way, it says so with the constraint it was stuck on rather than guessing.

**Levels stop being numbers here.** Prompt 134 left `Level` a computed natural because nothing could write a `Type`
without saying which one; a family parameterized by `(A : Type l)` can, so §2.1's third creation site opens now. The
change is not the solver — level constraints are first-order and a bare `?ℓ ≡ l` is the whole of the common case — it is
that `succ` and `max` stop computing on an unsolved arm, so every place that reads a level has to force it first. Do
this before the recursor generator, not after: a generated motive is exactly a term whose level nobody wrote.

**Termination is a measure the checker sees.** Every recursive definition presents a measure into a well-founded order.
The structural case — the measure is subterm size, supplied by the elaborator — must stay the ergonomic default, or
every ordinary fold in `stdlib/` acquires an annotation and the language gets worse for the 95% case to serve the 5%.
Mutual recursion uses a lexicographic combination. There is no `partial`, and a definition whose measure cannot be
checked is refused with the recursive call that broke it.

**Laws.** Positivity refuses the classic negative occurrences. A generated recursor's β-rule holds (ι-reduction on each
constructor). A compiled `match` is convertible to its recursor form. Coverage is complete: a match the checker accepts
never gets stuck. Every accepted recursive definition terminates on closed arguments, tested by evaluation to a normal
form under a budget that is generous but finite. Impossible branches are discharged without a body. The compile-fail
suite carries one case per refusal.

## Target

- Parameterized and indexed `data` in `musa-core`, with strict positivity, generated dependent recursors, case-tree
  compilation with coverage and unreachability reporting, index unification, and the well-founded termination checker.
- Level metavariables and their solver, with `Level` forced wherever it is read.
- New `Code` variants with `musa explain` text for: non-positive occurrence, incomplete match, unreachable branch,
  undecidable index constraint, and unchecked recursion.
- `crates/musa-core/tests/suite/{family_laws.rs, coverage_laws.rs, termination_laws.rs}` and the compile-fail cases.
- `docs/plan/code-map/`: `musa-core`'s row updated to "core complete".
- No change to `musa-compiler`.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-core
cargo nextest run --workspace
cargo nextest run --run-ignored all -p musa-core
cargo clippy --all-targets -p musa-core -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Add inductive families, dependent match, and termination checking`.

## Stop

- No records, enums, traits, operators, or method syntax. Prompts 136 and 137.
- No `Syntax<Cat>`, no quotation, no collection library. Prompts 138–141.
- No `partial`, no general recursion, no `fix`, no termination-checker escape hatch, no measure the checker cannot
  verify, and no "assume it terminates" flag — not even behind a feature.
- No coinduction, no sized types, no cumulativity.
- No proof-search, no tactic language, no automation for discharging impossible branches beyond the index unification
  specified here.
- No change to `musa-compiler` or `stdlib/`. The cutover is prompt 142.
