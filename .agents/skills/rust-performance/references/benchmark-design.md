# Benchmark Design

Use a microbenchmark to isolate one operation, a scenario benchmark for a realistic sequence, and an end-to-end run for pipeline claims. Build fixtures outside the timed section unless fixture work is part of the claim.

- Benchmark representative sizes and distributions, including likely boundary cases.
- Warm up and use repeated samples; do not treat small variance as a win.
- Track allocations and memory when a time change may trade them off.
- Add a bench or hook when an important path has no repeatable measurement.
- Review the command, input, environment, comparison method, and broader regression guardrail.
