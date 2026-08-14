---
id: 127dcfaf
slug: syntax-step-recursor
status: pending
depends_on: [127dcfae]
phase: 3
---

# Replace the Syntax Catamorphism with an Inherited-Context Recursor

## Task

Prompt 127dcfae froze the interface on paper. This prompt amends the candidate language pages to describe it and then
implements it: `SyntaxStep<C,A>` as an opaque, non-storable, sealed suspended recursive call; `run_syntax_step` as the
one operation that resumes it; the recursor with explicit inherited context; and the current `fold_syntax` derived from
it at `C = Unit` rather than owned as a primitive.

Prompt 127da's design law — "the fold is the only way into a syntax value" — is superseded here, deliberately and in
writing. `Syntax` remains opaque, paths remain compiler-derived, and `SourceInfo` remains unreadable; what changes is
that an adapter may look at a node before deciding whether, in what order, and under what context to read its children.

## Read

- The frozen interface from prompt [127dcfae](127dcfae-recursor-trial.md) and its research note under
  `docs/notes/research/language-design-closure/`. Where that note and research note 39 differ, the trial wins: it was
  written against programs.
- `docs/notes/research/language-design-closure/39-totality-and-structural-abstraction.md` §5 — the intrinsic equations
  (§5.2), the eleven laws (§5.3), and the rank-1 argument. §5.2's "Correction to the first draft" states why the child
  and its runner are one sealed value and not two, which is the property the implementation must not lose.
- Prompt [127da](127da-path-aware-syntax.md) in full — path derivation, `SourceInfo` with no eliminator, the builder
  facade, `checked_expression`, and the phase-local driver. Every one of those survives; only the eliminator changes.
  Its `status` stays `done` and its history is not rewritten; this prompt records the supersession.
- `docs/rules/language/00-semantics.md` and `docs/rules/language/02-core-calculus.md` §5, §5.5, §5.8 — the judgment
  forms, the closed type grammar, the strong-normalization measure, and the four builtin families. These are the
  candidate pages this prompt amends, and it amends them in this commit before any code changes, per
  [`../../rules/README.md`](../../rules/README.md).
- `docs/rules/language/05-verification.md` — where the adapter laws live.
- `crates/musa-compiler/src/core.rs`: `fold_syntax` at line 9519 and its caller at 9405; the `Builtin` and `Eliminator`
  enums; `BUILTIN_OWNERSHIP`; and the storable-data admission predicate that must exclude `SyntaxStep`.
- `crates/musa-compiler/src/core_budget.rs`: `Reduction::SyntaxFold` at line 144 and its spelling at 162.
- `docs/rules/language/02-core-calculus.md` §1.1 and the `d`/`a` variable classes — a sealed step is excluded from `d`
  because its hidden representation contains the current algebra.

## Design

**Amend the rules first, then implement.** The candidate pages gain the phase-local typing and evaluation rules for
`SyntaxStep<C,A>` and `run_syntax_step`, the recursor's rule with its inherited context, the structural-decrease case in
§5.5's measure, and the `d`-exclusion in the storability predicate; `05-verification.md` gains the eleven laws. Code
follows in the same commit. A prompt that implements against stale rules is the drift root `AGENTS.md` forbids.

**The step is sealed, and that is the whole safety argument.** `SyntaxStep<C,A>` binds together one immediate proper
child, the algebra of the recursor that exposed it, and the operation that resumes that recursor. It has no source
constructor, and no operation yields its child, its algebra, a path, a scope, a source range, or raw syntax.
`run_syntax_step(c, step)` supplies `c` to the sealed child's branch. Because the child and the runner cannot be
separated, a step captured in a closure and carried into a nested traversal still runs *its own* child under *its own*
algebra: there is nothing for a nested recursor to re-associate it with. No dynamic owner check is needed, and none is
added — a dynamic check would need a failure result that the operation has no way to produce.

**Rank 1 is preserved.** `C` and `A` are quantified only in the builtin's own scheme, and `SyntaxStep<C,A>` is one
nominal phase-local type constructor over ordinary type arguments. Nothing here introduces higher-rank or higher-kinded
polymorphism, and the checker's principal inference is unchanged. If the implementation finds it needs either, that is a
finding and a stop.

**The old fold is derived, not deleted.** With `C = Unit`, running every step in source order and passing the resulting
`List<A>` to the old group algebra is observationally the current `fold_syntax` — law 9. It stays available as a derived
convenience only if its name and its stated equations make the strict bottom-up behavior explicit; a name that hides the
eagerness is the thing that produced `Pending` in the first place. Existing callers keep working through the derivation
and are migrated by prompt 127dcfag, not here.

**Budget accounting is versioned and hides nothing.** Minting a step and running a step are charged by their own
reduction kinds, so capture and repeated running cost what they cost. Law 10 is the requirement; a step must not become
a way to buy work off the meter. Whether the cost table needs a version bump is decided by whether an existing weight
changes, not by the arrival of new reduction kinds — prompt 127dcfaa's argument applies unchanged.

**Every law gets executable evidence.** Note 39 §5.3's eleven laws are the frozen list, and the trial's law-to-program
table says what each owes. A law whose evidence is a paragraph is asserted, not implemented.

## Target

- `docs/rules/language/00-semantics.md` and `docs/rules/language/02-core-calculus.md` — the phase-local rules for
  `SyntaxStep<C,A>`, `run_syntax_step`, and the inherited-context recursor; the §5.5 structural-decrease case covering
  capture and nesting; the §5.8 builtin-family row; the `d`-exclusion; and the sentence recording that 127da's fold-only
  law is superseded and why.
- `docs/rules/language/05-verification.md` — the eleven laws, each naming its evidence.
- `crates/musa-compiler/src/core.rs` — `SyntaxStep` as a phase-local value; the recursor builtin with its branches and
  initial context; `run_syntax_step`; the derived `fold_syntax`; `BUILTIN_OWNERSHIP` entries with their hidden
  information; and the storability predicate rejecting `SyntaxStep` at the phase boundary with a diagnostic that says
  why.
- `crates/musa-compiler/src/core_budget.rs` — reduction kinds for minting and running a step, with their printed
  spellings.
- `crates/musa-language/`, `editors/tree-sitter-musa/` — only if the frozen surface needs syntax. If `run_syntax_step`
  is a builtin rather than a form, neither changes and the prompt says so.
- Tests in `crates/musa-compiler`, one per law: sealed formation; association under nesting, using prompt 127dcfae's
  hostile program; inherited context delivered exactly; path uniqueness; structural decrease with a captured step run
  later; repeatability under two different contexts; determinism; opacity, as compile-fail cases proving no operation
  reveals `SourceInfo`, scopes, raw syntax, path, or the algebra; fold derivation, as a differential test against the
  current `fold_syntax` over the existing corpus; budget accounting for capture and repeat; and phase conservativity, as
  compile-fail cases proving ordinary source can name neither `Syntax` nor `SyntaxStep`.
- The differential-evaluator coverage the laws suite already runs, extended to the new builtins.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo nextest run --run-ignored all -p musa-compiler
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Replace the syntax catamorphism with an inherited-context recursor`.

## Stop

- No public `Syntax` type, no `SourceInfo` eliminator, no path constructor from a number or a string, and no operation
  that forges a `BindingPath`. Prompt 127da's privacy rules are untouched.
- No cursor with parent or sibling navigation. A step goes down, once, to a child that was already there.
- No general recursion, no `fix`, no higher-rank polymorphism, no higher-kinded type variable, and no type class.
- No dynamic owner check and no failure result on `run_syntax_step`. If sealing turns out not to make one unnecessary,
  that contradicts the paper trial: report it and stop.
- No quotation, antiquotation, text-to-syntax, syntax reflection, or fresh-name operation.
- No staff adapter rewrite. Prompt 127dcfag owns the migration and its measurement, and doing it here would merge the
  interface's evidence with its first user's.
- No change to `docs/rules/constitution.md` or `docs/rules/obligations.md`. Totality, rank-1 HM, and the CBPV and
  dependent-type refusals all stand; if the implementation seems to need one relaxed, that is an amendment request under
  `docs/rules/README.md`, not a repair to make in passing.
