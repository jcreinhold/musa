---
id: 50
slug: windowed-observation
status: done
depends_on: [49]
phase: 3
---

# Deferred Observation — If, and Only If, It Is Measured to Be Needed

## Task

Decide, on measurement, whether to evaluate terms lazily through a window — `restrict I t` answered without
materializing the whole of `t` — and implement it if the measurement says so. The lever exists because prompt 46 proved
T5 (observation commutes with sharing) and prompt 49 made elaboration produce terms with real sharing in them.

**This prompt has two legitimate outcomes.** One is a lazy evaluator and a windowed projection. The other is a recorded
measurement showing the eager path is inside budget, the prompt closed as not needed, and the block finished. Do not
assume the first.

## Read

- `docs/interface/06-performance.md` — B2 (keystroke → re-engraved score visible, large case, ≤ 400 ms p95, previous
  engraving continuously visible) and §4: no speculative optimization; work like this "is considered only when B1 or B2
  is measured to fail on a real piece, and it becomes its own prompt with the measurement as its justification". This is
  that prompt; the measurement is its entry condition, not its conclusion.
- `docs/kernel/09-performance.md` — every row from prompts 38–47.
- `docs/kernel/10-term-calculus.md` T5, and prompt 47's `evaluate`.
- `apps/musa-desktop/ui/src/lib/score/Score.svelte` — the resident-page window the UI already maintains, and
  `docs/interface/02-engraving.md` §7 (page virtualization). If a window is going to be pushed down, this is where its
  bounds come from.
- PoSD ch. 20, and the rust-performance discipline: name the workload, reuse or add the smallest credible measurement,
  fix what the measurement points at, re-measure the changed path *and* a broader one that could regress.

## Design

### The gate, stated as a procedure

1. Measure B1 and B2 p95 on `tests/fixtures/large-score.musa` through the existing Playwright harness, and P1–P4
   through `cargo bench`. Twenty trials, current `main`.
2. **If B1 and B2 pass with ≥ 20% headroom**, stop. Write the numbers into `09-performance.md`, mark this prompt `done`
   with a one-paragraph statement that deferred observation was not built and what would reopen it (a fixture that fails
   the budget). Delete nothing, add nothing. That is a complete outcome.
3. **If either fails or is within 20%**, identify from the profile *which stage* costs: parse, elaborate, evaluate,
   project, engrave, or the IPC/worker round trip. Only if the answer is evaluate-or-project does this prompt's design
   apply. If it is engraving or the worker round trip, the finding belongs to prompt 22's surface, and this prompt
   records it and closes.

Step 3 is the step most likely to be skipped, and it is the one that matters: the budget can fail for reasons a lazy
evaluator cannot fix, and building one anyway would add a second evaluation path to a system whose cost is elsewhere.

### If the gate opens

```rust
/// Evaluate only what is observable through `window` (T5).
pub fn observe<A: Clone>(term: &Term<A>, window: Span) -> Timeline<A>;
```

with one law, tested before anything is built on it:

```text
observe(t, I)  ≡  evaluate(t).restrict(I)      -- same occurrences, same visible spans
```

so laziness is a cost decision that cannot change a meaning. Push `restrict` inward through `seq`, `over`, `shift`,
`scale`, and `let` — T1 and T5 are exactly the licences for those pushes — and note that the win comes from `seq`, where
a window inside one part of a long sequence lets every other part go unevaluated.

The consumer is the projection: `ScoreSnapshot` (closed at prompt 42, which is why this is now possible without
touching a single backend) gains a windowed constructor, and `musa-project` asks for the pages the UI has resident plus
its overscan. The snapshot's public interface does not change shape — a windowed snapshot answers the same questions
about the window it covers, and asking outside it is a bug the type should prevent. Decide how: a distinct type, or a
window on the snapshot that accessors respect. State the invariant either way — *a consumer must never silently receive
a partial answer to a whole-piece question.* That is the one way this feature can do real damage.

### Re-measure, including what could regress

Windowed evaluation makes a first paint cheaper and may make a *full* export slower (repeated windowed passes where one
eager pass would do). Measure both: B2 and P1–P4, plus a full MEI export of the large fixture. Report all of it,
including any regression accepted and why.

## Target

Outcome 2 (gate closed):

- `docs/kernel/09-performance.md`: the measurements and the decision.
- This prompt's frontmatter flipped to `done`, with the statement above in the body.

Outcome 1 (gate open):

- `crates/musa-kernel/src/term.rs`: `observe`, with the equivalence law in `tests/terms.rs`.
- `docs/kernel/10-term-calculus.md`: the equivalence stated as a theorem with its test named.
- `crates/musa-compiler/src/project.rs`: windowed projection and its partial-answer invariant.
- `crates/musa-project`, `apps/musa-desktop`: the window plumbed from the resident page set.
- `docs/kernel/09-performance.md`: before/after for B1, B2, P1–P4, and the full-export check.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo bench -p musa-compiler
cd apps/musa-desktop/ui && npm test          # B1/B2 assertions on both fixtures
cargo run -p musa-cli -- render tests/fixtures/large-score.musa --to mei -o /tmp/large.mei
```

Commit as `Add windowed observation` or `Record that windowed observation is not needed`, whichever the measurement
produced.

## Stop

- No lazy evaluation without the measurement. "It is obviously faster" is the thing this prompt exists to refuse.
- No incremental compilation, no Salsa, no dependency tracking across edits. A window is not a cache.
- No partial answers escaping to a caller that asked a whole-piece question — not for the outline, not for export, not
  for playback.
- No parallel evaluation. If the profile points there, it is a different prompt with a different set of risks.

## Outcome: the gate closed — no deferred observation was built

The measurement is in `docs/kernel/09-performance.md`, "Prompt 50 — the gate, measured, and closed". In short:

**B1 passes with 98% headroom (2 ms of 120 ms). B2 does not clear the 20% bar — 373 ms of 400 ms, 6.8% headroom — so
step 3 applied, and step 3 is decisive.** Splitting B2 into its stages shows 180 ms of debounce (fixed by design), 2
ms of round trip, and **191 ms of engraving**. The compiler is not in that number at all: the Playwright harness
stubs the shell, and the core's real share is measured separately at P1 large = 1.87 ms.

A perfect lazy evaluator would move B2 from 373 ms to 371 ms. Deferred observation is therefore **not built**, and
per this prompt's own step 3 the finding belongs to prompt 22's surface: B2's headroom is thin, and the way to widen
it is incremental or page-windowed engraving.

Nothing was deleted and nothing was added to the kernel. `Term::observe` does not exist; `restrict` is still
evaluated eagerly, which T5 says is the same answer.

### What was added

One thing, and it is a measurement rather than a feature: `perf.spec.ts` now prints each budget's measured p95 on
stdout, and splits B2 into round trip and engraving. Before this, a green run said "within budget" and nothing else,
so the headroom a decision like this turns on could only be read by breaking an assertion. `06-performance.md` §2
already asks for measurements rather than verdicts; this makes a passing run obey it too.

### What would reopen it

A fixture where elaboration is a material fraction of B1 — roughly 25 ms, which on the measured shape is about
20 000 occurrences, an order of magnitude past `large-score.musa`. The measurement reopens it, not an argument.
