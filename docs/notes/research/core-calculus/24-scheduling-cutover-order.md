# 24. Scheduling must precede frames

**Status: governs nothing.** This note records a dependency error found while executing prompt 172. The governing
scheduling judgment in `docs/rules/across-stages/03-machine-calculus.md` is unchanged.

## The contradiction in the prompt

Prompt 172 originally required both the new checked scheduler and migration of every current frame-scheduling caller,
while its Stop forbade instrument mapping and prompt 173 separately owned the one-frame instrument runtime. The current
production handoff cannot satisfy both requirements:

- `musa-score::PerformanceEvent` already carries an integer frame, so adapting it into `schedule` would make a frame
  assignment the source boundary for another frame assignment;
- its note payload contains interpreted `f32` and `f64` values, whereas scheduling consumes finite canonically encoded
  payload data already admitted under the language's generated `Storable` constraint; and
- its consumer is the old `RenderPlan`, whose block-defined instrument behavior is exactly what prompt 173 replaces.

An adapter would therefore create the second frame schedule prompt 172 forbids, introduce a Rust-only float encoding in
place of language admission, and perform part of prompt 173's instrument conversion without its one-frame laws.

## Repaired dependency

Prompt 172 implements and proves the generic checked operation over an already-admitted `EventTrack<C,A>`. It creates no
compatibility path from `PerformancePlan`. Prompt 173, after registering the instrument primitives that can consume
`EventBatch<A>`, cuts production preparation directly from exact gesture tracks through that operation. Prompt 174's
existing legacy-path audit then deletes `PerformancePlan`, `PerformanceEvent`, and every second frame schedule.

This does not weaken scheduling. Exact finite time mapping, explicit rounding and collapse, canonical occurrence order,
opaque handles, hygienic merge, bounded preparation and stepping, exact-once emission, overlay preservation under an
occurrence-local policy, and the conditional additive succession law all remain prompt 172 obligations. The repair only
prevents an invalid compatibility bridge from being mistaken for their first client.
