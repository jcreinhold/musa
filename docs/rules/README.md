# The rules

**Status: governing.** Everything in this directory decides what musa is. Where code and a page here disagree, either
the code is wrong or the page needs a deliberate repair — never silent drift.

The order below is the precedence order: a page is bound by everything above it and binds everything below it.

| Page | What it decides |
| --- | --- |
| [`constitution.md`](constitution.md) | The few decisions every part of musa must follow |
| [`obligations.md`](obligations.md) | The rules that fall out of those decisions |
| [`across-stages/`](across-stages/README.md) | The rules no single stage owns: what data exists, when it is valid, how one stage produces the next, what equality means |
| [`events/`](events/README.md) | The finite event-track core — exact tagged time, typed occurrences, `empty`/`event`/`follow`/`together`/`map_payloads`/`duration`, normalization, the backend contract |
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

The most recent authorized repair is note 79's source/host ownership correction for the candidate sound language. It
does not change constitution §§4, 8, or 9; it makes the lower-precedence sound page and prompt cone obey them.

1. **The reason.** Prompts 174b–176 made a public Rust `StudioSpec`, surface catalogue, and `WrittenQuantity`
   authoritative after prompt 167 had already demonstrated equivalent ordinary Musa declarations. The uncommitted
   prompt-177 implementation repeated the problem with fixed Rust gesture/control enums. Two adjacent semantic
   vocabularies would drift and require compiler releases for declarable library policy.
2. **Which current examples no longer work.** No `.musa` example is intentionally rejected. Direct Rust construction of
   studio, quantity, gesture, control, or instrument semantics stops being a supported public path; those APIs are
   unreleased and migrate to checked source artifacts and read-only projections.
3. **The replacement rule in plain language.** Declarable data and total policy live in ordinary `.musa`, normally the
   standard library. Rust owns only provenance/direct track construction, registered primitive private contracts,
   verified asset bytes, scheduling/DSP conversion, and private prepared/runtime state. A Rust projection is exactly
   derived and never independent authority. Dependent sound relationships use the existing Miller-pattern unifier.
4. **The formal specification and code map.** `language/08-performance-and-sound.md` §0 states the boundary;
   `../plan/code-map/{implementor-reference,process-runtime,stage-pipeline,spec-to-implementation-map}.md` records the
   repaired implementation cone.
5. **How stored files and public APIs migrate.** Source stays canonical and stored values are recomputed from checked,
   versioned declarations. The unreleased Rust construction APIs are removed or narrowed to opaque projections with
   differential laws. Private primitive identities/state and decoded/runtime values do not move into source.
6. **The record.**
   [`../notes/research/language-design-closure/79-source-owns-the-sound-language.md`](../notes/research/language-design-closure/79-source-owns-the-sound-language.md)
   preserves the defect, literature, replacement, prompt cone, and user authorization.

Before it, prompt 170's amendment synchronized
[`across-stages/01-stage-judgments.md`](across-stages/01-stage-judgments.md) §2 with the one-theory decision prompt 143
already admitted. It changes no accepted source program, but it changes a governing algorithmic claim, so it answers the
six requirements explicitly.

1. **The reason.** The lower-precedence stage judgment still named the first-order, call-local matcher that prompt 153
   replaced. That contradicted constitution §9, the candidate core specification, and the constraint queue in the
   implementation, leaving two governing answers to the same elaboration question.
2. **Which current examples no longer work.** None. The amendment describes the mechanism already exercised by the
   examples and standard library. Its characteristic cases are an omitted family parameter determined under local
   binders and a checking-only argument whose type is determined by a later written argument; both already compile.
3. **The replacement rule in plain language.** Omitted terms are scoped metavariables. A metavariable is assigned only
   by a unique Miller-pattern solution; a comparison that cannot yet be decided waits in the current declaration and is
   retried after progress; a survivor is an error. The fixed, at-most-two-pass elaboration of checking-only written
   arguments is separate from that queue and preserves written evaluation order.
4. **The formal specification and the code map.** The stage judgment above now states the rule and delegates its exact
   calculus to [`language/02-core-calculus.md`](language/02-core-calculus.md) §2.1. The bidirectional-elaboration row in
   [`../plan/code-map/spec-to-implementation-map.md`](../plan/code-map/spec-to-implementation-map.md) records the scoped
   meta, unifier, queue, settlement, zonking, and bounded spine deferral separately.
5. **How stored files and public APIs migrate.** They do not. Metavariables are private elaboration state, are fully
   zonked before a checked term crosses the crate boundary, and change neither source syntax nor stored identity.
6. **The record.**
   [`../notes/research/language-design-closure/74-language-pass-closure-blocker.md`](../notes/research/language-design-closure/74-language-pass-closure-blocker.md)
   preserves the contradiction, the two alternatives, and the authorization to choose this one.

Before it, prompt 143's amendment commits the language to **one type theory** — a dependently typed core with inductive
families — and reverses the stratified index admitted immediately before it. It answers the six requirements here rather
than by reference.

1. **The reason.** It is an engineering one, and it is that the core is currently three partial mechanisms where one
   would do. The eliminator is **non-dependent**: `family/assemble.rs`'s motive answers a type rather than a family, so
   a `match` refines nothing about the value matched, and the core has dependent Π formation sitting over simply-typed
   elimination. The index stratum is a **second sort of type** with its own solver, admitted on a count that a family
   discharges without it, and its one unique capability — index arithmetic — is used by **zero** committed `.musa`
   files. And instantiation is **first-order**: §2.1 takes Idris2's `checkRtoL` without the fallback that makes it a
   unifier, so implicit arguments, index unification in `match`, and any metavariable that outlives one call are not
   unavailable by decision but unavailable by omission. Each of the three was admitted to avoid a dependent core, and
   each now approximates one badly. The record is
   [`../notes/research/language-design-closure/53-one-theory.md`](../notes/research/language-design-closure/53-one-theory.md).
2. **Which current examples no longer work.** None, in `examples/` or `stdlib/`: the theory is a superset of what the
   index stratum accepted, minus the arithmetic nothing uses. What stops working is *compiler* code, and deliberately —
   the index solver, the generated recursors, and the trait tables are deleted rather than ported, under
   [`../plan/clean-break-ledger.md`](../plan/clean-break-ledger.md).
3. **The replacement rule in plain language.** There is one theory and the core is it: Π types, inductive families with
   parameters and indices, case trees, metavariables solved by pattern unification, and a universe hierarchy. Two types
   are the same type when both read back to the same term, and nothing accepts beside that relation — no subtyping in
   any form, which is the one refusal this amendment *adds*. What stays refused is the proof assistant's apparatus, not
   its typing power: no tactics, no proof search, no hint database, no opt-out from totality.
4. **The formal specification and the code map.** [`language/02-core-calculus.md`](language/02-core-calculus.md) §§1–3
   are rewritten by prompt 144, which exists so that this decision can be reviewed as a decision rather than as a
   specification diff; §1.5's index stratum and §1.1's non-dependent eliminator are retired there, and §1.4's refusal of
   the identity type is repaired to say it refused the apparatus. The trait specification is retired outright by prompt
   145 and gets no successor; prompt 146 deletes the mechanism it described.
   [`../plan/code-map/spec-to-implementation-map.md`](../plan/code-map/spec-to-implementation-map.md) is rewritten
   against the new core as each prompt lands, not in advance of them.
5. **How stored files and public APIs migrate.** No stored format changes: indices were erased at quotation, so nothing
   an index said was ever in a compiled term, an event track, the `% musa-events-3` interchange format, or a pinned
   digest. The public API changes are the seventeen `Pc12`/`Row12` builtins collapsing at prompt 164 and the trait
   declarations disappearing at prompt 146 — both clean breaks under the ledger, and neither touches a file a user has
   written.
6. **The record.** Note 53 above. Notes 42, 50 and 51 stand unedited beside it: the admission, the deletion, and the
   audit that found the deletion had taken one step too many. This amendment is what note 51 asked for and one step
   further than it proposed, because the stratum it proposed is the third of the three partial mechanisms.

Before it, prompt 142c's, which admitted a **stratified index** to §9. It is reversed by 143 above; its own six answers
stand for the record.

1. **The reason.** It is an engineering one, and it is counted. `stdlib/src/post_tonal/` needs pitch classes and
   twelve-tone rows; with no way to index a type by a number, the modulus is baked into **seventeen of the compiler's
   121 `Builtin` variants** — `pc12_of` through `row12_missing` — and every further modulus the repertoire needs (24 for
   quarter-tone practice, 19 or 31 for meantone, 13 for Bohlen–Pierce) is another seventeen. `stdlib/src/`
   `transformational.musa` carries a `fallback: Triad` parameter in a public signature, invented at every call site,
   because the type cannot say that a major triad is a triad. And a bar's contents summing to its meter — a linear
   equation over exact rationals, and the most common error in written music — is checked at run time. The record is
   [`../notes/research/language-design-closure/51-the-terseness-audit.md`](../notes/research/language-design-closure/51-the-terseness-audit.md).
2. **Which current examples no longer work.** None. Indices are additive: every existing declaration is an index-free
   one, and `examples/` and `stdlib/` compile unchanged. The rewrites this enables are ones we choose, not ones the
   change forces.
3. **The replacement rule in plain language.** A type may carry index arguments drawn from a fixed decidable arithmetic
   domain — ℕ, exact ℚ, and finite literal enums, under variables, literals, `+`, `-`, `*` by a literal, and comparison.
   Indices are erased before evaluation, are never matched on, and two indexed types are the same type when the solver
   proves their indices equal. Nothing else about the calculus changes, and every piece of proof-assistant machinery §9
   refuses stays refused — no identity type, no universe levels, no measures, no dependent motive, no index unification,
   no proof terms.
4. **The formal specification and the code map.** [`language/02-core-calculus.md`](language/02-core-calculus.md) gains
   §1.5, *Index refinement*, with the domain, the erasure rule, the conversion rule, and the refusals; §1.3's "no
   indexed families" is repaired to say what is still refused, which is inductive-family indices and index unification,
   and not this. [`language/03-musical-domains.md`](language/03-musical-domains.md) gains the indexed domains.
   [`language/citations.md`](language/citations.md) gains Xi and Pfenning, whose Dependent ML this follows.
   [`../plan/code-map/spec-to-implementation-map.md`](../plan/code-map/spec-to-implementation-map.md) carries the row,
   marked absent until prompt 142d.
5. **How stored files and public APIs migrate.** No stored format changes at all: indices are erased at quotation, so a
   compiled term, an event track, the `% musa-events-3` interchange format, and every pinned digest are byte-identical
   before and after — 142d's load-bearing check. The public API change is the seventeen builtins collapsing at prompt
   164, which is a clean break under [`../plan/clean-break-ledger.md`](../plan/clean-break-ledger.md) and touches no
   file a user has written.
6. **The record.** Note 51 above, with note 50 standing unedited beside it as
   [`../notes/research/core-calculus/18-vocabulary-amendment.md`](../notes/research/core-calculus/18-vocabulary-amendment.md)
   established.
   [`../notes/research/language-design-closure/52-the-musical-algebra.md`](../notes/research/language-design-closure/52-the-musical-algebra.md)
   is its other half: what the index is *for*, and why a pitch-class set in ℤ/12 and a bell pattern in a 12-pulse cycle
   are the same object.

It inherits the measurement its predecessor is answerable to, and adds one. The staff adapter is rewritten on the
surviving language and must be dramatically shorter than 2,404 lines; and `stdlib/src/post_tonal/` is rewritten for
arbitrary n and must lose the seventeen builtins without getting longer. If either number does not move, the diagnosis
was wrong whatever the checker's own line counts say.

Before it, prompt 128's amendment, narrowed by the course correction. §9's *Inferred* property is now *Checked
bidirectionally*; its *Total* property stands, enforced structurally; its refusal of dependent and refinement types is
narrowed to the lightweight dependency programs use — a result type may mention an earlier explicit argument — with the
proof-assistant machinery the amendment had admitted (indexed families, an identity type, universe levels,
constraint-solving traits, well-founded measures) deleted again after an audit found no committed program behind any of
it; its refusal of type-directed macros is narrowed to admit typed quotation; and obligations §10's second admission
route for measured engineering evidence stands. The reason is not musical. `stdlib/src/adapters/staff.musa` is 2,404
lines of Musa to read staff notation, and six argument builders, 27 hand-allocated role integers, an eight-field product
destructured to read one field, and a reading algorithm that runs backwards because a list cannot be constructed are
what the language cost it. Both records stand:
[`../notes/research/language-design-closure/42-dependent-core-decision.md`](../notes/research/language-design-closure/42-dependent-core-decision.md)
for the admission and
[`../notes/research/language-design-closure/50-the-course-correction-audit.md`](../notes/research/language-design-closure/50-the-course-correction-audit.md)
for the narrowing. It is answerable to a measurement: the staff adapter is rewritten on the surviving language, and if
the number does not move, the whole pass was wrong.

Before it, the vocabulary repair that followed prompt 127a: §3 and §8 now say a track has a **duration** where they said
*length*, because `length` was already carrying a second meaning — the number of elements in a list — and because an
occurrence's *position* is a different quantity with a different algebra. It changes no decision, only the words a
decision is stated in; its argument, its refused alternatives, and its answers to the six requirements above are in
[`../notes/research/core-calculus/18-vocabulary-amendment.md`](../notes/research/core-calculus/18-vocabulary-amendment.md).

Before it, prompt 127a replaced the account of a contextual `Music` value above an event-track with a separate process
graph below it. Its reason, refuted alternatives, and proof outline are in
[`../notes/research/core-calculus/`](../notes/research/core-calculus/README.md); what it obliges later prompts to delete
rather than alias is [`../plan/clean-break-ledger.md`](../plan/clean-break-ledger.md).

The other pages here are amendable in the ordinary way — a prompt that repairs them, committed before the code changes.
