# 78. Hostile review of the runtime conformance claim

**Status: governs nothing.** This review fixes note 77, the governing runtime rules, the implementation, and the
executable selector as its review object. It applies statement hygiene, exact-object, counterexample, dependency,
formalization-fidelity, and cross-document lenses. Findings precede the verdict.

## Findings

### High

1. **The first freeze still contained a second frame scheduler.**
   - **Location:** former `musa-score::PerformancePlan`, `PerformanceEvent`, and `lower_performance`; MIDI and debug
     callers.
   - **Type:** wrong relation or object; cross-document inconsistency.
   - **Problem:** prompt 172's repair required exact gesture tracks to be scheduled once by the checked scheduler, but
     these callers consumed a separately frame-tagged value. Raising this value as evidence for R7–R13 would have proved
     properties of the wrong schedule.
   - **Why it matters:** two frame lattices can disagree on rounding, ordering, and identity while each local suite is
     green.
   - **Repair applied:** deleted the scheduled value and all its event/lane/instance types. MIDI now consumes
     `GesturePlan`, projects exact physical seconds to its edge-local ticks, and never assigns audio frames. Debug
     output reports exact performed spans. Compiler tests inspect exact positions and rationals; checked frame behavior
     remains in the scheduler suite.

2. **The first gesture identity encoding was not visibly injective.**
   - **Location:** `Gesture::canonical_key`, quotient version 1.
   - **Type:** proof gap in an owner assumption.
   - **Problem:** delimiter-separated display fields were asserted to be complete without an argument that payload text
     could not collide with delimiters.
   - **Why it matters:** canonical-key equality is admitted payload equality; a collision can merge distinct occurrences
     during normalization and invalidate R3, R7, and R9.
   - **Repair applied:** the quotient version is 2; every sum constructor and numeric coordinate now has an explicit,
     exhaustive encoding, arbitrary source strings are length-framed, and a hostile delimiter law pins the distinction.
     The encoding no longer depends on `Debug`. This remains an owner identity encoding rather than a general
     serialization claim.

### Medium

1. **The draft theorem silently promoted tests to proofs.**
   - **Location:** initial matrix wording.
   - **Type:** encoding / intended-theorem mismatch.
   - **Problem:** finite property tests and migration oracles do not prove universal metatheory or primitive owner
     contracts.
   - **Repair applied:** note 77 states a conditional theorem, lists source builtin, primitive, canonical-key, and
     future batching contracts, and calls every executable row a falsifier. Behavioral equality, culture adequacy,
     uniqueness, and deadlines are explicit non-claims.

2. **Executable source and handbook prose still taught deleted semantic names.**
   - **Location:** tree-sitter corpus types, the architecture chapter, `AGENTS.md`, compiler comments, and the code map.
   - **Type:** cross-document inconsistency.
   - **Problem:** several parser examples still wrote `Music`, while the public-facade lists still named
     `compile_graph`.
   - **Repair applied:** executable examples now spell `EventTrack<WrittenTime, ScoreFact>`; facade prose names
     `prepare_audio`; the conformance script rejects the deleted runtime identifiers in production/documentation scope.

### Low

1. **The complete-program phrase and audio-first witnesses are representation witnesses, not products.**
   - **Location:** R14 program table.
   - **Type:** exposition issue only after repair.
   - **Problem:** “transcription” and “microphone” could be misread as claims that Musa currently ships an inference
     algorithm or device input.
   - **Repair applied:** the table says precisely that the former is a finite candidate with stated loss and the latter
     is an explicit typed frame input. Product algorithms, I/O, and practitioner adequacy are disclaimed.

## Verdict

- **Decision:** Correct conditional on named inputs.
- **Basis:** The repaired theorem is an assembly of four structural arguments on the exact implemented objects:
  normalized finite event tracks, finite prepared machine trees, finite checked schedules, and repeated one-frame audio
  steps. Negative controls cover the load-bearing boundary cases: multiplicity, half-open boundaries, initialized
  feedback, stale current-step data, handle collision, nonlinear succession, host partitioning, implicit seeds, and RT
  allocation/logging. The legacy scan finds no production second evaluator, public graph compiler, or second frame
  schedule.
- **Limits:** K1–K20 and note 67 are imported, not re-proved. Primitive implementations and canonical payload owners
  remain trusted according to their declared contracts. Property tests are finite. No mechanized proof is claimed. No RT
  deadline, arbitrary machine behavioral-equality decision, transcription uniqueness, or culture-specific theory
  adequacy is established.

No fatal, high, or medium finding remains after the repairs above.

## Clean passes

- Statement and proof agree on conditional correctness rather than absolute compiler correctness.
- The exact-object check passes: audio evidence begins at exact gesture tracks, not at a legacy frame projection.
- The additive hypothesis is present exactly where succession uses it; no unconditional `follow` theorem remains.
- Structural equality and behavioral equality are kept distinct.
- The pattern-unification boundary is imported at K10–K11 and not widened by runtime code.
- Whole-machine batching remains conditional and is currently unused by the reference renderer.
- Culture adequacy and practitioner review remain outside the verdict.
