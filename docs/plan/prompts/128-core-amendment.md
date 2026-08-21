---
id: 128
slug: core-amendment
status: done
depends_on: [127dcfb]
phase: 3
---

# Amend the Core to Admit a Dependent Foundation

## Task

Musa's surface language is not strong enough to write Musa in. Amend the governing documents to say what replaces it — a
small dependently typed core with bidirectional elaboration, a coherent trait system, real records, and typed quotation
— before any prompt writes a line of it. This is prompt 127a's own rule applied to the language: rules first, then code.

## Read

- `docs/rules/README.md`'s amendment procedure, all six requirements. This prompt discharges every one of them in the
  decision record its Target names, and the record is not optional prose: requirement 6 is what keeps the argument this
  amendment overturns visible after it is overturned.
- `docs/rules/constitution.md` §9 in full, and the paragraph after the five properties — "no dependent or refinement
  types … no type-directed macros" is the sentence this prompt deletes, and the sentence after it ("Each of those may be
  proposed again, and each must then meet the standard in the obligations") is the procedure being used, not
  circumvented.
- `docs/rules/obligations.md` §10 — "The failed examples must come first, and the smallest failing term must be
  recorded." The failing program is `stdlib/src/adapters/staff.musa`, and §10's current wording asks for *musical*
  operations. Whether engineering evidence counts is the question this prompt answers on the record.
- `docs/notes/research/language-design-closure/39-totality-and-structural-abstraction.md` §11.1 and §11.2 — the two
  checklists this amendment is measured against, written before the decision was taken and by an argument that
  recommended against it. §12.3's "record the smallest failing fold program before choosing syntax or a termination
  checker" is the rule the totality half of this amendment must satisfy.
- `docs/notes/research/language-design-closure/38-abstraction-totality-and-substitution.md` §10 and
  `26-language-design-decision.md` §§9–10 — the evidence set the earlier refusals were derived from, and why `print` and
  the adapter's construction half were not in it.
- `stdlib/src/adapters/staff.musa` in full, with a line count. `call1`–`call7` (the argument-list builders), the
  hand-allocated role integers passed to `syntax_built`, `data Pending`'s eight fields, the string dispatch on token
  kinds, and the header's own "**A list cannot be built**" are the measured evidence, and this prompt records the
  measurements rather than describing them.
- Prompt [127a](127a-core-calculus-governance.md) — the shape of a governance-only prompt in this repository, and the
  precedent that a governing amendment is committed before the code that needs it.
- `docs/plan/roadmap.md` §15 — the crate list and dependency lists a new crate has to enter through.
- `docs/rules/across-stages/01-stage-judgments.md` §2 and `docs/rules/across-stages/05-metatheory.md` §1 and §4 — the
  two governing documents outside the constitution that restate §9's inference rule in their own words. Amending §9
  without them leaves a governing document asserting principal types while the constitution says there are none.

## Design

**What is being amended, and what is not.** Constitution §9 keeps four of its five load-bearing properties untouched:
*Complete* calls, the *values / storable data* split, closure at the machine boundary, and purity and strictness. The
event-track and machine core `127a` installed is not reopened. What changes is how a program is *typed* and what
*totality* means.

- **Inferred** becomes bidirectional. Rank-1 Hindley–Milner and principal types are replaced by check/infer elaboration
  with metavariables and pattern-fragment unification. Annotations are still written at public signatures; what changes
  is that a signature may now mention a value.
- **Total** becomes well-founded rather than structural. Every accepted program still finishes. What is replaced is the
  *reason* it finishes: a checked well-founded measure rather than constructor size. The budget rule survives unchanged
  — a budget may stop an evaluation and may not change an accepted program's value.
- The refusal paragraph loses "no dependent or refinement types" and narrows "no type-directed macros" to admit typed
  quotation, which is elaboration-time and produces syntax of a stated category rather than dispatching on an inferred
  type. Every other refusal in that paragraph stands, and CBPV in particular is **not** admitted: §11.2 lists it as a
  separate choice and nothing in the evidence asks for it.

**Why totality is kept, and widened rather than dropped.** This is the half a reader will expect to have gone the other
way, so the amendment states it positively. Musa is almost entirely a compile-time language: adapters run in the
expansion phase, analyses run inside `musa check`, and the desktop re-typesets on a keystroke. Divergence is therefore
not a composer's infinite loop, it is a compiler hang and an editor that stops answering. Dependent conversion makes it
worse, because the checker becomes an evaluator: a diverging term can hang type checking, not merely evaluation. §12.3's
condition — record the smallest failing fold program — is met by the well-founded direction and *not* by the partial
direction, and the amendment says so: there is no `partial` keyword and no escape hatch, and adding one later needs
§12.3's evidence and this procedure again.

**Obligations §10 is amended, not evaded.** Its current standard asks for two musical operations that ordinary finite
data makes unclear or unsafe. The staff adapter is not a musical operation; it is 2,404 lines of Musa that the language
made three times longer than the notation it reads. The amendment adds a second admission route — *measured engineering
evidence in Musa itself*, with the failing program named, its size recorded, and the specific constructs it compensates
with enumerated — and keeps the musical route unchanged. It also keeps §10's real teeth: the failing program comes
first. It exists, it is committed, and prompt 145 has to beat it.

**The decision record is where the six requirements are discharged**, one heading per requirement, and it is written to
be read by someone who thinks this amendment is a mistake. It answers §11.2's five items point by point — including the
one it declines, CBPV — and it records what the earlier notes got right, because 39 §8's reasoning (no *musical*
operation needs a value in a type) is not refuted by this evidence and should not be quietly buried by it.

**`musa-calculus` enters the roadmap now**, before it exists, because roadmap §15 is where a new crate's dependencies
are declared and prompt 133 may not add one that the roadmap does not list. It is a leaf, the way `musa-events` is:
nothing in it knows what a pitch is.

**The consequential edits to `docs/rules/across-stages/` are part of this amendment, not prompt 149's audit.** Two
governing documents restate §9's inference rule in their own words: `01-stage-judgments.md` §2 says `A` is the principal
type of `e`, and `05-metatheory.md` §1 carries a reviewed result about Hindley–Milner inference with §4's evidence rule
under it. Amending §9 and leaving those is the silent drift `AGENTS.md` forbids, and prompt 149 cannot repair them — its
own Stop forbids amending `across-stages/`, and rightly, because a closure audit that quietly rewrites a governing claim
has audited nothing. The rule that separates the two is the same one 149 states: correcting a *pointer* is a repair,
changing what a document *claims* is an amendment, and an amendment belongs in the commit that caused it. Keep the edits
minimal — restate the rule, mark the superseded result as superseded and say by what — and do not rewrite the metatheory
matrix, which is prompt 148's.

## Target

- `docs/rules/constitution.md` §9: *Inferred* restated as bidirectional elaboration with metavariables, *Total* restated
  as well-founded, the dependent/refinement refusal deleted, the type-directed-macro refusal narrowed to admit typed
  quotation, and the remaining refusals left exactly as they are.
- `docs/rules/obligations.md` §10: the second admission route, with `stdlib/src/adapters/staff.musa` named as the
  recorded failing program and its measurements stated.
- `docs/notes/research/language-design-closure/42-dependent-core-decision.md`, and its row in that directory's
  `README.md`: the six amendment requirements under six headings, note 39 §11.2's five items answered one by one, the
  totality argument above, the measured staff evidence, and the refused alternatives — closed overloading alone,
  quotation without a dependent core, dropping totality, and CBPV.
- `docs/rules/across-stages/01-stage-judgments.md` §2: the source typing judgment restated as bidirectional, with no
  principal-type claim.
- `docs/rules/across-stages/05-metatheory.md` §1 and §4: the Hindley–Milner inference row marked superseded by this
  amendment and naming prompt 148 as the prompt that owes its replacement, and §4's inference evidence rule restated.
  Nothing else in the matrix moves.
- `docs/rules/README.md`'s "Changing a decision" section: this amendment recorded as the most recent one, with a link to
  the record, the way prompt 127a's is.
- `docs/plan/roadmap.md` §15: `musa-calculus` in the crate list and in the dependency lists, as a leaf below
  `musa-compiler`.
- `docs/rules/language/README.md`: the graduation list and the prompt ranges it names, repaired for this pass.

## Check

```sh
python3 scripts/renumber-prompts.py audit
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
git diff --check
```

This prompt writes no Rust, so the crate commands have nothing to run. Prompts 133–137 owe the executable evidence.

Commit as `Amend the core to admit a dependent foundation`.

## Stop

- No code. Nothing under `crates/`, `stdlib/`, `editors/`, `examples/`, `apps/`, or `packages/` changes.
- No rewrite of `docs/rules/language/`. Prompts 129–131 own those pages, and a constitution amendment that also rewrote
  the specification it governs would be two decisions in one commit.
- No `partial`, no general recursion, no `fix`, and no CBPV. Each is a separate amendment with its own evidence.
- No new surface syntax is decided here. This prompt says what the language may now be; 130 and 131 say what it is.
- No claim that the earlier refusals were wrong on their own evidence. They were reasoning about musical operations, and
  the record says so.
