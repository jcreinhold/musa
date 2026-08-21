---
id: 158
slug: recheck-the-whole-core
status: pending
depends_on: [149, 157]
phase: 3
---

# Close the Re-Checker Over the Whole Core

## Task

Prompt 149 built the re-checker over a seven-constructor term language and made extending it an obligation of every
prompt that added to the core. This prompt closes it: audit that each of 151–157 actually discharged its obligation,
cover what none of them owned, and turn the re-checker from a debug assertion into the conformance gate the crate has
cited since prompt 134.

**This prompt supersedes [`142i`](142i-core-re-checker.md)**, which specified the re-checker against the term language
prompts 147–157 replace, and which sat before all of them.

## Read

- [`142i-core-re-checker.md`](142i-core-re-checker.md) — the argument and the four citing sites.
- `crates/musa-calculus/TRUST.md` as prompt 149 wrote it — the three acceptance invariants this closes over.
- The Check sections of 151–157, each of which names an extension owed here.

## Design

**Audit first, build second.** The extension obligation is easy to satisfy shallowly: add an arm, return `Ok`. So this
prompt starts by re-reading each extension against the machinery it was supposed to cover, and the deliverable of that
reading is a table — construct, what the re-checker verifies about it, and the negative control that proves it can
reject. A row with no negative control is a row that has not been checked.

**What the closed re-checker verifies, beyond 149's three invariants.**

- *A metavariable solution mentions nothing outside its scope.* The occurs check and the scope check at solving time are
  elaboration's; this is the independent re-derivation of the same fact from the solved term.
- *A case-tree branch checks at the motive instantiated at that branch's pattern.* This is the invariant that makes
  dependent elimination sound, and it is the one nothing else in the crate re-states.
- *An `Impossible` branch really is impossible* — the index unification that ruled it out is re-run, not trusted.
- *Coverage is complete*, re-derived from the family rather than from the builder's bookkeeping.
- *Level parameters are instantiated consistently* across a declaration's uses.
- *Structural descent holds* for every recursive definition — the termination check re-run over the finished tree rather
  than assumed from the builder that produced it.

**From assertion to gate.** After this prompt the re-checker runs unconditionally over the whole standard library and
every `examples/` fixture in the conformance suite, and prompt 169's obligation matrix takes its result as a row. The
debug assertion in `program.rs` stays for development; the gate is what makes the TRUST.md claim testable.

**The honest limit, recorded rather than glossed.** A re-checker sharing the kernel's own `convert` and `eval` cannot
catch a bug *in* `convert` or `eval` — it re-derives using the same machinery it is checking. What it catches is
elaboration producing a term the kernel would reject, which is the overwhelming majority of what can go wrong and is
exactly the split TRUST.md draws. Say so in TRUST.md; a trusted base that overstates its guarantee is worse than one
that states a smaller one accurately.

## Target

- `crates/musa-calculus/src/kernel/recheck.rs`: the closed pass and the audit's missing arms.
- `crates/musa-calculus/TRUST.md`: the acceptance invariants extended to metavariables, case trees, families and levels;
  and the honest limit above.
- `crates/musa-calculus/tests/suite/recheck_laws.rs`: one negative control per construct, and the standard library
  re-checked end to end.
- `fuzz/`: the workspace's first fuzz target, and it exists because of this prompt rather than beside it. **The
  re-checker is an oracle**, which is what fuzzing needs and what musa has never had: generate a program, elaborate it,
  and assert the kernel accepts whatever elaboration produced — a crash, a rejection, or a scope violation is a bug in
  *us* by construction, with no expected output to write down. Hand-written negative controls cover the failures
  somebody thought of; this covers the ones that make de Bruijn indices worth worrying about. `~/Code/kan` fuzzes its
  kernel, frontend and binding layers separately; one target over the whole pipeline is the right size to start.
- `docs/plan/prompts/169-core-conformance.md`: the re-checker row added to the obligation matrix.
- `142i-core-re-checker.md`: `status: superseded` and a banner naming this prompt.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- check stdlib/src/*.musa examples/*.musa
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

The audit table is a deliverable and its absence is a failed Check: a construct with no negative control has not been
re-checked, whatever the suite says.

Commit as `Close the re-checker over the whole core`.

## Stop

- No new typing rules, and no rule the kernel does not already have.
- No attempt to verify `convert` or `eval` themselves. That is a different technique and a different prompt, and
  overstating this one's reach is the failure mode TRUST.md exists to prevent.
