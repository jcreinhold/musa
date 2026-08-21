---
id: 169
slug: core-conformance
status: pending
depends_on: [168]
phase: 3
---

> **Reinstated and repaired by notes [`51`](../../notes/research/language-design-closure/51-the-terseness-audit.md) and
> [`52`](../../notes/research/language-design-closure/52-the-musical-algebra.md).** Conformance is stated against the
> surviving calculus — the small core of note 50 plus the index stratum of 142d — and not against the dependent core
> prompt 129 specified. Rows of the old matrix whose mechanism is deleted are struck with the note that deleted them,
> not marked green.

# Discharge the Core's Obligation Matrix

## Task

Prompt 129 wrote an obligation matrix and assigned each row to a prompt. Prompts 133–168 built the mechanisms and proved
fragments of it as they went. This prompt closes it: every metatheoretic obligation of the dependent core discharged
against the implementation, the carried-forward obligations re-derived rather than assumed, and the privacy and
second-path audits the old core carried run against the new one. This prompt adds no feature.

## Read

- `docs/rules/language/02-core-calculus.md` §5's obligation matrix as prompt 129 wrote it and prompt 132 may have
  repaired it — the list, row by row, with the prompt each row was assigned to.
- Every law suite added by prompts 133–141, and the freeze from prompt 168. What is already proved is not proved again;
  what is *partly* proved is the interesting column, and the matrix has to say which is which rather than marking a row
  green because a related test passes.
- `docs/notes/research/language-design-closure/33-metatheory.md`, `34-proof-review.md`, `35-proof-repair.md`,
  `36-final-proof-review.md`, and `37-final-blocker.md` — the previous matrix, its reviews, and the blocker found at the
  last gate. The lesson from `37` is procedural: the obligation that fails is the one nobody wrote a program for.
- `docs/rules/language/02-core-calculus.md` §5.7 (track-construction safety) and §5.9 (the expansion phase, including
  law 11) — prompt 129 carried these forward as obligations to re-derive, and this is the prompt that owes the
  derivation over the dependent core.
- `docs/rules/events/12-payload-admission.md` and the `Storable` constraint from prompt 137 — the storable-data boundary
  is an event-track-facing claim, so its proof is about what can cross, not about what the constraint solver accepts.
- `docs/rules/across-stages/05-metatheory.md` — the across-stage claims that named principal types and now name
  something else. A claim there that this pass falsified is a governing-document repair, and whether to make it is stop
  condition 4, not a decision inside this prompt.
- Prompt [174](174-core-calculus-conformance.md) — the later audit of the machine and audio path. This prompt must not
  do 153's job, and 153 must not have to redo this one, so the boundary between them is stated explicitly here.

## Design

**One matrix, one row per obligation, three columns: the statement, the implementation that owns it, and the executable
evidence that runs.** A row whose evidence is a paragraph is not discharged. A row whose evidence is a test that would
pass even if the property were false is worse than an empty row, because it reads as coverage — so each row also says
what failure the evidence would actually catch.

**The rows.** NbE soundness and completeness; decidability of conversion; type preservation for elaborated terms;
canonicity for the closed storable-data types; strong normalization; strict positivity implying consistency; coverage
completeness; termination soundness — every accepted recursive definition denotes a total function; determinism of
evaluation and of elaboration; and the three-outcome budget law, including that exhaustion never becomes acceptance or
rejection.

**Two carried-forward obligations, re-derived.** §5.7's track-construction safety and §5.9's expansion-phase laws were
proved over rank-1 inference and a sealed-step recursor. Both statements survive; neither proof does. Re-derive them
over the dependent core, and where a proof gets *easier* — a `Storable` constraint is a more tractable object than a `d`
variable class — say so, because a simplification is evidence that the new foundation was the right one.

**The privacy audit and the second-path audit.** Does anything reachable from source now reveal a private
representation, a registry decision, a normal form the author was not meant to see, or a budget? Can two code paths
answer the same question — the elaborator's conversion and the re-checker's, the case-tree compiler and the recursor,
`musa-calculus`'s `Storable` and the event track's payload admission? A second path is not automatically wrong, but an
undocumented one always is.

**The boundary with prompt 174, stated.** This prompt owns the *language*: the core calculus, its elaboration, the
expansion phase, and the storable-data boundary. Prompt 174 owns the *cutover*: machine values, scheduling, one-frame
audio, and the removal of surviving old semantic paths on the sound side. The two matrices reference each other and do
not overlap; state which rows belong to which, so that a row does not end up unproved because each prompt assumed the
other had it.

**A failing row is the output.** If an obligation cannot be discharged, this prompt records the smallest program that
exhibits the failure and stops. A matrix with an honest gap is worth more than a matrix with a row marked green on the
strength of a related test.

## Target

- The conformance matrix, under `docs/notes/research/language-design-closure/`, one row per obligation with statement,
  owner, evidence, and the failure that evidence catches.
- The re-derived §5.7 and §5.9 proofs over the dependent core.
- The privacy audit and the second-path audit, recorded, with any second path either removed or documented as
  intentional with its reason.
- Any executable evidence the matrix found missing, added.
- `docs/rules/language/02-core-calculus.md` §5 updated to point at the discharged matrix, and repaired wherever the
  implementation proved a statement wrong.
- The stated boundary with prompt 174, written in both prompts.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
./scripts/check-syntax-adapter-conformance.sh
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Discharge the core's obligation matrix`.

## Stop

- No feature, no new syntax, no new builtin, no new diagnostic beyond what a missing piece of evidence requires.
- No amendment to `docs/rules/constitution.md`, `obligations.md`, or `docs/rules/across-stages/`. A falsified claim
  there is published as a blocker and handed back — that is stop condition 4.
- No machine, scheduling, or audio conformance. Prompt 174.
- No graduation of `docs/rules/language/`. Prompt 193, and only after everything it lists.
- No row marked discharged on the strength of a test that would pass if the property were false.
