# The Trusted Kernel Boundary

This file defines the trusted computing base for Musa.

When the independent re-checker runs, a bug in trusted code may let it accept an ill-typed program; an ill-typed
artifact produced by untrusted elaboration must be rejected. The re-checker runs in debug builds and the test corpus,
not on every release compilation. This is an implementation audit boundary, and everything below states how it is
enforced and where it runs.

Musa's boundary is *inside* one crate rather than across a dependency graph. `musa-calculus` is a leaf, its semantic
domain must stay private (roadmap §15.12), and an elaborator in a second crate would need `Value` in the facade — so the
line is drawn between two modules and checked by two laws rather than by a dependency gate.

## Trusted: `src/kernel/`

Everything here works on `Term` and `Value` and raises `CoreError`. It is a structural recursion with no search in it.

| Module | What it decides |
| --- | --- |
| `term`, `value` | what a finished term is, and what it evaluates to |
| `eval`, `quote` | normalization by evaluation, and reading a value back η-long |
| `context`, `scope` | the binders a term is read under, and the globals it resolves in |
| `family/` | inductive families, strict positivity, generated recursors, ι |
| `base`, `program` | the base/builtin data and the definition data a name resolves to |
| `recheck`, `checked` | the audit below, and the boundary type it is stated over |
| `sort`, `budget`, `origin`, `error`, `index`, `list`, `meta`, `room`, `visibility` | the small types the rest is stated in |

## Untrusted: `src/elaboration/`

These may be wrong without creating a silent soundness hole, because the kernel re-checks what they produce.

`raw` and `elab/` (bidirectional elaboration, metavariables, implicit insertion), `declare` and `declare_program`
(declaration groups and their dependency ordering), `case` and `rec` (case trees, coverage, the recursion rewrite),
`convert` (definitional equality *with* metavariable solving and mismatch paths), `admit` (the registry's structural
admission), `namespace`, `storable`, `show`, `refuse`.

`convert` is worth naming twice. It decides definitional equality, and it is untrusted — because the walk that decides
it also solves metavariables and builds a diagnostic path, and a kernel that shared that walk would be auditing itself
with the instrument under audit. The kernel's own conversion is `quote ∘ eval` compared by `Term`'s `PartialEq`.

### What a spine holds, and why it can be a term

`eval` is call by value with one exception, `02-core-calculus.md` §3's fifth strategy rule: a recursor is strict in its
target and lazy in its methods, so a blocked spine's argument is a `value::Arg`, which is either a value or a term with
the environment it is read in. Two things keep that inside the trusted account rather than widening it.

**It is a cost decision and not a semantic one, because the calculus is total.** Every δ-rule is a function of its
arguments and no rule emits a diagnostic (§2.4), so a method that was not evaluated and one that was are the same value.
`quote ∘ eval` therefore decides exactly what it decided before the rule existed, which is what invariant 2 asserts and
what the conformance oracle checks byte for byte.

**A delay is looked at in two places and the type says which.** `Arg` is not a `Value` and has none of a value's
operations, so a reader that met one has to say what it does about it: `eval::demanded` evaluates it, and `Arg::settled`
reads it only if something already has. ι selecting a method is the first place, and it is a frame of the machine rather
than a host call so that a fold does not grow the host stack (§4.1). A spine that stayed stuck being read back,
compared, or asked its type is the second, and those force through `demanded` before the value leaves. Everywhere else —
a constructor's fields, a projection's subject, a metavariable's pattern spine, a builtin's arguments — the head is not
a recursor, so nothing there is ever delayed, and each of those readers says so where it reads.

## The kernel acceptance invariants

**1. No unsolved metavariable crosses the boundary.** Enforced by `Checked::try_from`, which is the only way to obtain
the type `recheck` accepts. §2.1 never defaults and never generalizes, so a metavariable that survives elaboration is
not an under-determined program — the elaborator would have named it `Refusal::Unsolved` where it was written. It is a
solution that escaped its scope or a constraint left in a queue.

**2. The kernel does not search.** No backtracking, no solver, no heuristic ordering. Checking is structural recursion
and conversion is `quote ∘ eval`. This is what makes kernel acceptance a property of the language rather than of an
implementation's search order — the same argument `budget.rs` already makes about resource limits.

**3. Scope discipline holds.** Every de Bruijn index in a checked term names a binder that encloses it.

**4. A metavariable's solution stays in its scope.** §2.1 admits a solution only when it mentions no variable outside
the unknown's scope, and `unify::assign` enforces that where it writes one. The re-checker enforces it again, over the
solution that was actually stored and by whatever route it got there — because verification that trusts the pass it
verifies verifies nothing, and this one's failure mode is otherwise silent.

**5. A case tree's branches answer the motive its own pattern instantiates.** The invariant that makes dependent
elimination sound, and nothing else in the crate re-states it. It is checked without the kernel knowing what a tree is:
a tree body is read through its *emission*, and a generated eliminator's method type **is** the motive at that method's
pattern. A branch answering anything else is `Malformed::Mistyped`.

That is also what settles a refuted branch. `elaboration::case`'s `filtered` builds the motive of a branch that index
unification ruled out by *eliminating over the subject's index*, not by recording that it refuted — so ι reduces it to
an identity type exactly when the branch really is unreachable, and the `λx. x` the tree emits there is checked against
a type computed independently of the decision it is evidence for.

**6. A declaration's level parameters are instantiated consistently.** A use carries its own `Levels`, and the type the
re-checker derives for it comes from `Def::instance` *at those levels*. A use that named the wrong number of them is
`Malformed::LevelArity`; one that named the wrong levels derives a type the term around it does not accept.

**7. Coverage is complete, and every recursion descends.** The two questions an emitted term cannot be wrong about: a
missing alternative emits a shorter spine at a type that no longer mentions it, and a call that does not descend emits
an application like any other. Both are re-derived — coverage from the declaration group, descent from the finished tree
— and since prompt 158 the *kernel* is what asks them. The elaborator asks too, where it builds a tree; that is
bookkeeping, and this is the audit.

The third is why the re-checker exists in the shape it does. **Idris2 gets it for free**: `Term vars` is indexed by its
scope, so an index that escapes its binder does not typecheck in Idris2 itself. Prompt 147 chose plain de Bruijn indices
knowing that, and the re-checker is the mitigation that decision was taken against.

**A weaker compile-time version does exist in Rust, and Musa declines it deliberately.** `kan`'s `core-world` brands a
`WorldView` with an invariant lifetime and refuses to promote an `OpenTerm` to a `ScopedTerm` except through it, so
terms minted under different scopes cannot be mixed *at compile time*. That does not give Idris2's guarantee — index 3
is still not statically known to be in range — but it does statically kill the corruption case, so "Rust cannot express
this" is too strong and should not be repeated. Musa declines it for a stated reason: the brand is a lifetime parameter
on `Term`, and Musa's terms are `Arc`-shared and cross into `musa-compiler`, so the parameter would thread through a
facade prompt 148 spent a prompt keeping stable. `kan` pays that cost because its terms are already arena-allocated
behind a `'tcx` and its kernel is nine crates rather than one module. `Checked` is the same promotion with the check at
run time, and the re-checker is what makes a run-time check sufficient. If scope corruption ever shows up in practice,
the brand is the escalation.

## Where the audit runs

Behind `#[cfg(debug_assertions)]` on every elaborated declaration (`elaboration/declare_program.rs`), and
unconditionally over the whole fixture corpus in `tests/suite/recheck_laws.rs`. Not in release builds: it roughly
doubles the cost of checking, and its job is to catch *our* bugs rather than an author's.

Since prompt 158 there is also a closed entry point, `recheck_program`, which reads a whole document's definitions
rather than one term. `musa-compiler`'s suite runs it over the standard library, which is the gate that makes the claim
above a statement with a corpus behind it. It lives there rather than here because the standard library is `.musa`
source and this crate is a leaf with no parser.

Exhaustion is not a disagreement. The audit runs on its own meter at the context's budget, so a large declaration may
run out of steps — and turning that into a panic would make a debug build reject programs a release build accepts.
Neither is a δ-rule's refusal, which is a host rule answering about the author's own arguments.

## What it caught on the first run

The audit is not hypothetical. Turning it on rejected `stdlib/src/notation/staff.musa`, and the term it named was right:
`Split::read` in `elaboration/case.rs` read a family's parameters off the subject's type *unforced*, so a subject whose
type was still headed by a metavariable the solver had since filled read back as that metavariable, its parameters came
off it as nothing at all, and the emitted recursor spine was short by exactly the parameters — a `match` ι would then
never fire on. Nothing in the suite observed it, because nothing evaluated that `match`. The kernel observed it the
first time it was asked.

That is the claim working: elaboration had a bug, the kernel rejected the artifact, and the defect surfaced at the
declaration that caused it.

## What the generated law caught, and why it was the kernel's turn to be wrong

`generated_laws.rs` found a disagreement on `(let u : Unit = unit in fn (x : Unit) { x }) unit`, and that one was the
audit's own defect rather than elaboration's. `recheck`'s `peeled` reads a β-redex spine as the nested `let` the
evaluator reduces it to — that is how a Curry-style λ, which carries no domain, gets one at all: from the argument. It
walked the spine to the head and required the head to be a λ *exactly*, so a `let` written in front of the λ stopped the
walk, the audit fell back to inferring the λ, and a λ has no type of its own to derive. `check` had the corresponding
arm and had had it since prompt 149; `peeled` did not, and the shape reaches it only when a `let` stands in a function
position — which nothing hand-written in the corpus did.

The fix peels `let` and λ together, which is what the evaluator does anyway. Worth naming as a pattern rather than as an
incident: the two hypotheses were "the audit is incomplete" and "elaboration emits a term the audit is right to refuse",
and they are told apart by asking whether the term is one `eval` reduces. This one was, so the audit was short a rule.
Had it not been, the fix would have belonged on the other side of this file.

## What the re-checker does not cover yet

**The pass is closed.** Prompt 158 audited each extension prompts 151–157 owed, and the result — construct by construct,
with the negative control that proves each can reject — is the table in `tests/suite/recheck_laws.rs`. Two obligations
turned out to be answered by machinery nothing trusted was asking (coverage and descent), one turned out to be
discharged by the emission for a reason worth writing down (156's refuted branch), and the rest were met where their own
prompts said.

**What it cannot catch, stated rather than glossed.** This pass shares the kernel's `eval` and `quote`, and its
conversion is `quote ∘ eval` compared by `Term`'s `PartialEq`. So it **cannot** catch a bug *in* `eval` or in quotation:
it would re-derive the wrong answer using the wrong instrument and agree with itself. What it catches is elaboration
producing a term the kernel would reject — which is the overwhelming majority of what can go wrong here, and is exactly
the split this file draws. A trusted base that overstated its guarantee would be worse than one that states a smaller
guarantee accurately.

Two techniques would reach further and neither is this pass: a second evaluator written from the specification, which
`lib.rs`'s **"One evaluator"** invariant forbids for a stated reason, and a proof about `eval` itself, which is a
different discipline. Until then, `eval` and `quote` are trusted in the strong sense — they are the part of the trusted
base with nothing behind them.

## Enforcement

Two laws, both in `tests/suite/`:

- `boundary_laws.rs` — no code line under `kernel/` names the module `elaboration`, any type it declares, or any name
  the crate root re-exports from it. Rust cannot say "a module may not see its sibling", so the direction is checked
  over the source text.
- `recheck_laws.rs` — the audit over every fixture in the suite, the scope-discipline law, the audit table, and a
  negative control per construct. A re-checker nobody has seen reject anything is a function that returns `Ok`.
- `elaboration/audit_laws.rs` — the three controls that stage a *broken* `Compiled`. They sit on the untrusted side
  because `boundary_laws.rs` forbids `kernel/` to name anything here and building one means writing raw syntax, and they
  exist at all because elaboration refuses both defects at the front door: the audit is for the case where something got
  past it.
- `generated_laws.rs` — the claim over programs nobody wrote. The re-checker is an oracle, so a generated program needs
  no expected output: if elaboration accepted it, the kernel must accept what elaboration produced.
