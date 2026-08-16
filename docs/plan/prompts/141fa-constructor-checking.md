---
id: 141fa
slug: constructor-checking
status: done
depends_on: [134, 135, 141b]
phase: 3
---

# Check a Constructor Against Its Family

## Task

`02-core-calculus.md` §2 says "a constructor checks against its family at known parameters and indices". The elaborator
does not do this: a constructor of a parameterized family is checkable only by writing its parameters as ordinary
arguments, so `Some(2)` is refused and `Option.Some Nat 2` is required. Implement the rule the section already states,
in `check` and nowhere else.

This is not a new feature. The **value** side of the same boundary has done it since 141b, under the name
`family::realize`, and its doc comment says what §2 says in the same words. What is missing is the raw-term twin.

## Read

- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §2, and the sentence "A
  constructor checks against its family at known parameters and indices. A projection, a variable, and a literal infer."
  Then §1.1 on why a parameter is fixed across the declaration and an index is chosen per constructor — that is what
  makes reading a parameter off the expected type well defined and reading an index off it not.
- `crates/musa-core/src/elab.rs` — `Elaborator::check`, `bare`, and `abstracted`. `bare` is §1.3's rule, and it is the
  measurement below: it qualifies the written word and re-checks, and that re-check cannot succeed for a family with
  parameters, which is all three of `Option`, `List`, and `Result`.
- `crates/musa-core/src/family.rs` — `realize`, `realize_case`, and `element`. `realize` is this prompt's rule one level
  down and is the design to follow rather than to duplicate: it takes canonical data whose fields carry no parameters,
  plus the type it stands at, and supplies the parameters from the type. Its doc comment — "the parameters come before
  the fields, ι reads them by position, and nothing in the name or the fields says what they are" — is the argument.
- `crates/musa-core/src/base.rs`'s `Datum::Case`, whose `fields` doc says "the family's parameters are not among them".
  That is the same decision, taken for δ-rule answers, and it is why a rule that answers `Option.Some(x)` is already
  written the way source wants to write it.
- [`141b`](141b-base-types-and-builtins.md) for `Builtin::structural_with`'s closed-vocabulary invariant, which the
  rejected alternative below breaks.
- [`141g`](141g-raw-lowering.md)'s Design — "a lowering that took an expected type would be checking, and checking is
  what was moved". This prompt is the other half of that sentence: the reading cannot supply a parameter, so the checker
  must.
- `stdlib/src/context.musa` lines 41–44, which write `None` and `Some(register)` bare. It is the program the rule exists
  for, and it is representative rather than special.

## Design

**The rule is §2's, and it is not implemented.** Measured against the compiler's own context, with `Option Nat`
expected: `None` is refused, `Option.None` is refused, `Some(0)` is refused, `Option.Some(0)` is refused, and
`Option.Some Nat 0` is accepted. `Elaborator::bare` exists to make the first of those work — it reads an unqualified
word in the namespace the expected type names — and it can never succeed as things stand, because the qualified term it
hands on is a function of a parameter nobody wrote. A rule that is unreachable for every family that has parameters is
the shape of a missing case rather than of a working one.

**The value side already decided this, and the decision is `realize`.** A δ-rule answers `Datum::Case { constructor,
fields }` with no parameters among the fields, and `family::realize` supplies them from the type the answer stands at.
So the core has two doors, and canonical data comes in through one of them with parameters supplied and through the
other with parameters demanded. Making the second door match the first is not a new decision; it is the removal of a
disagreement.

**Widening `check` refuses nothing that is accepted today.** The rule fires only where the expected type is an element
of a family, the raw is a spine whose head names one of that family's constructors, and the written arguments number the
constructor's *fields*. Everything else — including the fully written `Option.Some Nat 0` — falls through to `Switch`
exactly as now. That is what makes this additive: no existing program changes meaning, and no existing law changes
spelling.

**Not by making a parameter implicit, and the reason is measured.** The obvious alternative is to generate a
constructor's type with its family's parameters as implicit Π binders, which is what several dependent languages do.
Trialled: 28 of `musa-core`'s 163 laws fail, because `infer(cx, Raw::var("Option.Some"))` stops answering a closed term
and answers an unsolved implicit instead — and a closed constructor term is exactly what 141b's `structural_with`
vocabulary requires. It would also change the meaning of every explicit spelling already written rather than accepting
one more. A change that rewrites twenty-eight laws to admit one program is evidence about the change.

**Indices are not parameters and this rule does not touch them.** §1.1 says a parameter is fixed across the declaration
and an index is what a constructor chooses. So the parameters of the expected type are the constructor's parameters by
construction, and its indices are its own — reading an index off the expected type would be assuming the answer that
coverage exists to check. A constructor whose indices do not match is refused where it is refused now.

## Target

- `crates/musa-core/src/elab.rs`: §2's constructor rule in `check`, doc-commented with the sentence it implements and
  the reason it reads parameters and not indices. `bare` becomes reachable — folded into the new rule if that is the
  narrower interface, kept beside it if it is not, but not left as a case that cannot fire.
- The term the rule builds is the term the explicit spelling builds. One answer, stated as a law rather than asserted.
- Laws in `crates/musa-core/tests/suite/`: a bare constructor, a qualified constructor, and an applied constructor each
  check against a parameterized family; the explicitly parameterized spelling still checks and yields the same term; a
  constructor of a family with no parameters is unaffected; a nested constructor (`Some(Ok(0))`) checks; an
  under-applied and an over-applied constructor are still refused, with the refusals they have now; an indexed family's
  constructor still checks its own indices.
- A law in `crates/musa-compiler` that the prelude's own families are writable the way source writes them — `None`,
  `Some(x)`, `Ok(x)`, `Err(x)`, and a `List.Cons` chain — since those five are what the surface lowers to and the
  compiler's context is where they are declared.
- `docs/plan/code-map/` rows for `musa-core`.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-core -p musa-compiler
cargo clippy --all-targets -p musa-core -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Check a constructor against its family`.

## Stop

- No plicity change to a generated constructor, family, or recursor type. The alternative was measured and rejected
  above; re-taking it is a repair of this prompt, not an implementation of it.
- No change to `realize`, to `case.rs`, or to coverage. This prompt makes the raw side agree with the value side; moving
  the value side would move the thing being agreed with.
- No new refusal and no new diagnostic code. Every program refused before this prompt is refused after it, with the same
  refusal — a rule that fires only where the elaborator used to fail has nothing new to complain about.
- No index read off the expected type. A constructor chooses its indices and coverage checks them.
- No surface change and no `.musa` change. `141g` reads the surface; this is the rule that makes what it reads
  checkable.
