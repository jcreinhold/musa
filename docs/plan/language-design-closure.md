# Close the Musa language design

**Status: superseded; retained as the record of a failed bounded attempt.** The candidate in
[`26-language-design-decision.md`](../notes/research/language-design-closure/26-language-design-decision.md) stopped at
its proof gate. [The final blocker](../notes/research/language-design-closure/37-final-blocker.md) records why.

Later work asked the broader source-to-audio question and passed its final paper review. That work lives in
[`../notes/research/core-calculus/`](../notes/research/core-calculus/README.md). Prompts 127a–127i, including inserted
prompts 127da–127dd, now form the active clean-break implementation path. They amend the rules first, repair the old
adapter proof blockers, replace contextual `Music` with ordinary inferred values, make finite event tracks and finite
machine descriptions part of one total source language, give machines one-frame step semantics, connect tracks to
machines through a checked scheduler, and audit the result before later sound work.

The numbered sections below preserve the original plan. They no longer direct implementation.

## Historical course correction after the failed proof gate

The original Tasks 1–6 remain evidence about the musical domain and stage boundaries. Their source calculus and proof
are not the active candidate.

The replacement is bounded to seven steps:

1. Test the common syntax-adapter boundary with complete staff and studio examples.
2. Rewrite the five paper programs with the selected syntax and inferred local types.
3. Specify inference, modules, expansion, the evaluation core, `Music`, and every later stage boundary.
4. Prove expansion safety, principal types, type safety, termination, `Music` closure, and stage composition.
5. Allow one hostile proof review, one repair, and one final review.
6. Promote the design only if the final review has no fatal, High, or Medium issue.
7. Rewrite the architecture and implementation prompts, then run the full repository gates.

This restart drops the old compatibility theorem. Partial, named, and default calls receive migration diagnostics and
mechanical rewrites instead. It does not implement compiler changes. It also stops isolated syntax experiments: note 26
is the sole active candidate unless a concrete trial or proof refutes it.

## Purpose and limits

The question is:

> What is the smallest total language in which musicians can define their own musical concepts and elaborate them into
> Musa's existing temporal and audio stages?

This work will test five musical cases, compare at most three language designs, and allow at most two proof-review
rounds. It will not design persistent compiled identities, a package cache, a registry, or version solving. It will not
reopen the accepted temporal and process semantics without a concrete counterexample. It will not present a package
named after a musical culture as adequate without review by a qualified practitioner.

## 1. Fix the question and audit the current language

Create `docs/notes/research/language-design-closure/` and state the following starting decisions.

- Keep the finite temporal kernel: exact rational time, typed occurrences, sequence, and unequal-duration overlay.
- Keep running signals outside that kernel. Audio process graphs have step semantics, not musical extent.
- Treat the derivation diagram as the link between representations. Do not seek one value that is at once source, score,
  analysis, gesture, and sound.
- Assume one finite, resolved import graph per build and fresh nominal type identities within that build.
- Keep exact Git source packages in scope. Defer registries, version solving, persistent binary identity, and compiled
  artifact caches.
- Reject worlds, links, call-by-push-value, dependent types, and similar machinery unless concrete musical cases force
  them.

Audit every current language feature and compiler-owned musical type. Classify it as part of the general language, a
notation adapter, a theory-library type, an analysis-only type, or something to remove or defer.

## 2. Test five musical cases

Write one concrete, sourced case study for each of these:

1. Common-practice tonal music: keep keys, chord construction, scale degree, harmonic function, spelling, voicing, and
   analysis distinct.
2. Flexible time: test unmeasured music, swing, rubato, polymeter, fermatas, and gradual tempo change.
3. Phrase-led music: use a Karnatak-inspired example in which notation does not determine the performance gesture.
4. Ensemble-led tuning: use a Balinese gamelan-inspired example in which pitch and acoustic effect depend on the
   ensemble.
5. Interactive music: use a bomba-inspired example in which musicians follow a finite protocol during an unbounded live
   performance.

For each case, give a small example, the needed data and operations, its path through Musa's stages, what the current
language can and cannot express, what belongs in a library, and what each stage loses or adds. Include both a
notation-led path and a sound- or performance-led path.

Admit a core feature only when two materially different cases need it, or when a safety theorem or stage boundary
requires it. Treat the culture-specific cases as pressure tests, not definitions of those musical practices.

## 3. Compare three small language designs

Test the same examples against no more than three candidates:

1. a plain, total, call-by-value language;
2. a restricted refinement or dependent-index language; and
3. a staged or polarity-based language such as call-by-push-value.

The default candidate has `Unit`, `Bool`, `Nat`, `Ratio`, `Text`, products, functions, `Option`, `List`, `Result`,
finite non-recursive user data, exhaustive matching, structures, signatures, abstract data members, private
constructors, finite folds, an abstract `Music` type, quotation of closed temporal terms, and build-local nominal
identity. It has no general recursion.

Dependent types or call-by-push-value win only if they remove real side conditions from at least two case studies
without making checking, inference, erasure, or stage semantics harder to explain. Otherwise reject them. Decide the
owner of every current compiler-owned musical type.

## 4. Write the formal language and stage semantics

Write a reference specification for types, terms, values, patterns, declarations, modules, finite nominal data,
bidirectional checking, exhaustive matching, sealing, call-by-value evaluation, resource charges, `Text`, `Result`, and
build-local nominal equality.

Define `Music` as a total recipe that reads an explicit musical context and either returns a finite temporal term or a
stated error. The context must not be hidden global state.

Define these boundaries:

1. source to resolved and typed core;
2. core evaluation to values and contextual music;
3. context application to a closed temporal term;
4. temporal evaluation to a finite timeline;
5. timeline to notation and analysis;
6. musical intent to finite performance gestures;
7. gestures, bindings, seed, and options to a prepared process graph; and
8. process steps to an audio history.

For each boundary, state its input, output, errors, equality rule, and derivation record. State plainly that source and
temporal terms normalize, a prepared process graph advances by deterministic steps, and an unbounded audio run does not
normalize to a finite value.

## 5. Write complete paper programs

Write small programs, with no ellipses, for a tonal package, a phrase-led package with explicit transcription loss, an
ensemble-tuning package with both musical intent and an acoustic target, a complete score-plus-studio path, and a live
input path that separates a finite protocol from an unbounded run.

Give each program a typing derivation, an evaluation or stage trace, its exact language features, and a note saying what
belongs to the package rather than the core. Do not label a block as executable `.musa` until the compiler accepts it.

## 6. Prove the design

Prepare a rapid proof draft, a proof outline, and the final metatheory. Prove decidable name resolution and checking,
substitution, preservation, progress, deterministic evaluation, termination, constructor opacity, preservation of the
retained current fragment, finite and typed `Music` closure, and composition of typed stage passes. Audit each rejected
partial compiler call and non-prefix ordinary partial call in the repository and give it a concrete named-wrapper
rewrite. Define the retained fragment by its translation rules rather than claim that every current checked term has a
translation.

Try both a direct termination proof and a translation into the existing proved core. Keep the proof with fewer special
cases.

Freeze the proof and ask an independent proof-review subagent to attack it. Allow one repair and one second review.
Promotion requires a verdict of correct under the stated contracts with no fatal, high, or medium issue. If the second
review still finds such an issue, record the blocker and stop. Do not prove package-cache correctness.

The first attempt spent those two reviews and stopped. The later breaking-change repair is a new proof target; it does
not alter either old review. Its first review found a false migration theorem but accepted the repaired source rules.
One exact repair and one final independent review form the bounded gate for this new target.

## 7. Promote only a passing design

Write a plain decision record saying what the language contains, what it omits, what `Music` means, where musical
concepts live, how notation and audio remain distinct but connected, why the chosen language is smaller than the
rejected candidates, and what evidence would reopen the decision.

If either the domain test or proof gate fails, leave the work in research and record the blocker. If both pass, update
the governing language and cross-stage specifications. Amend the constitution or obligations only through their stated
procedure and only if the result conflicts with them.

Write governance for a reader with undergraduate knowledge of music, mathematics, and programming languages. Give an
example before a formal rule, define each term at first use, keep one question per section, and replace opaque phrases
with concrete claims.

## 8. Update architecture and implementation prompts

Update the code map and roadmap with the selected source language, private compiler forms, build-local nominal
identities, the full source-to-audio path, and the exact implemented, partial, and absent states.

Add implementation prompts, before the current studio and audio work, for:

1. `Text` and structural `Result`;
2. ranked finite data and exhaustive matching;
3. abstract data members and private constructors; and
4. migration of musical concepts to their chosen owners plus trial theory packages.

Make each depend on the preceding prompt. Gate studio gestures and language conformance on the fourth.

Repair the pinned-package prompt so package dependencies and module imports are separate, exact source bytes establish
equality, hashes only locate candidates, repeated imports of one resolved package share a build node, and no stable
compiled interface or persistent value cache is promised.

Finish with documentation checks, focused language/kernel/compiler/project suites, the full repository gates required by
`AGENTS.md`, and a contradiction audit across rules, plans, book material, and research. Commit the evidence and
candidates, proofs and review, governing documents, and prompt stack as separate stable milestones on `main`.
