# The rules

**Status: governing.** Everything in this directory decides what musa is. Where code and a page here disagree, either
the code is wrong or the page needs a deliberate repair — never silent drift.

The order below is the precedence order: a page is bound by everything above it and binds everything below it.

| Page | What it decides |
| --- | --- |
| [`constitution.md`](constitution.md) | The few decisions every part of musa must follow |
| [`obligations.md`](obligations.md) | The rules that fall out of those decisions |
| [`across-stages/`](across-stages/README.md) | The rules no single stage owns: what data exists, when it is valid, how one stage produces the next, what equality means |
| [`kernel/`](kernel/00-purpose.md) | The finite event-track core — exact tagged time, typed occurrences, `empty`/`event`/`follow`/`together`/`map_payloads`/`duration`, normalization, the backend contract |
| [`desktop/`](desktop/README.md) | The desktop interface: visual language, engraving quality, interaction, states, performance budgets |
| [`style-guide.md`](style-guide.md) | `.musa` naming and spelling. Its machine-checkable subset is the lint pass, which cites this file by section number in its diagnostics |
| [`language/`](language/README.md) | The one total source language that builds both core values. **Candidate**, not yet binding |

## The core decisions

`constitution.md` answers nine questions. They do not prescribe Rust types or source syntax:

1. What can a user edit?
2. Must all music use the same theory?
3. How does musa represent finite musical time?
4. What is the thing that produces sound, and what is *not* that thing?
5. How do those two meet?
6. How can notation, analysis, MIDI, and audio describe one project without being treated as the same thing?
7. What does it mean for two stored results to be equal?
8. Does each kind of musical event get its own structure, or do they share one?
9. How many source languages are there, and what may they not do?

Read [`constitution.md`](constitution.md) for the answers, then [`obligations.md`](obligations.md) for what follows.

## Changing a decision

`constitution.md` and `obligations.md` may change, but only in a change that does all of the following:

1. gives a concrete musical or engineering reason;
2. shows which current examples no longer work;
3. states the replacement rule in plain language;
4. updates the formal specification and the code map;
5. explains how stored files and public APIs will migrate; and
6. records the change in [`../notes/research/`](../notes/research/README.md) so the old argument stays visible.

The most recent such amendment is prompt 128's: §9's *Inferred* property is now *Checked bidirectionally*, its *Total*
property is now well-founded rather than structural, its refusal of dependent and refinement types is deleted, its
refusal of type-directed macros is narrowed to admit typed quotation, and obligations §10 gains a second admission route
for measured engineering evidence. The reason is not musical. `stdlib/src/adapters/staff.musa` is 2,404 lines of Musa to
read staff notation, and six argument builders, 27 hand-allocated role integers, an eight-field product destructured to
read one field, and a reading algorithm that runs backwards because a list cannot be constructed are what the language
cost it. Its six answers, its refused alternatives — closed overloading alone, quotation without a dependent core,
dropping totality, and call-by-push-value — and its answers to the argument that recommended against it are in
[`../notes/research/language-design-closure/42-dependent-core-decision.md`](../notes/research/language-design-closure/42-dependent-core-decision.md).
It is answerable to a measurement: prompt 145 rewrites that same file, and if the number does not move, the amendment
was wrong.

Before it, the vocabulary repair that followed prompt 127a: §3 and §8 now say a track has a **duration** where they said
*length*, because `length` was already carrying a second meaning — the number of elements in a list — and because an
occurrence's *position* is a different quantity with a different algebra. It changes no decision, only the words a
decision is stated in; its argument, its refused alternatives, and its answers to the six requirements above are in
[`../notes/research/core-calculus/18-vocabulary-amendment.md`](../notes/research/core-calculus/18-vocabulary-amendment.md).

Before it, prompt 127a replaced the account of a contextual `Music` value above a temporal kernel with a separate
process graph below it. Its reason, refuted alternatives, and proof outline are in
[`../notes/research/core-calculus/`](../notes/research/core-calculus/README.md); what it obliges later prompts to delete
rather than alias is [`../plan/clean-break-ledger.md`](../plan/clean-break-ledger.md).

The other pages here are amendable in the ordinary way — a prompt that repairs them, committed before the code changes.
