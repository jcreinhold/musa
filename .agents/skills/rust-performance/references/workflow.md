# Performance Workflow

1. Define a representative workload and its input sizes.
2. Record a baseline with the command, machine conditions, and relevant metrics.
3. Profile or instrument to locate time, allocation, memory, or repeated work.
4. Change the smallest layer that owns the cost.
5. Re-run the focused workload and a broader guardrail.
6. Stop when the target is met, the evidence is inconclusive, or another cost now dominates.

Reject claims based on a workload that omits the affected pipeline stage. Report uncertainty and regressions, not only the best number.
