# Decision record: the course correction, and where each of its sections went

**Status: research. Governs nothing.** This records why `docs/course-correction.md` existed, what it decided, and which
governing document owns each of its thirty-six sections now that the memo has been deleted. The memo is not quoted here
beyond what is needed to make the audit checkable; the owners below hold the normative text.

## Why the memo existed

By late 2024 the compiler was acquiring a dedicated lowering case for every musical concept the surface language grew.
`retrograde`, `inversion`, `arpeggiation`, `harmonization`, and each new repetition form arrived as compiler structure
rather than as a program written in terms of something smaller. There was no lower algebra for any of it to be *made
of*, so every addition was permanent and every addition was load-bearing.

The memo named the failure — **domain-specific semantic accretion without a stable lower algebra** — and made one
correction: musical time is ambient, and the semantic core is a small finite calculus of typed occurrences over exact
rational time with `timeline`, `sequence`, and `overlay` as its structural basis. The surface language stays rich and
musician-facing and *elaborates into* that core. Neither picture is the other.

That correction held. The kernel graduated at prompt 12, the direct CST-to-score lowering was frozen and then retired at
prompt 41, and the operation set has taken **zero** new constructors since, across every falsification piece written
between prompts 57 and 75. The memo asked for evidence of semantic necessity before growth; the evidence has instead
been of sufficiency.

## Why it was dissolved

A correction memo is written in diff voice. It says "do not continue in that direction" and "the previous design was
drifting" — sentences that only mean something against the state of the tree on the day they were written. Two years of
prompts later the direction it warns against is gone, and the memo's thirty-six sections had been transcribed,
sharpened, and in one case *struck* by the specifications it created. Keeping it meant keeping a second, staler copy of
the kernel specification with governing force, which is how `docs/rules/kernel/08-open-questions.md`'s falsification
table came to be six prompts behind §33.

The correction is not withdrawn. It is now stated once, in the present tense, by the documents below.

## The audit

Every section, and its owner now. "Absorbed" means the owning document states the same rule normatively; "spent" means
the section was an instruction to a past implementation phase that has completed.

| § | Subject | Owner now |
| --- | --- | --- |
| Purpose | two pictures: rich surface, small kernel | `kernel/00-purpose.md` |
| 1 | surface concepts are not semantic primitives | `kernel/00-purpose.md` |
| 2 | time is ambient | `kernel/00-purpose.md`; `governance/01-constitution.md` §3 |
| 3 | the kernel semantic object | `kernel/03-denotational-semantics.md` |
| 4 | exact musical time | `kernel/03-denotational-semantics.md`; `governance/01-constitution.md` §3 |
| 5 | only three essential structural forms | `kernel/01-grammar.md`; `kernel/10-term-calculus.md` for the scope rule |
| 6 | `timeline` | `kernel/01-grammar.md`, `03-denotational-semantics.md` |
| 7 | `sequence` | `kernel/03-denotational-semantics.md`, `04-algebraic-laws.md` |
| 8 | `overlay` | `kernel/03-denotational-semantics.md`, `04-algebraic-laws.md` |
| 9 | ambient extension is not silence padding | **absorbed then struck.** Became `kernel/02-static-semantics.md` K3 and `03-denotational-semantics.md` D4; both were struck at prompt 37 and the operation removed. History, not a rule. |
| 10 | sequence and overlay are not a semiring | `kernel/04-algebraic-laws.md` (the non-laws) |
| 11 | synchronized interchange | `kernel/04-algebraic-laws.md` |
| 12 | payloads typed but musically opaque | `kernel/02-static-semantics.md`; `kernel/12-payload-admission.md`; `governance/01-constitution.md` §7 |
| 13 | payload mapping is functorial | `kernel/04-algebraic-laws.md` |
| 14 | time scaling is an external action | `kernel/04-algebraic-laws.md` |
| 15 | delay is derived | `kernel/04-algebraic-laws.md` |
| 16 | `Timeline` is not assumed to be a monad | `kernel/08-open-questions.md` |
| 17 | temporal locality and restriction | `kernel/03-denotational-semantics.md`, `04-algebraic-laws.md` |
| 18 | `Pattern` belongs above the finite kernel | `kernel/08-open-questions.md` Q1 |
| 19 | no eager expansion of loops and repetitions | `kernel/06-surface-elaboration.md`; `kernel/10-term-calculus.md` |
| 20 | provenance lives outside the semantic quotient | `kernel/00-purpose.md`; `kernel/05-normalization.md` |
| 21 | key, meter, harmony as typed interval payloads | `kernel/06-surface-elaboration.md` |
| 22 | tempo is a monotone map `Beat → Second` | `kernel/06-surface-elaboration.md`; `kernel/07-backend-contract.md` |
| 23 | audio is a separate semantic layer | `governance/01-constitution.md` §4 |
| 24 | kernel serialization grammar | `kernel/01-grammar.md` |
| 25 | canonical normal form | `kernel/05-normalization.md` |
| 26 | compiler architecture | `architecture/stage-pipeline.md` §1 — the CST/HIR drawing moved there in this cleanup; the deep-module rule is `AGENTS.md`'s standing convention |
| 27 | relationship to `ScoreSnapshot` | `kernel/07-backend-contract.md` |
| 28 | relationship to `NotationPlan` | `kernel/07-backend-contract.md` |
| 29 | immediate implementation changes | **spent** — see below |
| 30 | migration strategy | **spent** — see below |
| 31 | surface design left open, candidate exists | `docs/rules/language/README.md` |
| 32 | open questions, do not prematurely decide | `kernel/08-open-questions.md` |
| 33 | falsification tests | `kernel/08-open-questions.md` — the memo's table was *ahead* of the kernel's; the kernel's was replaced with it in this cleanup |
| 34 | the governing design rule | `kernel/00-purpose.md`, quoted verbatim |
| 35 | immediate directive | **spent** — its architecture diagram is `kernel/00-purpose.md`'s |
| 36 | the amendment: what the core is a calculus of | `governance/01-constitution.md` §7 and §4 |

## §29, §30, §35 — the spent sections

These three were addressed to a specific implementation phase and that phase is closed. They are recorded here rather
than in a specification because a specification says what is true, and these said what to do next in 2024.

**§29 asked the compiler to stop, preserve, and add.** *Stop*: no new dedicated lowering case for `retrograde`,
`inversion`, `arpeggiation`, `variation`, `harmonization`, or new repetition and motif forms, and do not grow the direct
CST-to-score lowering into the permanent semantic model. *Preserve*: lossless parsing, formatting, source spans, exact
rational time, provenance, the working `ScoreSnapshot`, the working `NotationPlan`, and notation backends that consume
only `NotationPlan`. *Add*: one deep temporal-kernel crate owning exact temporal domains, typed occurrences, sequence,
overlay, restriction, normalization, semantic equality, and the algebraic property tests — explicitly **not** split into
a microcrate per concept.

All three landed. The stop list is now `kernel/00-purpose.md`'s design rule applied; the preserve list is what prompts
08–12 protected while the kernel was built beside the old path; the add list is `musa-kernel`, still one crate.

**§30 was the six-step migration**: write the kernel specification, then the crate, then the law suite, then the
elaboration, then differential validation against the existing lowering, and only then switch. Prompts 08–12 are that
sequence, one prompt per step but for the law suite and the switch. It is worth recording that the migration was run in
this order deliberately: the differential validation at step 5 is the only reason the switch at step 6 could be made
without a semantic freeze, and the old lowering was kept alive as the differential oracle until prompt 41 retired it.

**§35 was a twelve-item directive** restating the same phase in imperative form — freeze semantic grammar growth, do not
rewrite working notation infrastructure, specify and implement the kernel, treat ambient time as foundational, use exact
rationals, make the three forms the initial basis, keep payload semantics out, keep provenance above the quotient,
elaborate existing syntax without a source redesign, validate differentially, add nothing speculative, and falsify with
real music before extending. Its closing architecture diagram — source → HIR → kernel → notation / analysis /
performance → audio — is the one now drawn in `kernel/00-purpose.md`.

## §36 — the residue of the amendment

§36 was the memo-side face of the core-boundary decision (prompt 126); the two documents mutually required each other,
and both are now dissolved into `governance/01-constitution.md` §7 and §4. `61-core-boundary-decision-record.md` holds
that decision's evidence and candidates. Two observations from §36 have no home there and are worth keeping:

- **The answer was already written.** The memo said the kernel is a calculus of *typed* occurrences and left the type
  parameter unexamined, because there was one payload and no reason to look. §12 already said payloads are typed and
  musically opaque; §13 already said payload mapping is functorial and not a temporal primitive. Those two sections were
  the answer. The code had agreed with them before anybody checked: `Term<A>`, `Timeline<A>`, and `Occurrence<A>` are
  generic in `A`, the whole payload contract is `Canonical::canonical_key`, and the law suite proves L1–L24 at payload
  `u8` rather than at `ScoreFact`. The amendment made explicit a generality the implementation had never given up.
- **One claim was withdrawn, not clarified.** The old statement that a canonical key is injective on stored values while
  deliberately dropping fields cannot be true of both halves at once, and hashing unframed display text was withdrawn
  with it. `kernel/12-payload-admission.md` carries the replacement: an owner, an explicit versioned quotient, and a
  complete framed semantic encoding.
