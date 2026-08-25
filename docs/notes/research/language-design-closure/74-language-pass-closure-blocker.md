# 74. The language-pass closure blocker

**Status: governs nothing.** This note records the governing contradiction found by prompt 170's repository-wide audit.
Prompt 170 may repair stale pointers in governing documents, but its Stop forbids changing a claim under
`docs/rules/across-stages/`; the decision therefore returns to the amendment procedure.

## The contradiction

[`docs/rules/across-stages/01-stage-judgments.md`](../../../rules/across-stages/01-stage-judgments.md) §2 says that an
omitted type parameter is solved by **first-order matching against the written arguments' types**. That is the
superseded calculus, not the language Musa now specifies or implements.

The higher governing rule in [`docs/rules/constitution.md`](../../../rules/constitution.md) §9 instead requires scoped
metavariables solved by pattern unification, with undecidable constraints postponed rather than guessed. The candidate
language specification gives the same rule in
[`docs/rules/language/02-core-calculus.md`](../../../rules/language/02-core-calculus.md) §2.1 and explicitly identifies
first-order, call-local matching as the mechanism it replaced.

The implementation follows the latter rule:

- `crates/musa-calculus/src/kernel/unify.rs` implements Miller's pattern fragment;
- `crates/musa-calculus/src/elaboration/convert.rs` records comparisons that are blocked on unsolved metavariables; and
- `crates/musa-calculus/src/elaboration/elab/metas.rs` retries that finite queue and refuses a survivor when the
  declaration closes.

This is a claim conflict, not a stale prompt-rank pointer. Replacing “first-order matching” with the current
metavariable, pattern-unification, and postponement rule changes the algorithm the governing stage judgment says Musa
uses. Prompt 170 therefore has no standing to make the edit.

## Why closure cannot continue around it

Prompt 170's Target is a contradiction audit that leaves every repair made and every blocker published. Its Design and
Stop say that a falsified claim in `across-stages/` ends the prompt and is handed back. Continuing to repair the book,
code map, clean-break ledger, and prompts 171–174 would produce a closure report while a higher governing account of the
language remained false. It would also make those downstream repairs choose silently between two governing algorithms.

Prompt 170 therefore remains `pending`. The two prompt-file repairs already committed for it remain valid; neither
depends on this decision.

## Decision required

The repository evidence supports amending `across-stages/01-stage-judgments.md` §2 to defer to the current bidirectional
judgment in the constitution and `language/02-core-calculus.md`: scoped metavariables, Miller-pattern solutions,
postponed blocked comparisons, and an error for anything still unsolved at declaration end. The alternative is to
restore first-order-only instantiation in the constitution, candidate specification, implementation, and the completed
prompt-153 work. That would reopen finished work and contradict the obligation matrix prompt 169 just closed.

Whichever choice is authorized must land as a governing-document amendment before prompt 170 resumes.

## Resolution

On 2026-08-24 the first alternative was authorized: preserve pattern-unification discipline and amend the stale stage
judgment rather than restore first-order-only instantiation. The amendment states contextual metavariables, unique
Miller-pattern assignments, finite postponement and retry within one declaration, refusal of survivors, and the separate
bounded deferral of checking-only arguments.

The implementation audit for the amendment also exposed a narrower defect the contradiction audit had not: the unifier
recognized arbitrary distinct-variable spines but its solution quotation inverted only the identity spine. That is a
sound incompleteness, not permission to narrow the rule silently. The code map records it as owed, and prompt 170 does
not resume until the contextual read-back can preserve permutation and weakening without adding term substitution.
