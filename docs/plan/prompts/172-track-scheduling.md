---
id: 172
slug: track-scheduling
status: pending
depends_on: [171]
phase: 3
---

# Schedule Event Tracks into Running Sources

## Task

Implement the checked connection from a finite `EventTrack<C,A>` to a machine that emits finite event batches one audio
frame at a time. Make every time, rounding, collision, ordering, and identity decision explicit.

## Read

- The prompt-127a scheduling specification; research `05-selected-calculus.md` §7 and `06-proof-outline.md` §5.
- Prompt 149's trusted boundary, prompt 169's K1–K20 matrix, and note 67's adapter freeze. Scheduling consumes a
  rechecked `EventTrack<C, A>` produced by the dependent, bidirectionally elaborated language; it does not reinterpret
  source terms, contextual `Music`, phase syntax, or a rank-1 type.
- Current tempo/groove/fermata/polytempo realization, `PerformancePlan`, frame scheduling, event windows, ids, and seek.
- Existing exact rational and `Progress` support.

## Design

> **Cutover-order repair.** The pre-cutover `PerformancePlan` cannot be a caller of this operation. Its
> `PerformanceEvent` is already frame-tagged, contains approximate `f32`/`f64` interpretation, and is consumed by the
> old public graph plan. Wrapping those events in a `Schedule` would schedule a schedule, invent canonical encodings for
> floats, and leave two frame authorities. Converting the resulting `EventBatch<A>` to sound instead requires the
> registered instrument primitives and one-frame plan that prompt 173 owns. This prompt therefore installs and proves
> the checked scheduler without adapting `PerformancePlan`; prompt 173 cuts the production audio callers directly from
> exact admitted gesture tracks to this source, and prompt 174 deletes `PerformancePlan` and checks that no second frame
> schedule survives. Note 24 records why this is a dependency repair rather than a weakened scheduling judgment.

Implement

```text
schedule(format, policy, time_map, track)
    -> Result<Schedule<A>, ScheduleError>
```

for storable `A`. `TimeMap<C>` maps the finite queried boundary set to exact physical time. `SchedulePolicy` maps those
times to bounded frames and fixes collapse and same-frame order. Success requires nonnegative, representable,
nondecreasing assignments and end no earlier than start. Record exact source boundary, physical result, frame,
rounding/collision choice, and policy version.

The `Storable` premise is the generated language constraint already rechecked at the prompt-149 boundary. Scheduling may
require its canonical finite encoding, but must not substitute a second Rust-only notion of payload admission.

Create one opaque handle per occurrence. Instruments may compare handles only for equality. Merging two scheduled
sources injects left and right handles into disjoint sets before sorting. An occurrence-local policy gives equal timing
to exact duplicate occurrences and never shifts or drops one in response to a neighbor.

The scheduled source stores a finite table, cursor, and bounded countdown. After the final batch it reaches `Finished`
and emits empty batches without changing state. At frame `j`, a connected machine reads the batch before producing
output frame `j`; positive occurrences use `[start,end)`.

## Target

- Exact `TimeMap`, `SchedulePolicy`, decision/error records, event batches/handles, source machine, and batch merger.
- A scheduler facade ready for prompt 173's direct production cutover; no adapter from the already-frame-tagged
  `PerformancePlan` and no second frame-event container introduced here.
- Laws for exact-once boundaries, determinism, monotonicity, half-open spans, point/collapsed events, handle renaming,
  unequal tracks, occurrence-local `together`, additive `follow`, finished-state memory, seek, and adversarial bounds.
- Plain diagnostics that show written boundary, exact physical value, chosen frame, and policy.

## Check

```sh
cargo nextest run -p musa-events -p musa-compiler -p musa-dsp -p musa-project
cargo clippy --all-targets -p musa-events -p musa-compiler -p musa-dsp -p musa-project -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
```

Commit as `Schedule event tracks into frame machines`.

## Stop

- No hidden default policy, float musical time, unbounded counter, handle-derived randomness, neighboring-event rewrite
  inside an occurrence-local policy, instrument mapping, or audio mixing.
- No unconditional law for nonlinear succession.
