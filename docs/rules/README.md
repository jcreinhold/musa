# The rules

**Status: governing.** Everything in this directory decides what Musa is. If code and a rule disagree, repair one of
them deliberately; never leave the contradiction in place.

## Precedence

Read this table from top to bottom. A document is bound by everything above it and binds everything below it.

| Page | What it decides |
| --- | --- |
| [`constitution.md`](constitution.md) | The commitments every part of Musa must preserve |
| [`obligations.md`](obligations.md) | The consequences of those commitments |
| [`across-stages/`](across-stages/README.md) | Values, judgments, identity, provenance, preparation, and execution across stage boundaries |
| [`events/`](events/README.md) | Finite event tracks over exact typed time |
| [`desktop/`](desktop/README.md) | The desktop interface and its performance contract |
| [`language/`](language/README.md) | The one total source language and its elaboration boundaries |
| [`style-guide.md`](style-guide.md) | Source spellings and naming that remain meaningful after parsing |

Adjacent stage specifications own their domains and defer to `across-stages/` where values cross between them. The
roadmap and implementation plan sit below all of these rules; the complete precedence ladder is in
[`../README.md`](../README.md#which-document-wins).

## The core decisions

[`constitution.md`](constitution.md) answers nine questions without prescribing Rust types or surface punctuation:

1. What may a user edit?
2. Must all music use one theory?
3. How is finite musical time represented?
4. What produces sound, and what does not?
5. How do score facts and machines meet?
6. How may notation, analysis, MIDI, and audio describe one project without becoming one model?
7. What does exact stored identity mean?
8. Do different payloads require different temporal structures?
9. How many source languages are there, and what may they not do?

Read the constitution for the answers, then [`obligations.md`](obligations.md) for what follows from them.

## Changing a decision

The constitution and obligations may change only in a revision that:

1. gives a concrete musical or engineering reason;
2. shows which current examples fail under the existing rule;
3. states the replacement in plain language;
4. updates every affected formal specification and code-map entry;
5. explains the migration of stored files and public APIs; and
6. records the rejected rule and the argument in [`../notes/research/`](../notes/research/README.md).

Lower-precedence rules use the same discipline at the scale of the decision they repair. Editorial changes may improve
wording or remove history without an amendment only when they preserve the decision exactly.

## Amendment record

Research notes preserve the full arguments; this index does not duplicate them. The current specification includes:

- the unified dependent-application correction
  ([note 83](../notes/research/language-design-closure/83-one-dependent-application.md));
- source ownership of declarable sound semantics
  ([note 79](../notes/research/language-design-closure/79-source-owns-the-sound-language.md));
- one dependent type theory with inductive families and pattern unification
  ([note 53](../notes/research/language-design-closure/53-one-theory.md));
- keyboard audition and finite reviewed MIDI capture in place of step entry
  ([note 88](../notes/research/88-keyboard-capture-not-step-entry.md));
- the dependent-core admission and its measured correction
  ([notes 42](../notes/research/language-design-closure/42-dependent-core-decision.md) and
  [50](../notes/research/language-design-closure/50-the-course-correction-audit.md)); and
- the event-track/process separation introduced by prompt 127a
  ([core-calculus decision records](../notes/research/core-calculus/README.md)).

Those notes explain how the rules arrived here. The files in this directory are the current answer.
