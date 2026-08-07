---
id: 10
slug: kernel-laws
status: done
depends_on: [09]
phase: 1
---

# Temporal Kernel Law Tests

## Task

Prove the algebra is the one specified: implement the complete property-test suite of `docs/kernel/04-algebraic-laws.md`
(course correction §30 Step 3) against `musa-kernel`, including the explicit non-laws. This is the mathematical
acceptance gate for the kernel; the "Status: candidate" banner does not come off until this and prompt 11 pass.

## Read

- `docs/kernel/04-algebraic-laws.md` — the normative law list with formal statements; each law names its property test
  there, and this prompt must make that cross-reference real.
- Course correction §§7–17 (the laws in prose), §10 (non-distributivity), §11 (synchronized interchange with its
  duration-equality preconditions), §30 Step 3 (the checklist).
- Prompt 09's `musa-kernel` public surface; the proptest conventions already used in `musa-language`/`musa-compiler`
  (module-level `arithmetic_side_effects` allowance with justification, small case counts).

## Design

- One proptest per law, named exactly as `04-algebraic-laws.md` names it. Generate small timelines (extents and spans
  with denominators up to ~16, payload `u8` or short strings — payloads are arbitrary) and compare **semantic** equality
  (normalized forms), not internal representation.
- Required properties (from §30 Step 3):
  - `sequence` associativity; `sequence` zero identity (`(0, ∅)` on both sides); duration additivity.
  - `overlay` associativity; `overlay` commutativity; fixed-duration identity `(d, ∅)`; **non-idempotence**:
    `overlay(M, M) ≠ M` whenever `M` has a non-empty occurrence multiset (multiplicity doubles).
  - Ambient extension: `extend(d, d) = id`; `extend(e, f) ∘ extend(d, e) = extend(d, f)`; overlay respects extension.
  - Restriction: identity at the full extent; nested composition `restrict_K ∘ restrict_J = restrict_K` for `K ⊆ J ⊆ I`;
    whole spans preserved (visible span crops, whole span never moves).
  - Payload map: identity and composition; preserves `sequence` and `overlay`.
  - Time scaling: `scale_1 = id`; `scale_r ∘ scale_s = scale_{rs}`; preserves `sequence` and `overlay`.
  - Synchronized interchange: when `duration(M) = duration(N)` and `duration(P) = duration(Q)`,
    `(M ⊕ N);(P ⊕ Q) = (M;P) ⊕ (N;Q)` — and a deliberate counterexample test showing the equation **fails** when the
    preconditions do not hold.
  - Non-distributivity (§10): construct explicit `M, N, P` with `M;(N ⊕ P) ≠ (M;N) ⊕ (M;P)` — a unit test, not a
    property, with the multiplicity argument in a comment.
- A law that cannot be expressed against prompt 09's surface means the surface is wrong: repair prompt 09's prompt file
  or the spec (whichever is incorrect), commit the repair, then implement.

## Target

- `crates/musa-kernel/tests/laws.rs`: the suite above.
- Any spec corrections discovered while encoding the laws, committed with the prompt-repair note in the message.

## Check

```sh
cargo nextest run -p musa-kernel
cargo clippy --all-targets -p musa-kernel -- -D warnings
cargo fmt --check
grep -c "#\[test\]" crates/musa-kernel/tests/laws.rs   # >= the law count in 04-algebraic-laws.md
```

Commit as `Prove the temporal-kernel laws`.

## Stop

- No new kernel operations discovered "while testing" — a gap is a spec repair, not a feature.
- No elaboration or compiler tests (prompt 11).
- No performance assertions; correctness only (small case counts are fine).
