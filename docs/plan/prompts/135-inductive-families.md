---
id: 135
slug: inductive-families
status: done
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
- Prompt [129](129-dependent-core-spec.md)'s K paragraph and prompt 132's finding on it, as
  `docs/rules/language/02-core-calculus.md` §1.4 now records it. **The trial found K unnecessary, and found that no
  program unifies an index at all**; §1.4 binds this prompt to two consequences and the Design below discharges both.
- `crates/musa-compiler/src/core.rs`'s existing finite-data machinery and
  `crates/musa-compiler/tests/suite/finite_data_laws.rs` — the current positivity rule and the laws that hold today. The
  new check is strictly stronger; the old laws should still pass when restated over the new declarations, and any that
  cannot is a finding worth recording.
- `crates/musa-compiler/tests/suite/literal_pattern_laws.rs` and prompt [127dca](127dca-text-patterns-match.md) —
  literal patterns already exist and have laws. Case-tree compilation has to keep them, and a literal is the one pattern
  that does not decompose into constructors.
- Peyton Jones ch. 3 and ch. 6 — the enriched calculus and its transformations, which is the shape the termination rule
  takes here: a recursive call is *rewritten* into an induction hypothesis before elaboration rather than checked after
  it, so the check and the compilation are one pass and there is no fixed point left in the core to take on trust.
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

**Index unification is the solution rule, and nothing wider.** Splitting refines indices, and the rule that does the
refining is the *solution* rule: a scrutinee whose index arguments are distinct variables of the local context is
generalized into the recursor's motive, and each constructor's chosen index then refines those variables in that
constructor's method automatically. `Vec A n` at a variable `n` is exactly this case, and it needs no equality proofs,
no injectivity lemma, and no deletion rule — the motive *is* the refinement.

A scrutinee whose index argument is **not** a variable — `Vec A (succ n)` — is refused, with the index it was stuck on
named. This is a deliberate narrowing and it is argued rather than assumed:

- `02-core-calculus.md` §1.4 records prompt 132's finding that **no program unifies an index at all**, and binds this
  prompt to two consequences, of which the first is that "the coverage checker may not use a unification rule that
  requires K until a program requires one, which costs nothing today and is checkable at the rule rather than at its
  uses". Discharging an impossible branch needs conflict and injectivity; making the *remaining* branch usable needs
  deletion, which is the rule that requires K. Refusing at the scrutinee is that check, at the rule.
- The machinery a forced index needs — a discriminator motive built by large elimination over the index family, one per
  forced position — is several hundred lines that no program in `stdlib/` or `examples/` would execute. Building it now
  would be root `AGENTS.md`'s deep-module rule read backwards — generalizing unused functionality rather than the
  interface, which Ousterhout ch. 8 names as the expensive way to be wrong.

**Re-opening is an ordinary repair of this prompt**, with the evidence stated in advance, exactly as §1.4 states it for
K: a program whose scrutinee's index is a constructor application, and a `match` on it that the solution rule cannot
type. Prompt 141's `Vec A n` is the first candidate and is probably not one, since a length-indexed vector is
scrutinized at a variable length and built at a forced one.

Under this rule no branch is impossible — a constructor's chosen index always meets a variable — so **"impossible
branches are discharged without a body" is vacuously true and is not a law this prompt can state.** The unreachability
report is about arms an earlier arm already covers, which is a property of the pattern matrix and does not depend on
indices at all.

**Levels stop being numbers here.** Prompt 134 left `Level` a computed natural because nothing could write a `Type`
without saying which one; a family parameterized by `(A : Type l)` can, so §2.1's third creation site opens now. The
change is not the solver — level constraints are first-order and a bare `?ℓ ≡ l` is the whole of the common case — it is
that `succ` and `max` stop computing on an unsolved arm, so every place that reads a level has to force it first. Do
this before the recursor generator, not after: a generated motive is exactly a term whose level nobody wrote.

**Termination is a measure the checker sees.** Every recursive definition presents a measure into a well-founded order.
The structural case — the measure is subterm size, supplied by the elaborator — must stay the ergonomic default, or
every ordinary fold in `stdlib/` acquires an annotation and the language gets worse for the 95% case to serve the 5%.
There is no `partial`, and a definition whose measure cannot be checked is refused with the recursive call that broke
it.

**The structural measure reads the recursive position off the definition's own `match`, and constrains nothing else.** A
definition recurses on one argument; the `match` at the top of its body says which, by having that argument as a
subject. A call is then a call on a *pattern binder in that column*, and it compiles to the induction hypothesis that
branch was handed. The remaining arguments are unconstrained, and that is not laxity: the hypothesis is the answer for
this branch's field at the indices that field has, so conversion decides the rest and a second check here would decide
it twice.

This replaces an earlier reading of the same rule — "exactly one argument differs from the binder in that position" —
which implementation showed to be wrong rather than merely narrow. `count : (n : Nat) → Vec A n → Nat` recursing on the
tail *must* pass a different index too, because `ys : Vec A k` and nothing else type-checks; under the one-argument rule
no recursion over an indexed family is expressible at all, which would have made prompt 141's `Vec A n` unusable for the
thing it exists for. The finding is recorded here rather than in the code alone because it is the kind of rule that
reads plausible until a dependent type meets it.

**Mutual recursion between definitions is deferred, with the condition for re-opening stated.** `rec` binds one name, so
two definitions calling each other cannot be written; mutual recursion between *families* is what the generated mutual
recursor already provides, and that is the case the musical library needs — a syntax tree and its list of children, an
even/odd pair. The lexicographic combination this paragraph once promised is what mutual definitions would need, and
§2.4 admits one. Re-opening is an ordinary repair of a later prompt, and the evidence is a program in `stdlib/` or
`examples/` that needs two definitions in one recursive knot and cannot be written as one definition over a mutual
family. A top-level `match` on two of the definition's own arguments is refused for the same reason and names the call,
rather than being silently accepted as a measure nobody checked.

**Laws.** Positivity refuses the classic negative occurrences. A generated recursor's β-rule holds (ι-reduction on each
constructor). A compiled `match` is convertible to its recursor form. Coverage is complete: a match the checker accepts
never gets stuck. Every accepted recursive definition terminates on closed arguments, tested by evaluation to a normal
form under a budget that is generous but finite. A scrutinee at a forced index is refused, naming the index. The
compile-fail suite carries one case per refusal.

## Target

- Parameterized and indexed `data` in `musa-core`, with strict positivity, generated dependent recursors, case-tree
  compilation with coverage and unreachability reporting, index refinement by the solution rule, and the well-founded
  termination checker.
- Level metavariables and their solver, with `Level` forced wherever it is read.
- New `Code` variants with `musa explain` text for: non-positive occurrence, incomplete match, unreachable branch,
  forced index, and unchecked recursion. `musa-compiler`'s diagnostic registry is the one file this prompt touches
  there; its checker is untouched.
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
- No proof-search, no tactic language, and no discriminator motive, injectivity lemma, or conflict rule for a forced
  index — §1.4 forbids the wider rules until a program needs them, and the Design says what re-opening would take.
- No change to `musa-compiler` or `stdlib/`. The cutover is prompt 142.
