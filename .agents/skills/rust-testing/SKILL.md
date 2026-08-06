---
name: rust-testing
description: 'Use for Rust correctness tests: regressions, property tests, integration, compile-fail, minimal repros, law-based tests, brittle or shallow suites, and wrong-layer scope.'
---

# Test Engineering

Test the contract at the narrowest layer that still covers the risk. Use `rust-performance` for timing, allocation, or scaling guards.

## Orient First

| Pressure | Test |
| --- | --- |
| False confidence | A serious bug class can still escape. |
| Shallow coverage | Examples replace an invariant. |
| Brittle assertion | A harmless refactor breaks it. |
| Missing oracle | Nothing can distinguish right from wrong. |
| Wrong layer | The test is too local or too broad. |
| Missing regression | A fixed bug lacks its smallest reproducer. |
| Slow suite | Routine runs skip it. |

If unclear, run `bash .agents/skills/rust-testing/scripts/audit-test-surface.sh <path>` from the repo root.

## Rules

1. State the contract or invariant first.
2. Test at the highest layer that still isolates the risk.
3. Add the smallest regression before a bug fix.
4. Prefer laws, roundtrips, preservation, and reference models where available.
5. Cover rejection and forbidden states with a negative test.
6. Keep tests fast; keep helpers and generators near their users.

Read `references/test-taxonomy.md` and `references/mathematical-code.md` for law-shaped tests.

## Names

- Use `<area>_<suite>.rs`; suite suffixes: `laws`, `regressions`, `validation`, `generators`, `helpers`.
- Use `<subject>_<property>` for laws, `regression_<case>_<behavior>` for regressions, and `<subject>_<invalid_case>_<rejects_or_fails>` for negative cases.
- Name the protected contract, not the test tool.

## By Context

- **Feature:** test the contract and a boundary case.
- **Bug:** add the minimal failing regression; add a law if it exposes a broader gap.
- **Refactor:** preserve contracts without asserting old internals.
- **Math:** use laws, roundtrips, and a negative case where relevant.
- **CLI:** assert stable user-visible output, files, exit status, or state.
- **Performance:** keep a correctness test if needed, then hand off to `rust-performance`.

## Smells

| Smell | Fix |
| --- | --- |
| Smoke tests without an invariant | Test the contract. |
| Bare `is_err()` | Assert a stable rejection fact. |
| Unstable full-text snapshot | Assert structure or stable fragments. |
| Huge fixture | Shrink to a reproducer. |
| Weak generator | Improve its cases and shrinking. |
| Broad test for a local law | Move it down a layer. |
| Benchmark loop in `#[test]` | Use a benchmark or profile. |

## Validation

Start with:

```bash
cargo nextest run -p <affected-crate>
```

Widen only when the change spans crates or layers. Read `references/failure-smells.md` for examples and `references/perf-handoff.md` for the performance handoff.

## Related Skills

- `rust-performance` for performance guards.
- `module-design` when tests expose a bad module boundary.
