# Musa language candidate

**Status: candidate. Not yet governing.** These documents are the implementation contract for prompts 93–143. Until
prompt 144 completes its compatibility audit, `docs/course-correction.md`, `docs/kernel/`, and the relevant settled
parts of `docs/initial-design-roadmap.md` take precedence. A contradiction is a prompt defect to repair, not permission
to implement whichever text is convenient.

**`docs/language-correction.md` governs over this directory.** It corrects four faults these documents accumulated
through prompt 107 — an unproved musical-domain extension, a decorative standard-library manifest, a module system that
cannot nest, and `use` spelled for two unrelated statements. Where it and a section here disagree, it wins, and the
section is repaired in the prompt that implements the correction. Prompts 108–113 are that work.

This candidate specifies the language *above* the temporal kernel and the sound pipeline *after* it. It does not add a
fourth kernel combinator or make sound a temporal-kernel concern. The source remains canonical; every UI edits or
projects source rather than owning another mutable score, instrument, or mix model.

## Document map

| File | Contract |
| --- | --- |
| `00-semantics.md` | representations, staging judgments, ownership, closure, equality, provenance |
| `01-surface.md` | settled surface grammar, desugarings, and acceptance corpus |
| `02-core-calculus.md` | total value calculus and its metatheoretic obligations |
| `03-musical-domains.md` | typed theory domains, definitions, sources, and counterexamples |
| `04-templates-and-modules.md` | declaration templates, stable identity, signatures, static functors |
| `05-verification.md` | invariants, assertions, analyses, laws, and implementation gates |
| `07-analysis.md` | the analysis boundary, findings and evidence, and the admission rule for a new kind |
| `08-performance-and-sound.md` | score-to-gesture-to-instrument-to-signal semantics |
| `09-assets-and-packages.md` | reproducible assets, packages, sample maps, clips, and fixed media |

The numbering deliberately leaves room for future notation and analysis documents without renumbering the sound and
project contracts.

## Graduation

Prompt 144 may mark this specification governing only after all of the following hold:

1. prompts 93–143 have discharged the proof, compatibility, performance, diagnostics, editor, and audio obligations
   named here;
2. every pre-candidate example either retains its meaning or has an explicit, tested migration diagnostic;
3. the kernel law suite still passes unchanged and no surface convenience has entered `musa-kernel`;
4. live and offline rendering agree, part routing is isolated, and builds are reproducible from the project closure;
5. the roadmap, course correction, kernel documents, style guide, implementation, and prompt stack pass a final
   contradiction audit.

The earlier `docs/elaboration-language.md` remains design history and rationale. It is not an alternative specification.
