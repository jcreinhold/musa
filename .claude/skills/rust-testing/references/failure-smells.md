# Failure Smells

- Examples without a contract: add an invariant or law.
- Weak or brittle assertion: assert a stable, relevant fact.
- Helper hides the assertion: inline the key check.
- Same bug covered at many layers: keep the most diagnostic test.
- Timing loop in a correctness test: use a benchmark or profile.
