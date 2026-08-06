---
name: rust-performance
description: 'Use for measurement-driven Rust performance work: throughput, allocations, memory footprint, cache locality, hot-path cloning, layout, hashing, pipeline latency, profiling, benches, regressions.'
---

# Optimizing Rust Performance

Measure a real workload, find the bottleneck, make the smallest matching change, and measure again.

## Read First

Read `references/workflow.md` for every task. Then load only the needed reference:

- `repo-surfaces.md` — choose or add measurement support.
- `hotspot-classes.md` — compiler-style hot paths.
- `intervention-patterns.md` — ownership, allocation, layout, hashing, indexing, caching, and branches.
- `benchmark-design.md` — benches and claims.
- `external-lessons.md` — broader Rust practice.

## Workflow

1. Name the representative workload.
2. Reuse the nearest credible bench or profile; add the smallest one if missing.
3. Determine whether time, allocation, memory, layout, or repeated work causes the cost.
4. Choose the lowest-risk fix that matches it.
5. Re-measure the changed path and a broader workload that could regress.
6. Report numbers, command, workload, and uncertainty.

## Rules

- Do not optimize from intuition or claim a pipeline win from a toy benchmark.
- State data shape, access and mutation pattern, and lifetime before changing a data structure.
- Check both time and memory when a change can trade one for the other.
- Prefer removing phase-local allocations, clones, and repeated work before instruction-level tuning.
- Measure before using SIMD, parallelism, custom allocators, or `#[inline(always)]`.
- Confirm microbench results with a broader workload when pipeline throughput may change.

## Review

Check that the benchmark represents the claimed win, the intervention fits the bottleneck, costs did not move to a different stage, lifetime ownership remains sound, and the new measurement can catch regressions.
