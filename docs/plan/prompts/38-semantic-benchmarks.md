---
id: 38
slug: semantic-benchmarks
status: done
depends_on: [37]
phase: 3
---

# A Measured Baseline for the Semantic Pipeline

## Task

Before the migration of prompts 39–43 moves every temporal fact into one timeline, establish what the semantic pipeline
costs today on a real workload, so every later prompt in this block can prove it did not regress. Add a benchmark
harness, name the workloads, record the baseline numbers in a checked-in table, and wire the two budgets that
compilation sits behind (`docs/rules/desktop/06-frame-budgets.md` B1, B2) to a Rust-level measurement instead of only an
end-to-end one.

## Read

- `docs/rules/desktop/06-frame-budgets.md` — the ten budgets, the reference workloads (`examples/glass-mountain.musa`
  small, `tests/fixtures/large-score.musa` large), p95 over 20 trials, and §4: no speculative optimization; an
  incremental compiler is considered "only when B1 or B2 is measured to fail on a real piece, and it becomes its own
  prompt with the measurement as its justification". This prompt supplies the instrument that would justify it.
- Roadmap §17 (testing strategy and dependency lists), §15 (the dependency lists a new crate must come from).
- `crates/musa-project/tests/suite/large_score_generators.rs` (how the large fixture is produced today).
- PoSD ch. 20 "designing for performance": measure first, and prefer a design change that removes work over tuning that
  makes the same work faster.

## Design

### Repair the dependency list first

The workspace has no benchmark harness and roadmap §15 lists none, so the prompt begins with a spec repair, committed
before the code: add the chosen harness to §15's development-dependency list with one sentence of justification, next to
`insta` and `proptest`. Prefer **`divan`** — it needs no `cargo-criterion`, runs under a plain `cargo bench`, and
reports per-iteration allocation counts, which matter here because the migration's risk is allocation, not arithmetic.
If it is rejected for any reason, `criterion` is the fallback; record which and why.

### The workloads, named once

Benchmarks live in `crates/musa-compiler/benches/pipeline.rs` and measure exactly four things, on both the small and
large fixtures:

| id | What | Why it is the right thing to watch |
| --- | --- | --- |
| P1 | `compile` end to end | B1's server-side share: a keystroke's cost is a compile |
| P2 | elaboration only (parse excluded) | isolates what prompts 39–41 change |
| P3 | the snapshot projection (adapter) | the stage prompt 39 creates, and the one most likely to regress |
| P4 | canonical form of the whole piece | what prompt 43's semantic identity will pay on every edit |

P2/P3 need the pipeline's phases separable for measurement. Do **not** add public API for this: use `#[doc(hidden)]`
benchmark entry points or `#[cfg(feature = "bench")]`, and say in the module docs that these exist for measurement only.
An interface that grows because a benchmark wanted a seam is a benchmark leaking into a design.

Report allocations alongside time for P2–P4. The migration replaces one `Vec<ScoreEvent>` per voice with one
heterogeneous multiset per piece plus projections; if that costs, it will cost in allocation and in the `String` keys
`Canonical` produces, and the numbers should say so rather than a later prompt guessing.

### The baseline table

`docs/rules/events/09-pipeline-baseline.md` (new) records: the harness, the exact command, the machine class, and a
table of P1–P4 × {small, large} with median and p95. Every prompt from 39 on re-runs the same command and appends its
row, so the block carries its own regression history in one file. A prompt in this block is not done while its row is
missing.

The table is a record, not a gate — machines differ. The gate is the **relative** rule stated here: no prompt in this
block may regress P1 or P2 on the large fixture by more than 10% against the row before it without saying so in its
"Repairs made while implementing" section and justifying the trade.

### A large fixture the compiler owns

`tests/fixtures/large-score.musa` was built for the UI at prompt 22 and lives at the repo root. Leave it there and point
the benchmark at it; do not copy it into the crate. If it turns out not to exercise the phase-2 constructs the migration
touches (ties, slurs, tuplets, dynamics), extend it with a section that does, and re-record the goldens it feeds in the
same commit.

## Target

- `docs/plan/roadmap.md` §15: benchmark harness added to the development-dependency list (separate commit, first).
- `crates/musa-compiler/benches/pipeline.rs`: P1–P4.
- `crates/musa-compiler/Cargo.toml`: `[[bench]]`, dev-dependency.
- `docs/rules/events/09-pipeline-baseline.md`: harness, command, machine class, baseline table, the 10% rule.
- `tests/fixtures/large-score.musa`: extended only if it lacks phase-2 constructs.

## Check

```sh
cargo bench -p musa-compiler
cargo nextest run -p musa-compiler
cargo clippy --all-targets -p musa-compiler -- -D warnings
cargo fmt --check
test -s docs/rules/events/09-pipeline-baseline.md
```

Commit the roadmap repair as `Add a benchmark harness to the dependency list`, then the rest as
`Measure the semantic pipeline before the event track migration`.

## Stop

- **Change nothing that the benchmark measures.** If a benchmark reveals something slow, that is a finding for prompt 43
  or a new prompt, not work to do here. A baseline taken after an optimization is not a baseline.
- No new public API on `Compilation`, `ScoreSnapshot`, or the event track.
- No CI wiring, no performance dashboards, no historical tracking beyond the checked-in table.
- No UI-side (Playwright) budget work; B1/B2's end-to-end harness already exists and is prompt 22/26's.

## Repairs made while implementing

- Roadmap §15 had **no** development-dependency list to add the harness to — `insta` and `proptest` were named only in
  §17. The repair commit adds §15.10 (the three dev dependencies, with `divan`'s justification) and §17.7 (what a
  benchmark is for, and that it never widens a public interface), so the rule the prompt cites now exists to be cited.
- The prompt asks for median and p95; `divan` reports fastest, median, mean, and slowest, not p95. The table carries the
  **median**, and says so — p95 over 20 trials is the end-to-end harness's statistic in
  `docs/rules/desktop/06-frame-budgets.md`, and restating it here would have meant a second harness for one number.
- P2 needed parsing separable from elaboration, so `elaborate` was split into `elaborate` (parse, then) and
  `elaborate_parsed`. P3 and P4 needed the voice timelines the adapter consumes, which nothing keeps: `Lowering` gained
  a `timeline_sink: Option<Vec<_>>`, `None` on every production path. Both are internal; the public surface is unchanged
  and the only new `pub` is the `#[doc(hidden)]` `bench` module.
- The large fixture had none of the phase-2 constructs, so the generator gained a four-bar **coda** — slur, tie, 3:2
  tuplet, two dynamics, accents and staccatos — played by every line, each bar summing to 4/4 so the parts stay aligned.
  Its goldens (MEI, snapshot JSON) were re-recorded in the same commit, and a new test
  (`large_score_exercises_the_expressive_notation_path`) asserts the constructs by name so the fixture cannot quietly
  stop measuring them.
- The baseline reports two findings without acting on either (the prompt forbids acting): elaboration is ~85% of
  end-to-end compile on the large workload, and **canonical form costs more than producing the score does** — P4 exceeds
  P2 on an already-materialized timeline. That is prompt 43's `Canonical`-key question, now with a before-number.
