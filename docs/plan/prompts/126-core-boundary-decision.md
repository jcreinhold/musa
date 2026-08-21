---
id: 126
slug: core-boundary-decision
status: done
depends_on: [92, 108, 119, 125]
phase: 3
---

# What the Core Is a Calculus Of

## Task

Decide, in writing and with evidence, what Musa's core calculus is *of* — and do it before prompts 175–193 spend
nineteen prompts assuming an answer. Peyton Jones (1987) §3 states the criterion this prompt applies: the translation
into the core *is* the language's semantics, so a surface construct that translates into nothing has no semantics beyond
whatever its compiler pass happens to do. Take the census, name the candidates, test them against that criterion, and
record the decision together with the repairs, deletions, and new prompts it forces. This prompt produces a governing
document and a repaired prompt stack. It produces no code.

## Read

- Peyton Jones (1987) §§1.2–1.3 and 3 — the enriched-language-to-core translation, and why the core is the semantics.
  §§2.1–2.3 for what makes a calculus a calculus rather than a data structure with functions attached.
- `docs/rules/events/` — the event-track as ontology, and the elaboration story it fixes.
- `docs/rules/events/00-overview.md`, `04-timeline.md`, `05-normalization.md`, `06-surface-elaboration.md`,
  `10-term-calculus.md` — the laws that would have to be re-proved at any new payload.
- `docs/rules/language/02-core-calculus.md` and `docs/rules/language/00-semantics.md` — the objects the language claims
  to compute.
- `crates/musa-events/src/timeline.rs` and `src/occurrence.rs` — the payload type parameter as it actually stands.
- `crates/musa-compiler/src/elaborate.rs` — every existing surface-to-event track translation, which is the census's
  evidence.
- `crates/musa-dsp/src/spec.rs` and roadmap §13 — the studio graph, the largest surface with no calculus under it.
- Roadmap §2's layer table, which any answer must leave standing.
- OMT `007-other-aspects-of-notation.md` (dynamics, articulation, and marks as *notation*), `074-swing-rhythms.md` and
  `118-metrical-dissonance.md` (performed time that is not notated duration), and `114-core-principles-of-
  orchestration.md` with `115-subtle-color-changes.md` (instrument color as a compositional discourse of its own). Music
  theory keeps notation, performance, and orchestration as separate vocabularies; a core that fuses them is making a
  claim about music, not only about types.

## Design

### 1. The census comes first

Produce one table with a row for **every** construct the surface language accepts — statement, expression form,
declaration, and block — and three columns: the construct, the event-track term or `ScoreFact` variant it elaborates to,
and the elaboration site in `crates/musa-compiler/src/elaborate.rs`. A construct that elaborates into no event-track
term gets the word **none** and a note on what happens to it instead. Exhaustiveness is checkable, not a matter of care:
drive the row list from `crates/musa-syntax/src/keywords.rs`, the table prompt 84 made exhaustive by construction, and
fail the prompt if a keyword has no row.

Two claims made in conversation are premises the census must confirm or refute, not assume:

1. `Timeline<A>` and `Occurrence<A>` are payload-polymorphic, and outside `musa-events` are instantiated at `ScoreFact`
   and nothing else.
2. `GestureTimeline` — the exact instrument-independent control timeline prompt 177 is meant to build — is named in
   several design documents and in no line of code.

If either is false, say so and let the corrected fact drive the decision.

### 2. The candidates

Exactly three, plus one question that must be settled separately.

- **A. Score-only core (the status quo).** The event track is a calculus of notated occurrences. Performance,
  instruments, and sound are compiler pipelines with no calculus, and the census's `none` rows are permanent.
- **B. One event track, several payloads.** The same finite temporal calculus, instantiated at `ScoreFact`, at a
  gesture/control payload, and at whatever the instrument boundary requires. `timeline`/`sequence`/`overlay`,
  normalization, and the semantic hash are reused rather than reimplemented per layer.
- **C. One dependently typed core.** Payloads and the indices that constrain them — part, voice, meter, tuning,
  transposition — become types in a single dependently typed calculus checked by normalization by evaluation.

The separate question is **signals**. A DSP signal is coinductive: an unbounded stream consumed at a sample rate. The
event-track is inductive, finite, and total, and its totality is load-bearing for every law in `docs/rules/events/`.
Decide whether the sound layer enters the core at all, and decide it in those terms rather than by taste. If it stays
out, name the object that crosses the boundary — the prepared render plan — and state the law that relates a score term
to its rendering.

`~/Code/kan` may be read as a reference implementation for how much machinery candidate C costs in practice. It is an
unrelated project; no code, dependency, or vocabulary transfers from it, and the decision document must not cite it as
justification for anything.

### 3. The tests each candidate must pass

- **Translation totality.** Every census row is translated, or the candidate is rejected by naming the rows it cannot
  serve. This is §3's criterion applied literally.
- **Law survival.** For each event-track law in `docs/rules/events/05-normalization.md` and `10-term-calculus.md`: does
  it hold at the new payload unchanged, hold with a new proof, or fail? A candidate that needs a new proof must say
  which.
- **Layer separation.** Roadmap §2's table survives intact: written pitch ≠ MIDI number, notated duration ≠ performed
  duration, voice ≠ mixer track, part ≠ synthesizer, dynamic marking ≠ decibels. A candidate that collapses a row is
  rejected regardless of its elegance — OMT keeps these vocabularies apart, and so does the roadmap.
- **Cost in prompts.** For each of 127–193: unchanged, repaired (and how), deleted, or replaced. A candidate whose
  ledger is missing a prompt is not costed.
- **Cost to undo.** What reversing the decision costs after the block is built. Candidate C's answer here is the whole
  argument against it, and must be written down rather than gestured at.

### 4. The decision is allowed to rewrite the plan

This prompt's outcome will almost certainly require prompts that do not exist. Create them.
`docs/plan/prompts/README.md` fixes the anatomy; `scripts/renumber-prompts.py` performs the insertion as one transaction
— `make-room --at N` opens a rank and rewrites every reference to the ranks that move. Read its dry-run report before
applying it: the uncued ranges it takes for rank spans, and the prose it could not reach, are both printed, and both
need a human. Update `docs/plan/prompts/README.md`'s index table and block summaries in the same commit.

Repairing an existing prompt is prompt repair under `docs/plan/prompts/README.md` §5 and stays in this commit, because
the repair and the decision that forces it are one change.

## Target

- The core-boundary decision record (`docs/notes/research/61-core-boundary-decision-record.md`): the census, the three
  candidates, the five tests applied to each, the chosen answer, the signal question settled, and the ledger over
  prompts 127–193. It states its own precedence: it governs over `docs/rules/language/` where they differ, and sits
  under the event-track specification — the amendment is written into the governing documents themselves, in this
  commit, or it has not been made.
- Repairs to the `Design` and `depends_on` of every prompt in 127–193 the decision changes, and new prompt files for
  work the decision requires that no prompt covers, inserted with `scripts/renumber-prompts.py`.
- Updated `docs/plan/prompts/README.md` (index table, block summaries) and `AGENTS.md` (governing-document list, prompt
  count).
- A `docs/rules/constitution.md` §7 listing what the chosen answer takes off the table, so that a later prompt cannot
  quietly re-open it.

## Check

```sh
python3 scripts/renumber-prompts.py audit
mdwright fmt-check docs/rules/*.md docs/plan/prompts/*.md docs/rules/language/*.md AGENTS.md
cargo nextest run -p musa-events -p musa-compiler
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

The audit must report no duplicate ranks, no unused ranks, and no dependency on a prompt that does not exist. The census
must have a row for every spelling in `crates/musa-syntax/src/keywords.rs`; state in the document how that was verified
and by what command. The Rust checks must pass **unchanged** — a green tree is the evidence that this prompt decided
rather than implemented. Commit as `Decide what the core is a calculus of`.

## Stop

- No implementation. No new event-track payload in code, no new crate, no change to `crates/musa-events`'s public
  surface, no dependency added.
- No rejection by preference. A candidate is rejected by a census row it cannot translate, a law it breaks, or a roadmap
  §2 row it collapses — never by tone.
- No unconditional deferral of the signal question. Deferring is allowed; deferring without naming the measurement or
  event that reopens it is not.
- No graduation of `docs/rules/language/`, and no change to what prompt 193 audits. This prompt may change what the
  later prompts *do*, not whether they are checked.
- No renumbering by hand, and no renumbering of finished prompts to make a range look tidy.
