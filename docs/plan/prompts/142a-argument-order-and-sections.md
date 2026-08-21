---
id: 142a
slug: argument-order-and-sections
status: pending
depends_on: [142]
phase: 3
---

# Make Higher-Order Calls Terse: Reorder the Spine, and Let a Section Be Written

## Task

Two changes to the elaborator, both small, both aimed at the same defect: a Musa author cannot write an ordinary
higher-order API without arranging the signature around the checker's weakness, and cannot name a transformation at all.
Neither needs an amendment, an index, or a metavariable that survives its call.

1. **Reorder the argument spine.** When an argument's expected type still mentions an unsolved hole and the argument is
   a checking-only form, defer it: check the rest of the spine first, which solves the hole, then come back. Argument
   order stops being semantically significant for un-annotated lambdas.
2. **Admit written sections.** `f(a, _)` is the function that takes what `_` stands for. Under-applying without a `_`
   stays a type error, so every slot is still named at every call.

## Read

- [`51-the-terseness-audit.md`](../../notes/research/language-design-closure/51-the-terseness-audit.md) §§5–6, which is
  this prompt's whole argument, including the two library sites that motivate it.
- `crates/musa-calculus/src/elab/spine.rs`, the `apply_spine` loop — it already computes both predicates this prompt
  needs, `crate::unify::mentions_unsolved(&domain)` and `argument.checks_only()`, and uses them to choose *infer versus
  check* rather than to reorder.
- `crates/musa-calculus/src/elab/check.rs`'s `lambda`, which pushes `scope.assume(name, here, expected_domain)` with a
  domain that may be a hole. That is the failure this prompt removes.
- `~/Code/Idris2/src/TTImp/Elab/App.idr`, `checkRestApp` and `checkRtoL`, and `needsDelayExpr` in the same file. The
  design is taken from there; read the comment beginning "In theory we can check the arguments in any order."
- `docs/rules/language/02-core-calculus.md` §1.3's completeness rule and §2.1, both of which this prompt repairs.
- `stdlib/src/post_tonal/pcset.musa`'s `transposed_by`, whose comment records the section that could not be written, and
  `stdlib/src/list.musa`'s fold signatures, whose argument order is the workaround.

## Design

**Reordering is bounded to one spine and is not postponement.** Each argument is elaborated exactly once; there is no
constraint queue, no wakeup discipline, no fixpoint, and no retry-to-convergence. The rule is: walk the spine left to
right; an argument whose domain `mentions_unsolved` and which `checks_only()` is *skipped* and its slot filled with a
hole-typed placeholder; when the walk ends, the skipped arguments are checked in their original order against their
now-solved domains. An argument still un-typed at that point is the error §2.1 already reports, naming the parameter —
not a new failure mode. Determinism is by construction: the skip set and the second-pass order are both fixed by the
written argument order, so the answer cannot depend on which branch ran first.

Idris2 falls back from right-to-left to left-to-right on `InvalidArgs`. Musa does not need the fallback, because Musa
does not have `%search`, `with`, or ambiguous name resolution in the spine — the two-pass order is total. If a program
is found that needs it, that is a finding for prompt 144, not a silent addition here.

**A section is surface, not a core form.** `f(a, _)` elaborates to `fn (x) { f(a, x) }` in `lower/`, before the core
sees anything, and the core's completeness rule is untouched: the elaborated term applies `f` to two arguments. This is
the enrichment `AGENTS.md` requires and Peyton Jones ch. 3 performs — a source convenience translated away — rather than
a new term shape. Multiple holes read left to right: `g(_, b, _)` is `fn (x, y) { g(x, b, y) }`.

**`.` composes.** `f . g` is `fn (x) { f(g(x)) }`, at the same lowering. It is admitted here because a section with
nothing to compose it with buys nothing, and because note 52 §2.2 needs both to name a group element.

**§1.3's stated reason for the completeness rule does not survive and is repaired, not deleted.** The rule protects
against silence — `f(x)` quietly becoming a function — and the written `_` preserves that protection exactly. What it
does not protect is §1.2's storability, which the checker computes structurally and which a section does not weaken: a
section is a function value, and where a function value may not go, §1.2 already refuses it. The repaired §1.3 says
that, and stops citing §1.2 for a rule §1.2 does not imply.

## Target

- `crates/musa-calculus/src/elab/spine.rs`: the two-pass walk. No new public item.
- `crates/musa-compiler/src/lower/`: `_` in an argument list and the `.` operator, lowered to `Raw` lambdas.
- `crates/musa-syntax/`: the token and grammar for `_` in an argument position; `editors/tree-sitter-musa` follows under
  the drift law.
- `docs/rules/language/02-core-calculus.md`: §1.3's completeness rule repaired to admit written sections and to stop
  citing §1.2; §2.1 restated as the two-pass rule, replacing "nothing is postponed, nothing is retried" with what the
  elaborator does — one deferral, bounded to the spine, resolved at its end.
- `docs/rules/language/01-surface.md`: `_` and `.` in the expression grammar.
- Law suites: an un-annotated lambda in every argument position of a two- and three-argument generic call; a section in
  each slot; `f . g` at three types; the still-unsolved case reporting at the call and naming the parameter.

## Check

```sh
cargo nextest run -p musa-calculus -p musa-compiler -p musa-syntax
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
cd editors/tree-sitter-musa && tree-sitter generate && tree-sitter test
```

And the ergonomic evidence, which is the point of the prompt: `stdlib/src/list.musa`'s folds take the combining function
*first* in at least one new law-suite fixture and still elaborate, and `stdlib/src/post_tonal/pcset.musa` gains
`fn transposition(index: Nat) -> Pc12 -> Pc12 { transposed_by(index, _) }` and the comment refusing it is deleted.

## Stop

- No general postponement, no constraint queue, no retry loop. If a program needs one, it is a finding, not a fix.
- No currying: `f(a)` on a two-parameter `f` stays a type error. The `_` is required.
- No indices, no traits, no law checking — 142c through 142e.
- Do not reorder arguments in the *emitted* term. The spine is elaborated in a different order and built in the written
  one; evaluation order is unchanged and the law suite proves it.
