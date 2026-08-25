---
id: 169
slug: core-conformance
status: pending
depends_on: [168]
phase: 3
---

> **Reinstated by notes [`51`](../../notes/research/language-design-closure/51-the-terseness-audit.md) and
> [`52`](../../notes/research/language-design-closure/52-the-musical-algebra.md), then repointed by
> [`53`](../../notes/research/language-design-closure/53-one-theory.md).** Conformance is stated against the dependent
> core prompts 143–162 implemented. Rows of an old matrix whose mechanism was later replaced are struck with the note
> that replaced them, not marked green.

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
- `docs/notes/research/language-design-closure/33-metatheory.md` and the deleted historical `34-proof-review.md`,
  `35-proof-repair.md`, `36-final-proof-review.md`, and `37-final-blocker.md` at `d0f4a527^`
  (`git show d0f4a527^:<path>`) — the previous matrix, its reviews, and the blocker found at the last gate, consulted as
  historical evidence rather than restored against the notes-retention policy. The lesson from `37` is procedural: the
  obligation that fails is the one nobody wrote a program for.
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

**And the re-checker, which is a row rather than a method.** Prompt [158](158-recheck-the-whole-core.md) closed
`musa-calculus`'s kernel audit over the whole core and stated it as
[`recheck_program`](../../../crates/musa-calculus/src/lib.rs). Its row says: *elaboration produces only terms the kernel
accepts* — owned by `kernel/recheck.rs`, evidenced by `recheck_laws.rs`'s per-construct table, `generated_laws.rs`'s
oracle over programs nobody wrote, and the two corpus gates in `musa-compiler`'s `document::laws` that run the pass over
the standard library and every `examples/` fixture. The failure it catches is elaboration emitting an ill-typed,
uncovered, non-descending, or scope-corrupt term.

The row also carries this matrix's own honest limit, and putting it here rather than only in `TRUST.md` is deliberate:
**the re-checker is not evidence for any other row.** It shares `eval` and `quote` with the thing it checks, so it
cannot witness NbE soundness, strong normalization, or determinism — a bug in the evaluator would be re-derived
identically and agree with itself. Reading it as general evidence is exactly the "green because a related test passes"
failure this prompt's Read section names, and it is the easiest one to commit here because the pass is broad and its
name sounds like it covers everything.

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
not overlap; state which rows belong to which, so that 174 does not redo this prompt and a row does not end up unproved
because each prompt assumed the other had it.

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
