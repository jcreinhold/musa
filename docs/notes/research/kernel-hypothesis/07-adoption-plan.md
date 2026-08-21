# From hypothesis to evidence-backed implementation

The path from this directory to code. The organizing discovery is that **the prompt stack already contains most of the
experiments**, so the plan is mostly about *not* writing new prompts and instead reading the friction that the planned
ones produce.

---

## 1. The rule

> No atom becomes an implementation prompt until a **named consumer** demands it, a **failing test** encodes its
> falsifier, and its **cost** has been measured.

Gate 0 (`06-evidence-log.md`) is the argument for this rule: an hour of probing overturned two headline claims. The same
hour spent writing prompts would have produced work that had to be unwound.

Three things must be true before an atom is built, and "the design is elegant" is not one of them:

| Requirement | What it looks like | Why |
| --- | --- | --- |
| **Demand** | A consumer that is worse off without it, named by crate and function | §34's "multiple independent consumers," and the defense against a shallow module |
| **Falsifier** | A test that fails today and passes after | Distinguishes a real gap from an aesthetic preference |
| **Cost** | A measurement against `docs/rules/events/09-performance.md`'s budgets | A semantics that cannot be computed is not a semantics for this project |

---

## 2. The stack is already the experiment

Four pending prompts test four of the six atoms without a single line being added to the plan. They were written before
this directory existed, which makes them unbiased instruments.

| Prompt | What it builds | Which atom it gates | The reading to take |
| --- | --- | --- | --- |
| [116](../../../plan/prompts/116-explicit-theory-assertions.md) — Explicit Musical Assertions | checked claims over a passage, with provenance and teaching diagnostics | **Atom 6 / Amendment VI** | Its Task already says "nothing becomes a global style rule merely because it can be checked locally." If the assertion family needs invariance side-conditions to be stated by hand, Atom 6 has its demand. |
| [117](../../../plan/prompts/117-analysis-service.md) — Typed Observation Service | the evidence model | **Amendment VI** | Whether a finding can carry its derivation, or degenerates to a verdict |
| [118](../../../plan/prompts/118-tonal-analysis.md) — Evidence-Based Tonal and Cadential Analysis | key and cadence claims | **Atom 4 (layers), Amendment III** | Cadence detection needs metrical position. If it has to ask "which meter region am I in" and the answer is ambiguous under a hypermetrical reading, Atom 4 has its demand. |
| [119](../../../plan/prompts/119-voice-leading-and-counterpoint.md) — Voice-Leading and Counterpoint Profiles | SATB, species 1–5, jazz motion | **Atom 2 (succession)** | Its Read section already says to read "current voice identity." This is the decisive test. |

Prompt 119 is the one to watch. It is the consumer that most wants lines, and its own Design already encodes Amendment
VI almost word for word — profiles "state their style," rules are marked "definitional, hard within that exercise, or a
guideline" (the three tiers of `03-claims-and-styles.md`), and findings "never present one historical pedagogy as a
universal law of music." **Amendment VI needs no new prompt. It needs prompt 119 executed as written.**

### The kill criterion for Atom 2 — **fired; Atom 2 withdrawn (`06-evidence-log.md` Gate 2)**

Implement prompt 119 against the *existing* per-note `VoiceId`. Then answer, with the code in front of you:

- Did any profile need a line relation the tag could not give? Species counterpoint needs "the note before this one in
  the same line" — does `VoiceId` plus time ordering supply it cleanly, or did the implementation build a shadow line
  structure to get there?
- Did anything need a partial order (OMT `110`) or a splitting line?

**If prompt 119 lands cleanly on tags, Atom 2 is withdrawn** — to a note in `docs/rules/events/08-open-questions.md`
saying Q3 was tested by a real consumer and the working stance held. That is a good outcome, not a failure: it closes an
open question with evidence instead of leaving it open forever.

**If prompt 119 builds a shadow line structure**, that structure is the demand, the falsifier, and the design, all
three, discovered by a consumer rather than proposed by a document.

**It landed cleanly.** Prompt 119 shipped in commit `90db0b1` with seven profiles and twenty-four rules over a `Strand`
that is nothing but the lane's own `(PartId, VoiceId)` and its tones in time order. No relation was added, and neither a
partial order nor a splitting line appeared. Gate 2 records the reading in full; Q3 is closed in
`docs/rules/events/08-open-questions.md`; Track C below is closed with it.

---

## 3. The three tracks, ordered by evidence per unit of effort

### Track A — Atoms 5 and 6, no kernel change (start here)

Payload torsors, group actions, and orbit identification are **library and type-system work**. They do not touch
`musa-kernel`, they do not touch the denotation, and they were untouched by Gate 0.

1. **Audit the existing quotients.** Find every forgetful map in `musa-compiler`'s music domains — `NoteName → Pc12`,
   octave reduction, `PcSet12 → set class`, `Row12 → row class`. Count how many state their invariance, and how many
   state it in a comment. *Half a day. This is the demand measurement for Atom 6.*
2. **Write the torsor laws as property tests** over the existing `Pitch`/`Interval` types: `p + (q − p) = q`,
   `(p − q) + (q − r) = p − r`, no `Pitch + Pitch`. If they already hold, Atom 5's Tier-1 claims are already true and
   only need to be *stated*. *A day.*
3. **Prototype one quotient with an invariance obligation** — `Pc12` is the right one — and see whether the obligation
   is discharged by construction or lands on a caller. If it lands on a caller, the cheap quotient former is not enough
   and Q-F reopens. *A day.*

Only then consider a prompt. A likely shape: one prompt stating the torsor laws and the invariance discipline for the
existing types, inserted in the phase-3 block near 115–116, changing no kernel document.

### Track B — Atom 4, narrowed (second)

`06-evidence-log.md` leaves this the strongest structural claim, and `05-open-questions.md` Q-B already argues for the
narrow reading: **the notated meter is a layer, not a region; heard hypermeter is analysis.**

1. **Establish demand from prompt 118.** Cadence and phrase analysis need metrical position; watch whether the region
   model produces an ambiguity the code has to paper over.
2. **Write the explicit-polymeter falsifier as a fixture.** A short `.musa` example with two simultaneously notated
   meters, from OMT `098` §Polymeter. Does it compile? Does it engrave? If it cannot be written at all, that is the
   falsifier, in the repo, executable — and `examples/` is the right home for it, since the AGENTS.md convention is that
   examples are specifications rather than demos.
3. If it fails, the prompt is narrow: *meter is a layer*, changing `elaborate.rs:980`'s region model and nothing in
   `musa-kernel`. This does not require touching `docs/course-correction.md`.

### Track C — Atom 2 — **closed, not started**

The gate fired against it. Prompt 119 was the consumer that most wanted lines and it did not want a kernel relation, so
under §1's rule there is no demand and there is no track. What remains true from Gate 0 — partial ordering (OMT `110`)
and divisi are inexpressible, and the voice tag is a convention no kernel law states — is preserved in
`06-evidence-log.md` and costs nothing to leave preserved.

Reviving Track C requires a *named consumer* for one of those two, not a better argument for the atom. The two that
could plausibly supply it: a serial analysis that must hold a partially ordered row (OMT `110` §"Partially ordered
sets"), and a divisi engraving feature where one written line splits and rejoins. If either arrives, this section is the
starting point and Gate 2 is the thing it has to beat.

### Track D — Atom 3, parked

`docs/rules/events/11-realization.md` is governing and its reasons 1 and 2 stand. Revive only if a real user needs
**ossia** — notated, bounded, performer-chosen alternatives — and even then, the first design to try is a payload one
under the existing realization mechanism, not a kernel constructor. If a kernel constructor is ever proposed again, it
must answer prompt 66's reason 2 (does `let`-sharing share the decision?) *first*, because
`02-denotational-semantics.md` E4 answers it by accident and that is not an answer.

---

## 4. What changes in the governing documents, and when

Nothing yet. Concretely:

- **Tracks A and B change no governing document.** Atom 4's narrow form is a `musa-compiler` change; Atoms 5 and 6 are
  library and type discipline. This is why they go first — they are reversible.
- **Only Track C would touch `docs/course-correction.md` and `docs/rules/events/`**, and only after prompt 119 supplies
  evidence. A kernel-shape change is the one irreversible-feeling move here and it should be the last one made.
- **Renumbering.** Per the standing convention, new work becomes numbered prompts with the rest renumbered. Track A and
  B prompts belong in the phase-3 block (115–127). A Track C prompt belongs *after*
  [172](../../../plan/prompts/172-language-conformance.md), the whole-language conformance and graduation prompt,
  because changing the kernel denotation before the language is graduated against it would invalidate the graduation.
- **This directory's status.** It stays research. If Tracks A and B land, `00-constitution.md`'s Amendments III, V, and
  VI get folded into `docs/course-correction.md` as amendments to it, and Amendments II and IV are either withdrawn or
  carried forward with the evidence log attached.

---

## 4a. What the implementation literature changed (added after reading Peyton Jones 1987)

Reading *The Implementation of Functional Programming Languages* produced four items. None of them changes an atom;
three landed as edits elsewhere and are recorded here so the plan stays the single index of outstanding work.

1. **The two calculi are staged, and now say so.** `docs/rules/language/02-core-calculus.md` §6.1. The classical surface
   → enriched → ordinary arrangement (§3.1 of the book) is *not* musa's: the core calculus has functions and the kernel
   has none, so no simplifying transformation connects them and the relationship is evaluation applied twice. This
   retires the "two cores no document names" problem noted earlier in this project without changing any code. **Done.**
2. **Full laziness is a real, measured-later gap.** `docs/rules/events/10-term-calculus.md` §"Provenance of the sharing
   discipline" and prompt 127's Design. Musa shares whole motif bodies keyed on arguments; Chapter 15 shares a body's
   argument-independent subexpressions across *different* arguments. Chapter 23's warnings against the technique are
   about lazy evaluation and do not transfer to a strict total calculus, so musa is unusually free to take the win — but
   the rule of §1 applies and prompt 127 measures before anything is built. **Delegated to prompt 127.**
3. **Analyses should be abstract interpretations.** Prompt 117's Design now requires each analysis kind to declare an
   abstract domain, an abstraction map, and a soundness claim. This is a better frame than the one
   `03-claims-and-styles.md` proposed for Tier 2: "evidence-carrying finding" says what a finding *contains*, while an
   abstraction map says what it *means*, and only the second makes the fact/candidate/conflict classification precise.
   **Amendment VI is improved by this and should be restated in its terms if it is ever promoted.**
4. **Flat patterns are a boundary, not an omission.** `docs/rules/language/02-core-calculus.md` §6.2, checked by prompt
   172. **Done.**

The general lesson is the one Gate 0 taught in a different form: musa has been re-deriving a literature it does not
cite, and mostly getting it right. Citing it is cheap and makes the refusals arguable. Prompt 172 now audits that every
"deliberately absent" item carries either a citation or a musical falsifier — the same standard the music-theory claims
have always been held to.

## 5. The optional formal track

The metatheory here is small, finite, and unusually formalizable: Theorem R, T1–T7, the torsor laws, and the
hereditary-conflict argument are all first-order facts about finite structures. Formalizing them in Lean would make
"theoretically backed" literal rather than rhetorical, and the tooling is already available in this environment.

**It is not on the critical path**, and it should not start before Track A, for the reason Gate 0 demonstrated: it is
much cheaper to refute a claim with a one-hour probe than to discover during formalization that the claim was about the
wrong system. The right moment is after an atom has demand and a falsifier — formalize what is being built, not what is
being considered.

The highest-value target if it does start: **the torsor and invariance laws of Track A**, because they are the ones that
will be relied on by library code that other code depends on, and because a mistake there is silent.

---

## 6. Immediate next actions

In order, smallest first:

0. Nothing from §4a. Items 1 and 4 are done; items 2 and 3 are prompt repairs already applied and will be discharged by
   prompts 127 and 117 when the stack reaches them.
1. Run Track A step 1 — the quotient audit. It is half a day and it either produces Atom 6's demand or removes it.
2. Write the explicit-polymeter fixture (Track B step 2). It is an hour and it is a permanent regression test either
   way.
3. ~~Resume the prompt stack at [114](../../../plan/prompts/114-angle-bracketed-type-parameters.md) and run through 119
   as written, reading 116–119 as the gates in §2 rather than as ordinary features.~~ **Done**; 114–119 are all `done`.
4. ~~Return here after 119 with the evidence, and decide Atom 2 then.~~ **Done**; see Gate 2. Atom 2 is withdrawn and
   Track C is closed, which leaves Track A (items 1 and 2 above) as the whole of the outstanding work here.

Nothing in this list is speculative work, and nothing in it is unwound if the hypothesis is wrong.
