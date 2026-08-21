---
id: 142c
slug: index-amendment
status: pending
depends_on: [142b]
phase: 3
---

# Amend the Constitution for a Stratified Index, and Write the Specification Before the Code

## Task

`constitution.md` §9 refuses refinement types, narrowed by prompt 128 and again by the course correction to "a result
type may mention an earlier explicit argument". Note 51 argues that the narrowing went one step too far and that the
corpus is paying for it — 17 compiler builtins hardcoded to one modulus, a `fallback` parameter in a public library
signature, and a bar whose contents cannot be summed at compile time. This prompt takes the amendment under
`docs/rules/README.md`'s procedure and writes the specification. **It changes no code.**

## Read

- [`51-the-terseness-audit.md`](../../notes/research/language-design-closure/51-the-terseness-audit.md) in full. §4 is
  the mechanism, §3 the evidence, §8 the six requirements already answered, §9 what would falsify it.
- [`52-the-musical-algebra.md`](../../notes/research/language-design-closure/52-the-musical-algebra.md) §§1 and 4 — the
  generality the index is *for*, and the table of what it does and does not ask of the language.
- `docs/rules/README.md`, "Changing a decision" — the six requirements, and the standing record that the current
  amendment "is answerable to a measurement", which this one inherits.
- `docs/rules/language/02-core-calculus.md` §1.1, §1.3, §2.2, and §3, which this prompt repairs.
- Xi and Pfenning, *Dependent Types in Practical Programming* — the stratification this follows. `citations.md` gains
  the row.

## Design

**The rule, in one sentence.** A type may carry index arguments drawn from a fixed decidable domain; indices are erased
before evaluation, never matched on, and two indexed types are the same type when the solver proves their indices equal.

**What the index domain is.** Exactly three sorts, all decidable, all already in the language: `Nat`, exact `Ratio`, and
finite literal enums (which is what `Syntax<Cat>` already indexes over). The expression language is variables, literals,
`+`, `-`, `*` by a literal, and comparison — Presburger, so equality and entailment are decidable. Nothing else enters,
and a would-be index outside it is a refusal naming the expression.

**What this is not, stated because the confusion is the expensive one.** It is not inductive-family indices.
Enumerations keep parameters only, exactly as §1.1 says after note 50; a constructor never chooses an index; there is no
index unification, no dependent motive, and no `J`. An indexed type is a *declared or base* type applied to index
expressions, and its constructors are the refinement constructors §2.2 already describes. `family/` does not grow.

**Where the index is decided.** In the solver, never in the conversion checker. `convert.rs` meets an index the way it
meets any opaque payload — it asks the index module whether the two are equal and takes the answer. This is the
decomplecting note 51 §4 argues for, and it is the property that keeps the term core the size 142b leaves it.

**Erasure is total.** No stored format changes: a compiled term, an event track, and the `% musa-events-3` interchange
format are byte-identical before and after. This is requirement 5 of the six, and it is why the amendment is cheap.

**What is honestly checkable, stated so 142d does not over-promise.** `Row(n)`, `Pc(n)`, `Ic(n)`, `Voicing(4)`, and
`Icv(n)` are checkable because the index is a parameter fixed at construction or derived arithmetically from one.
`Bar(m)` — the contents sum to the meter — is checkable exactly when the durations are statically known, which is the
common case in written notation and in every expansion an adapter produces; a bar assembled from a runtime list gets a
checked constructor and a runtime refusal, as today. The specification says which case it is at each type rather than
implying the strong one everywhere.

## Target

- `docs/rules/constitution.md` §9 and `docs/rules/obligations.md`: the narrowing, stated in plain language, with the
  refusal of proof-assistant machinery **unchanged** — no identity type, no universes, no measures, no dependent
  motives, no index unification.
- `docs/rules/language/02-core-calculus.md`: a new §1.5, *Index refinement*, with the domain, the erasure rule, the
  conversion rule, and the refusals; §1.3's "No indexed families" repaired to say what is still refused; §2.2 pointed at
  §1.5 for the refinements that now have a type.
- `docs/rules/language/03-musical-domains.md`: the indexed domains of note 52 §1 — the period and division parameters,
  and why `Cyclic(n)` serves pitch class and rhythmic cycle alike.
- `docs/rules/language/citations.md`: Xi and Pfenning.
- `docs/plan/code-map/spec-to-implementation-map.md`: the row, marked absent.
- `docs/plan/clean-break-ledger.md`: the seventeen builtins, to be collapsed at 143.

## Check

```sh
make docs-check
```

And the procedural check, which is the real one: `docs/rules/README.md`'s "Changing a decision" section names this
amendment, links note 51, and answers all six requirements in the page itself — not by reference alone. A reader who
opens only `constitution.md` and `rules/README.md` learns what changed, why, what breaks (nothing), and how to migrate.

## Stop

- No code. Not one line of `musa-calculus`.
- No index-level functions over lists, no existential indices, no proof terms, no `Vec`. If 142d finds it needs one,
  that is a repair to this prompt first, committed before the code.
- Do not reopen `Id`, universes, measures, or constraint-based traits. Note 51 §7 keeps them deleted and this amendment
  does not touch them.
