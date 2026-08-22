# The Trusted Kernel Boundary

This file defines the trusted computing base for Musa.

If a bug exists in trusted code, the kernel may accept an ill-typed program. If a bug exists in untrusted code, the
kernel rejects the artifact. That is the whole claim, and everything below exists to make it enforceable rather than
aspirational.

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

## The kernel acceptance invariants

**1. No unsolved metavariable crosses the boundary.** Enforced by `Checked::try_from`, which is the only way to obtain
the type `recheck` accepts. §2.1 never defaults and never generalizes, so a metavariable that survives elaboration is
not an under-determined program — the elaborator would have named it `Refusal::Unsolved` where it was written. It is a
solution that escaped its scope or a constraint left in a queue.

**2. The kernel does not search.** No backtracking, no solver, no heuristic ordering. Checking is structural recursion
and conversion is `quote ∘ eval`. This is what makes kernel acceptance a property of the language rather than of an
implementation's search order — the same argument `budget.rs` already makes about resource limits.

**3. Scope discipline holds.** Every de Bruijn index in a checked term names a binder that encloses it.

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

## What the re-checker does not cover yet

The pass covers the term language as prompt 147 left it. Each of these prompts owes it an extension, and each says so in
its own **Check** section rather than leaving it to be remembered:

| Prompt | What it adds that the re-checker must learn |
| --- | --- |
| 151 | the index stratum's removal — one arm fewer |
| 152 | a level parameter instantiated consistently |
| 153 | a metavariable solution, and `Checked::try_from`'s first real rejection |
| 154 | the elaborated form of a trait method |
| 155 | a case-tree branch against an instantiated motive |
| 156 | an index chosen by a constructor |
| 157 | records as data |

## Enforcement

Two laws, both in `tests/suite/`:

- `boundary_laws.rs` — no code line under `kernel/` names the module `elaboration`, any type it declares, or any name
  the crate root re-exports from it. Rust cannot say "a module may not see its sibling", so the direction is checked
  over the source text.
- `recheck_laws.rs` — the audit over every fixture in the suite, the scope-discipline law, and a negative control. A
  re-checker nobody has seen reject anything is a function that returns `Ok`.
