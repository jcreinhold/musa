# Third design audit

**Status: findings found after the second proof review. This document does not set Musa's rules.**

## Purpose

The second review checked source functions and typed failure. This pass checked the exact boundary between a finite
event track and a machine that may run for a long time.

## Findings

### High

1. **The scheduled source could grow its counter forever.**
   - **Location**: selected calculus §7.1 and proof of Theorem 5.2.
   - **Problem**: the draft stored a natural-number frame counter and incremented it even after the last scheduled
     event. A mathematical natural number needs more bits as it grows. That contradicts the fixed-memory rule for an
     audio unit.
   - **Repair**: use a bounded frame count accepted by scheduling. Store a cursor and a bounded countdown to the next
     batch. After the final batch, enter a `Finished` state that emits an empty batch without changing.

### Medium

1. **The scheduling policy was missing from the function's input type.**
   - The prose relied on a policy for rounding, collisions, and message order, but the displayed `schedule` type did not
     accept one. The repair makes `SchedulePolicy` an explicit finite input.

2. **Two scheduled sources could reuse the same private handle.**
   - Scheduling two tracks separately may give both first events the same local handle. Plainly merging their batches
     would then make two events look like one to an instrument. The repair adds a batch-merging machine that first tags
     left and right handles into disjoint sets, then sorts the messages.

3. **The word “occurrence-local” still allowed copy-number tricks.**
   - A policy could move the second of two exact duplicate events because the sorting pass called it copy two. Then
     scheduling an overlay would not agree with scheduling each side. The repair requires exact duplicates to receive
     equal time decisions. Copy numbers may keep handles apart, but may not alter timing.

### Low

1. **“Side-by-side product” claimed too much.**
   - Stateful machines have a side-by-side operation, but it need not be a categorical product. The repair names the
     weaker and correct structure: a symmetric monoidal category, meaning chains and side-by-side wiring can be
     regrouped without changing behavior.

2. **The storable-data closure rule was implicit.**
   - The repair now says directly that a well-formed event track, primitive description, or machine description is
     storable data when its payloads, ports, and stored configuration are storable data.

## Verdict

- **Decision**: Incomplete before repair.
- **Basis**: the unbounded counter conflicts with the audio memory contract, and the missing policy and handle hygiene
  leave the main track-to-audio theorem under-specified.
- **Clean passes**: the finite scheduler algorithm, explicit time decisions, stored feedback, and one-frame audio
  meaning remain sound after these local repairs.

