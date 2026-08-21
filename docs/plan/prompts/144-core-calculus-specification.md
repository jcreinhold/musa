---
id: 144
slug: core-calculus-specification
status: pending
depends_on: [143]
phase: 3
---

# Rewrite the Core Calculus Specification as One Theory

## Task

Prompt 143 decided; this prompt writes it down. `02-core-calculus.md` §§1–3 are rewritten as a single presentation of
the dependent core — syntax, judgments, conversion, and the metatheory the implementation is answerable to — with the
section numbers preserved so that every cross-stage reference still lands. **It changes no code.**

The specification is written *before* any prompt implements it, which is the rule 142c already followed and the reason
that prompt is cited rather than repeated.

## Read

- `docs/rules/language/02-core-calculus.md` in full — every section is either rewritten or repaired.
- `/Users/jcreinhold/Code/Idris2/src/Core/TT/Term.idr`, `Core/Value.idr`, `Core/Normalise.idr` — the reference.
- `crates/musa-calculus/src/value.rs` — the NbE presentation the crate already has, which the specification must
  describe rather than replace. There is no substitution function in the crate and there must not be one.
- `docs/rules/across-stages/04-identity-and-realization.md` — what identity digests, so §1.5's erasure argument can be
  closed rather than replaced.

## Design

**The term language, seven constructors.** `Var`, `Named`, `Meta`, `Bind`, `App`, `Lit`, `Universe`. Three binders —
`Lam`, `Pi`, `Let` — under one `Bind`, each carrying a `Filling` of `Written` or `Inferred`. Reduction behaviour is not
in the term language: a `Named` carries a `Role`, and the context maps it to a `Definition`. `Base` stops being a term
constructor holding an `Arc<BaseDeclaration>`, which is a context entry smuggled into a term.

**The judgments.** `Γ ⊢ t ⇐ A` and `Γ ⊢ t ⇒ A`, bidirectional, unchanged in shape from what the crate does today. What
changes is that checking may create a metavariable and postpone, so the specification states the constraint queue as
part of the judgment rather than as an implementation note.

**Conversion, and the one equation.** `A ≡ B` iff `quote(eval A) = quote(eval B)`, α-syntactically, with η at `quote`
for Π and for one-constructor data. **This is now true as written**, because there is no erasing wrapper and no
acceptance rule outside conversion. The specification states it as an `iff` and states the obligation it puts on every
later prompt: *no rule may accept a program that conversion would reject.* `Accepts` violates it today; 158 removes it.

**Erasure, closed rather than replaced.** §1.5's erasure protected byte-identity of stored artifacts.
`04-identity-and-realization.md` digests `EventTrack<PerformedTime, Gesture>` projections, bindings, seed and options —
**never a core term**. So no artifact contains a type, nothing is protected, and nothing replaces the rule. Say this in
the specification, with the citation, so it is not re-derived later.

**Metatheory, and what is claimed.** Totality ⟹ normalization ⟹ decidable conversion, as today. The specification says
plainly which links are *argued* and which are *tested*: musa does not ship a formalized metatheory and must not pretend
to. `05-metatheory.md` gains the row that says so.

**Sections retired.** §1.4 (identity type as core machinery) becomes "declarable as a family, and not built in"; §1.5
(index refinement) is deleted with a pointer to 143's amendment; §2.1's first-order matching becomes §2.1 *Unification*,
specified but implemented at 152.

## Target

- `docs/rules/language/02-core-calculus.md` §§1–3 rewritten; §1.4 repaired; §1.5 removed; §2.1 replaced. Section numbers
  of every surviving section unchanged.
- `docs/rules/language/citations.md`: Coquand's NbE, Abel's normalization, Idris2's `Core/TT`.
- `docs/rules/across-stages/05-metatheory.md`: the argued-versus-tested row.
- `docs/plan/code-map/`: the rows that name §1.5 and §2.1 marked as owed to prompts 150 and 152.

## Check

```sh
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
! grep -rn 'Indexed\|index stratum' docs/rules/language/02-core-calculus.md
git diff --stat -- crates/ stdlib/ examples/    # must be empty
```

Commit as `Rewrite the core calculus specification as one theory`.

## Stop

- No code.
- No repair of the documents that *depend* on this one — `11-quotation.md`, `01-surface.md`, `10-traits.md`. That is
  145, separated because those three are read by different people than this one is.
