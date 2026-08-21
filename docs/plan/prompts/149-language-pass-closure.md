---
id: 149
slug: language-pass-closure
status: superseded
depends_on: [148]
phase: 3
---

> Superseded by the course correction: `docs/notes/research/language-design-closure/50-the-course-correction-audit.md`.
> The pass this prompt closed has been replaced; closure is the correction's final report.

# Close the Language Pass and Repair What It Left Behind

## Task

Twenty-two prompts changed what Musa is. Audit `docs/rules/`, `docs/plan/`, `docs/book/`, `docs/plan/code-map/`, and
`AGENTS.md` for anything that still describes the old language, repair the surviving prompts so they name the new one,
and leave the stack in a state where prompt 150 can start without rediscovering any of this. This prompt adds no
feature.

## Read

- Every commit of prompts 128–148 (`git log --oneline 127dcfb..HEAD`), and every note they wrote under
  `docs/notes/research/language-design-closure/`. The repairs this prompt makes are the ones those commits implied and
  did not have standing to make.
- All of `docs/rules/language/`, `docs/rules/events/`, `docs/rules/across-stages/`, `docs/rules/constitution.md`,
  `docs/rules/obligations.md`, and `docs/rules/style-guide.md` — read for contradictions with each other, not only with
  the code.
- `docs/plan/roadmap.md`, `docs/plan/clean-break-ledger.md`, `docs/plan/code-map/`, and
  `docs/plan/language-design-closure.md`.
- `docs/book/` in full. The book teaches, quotes fixtures, and generates signatures from the compiler's own record; a
  language change that leaves the book teaching the old one is the most user-visible drift this pass can produce.
- Prompts [150](150-machine-runtime.md), [151](151-track-scheduling.md), [152](152-one-frame-audio.md), and
  [153](153-core-calculus-conformance.md), and the sound block at 154 and beyond — the prompts that have to run next.
- `AGENTS.md`'s prompt count and its description of the 127 block, which this pass changed.

## Design

**Repair at reach, not speculatively.** 150–153 are next and are repaired here in full: their `depends_on`, their Read
sections, and any Design that assumed rank-1 inference, contextual `Music`, or the old phase API. The sound block at 154
and beyond is repaired *when reached*, under the prompt README's §6 procedure — pre-repairing eighteen prompts against a
language whose implementation is four prompts old is guesswork, and guesswork committed to the plan is worse than a gap,
because the next reader cannot tell which parts were checked.

**The contradiction audit is between documents, not only against code.** The most likely surviving contradictions are:
`docs/rules/across-stages/05-metatheory.md`'s principal-type claim; `docs/rules/events/`'s payload-admission wording
against the `Storable` constraint; `docs/rules/language/06-performance.md`'s baseline against prompt 144's re-measured
numbers; `docs/rules/desktop/`'s error-and-states voice against the diagnostics prompts 134–140 added; and the
style-guide rules against what prompt 143 moved out of the compiler. Check each explicitly rather than trusting a
link-checker to notice, because a document can be internally consistent, well-linked, and wrong.

**One contradiction is already found and is this prompt's to repair.** `docs/rules/language/01-surface.md` §2 introduces
its `figure()`/`subject` example with "a track value is an ordinary value, and `in scale` is lexical rather than
captured", and closes with "an absent scale makes `step` a type-context diagnostic, not an implicit C-major choice" —
but the example between them claims "the two uses differ under `≈facts` … saving `subject` does not freeze the scale",
which only *dynamic* capture produces. Under the two sentences that bracket it, `fn figure()`'s body has no enclosing
`in scale`, so its `step 1` is the diagnostic the closing line describes and the example does not compile, let alone
compile two ways. It is the deleted contextual-`Music` behaviour surviving inside the paragraph that deletes it; both
halves entered in one commit (`0f18eb7`, the event-track cutover), so neither is stale relative to the other and the
example is simply the wrong illustration of a correct claim. Prompt [`141k`](141k-notation-lowering.md) implemented §2's
two normative sentences and withdrew the example as a citation, which is as far as an implementation prompt's standing
reaches. Delete the example and its sentence, or replace it with one that illustrates lexical scoping — a scale-taking
`fn figure(under: Scale)` is the idiom the standard library already writes — and check `docs/book/src/guide/cookbook.md`
and `examples/scale-context.musa` for the same claim.

**A stale cross-reference is not an amendment.** `docs/rules/events/06-surface-elaboration.md` and
`docs/rules/across-stages/05-metatheory.md` name prompt ranks that this pass superseded; correcting a pointer to say
which prompt now owns the work changes no decision and is an ordinary repair. Changing what one of those documents
*claims* is an amendment. Keep the two apart in the diff, and if a single edit is both, it is an amendment.

**Where a governing document is genuinely falsified, stop.** This prompt may repair `docs/plan/`, `docs/book/`,
`docs/plan/code-map/`, and `AGENTS.md` freely, and may repair `docs/rules/language/` as its own candidate specification.
It may repair stale prompt-rank pointers anywhere. It may **not** amend a claim in `docs/rules/constitution.md`,
`obligations.md`, `events/`, `across-stages/`, or `desktop/`. If one of those is falsified, the finding is published and
the decision is handed back — the same rule prompts 147 and 148 worked under, and the reason prompt 128 exists as a
separate prompt at all.

**The ledger closes or says what is left.** `docs/plan/clean-break-ledger.md` names deletions owned by prompts that no
longer exist and by prompts that have not run. Mark each row discharged, reassigned to 150–153, or — if a row turned out
not to be a deletion after all — struck with its reason.

**Say what the pass cost and what it bought, once, in one place.** Prompt 145's measurement, prompt 146's asymmetry,
prompt 143's registry survey, and prompt 144's performance numbers are four separate records of the same question. A
short closing note that puts them together is what a reader in a year will actually find, and it is the honest place to
record anything that did not work out — a mechanism nobody used, a prediction that was wrong, a cost that was higher
than note 39 §11.2 estimated.

## Target

- Contradiction audit across `docs/rules/`, `docs/plan/`, `docs/book/`, `docs/plan/code-map/`, and `AGENTS.md`, with
  every repair made and every blocker published.
- Prompts 150–153 repaired: `depends_on` naming 149, Read and Design sections naming the language that exists.
- `docs/plan/clean-break-ledger.md` closed or reassigned, row by row.
- `AGENTS.md`: prompt count, the 127-block description, and the `docs/rules/language/` graduation prompt reference, all
  correct.
- `docs/book/` teaching the new language, with its fixtures and generated signatures current.
- The closing note under `docs/notes/research/language-design-closure/`, putting the four measurements together and
  recording what did not work.
- `docs/plan/code-map/` accurate for every crate the pass touched, including `musa-calculus`.

## Check

```sh
python3 scripts/renumber-prompts.py audit
cargo build --workspace
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
./scripts/check-syntax-adapter-conformance.sh
cd editors/tree-sitter-musa && tree-sitter test
pnpm -r check && pnpm -r test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
/Users/jcreinhold/.cargo/bin/mdwright check docs/rules docs/plan docs/book docs/notes
PATH=/Users/jcreinhold/.cargo/bin:$PATH make lint-ui
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Close the language pass`.

## Stop

- No feature, no syntax, no builtin, no new law.
- No amendment to `docs/rules/constitution.md`, `obligations.md`, `events/`, `across-stages/`, or `desktop/`. Publish
  the blocker and hand it back.
- No repair of prompts 154 and beyond. They are repaired at reach.
- No graduation of `docs/rules/language/`. Prompt 172.
- No new prompt. If the audit finds uncovered work, it says so with the evidence, and writing the prompt is the next
  decision rather than a step taken inside this one.
