---
id: 143
slug: one-theory-amendment
status: pending
depends_on: [142f]
phase: 3
---

# Commit to One Theory, and Amend the Constitution to Say So

## Task

The core is three partial mechanisms where one would do: an erased index stratum (§1.5), a non-dependent eliminator
(§1.1), and first-order instantiation in place of unification (§2.1). Each was admitted to avoid a dependent core, and
each now approximates what a dependent core does properly. This prompt takes the amendment under
`docs/rules/README.md`'s procedure and commits the language to **one theory: a dependently typed core with indexed
families, in the shape of Idris2's `Core/TT`**. It changes no code.

This reverses two committed amendments — prompt 128's admission of the language pass and 142c's admission of the index
stratum — so it must state its evidence rather than assert a preference.

## Read

- `docs/rules/README.md`, "Changing a decision" — the six requirements. This amendment answers all six and the Design
  section below is where.
- `docs/rules/constitution.md` §9 and `docs/rules/obligations.md` — what is being narrowed and what is not.
- `docs/rules/language/02-core-calculus.md` §1.1, §1.3, §1.4, §1.5, §2.1, §3 — every section this retires or repairs.
- `docs/plan/prompts/142c-index-amendment.md` — the amendment being reversed, and the count it rested on.
- `/Users/jcreinhold/Code/Idris2/src/Core/TT/Term.idr` — the reference shape. Twelve constructors; musa keeps seven.

## Design

**The commitment, in one sentence.** Musa's core is a dependently typed λ-calculus with Π, Σ-as-one-constructor-data,
inductive families with indices, a universe hierarchy, metavariables solved by pattern unification, and case trees — and
nothing beside it.

**The evidence, item by item.**

*The index stratum was admitted on a count.* 142c cited 17 of 121 compiler builtins hardcoded to modulus 12. That count
is real and the stratum is not what discharges it: `Base` already carries `kind: Term`, so a base type may take
parameters, and `Syntax<Cat>` already is a type applied to an index. What the solver uniquely buys is index *arithmetic*
— `Bar(p+q)` — and **no committed `.musa` file uses it**. 142c's own Design conceded `Bar(m)` is checkable only when the
durations are static.

*The stratum made §3 false.* §3 states `A ≡ B iff quote(A) = quote(B)`. §1.5 erases indices at `quote`. Together they
say `Pc(12) ≡ Pc(24)`. The code does not do that, because `convert.rs` compares `Form::Indexed` structurally on
*values*, before quoting — a conversion rule read-back cannot decide. That is the defect, and it is the same defect
class as subtyping, which is why this amendment refuses that too.

*The eliminator is non-dependent.* `family/assemble.rs`'s `motive_type` answers a type rather than a family: a method's
result and its induction hypothesis are both the plain `R_j`, and nothing is applied to the value eliminated. So the
core has dependent Π formation over simply-typed elimination, which is not a theory — it is two halves of two.

*Instantiation is first-order.* §2.1 takes Idris2's `checkRtoL` without its fallback. Every construct that wants a real
solution — implicit arguments, index unification in `match`, a metavariable that outlives one call — is unavailable
because the mechanism underneath it is not there.

**What is refused, and stays refused.** This is a language for music, not a proof assistant. The amendment admits the
*theory*; it does not admit the apparatus. No tactic language, no hint database, no proof search, no `auto` implicits,
no interactive holes as a workflow, no opt-out from totality, no `assert_total`, no `Type : Type`. `Equal`/`Refl`
becomes declarable because a family is declarable — that is a consequence of the theory, not a proof assistant, and
§1.4's refusal is repaired to say which of the two it was refusing.

**What is refused newly.** *Subtyping, in every form.* Not cumulativity, not subsumption, not coercive subtyping. The
property this amendment exists to buy is that `≡` is decided by one mechanism — evaluate, read back, compare — and
subsumption is not decided that way. Roadmap §2's layer table argues the same from the domain: its whole point is that
written pitch and MIDI number must **not** be interchangeable, and implicit conversion is the mechanism for making them
so. Musa has exactly one coercive rule today, `Accepts` in `base.rs`; prompt 159 deletes it rather than generalizing it.

**What is admitted that was not.** A universe *hierarchy* with level variables, replacing the two fixed points of
`level.rs`. Prompt 152 owns the design; the amendment's part is to say the ceiling is gone. Non-cumulative, which is the
sound and conventional choice and is argued at 151.

**Traits are not narrowed — they are removed.** `10-traits.md` is retired outright by prompt 145 and gets no successor
document. The evidence is prompt 146's, restated here because the amendment must carry it: 6 traits, 82 call sites, **0
trait-constrained signatures**, and `Eq`'s five instance bodies are literally the five δ-builtins.

## Target

- `docs/rules/constitution.md` §9: one theory, named, with the refusals above stated as refusals of *apparatus* rather
  than of typing power, and the new refusal of subtyping.
- `docs/rules/obligations.md`: the obligations that follow — totality, decidable conversion, read-back equality as the
  *only* conversion rule, and the ban on any acceptance rule conversion cannot see.
- `docs/rules/README.md`: the amendment record, with this prompt's number and its six answers.
- `docs/plan/prompts/142c-index-amendment.md` and `142d-index-stratum.md`: a banner naming this prompt as the reversal.
  They are not deleted; the ledger links to them.
- `docs/notes/research/language-design-closure/`: a new decision record carrying the measurements above so the
  amendment's evidence has a home outside the prompt.

## Check

```sh
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
git diff --stat -- crates/ stdlib/ examples/    # must be empty
```

The last line is the check that matters: this prompt changes no code, and a diff under those three paths means it did.

Commit as `Commit the language to one theory`.

## Stop

- No code. Not one line of `crates/`, `stdlib/`, or `examples/`.
- No rewrite of `02-core-calculus.md` §§1–3 — that is prompt 144, and it is separated so the amendment can be reviewed
  as a decision rather than as a specification diff.
- No new refusals beyond the two named. In particular this prompt does not decide whether `Level` becomes polymorphic or
  how; 151 does.
