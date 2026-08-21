# Language design closure

**Status: notes 01–41 govern nothing; note 42 is an amendment and governs.** This directory carries out
[`docs/plan/language-design-closure.md`](../../../plan/language-design-closure.md). Its purpose is to settle Musa's
source language, not to invent another temporal kernel or a package cache.

The five musical cases support the small call-by-value design. The repository is private and pre-release, so the later
repair rejects partial calls of compiler-owned operations instead of adding foreign function values to the core. The
final review found two false claims in the proof for retained programs: partial calls do not handle intervening defaults
correctly, and the proof does not relate a function's captured defaults to the surrounding declaration values. See
[17-final-proof-review.md](17-final-proof-review.md) and [18-final-blocker.md](18-final-blocker.md). The design was not
promoted. The user then set a clearer goal: easy local reasoning, broad type inference, and a surface shaped by musical
work rather than compiler tradition. [19-inference-course-correction.md](19-inference-course-correction.md) starts the
inference-first target. [20-compiler-pipeline.md](20-compiler-pipeline.md) gives its private compiler forms.
[21-surface-syntax.md](21-surface-syntax.md) replaces the Rust-like surface target with an indentation-based functional
candidate. [22-syntax-extension.md](22-syntax-extension.md) adds bounded package syntax adapters, and
[23-values-not-types.md](23-values-not-types.md) keeps note values out of the type language.
[24-pipeline-and-syntax-review.md](24-pipeline-and-syntax-review.md) reviews those four notes and holds the
paper-program target open on four High findings. [25-surface-and-elaboration.md](25-surface-and-elaboration.md) records
the last syntax exploration. [26-language-design-decision.md](26-language-design-decision.md) now supersedes notes 19–25
as the single active candidate. None of these notes changes governing documents.

## The question

> What is the smallest total language in which musicians can define their own musical concepts and elaborate them into
> Musa's existing temporal and audio stages?

Here, **total** means that every accepted source expression finishes with a value or a stated error. It does not mean
that a live performance finishes. A finite program may describe a processor or interaction that runs for as long as
musicians keep playing.

## Starting point

The following results have survived proof review and are not reopened here without a counterexample.

- A finite timeline has an exact rational length and a finite multiset of typed occurrences.
- Sequence adds lengths. Overlay takes the greater length. Overlay neither requires equal lengths nor inserts rests.
- The source expression language is pure, deterministic, and terminating.
- A valid finite audio graph advances by deterministic steps. Its possible input and output histories may be unbounded.
- Representations are connected by typed conversion records with explicit losses. They are not forced into one common
  value.

## Limits

This inquiry uses five musical cases, three language candidates, and at most two proof reviews. It does not design a
registry, version solver, stable compiled interface, or compiled-value cache. Exact Git source packages remain planned,
but they do not decide the source calculus.

The Karnatak, Balinese gamelan, and bomba cases are pressure tests based on limited published sources. They are not
package specifications. A package using one of those names needs review by a practitioner or specialist before it can
ship.

## Reading order

1. [01-current-language-audit.md](01-current-language-audit.md) says what Musa has and where each feature belongs.
2. [02-five-musical-cases.md](02-five-musical-cases.md) tests the language against five different kinds of musical work.
3. [03-language-comparison.md](03-language-comparison.md) tests three small language designs.
4. [04-source-calculus.md](04-source-calculus.md) states the selected language.
5. [04a-formal-rules.md](04a-formal-rules.md) gives the complete typing, module, and evaluation rules.
6. [05-stage-semantics.md](05-stage-semantics.md) states the path from source to notation, performance, and sound.
7. [06-paper-programs.md](06-paper-programs.md) tests the rules with complete programs.
8. [07-proof-prototype.md](07-proof-prototype.md) attacks the proof and compares two proof routes.
9. [08-proof-outline.md](08-proof-outline.md) fixes the theorem statements and proof order.
10. [09-metatheory.md](09-metatheory.md) gives the full proof under explicit compiler and stage contracts.
11. [10-proof-review.md](10-proof-review.md) records the failed first independent review.
12. [11-proof-repair.md](11-proof-repair.md) records the one permitted repair.
13. [12-proof-review.md](12-proof-review.md) gives the second and final proof review.
14. [13-final-blocker.md](13-final-blocker.md) records why promotion stopped and what survived.
15. [14-breaking-change-repair.md](14-breaking-change-repair.md) reopens the work under the explicit pre-release
    compatibility decision.
16. [15-proof-review-breaking-repair.md](15-proof-review-breaking-repair.md) attacks that decision and finds a false
    migration theorem.
17. [16-retained-translation-repair.md](16-retained-translation-repair.md) defines the exact retained call shapes and
    repairs the open-term proof.
18. [17-final-proof-review.md](17-final-proof-review.md) gives the final permitted review and finds two false proof
    claims.
19. [18-final-blocker.md](18-final-blocker.md) records why promotion stopped and the simplest design to test if the work
    is reopened.
20. [19-inference-course-correction.md](19-inference-course-correction.md) replaces the annotation-heavy target with
    Rust-like syntax, Hindley–Milner inference, explicit closures, and complete calls.
21. [20-compiler-pipeline.md](20-compiler-pipeline.md) gives each private compiler form one job and shows where the
    common front end branches toward notation, analysis, performance, and sound.
22. [21-surface-syntax.md](21-surface-syntax.md) tests brace-based, S-expression, and indentation-based syntax, selects
    the indentation-based form for the next paper programs, and records the first, too-broad rejection of public macros.
23. [22-syntax-extension.md](22-syntax-extension.md) corrects that rejection and proposes one bounded, hygienic
    expansion system for package-owned notation and studio syntax.
24. [23-values-not-types.md](23-values-not-types.md) explains why pitch literals and transformations are ordinary values
    and functions rather than a reason to add dependent types.
25. [24-pipeline-and-syntax-review.md](24-pipeline-and-syntax-review.md) reviews notes 20–23 against *Open Music Theory*
    and Peyton Jones, and names what must be settled before the five paper programs are written.
26. [25-surface-and-elaboration.md](25-surface-and-elaboration.md) makes the canonical surface locally checkable for
    machine authorship, and moves the choice of notation to packages through a Lean-derived expansion and printing pair.
27. [26-language-design-decision.md](26-language-design-decision.md) consolidates the inferred source language, fixed
    compiler pipeline, bounded syntax adapters, standard staff syntax, and editing contract into one candidate.
28. [27-adapter-trials.md](27-adapter-trials.md) tests that boundary with complete staff and studio blocks, expansions,
    diagnostics, and source-preserving edits. Both pass without compiler privilege.
29. [28-five-programs.md](28-five-programs.md) rewrites the tonal, flexible-time, phrase-led, ensemble-tuning, and live
    cases in the active language. The five programs pass without a rejected feature or a required local annotation.
30. [29-source-and-expansion-spec.md](29-source-and-expansion-spec.md) defines the source types, principal inference,
    modules, finite folds, bounded expansion, editing, lowering, evaluation, and deterministic charges.
31. [30-stage-spec.md](30-stage-spec.md) defines `Music` as a finite closed recipe and states every boundary from source
    values through temporal terms, notation, gestures, prepared processes, and audio histories.
32. [31-proof-prototype.md](31-proof-prototype.md) attacks the narrowed rules and selects the direct termination proof.
33. [32-proof-outline.md](32-proof-outline.md) freezes the theorem statements, assumptions, and dependency order.
34. [33-metatheory.md](33-metatheory.md) proves principal inference, type safety, termination, bounded expansion,
    privacy, `Music` closure, and exact-anchor stage composition.
35. [34-proof-review.md](34-proof-review.md) rejects the first proof target with four exact High-severity
    counterexamples and five Medium-severity gaps.
36. [35-proof-repair.md](35-proof-repair.md) records the one permitted repair: explicit transformer builders, join
    erasure, expression charges, pre-close score-map rewriting, and complete pass coverage.
37. [36-final-proof-review.md](36-final-proof-review.md) rejects the repaired proof with four new High-severity
    counterexamples.
38. [37-final-blocker.md](37-final-blocker.md) stops the bounded effort, records what survived, and names the four
    definitions required before this work should reopen.
39. [38-abstraction-totality-and-substitution.md](38-abstraction-totality-and-substitution.md) reviews the staff
    adapter's shape as implementation evidence: what per-type eliminators, totality, and the environment evaluator each
    cost, why each was chosen, and how the alternatives work. It corrects four claims and decides nothing.
40. [39-totality-and-structural-abstraction.md](39-totality-and-structural-abstraction.md) recommends keeping the total,
    rank-1 source foundation, repairing everyday ergonomics, replacing the primitive syntax catamorphism with a total
    inherited-context recursor over sealed child steps, and deferring higher-kinded container abstraction and dependent
    types until real programs earn them. It corrects the fire-triangle premise, several claims in note 38, and its own
    first recursor design; its later qualification separates the association lemma delivered by sealing from the
    reducibility argument still owed for source termination.
41. [40-sealed-step-recursor-trial.md](40-sealed-step-recursor-trial.md) trials that recursor on five complete programs
    before any code implements it: staff, studio, an anchored edit, the degenerate leaves, and a hostile nested
    traversal. The interface passes and is frozen; the reducibility argument goes through under definition acyclicity,
    with no dynamic owner check, failure result, higher rank, or affine restriction. It finds the fold *incomplete*
    rather than merely awkward — a re-descending reader is unwritable under it — and corrects note 39 three times: the
    right-to-left accumulator survives, studio gains nothing so the derived bottom-up fold stays public, and the
    normalization proof needs acyclicity as a stated premise.
42. [41-staff-on-the-repaired-interface.md](41-staff-on-the-repaired-interface.md) rewrites the staff adapter against
    the five landed repairs and measures each one's contribution separately. The ergonomic repairs did the shrinking
    (127 code lines); the recursor added 61 and bought the one thing the fold could not express — a group inside `( … )`
    read the way the notation wants. It corrects the trial's line count (44 → 34 on the slice becomes 149 → 187 at full
    size, for three stated reasons), confirms that `Pending` is the notation's cost and not the interface's, refutes the
    prompt's own expectation that `C` would carry the meter, and records that a single pass costs 2.1× the constructed
    cells two passes did, at 7% of the budget.
43. [42-dependent-core-decision.md](42-dependent-core-decision.md) is a **decision, and it governs**: constitution §9's
    *Inferred* property becomes bidirectional elaboration, *Total* becomes well-founded rather than structural, the
    refusal of dependent and refinement types is deleted, the refusal of type-directed macros is narrowed to admit typed
    quotation, and obligations §10 gains a second admission route for measured engineering evidence. The evidence is
    `stdlib/src/adapters/staff.musa` at 2,404 lines and 93,252 bytes, measured construct by construct. It discharges
    `docs/rules/README.md`'s six requirements and note 39 §11.2's five items, declines CBPV and `partial`, and states
    that note 39 §8.1's first reason — no *musical* operation needs a value in a type — is not refuted but was answering
    a different question. Prompt 145's rewrite of the same file is the gate that decides whether it was right.

44. [43-dependent-language-trial.md](43-dependent-language-trial.md) trials the language prompts 129–131 specified on
    ten complete programs before any code implements it: the staff adapter's emitting section, its twenty call sites,
    its printer, its dispatch table, `Pending`, `document_read`'s `call7`, note 28's five programs, and the studio
    adapter's `validate`. No falsifier fires and eight things change. Quotation is a larger win than prompt 131 claimed
    — the fourteen phase operations become **seven**, not merely the role integers — but the dispatch table has *three*
    halves rather than two, and the third is fifteen notation keywords no type system can remove. `Syntax<Cat>` earns
    its keep only at the splice boundary and needs a third introduction form, a checked parse; `Cat` loses two unused
    cases. **K is dropped**, discharging `02-core-calculus.md` §1.4's nomination of this prompt: no program unifies an
    index, and Hedberg gives K as a theorem for every type the corpus declares. Three specified spellings turn out to be
    unwritable in the grammar that specifies them. It predicts prompt 145's rewrite lands at 2,050 ± 100 lines — a 14%
    reduction that **does not clear the bar**, argues the line count measures the file rather than the language, and
    proposes a ten-row table of counts in its place. It records one gap the pass does not cover: hidden constructors
    have no spelling, and no prompt from 133 to 149 adds one.

45. [44-audit-against-smalltt-and-peyton-jones.md](44-audit-against-smalltt-and-peyton-jones.md) audits the core built
    by prompts 133–136 against `~/Code/smalltt` and Peyton Jones chapters 3–6, and classifies every divergence as
    **backed** or **ad hoc**. Seven divergences; two backed, five ad hoc, none of them about soundness. The measured one
    is `match`: an arm's body is duplicated once per case-tree leaf and **re-elaborated** at each copy, so elaborated
    term size and elaboration time both grow **2.2× per matched column** — 14.9 MB and 45 ms at ten columns for eleven
    lines of source. That is Peyton Jones §5.4.1's `unwieldy` exactly, and notes 24 §H5 and 26 §2.4 had already found it
    and chosen local join points as the answer; prompt 135 silently took the option those notes rejected. The proposed
    replacement is note 26's join point in a calculus without labels — one `let`-bound function per arm, applied at each
    leaf, well-typed because a variable pattern's binder *is* the abstraction that makes the body uniform. The other
    four ad hoc findings are one decision seen four ways: musa-calculus has no top-level definition scope, so it cannot
    fold anything, so none of smalltt's speed techniques — glued evaluation, approximate conversion, the three quotation
    modes, approximate occurs checking — are available. Prompt 142 hands that core the standard library. Also: neutral
    spines are left-nested `Arc` chains with the head O(n) away, `rigid` has no structural arm for two lambdas or two
    records (which prompt 137's dictionaries make a hot path), and `convertible` is a second and maximally naive
    conversion path. Repairing §6.2's stated reason for declining the fat bar would be an amendment, and is left to the
    user.

46. [45-phase-registry-survey.md](45-phase-registry-survey.md) is prompt 138's survey of the phase registry once
    `Syntax` has a category and the kind and delimiter arguments have real types: all seventeen entries, each with its
    signature, what it still hides, and whether prompt 139, prompt 143, or nothing at all deletes it. Two entries lost a
    claim to the retyping — `syntax_group` no longer hides the delimiter set, and `checked_expression` no longer asks
    whether a group names a real one — which leaves thirteen builders hiding the same one fact, and is the measured form
    of `11-quotation.md` §5's argument for replacing all of them with one quote. It records where §5's fourteen-entry
    table now needs a count repaired, and one thing the prompt did not predict: a syntax value cannot say what category
    it has, because the category is a claim the checker erased, so the evaluator's `admits` check has to be told not to
    compare them.

47. [46-collections-and-the-vec-answer.md](46-collections-and-the-vec-answer.md) records prompt 141's answer on
    `Vec A n` and what the collection library found in the core it was written against. **`Vec A n` does not ship**:
    note 43 §7's mechanism table gives it no program, §10 records that the staff adapter's two fixed-arity things are
    enums whose cases are named, and §5.6 records that the corpus is non-dependent — so the ledger is empty on both
    sides of the prompt's own test. The index *mechanism* stays, with `Syntax<Cat>` as its user and `Vec` as the fixture
    the family, coverage, and termination laws are stated over; a program that computes an arity re-opens the type as an
    ordinary prompt. The note also records why the collection library is a fixture in `collection_laws.rs` rather than
    crate items — `musa-calculus` is a leaf calculus whose only pre-declared thing is `Storable` — and three defects
    writing it found: an accumulating recursion was silently miscompiled, because the hypothesis a split binds stood at
    the branch's own accumulator and the call's new one was dropped; the re-checker had no rule for a `let` in checking
    position; and a `match` whose goal was still a metavariable was an internal error rather than a program.

48. [47-diagnostics-about-another-document.md](47-diagnostics-about-another-document.md) records what prompt 141a's
    `Cause` is: a diagnostic about a document other than the one being compiled, carried whole rather than flattened to
    one string. The invisible cost of the flattening is the part worth keeping — the constraint had begun to shape the
    diagnostics themselves, with `quote_splice` carrying a comment instructing future authors to write distinctions into
    the *message* because a distinction in a note was one nobody would ever read. The note states the three properties
    that make a cause safe to carry (its spans are in its own document, it has no fixes, it holds no causes), records
    why embedding the module's text in the serde shape and re-reading it from disk were both rejected, and names the
    obvious next application: `Code::Import`'s `` `{path}` does not compile ``, which today folds one parse error into a
    note and tells the reader to run a second compilation. It is left alone because a library can import and a library's
    diagnostics are reported rather than handed back — two decisions that belong to a prompt whose Task is the import
    contract.

49. [49-simplifying-the-core-and-elaborator.md](49-simplifying-the-core-and-elaborator.md) is the removal inventory
    written on the directive that the core and elaborator had grown past what Musa needs. It is note 50's raw material.

50. [50-the-course-correction-audit.md](50-the-course-correction-audit.md) is the course correction itself: every
    mechanism of the core named against the committed Musa program that requires it, a seven-phase plan, and the record
    that superseded prompts 143–149. Its deletions of the identity type, universe polymorphism, postponed constraints,
    general measures, constraint-based traits, and the old `core.rs` checker stand. Phases 0, 1, and 3 landed; the
    compiler shed 32,482 lines.

51. [51-the-terseness-audit.md](51-the-terseness-audit.md) audits note 50 against the goal note 50 was serving. Note
    50's rule tested *smallest* and nothing tested *practical*, *ergonomic*, or *useful*, and the number the
    constitution named as the falsifier has not moved — the staff adapter is 2,515 lines against the 2,404 the amendment
    was granted on. Three decisions are named as missteps: deleting indices, whose cost the corpus is paying as 17
    builtins hardcoded to one modulus and a `fallback` parameter in a public signature; deleting the elaboration order
    that makes un-annotated lambdas work, where Idris2's `checkRtoL` is the bounded fix and Musa already holds both of
    its predicates; and refusing partial application, which forbids naming T₃. The replacement for indexed families is a
    stratified Dependent ML index domain where index equality is decided by arithmetic and never by unification — the
    decomplecting move deletion was not. §8 answers `docs/rules/README.md`'s six requirements.

52. [52-the-musical-algebra.md](52-the-musical-algebra.md) is the other half: the five structures a Musa author should
    be able to name — torsor, group action, orbit and stabilizer, quotient with a chosen section, and the free
    construction — each exhibited with the workaround the standard library writes instead. It records that a pitch-class
    set and a bell pattern are one object, so an indexed `Cyclic(n)` serves twelve-tone theory, 24-EDO, and West African
    rhythm at once; and that laws over a finite indexed carrier are decidable **by enumeration**, which is how Musa gets
    rigorously checked algebraic structure with no proof machinery and no identity type.

The proof gate failed for the design notes 19–41 pursued, and nothing in *those* notes moved to `docs/rules/` or into
implementation prompts. Note 42 is the exception and says why: it is an amendment taken under
[`docs/rules/README.md`](../../../rules/README.md)'s procedure, on engineering evidence those notes did not weigh, and
it is answerable to a measurement rather than to a proof.
