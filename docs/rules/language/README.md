# Musa language candidate

**Status: candidate. Not yet governing.** These documents are the implementation contract for prompts 93–192. Until
prompt 193 completes its conformance audit, everything above this directory in
[the precedence ladder](../../README.md#which-document-wins) takes precedence: `docs/rules/`,
`docs/rules/across-stages/`, `docs/rules/events/`, and the relevant settled parts of `docs/plan/roadmap.md`. A
contradiction is a prompt defect to repair, not permission to implement whichever text is convenient.

Two consequences of the governing core boundary are worth restating here, because both are easy to misread locally.

First, an **event track** is a container of occurrences of *any* storable payload over exact rational time, in a stated
coordinate. So the gesture track of `00-semantics.md` §1 and `08-performance-and-sound.md` is
`EventTrack[PerformedTime, Gesture]` — the same structure at a different payload — not a structure of its own.

Second, the **machine** is not outside this language. Prompt 127a made the sound side a second core value built by the
same total source language, and made `schedule` the named, checked operation that connects a track to it
(`../constitution.md` §4–§5). What stays outside is the audio *history*, because a history is coinductive and no source
value is.

This candidate therefore specifies one language that builds both core values, and the pipelines after each. It does not
add a seventh track operation and does not make sound a track concern. The source remains canonical; every UI edits or
projects source rather than owning another mutable score, instrument, or mix model.

Third, the constitution's amendment at prompt 128 replaced the *type discipline* this candidate was written against, and
the course correction then replaced the replacement. Prompt 128's evidence — the staff adapter — stood; what prompts
129–142 built around it was the standard proof-assistant checklist, and the audit of every committed program found none
of that machinery in use. Prompt 143 then corrected the correction: there is **one** type theory and the core is it — Π,
inductive families with indices, case trees, pattern unification, and a predicative hierarchy — with traits removed
rather than narrowed and subtyping refused in every form. `02-core-calculus.md` and `11-quotation.md` describe what
survived; there is no trait document, and prompt 146 deletes the mechanism one would have described. Both records stand:
[`../../notes/research/language-design-closure/42-dependent-core-decision.md`](../../notes/research/language-design-closure/42-dependent-core-decision.md)
and the correction's
[`../../notes/research/language-design-closure/50-the-course-correction-audit.md`](../../notes/research/language-design-closure/50-the-course-correction-audit.md).

Prompt 132 then trialled the language on ten complete programs before any code implemented it, and corrected the
specifications where a program contradicted them:
[`../../notes/research/language-design-closure/43-dependent-language-trial.md`](../../notes/research/language-design-closure/43-dependent-language-trial.md)
§13 lists each correction with the program that forced it. The trial's deepest finding is the one the course correction
took seriously: no program unifies an index, and none ever needed the identity type the index machine existed to
support. Its surviving corrections stand — `11-quotation.md`'s `Cat` has two cases and a checked-parse introduction
form, and `01-surface.md`'s type grammar has a multi-parameter function type.

## Document map

| File | Contract |
| --- | --- |
| `00-semantics.md` | representations, staging judgments, ownership, closure, equality, provenance |
| `01-surface.md` | settled surface grammar, desugarings, and acceptance corpus |
| `02-core-calculus.md` | the one total source language, and the obligations it owes |
| `03-musical-domains.md` | typed theory domains, definitions, sources, and counterexamples |
| `04-templates-and-modules.md` | packages, the module tree, and what the declaration templates became |
| `05-verification.md` | invariants, assertions, analyses, laws, and implementation gates |
| `06-elaboration-baseline.md` | the pre-migration performance and compatibility baseline this candidate is measured against |
| `07-analysis.md` | the analysis boundary, findings and evidence, and the admission rule for a new kind |
| `08-performance-and-sound.md` | score-to-gesture-to-instrument-to-audio semantics |
| `09-assets-and-packages.md` | reproducible assets, packages, sample maps, clips, and fixed media |
| `11-quotation.md` | `Syntax : Cat -> Type`, quoting and splicing, derived identity, and quotation as a pattern |
| `citations.md` | every theoretical claim in these documents, and the chapter or proof it comes from |

The numbering deliberately leaves room for future notation and analysis documents without renumbering the sound and
project contracts.

These documents **decide**; [`../../book/`](../../book/src/introduction.md) **teaches**. The teaching pages quote
fixtures rather than inventing syntax and generate every standard-library signature from the compiler's own record, and
`scripts/check-docs.sh` holds them to it. Where the two disagree, the specification is right and the book has a bug. The
implementor's path — grammar to events, laws, ownership, and extension recipes — is
[`../../plan/code-map/implementor-reference.md`](../../plan/code-map/implementor-reference.md).

## Graduation

Prompt 193 may mark this specification governing only after all of the following hold:

1. prompts 93–192 have discharged the proof, compatibility, performance, diagnostics, editor, and audio obligations
   named here — including the language pass at 128–170, whose obligations replace rather than extend the ones
   `02-core-calculus.md` §5 carried before it;
2. every pre-candidate example either retains its meaning or has an explicit, tested migration diagnostic;
3. the core law suite passes and no surface convenience has entered `musa-events` or `musa-calculus`. It does *not* pass
   unchanged: the course correction's final phases re-derive the metatheory matrix against the surviving calculus, and
   prompt 142 remains the one prompt permitted to have moved the compatibility oracle. Both are audited by the entries
   they leave behind, not by the suite being untouched;
4. live and offline rendering agree, part routing is isolated, and builds are reproducible from the project closure;
5. the roadmap, governance decisions, events and across-stage documents, style guide, implementation, and prompt stack
   pass a final contradiction audit.

The reasoning that produced this candidate — the design essay it was split out of, and the corrections applied to it
through prompt 113 — is in `docs/notes/research/` as a decision record. It is history, not an alternative specification.
