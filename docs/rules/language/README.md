# Musa language candidate

**Status: candidate. Not yet governing.** These documents are the implementation contract for prompts 93–171. Until
prompt 172 completes its conformance audit, everything above this directory in
[the precedence ladder](../../README.md#which-document-wins) takes precedence: `docs/rules/`,
`docs/rules/across-stages/`, `docs/rules/kernel/`, and the relevant settled parts of `docs/plan/roadmap.md`. A
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

## Document map

| File | Contract |
| --- | --- |
| `00-semantics.md` | representations, staging judgments, ownership, closure, equality, provenance |
| `01-surface.md` | settled surface grammar, desugarings, and acceptance corpus |
| `02-core-calculus.md` | the one total source language and its metatheoretic obligations |
| `03-musical-domains.md` | typed theory domains, definitions, sources, and counterexamples |
| `04-templates-and-modules.md` | declaration templates, stable identity, signatures, static functors |
| `05-verification.md` | invariants, assertions, analyses, laws, and implementation gates |
| `06-performance.md` | the pre-migration performance and compatibility baseline this candidate is measured against |
| `07-analysis.md` | the analysis boundary, findings and evidence, and the admission rule for a new kind |
| `08-performance-and-sound.md` | score-to-gesture-to-instrument-to-audio semantics |
| `09-assets-and-packages.md` | reproducible assets, packages, sample maps, clips, and fixed media |
| `citations.md` | every theoretical claim in these documents, and the chapter or proof it comes from |

The numbering deliberately leaves room for future notation and analysis documents without renumbering the sound and
project contracts.

These documents **decide**; [`../../book/`](../../book/src/introduction.md) **teaches**. The teaching pages quote
fixtures rather than inventing syntax and generate every standard-library signature from the compiler's own record, and
`scripts/check-docs.sh` holds them to it. Where the two disagree, the specification is right and the book has a bug. The
implementor's path — grammar to kernel, laws, ownership, and extension recipes — is
[`../../plan/code-map/implementor-reference.md`](../../plan/code-map/implementor-reference.md).

## Graduation

Prompt 172 may mark this specification governing only after all of the following hold:

1. prompts 93–171 have discharged the proof, compatibility, performance, diagnostics, editor, and audio obligations
   named here;
2. every pre-candidate example either retains its meaning or has an explicit, tested migration diagnostic;
3. the core law suite still passes unchanged and no surface convenience has entered `musa-kernel`;
4. live and offline rendering agree, part routing is isolated, and builds are reproducible from the project closure;
5. the roadmap, governance decisions, kernel and across-stage documents, style guide, implementation, and prompt stack
   pass a final contradiction audit.

The reasoning that produced this candidate — the design essay it was split out of, and the corrections applied to it
through prompt 113 — is in `docs/notes/research/` as a decision record. It is history, not an alternative specification.
