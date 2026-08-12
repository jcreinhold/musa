# Musa governance

These documents say what Musa is, independently of one compiler organization or one musical package.

- [01-constitution.md](01-constitution.md) contains only commitments whose removal would change the identity of the
  project.
- [02-obligations.md](02-obligations.md) derives non-obvious constraints from those commitments.

The constitution governs the formal specifications in `docs/spec/`, the existing stage specifications in `docs/kernel/`
and `docs/language/`, the implementation architecture, and the prompt stack. It does not make every candidate research
note normative. `docs/scratch/` remains the audit trail in which failed candidates stay visible.

An amendment is deliberate work. It must:

1. name the concrete musical or implementation counterexample;
2. state which amendment and obligations change;
3. repair the affected formal specification before or with implementation;
4. identify migrations for stored identity and source compatibility; and
5. add or repair the prompt which executes the change.

An implementation discrepancy does not amend the constitution by accident. Either the implementation is repaired, or the
discrepancy becomes evidence in an explicit amendment.
