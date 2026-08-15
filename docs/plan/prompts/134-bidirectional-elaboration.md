---
id: 134
slug: bidirectional-elaboration
status: pending
depends_on: [133]
phase: 3
---

# Elaborate Bidirectionally, With Metavariables

## Task

Add check/infer elaboration to `musa-core`: a surface-independent raw term goes in, a typed core term comes out.
Metavariables, pattern-fragment unification, implicit-argument insertion, constraint postponement, and the diagnostics
for the two failures this introduces — an unsolved metavariable and a conversion mismatch — which are the first errors
in Musa's history that talk about a normal form the author never wrote.

## Read

- `docs/rules/language/02-core-calculus.md` §2 as rewritten by prompt 129: the check and infer modes, the two
  mode-switch rules, where metavariables are created and solved, the pattern-fragment restriction, and postponement.
- Prompt [133](133-core-crate.md)'s Design paragraph on why `Value` stayed private — this prompt is the caller that
  argument was made for, and it either vindicates the boundary or falsifies it. If checking against a value turns out to
  need something the facade cannot express, that is a repair of 133's facade, not a reason to make `Value` public.
- `crates/musa-compiler/src/infer.rs` — the rank-1 HM unifier this eventually replaces, and specifically how it reports
  a mismatch today. The new checker's messages have to be at least as good, and it is easy for them to be much worse.
- `crates/musa-compiler/src/diagnose.rs`'s `Code` enum and `crates/musa/src/main.rs`'s `cmd_explain` — diagnostics here
  are named variants with an explainable rule behind them, not numbers.
- `docs/rules/language/05-verification.md` for what an implementation gate is, and `07-analysis.md` §"admission rule"
  for the standard a new kind of finding must meet.
- Peyton Jones ch. 9 (a type checker written out) for the algorithmic shape, read for the analysis rather than the
  machinery: the unification there is first-order and syntactic, and this one is not.

## Design

**The raw term is the boundary, and it is surface-independent.** `musa-compiler` will translate Musa syntax into it;
nothing about it mentions pitches, bars, or `.musa` grammar. Keeping it that way is what lets `musa-core` stay a leaf
and lets the elaborator's tests be written without a parser.

**Metavariables are contextual.** A meta is created together with the context it may refer to, and solved by a term in
that context, so a solution can never capture a variable that was not in scope at the creation site. Spelling this out
now is cheaper than debugging a scope escape later, and it is the invariant every other rule in this prompt leans on.

**Unification stays in the pattern fragment, and the restriction is enforced rather than hoped for.** A constraint
`?m x₁ … xₙ ≡ t` is solved when the arguments are distinct bound variables; anything else is postponed, and a postponed
constraint that is still blocked when elaboration ends is an error naming the term it came from. This is the line that
keeps unification decidable and most-general, so the code should refuse loudly at the boundary rather than "try harder"
— a heuristic here is search, and 130 already refused search.

**Implicit arguments are inserted here and nowhere else.** A binder marked implicit produces a metavariable at each use;
the core, per 129, has one Π and never learns about plicity. The one subtlety worth a doc comment is when insertion
stops: an implicit is not inserted when the expected type is itself an implicit Π, or the elaborator will happily insert
forever.

**Two new diagnostics, and they are the first that must show a normal form.** An unsolved metavariable says what could
not be determined, where it was created, and what constraint was still blocked. A conversion mismatch says both sides —
and this is the hard part — in terms the author can recognize: normalized far enough to be honest, unfolded no further
than necessary, with the surface node each side came from (129 §7's provenance) so the message can point at source. A
conversion error that prints two 40-line normal forms is a failed diagnostic even when it is a correct one, so this
prompt owes a stated policy on how much to unfold and a test that holds it.

**Budgets reach type checking now.** Conversion evaluates, so `check` and `infer` carry the budget and can return
exhaustion. Per 129 §4 that is a third outcome with its own message — "this program was not rejected; the checker ran
out of room" — and it must never be reported as a type error.

**Laws and gates.** Elaboration is deterministic. An elaborated term type-checks in the core (the elaborator's output is
independently re-checkable, and the test re-checks it — this is the single most valuable invariant in the whole crate).
Solved metavariables are stable under further solving. A term with no implicits elaborates to itself. Postponement
terminates. And the negative suite: every refusal above has a compile-fail test with the diagnostic it names.

## Target

- Check/infer elaboration in `musa-core` over the raw term, with contextual metavariables, pattern-fragment unification,
  postponement, and implicit insertion.
- An independent re-checker for elaborated terms, used by the tests as the primary correctness gate.
- Two new `Code` variants and their `musa explain` text, with the unfolding policy stated where the code lives.
- `crates/musa-core/tests/suite/{elaboration_laws.rs, unification_laws.rs}` and a compile-fail suite for the refusals.
- `docs/plan/code-map/`: `musa-core`'s row updated.
- No change to `musa-compiler`'s checking path. Still nothing calls this.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-core
cargo nextest run --workspace
cargo clippy --all-targets -p musa-core -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

`existing_language_behavior_matches_the_migration_oracle` in
`crates/musa-compiler/tests/suite/elaboration_compatibility.rs` must be untouched. Prompt 142 is the only prompt
permitted to move an entry in it.

Commit as `Elaborate bidirectionally, with metavariables`.

## Stop

- No inductive families, no dependent match, no coverage, no termination checker. Prompt 135.
- No traits, no dictionaries, no instance resolution. An "instance metavariable" is not a metavariable, and prompt 137
  owns that mechanism.
- No higher-order unification outside the pattern fragment, no unification heuristic, no "try the obvious solution"
  fallback.
- No surface syntax, no parser change, no `.musa` file change.
- No deletion of `crates/musa-compiler/src/infer.rs`. It stays until prompt 142.
- No public `Value`. If this prompt proves the facade wrong, repair 133's facade as a repair commit and say what the
  caller needed.
