---
id: 158
slug: recheck-the-whole-core
status: in-progress
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
- `docs/plan/roadmap.md` §15.10 (the closed development-dependency list) and §17.2 (`proptest` as the generator), which
  decide the shape of the generated-program law — see the repair recorded in the Target.

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
- `crates/musa-calculus/tests/suite/recheck_laws.rs`: the audit table, one negative control per construct, and
  [`recheck_program`](../../../crates/musa-calculus/src/lib.rs) run over every program the suite builds.
- The standard library re-checked end to end, **in `musa-compiler`'s suite rather than this one**. *Repaired during
  implementation.* The Target put it in `recheck_laws.rs`, and `musa-calculus` cannot read it: the standard library is
  `.musa` source, the crate is a leaf with no parser, and giving it one would invert the dependency direction root
  `AGENTS.md` states one-way. The gate goes where the source can be read, and the pass it runs is the same
  `recheck_program` this prompt adds.
- `crates/musa-calculus/tests/suite/generated_laws.rs`: a generated-program law, and it exists because of this prompt
  rather than beside it. **The re-checker is an oracle**, which is what generative testing needs and what musa has never
  had: generate a program, elaborate it, and assert the kernel accepts whatever elaboration produced — a crash, a
  rejection, or a scope violation is a bug in *us* by construction, with no expected output to write down. Hand-written
  negative controls cover the failures somebody thought of; this covers the ones that make de Bruijn indices worth
  worrying about.

  *Repaired during implementation.* This bullet read `fuzz/`, "the workspace's first fuzz target", citing `~/Code/kan`,
  which fuzzes its kernel, frontend and binding layers separately. Three things say otherwise, and none of them touch
  the argument above — which is about having an **oracle**, not about libFuzzer's scheduler.

  First, the dependency is not available: roadmap §15.10 lists the workspace's development dependencies and says in as
  many words that **the list is closed** — `insta`, `proptest`, `divan`. `libfuzzer-sys` and `arbitrary` are not on it,
  and root `AGENTS.md` requires a new crate to come from a §15 list. §17.2 already names `proptest` as the generator
  and shrinking is the property this prompt wants most: a counterexample to kernel acceptance is only useful if it is
  small enough to read.

  Second, a `fuzz/` directory is its own workspace, so it is outside `cargo build --workspace`, outside
  `cargo nextest run`, outside clippy, and outside every command in this prompt's Check. Nothing here would ever run
  it. That is precisely the failure the Design section names one paragraph earlier — a row with no negative control is
  a row that has not been checked — committed against this prompt's own deliverable. A `proptest` law runs on every
  `cargo nextest run --workspace`, which is where a gate belongs.

  Third, byte-mutation is the wrong shape for this oracle. The interesting bug is scope corruption in a *well-typed*
  program, so the generator has to produce programs elaboration accepts; a mutator over bytes spends nearly all of its
  budget on programs elaboration refuses for uninteresting reasons, and the oracle says nothing about those. A
  structured generator over `Raw` is what reaches the terms that make de Bruijn indices worth worrying about.

  **What is given up, recorded rather than glossed:** coverage-guided mutation and a persistent corpus that grows
  across runs. `proptest` explores what its generator was written to reach and nothing else, so a construct absent from
  the generator is untested by it — which is why this bullet is a complement to the negative-control table and not a
  replacement for it. If kernel acceptance ever fails in a way the generator could not have reached, a real fuzz
  target is the escalation, and taking it means putting `libfuzzer-sys` on §15.10 deliberately rather than in passing.
- `docs/plan/prompts/169-core-conformance.md`: the re-checker row added to the obligation matrix.
- `142i-core-re-checker.md`: `status: superseded` and a banner naming this prompt.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo nextest run -p musa-compiler the_kernel_rechecks
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

The audit table is a deliverable and its absence is a failed Check: a construct with no negative control has not been
re-checked, whatever the suite says.

**The corpus gate is a test, not a CLI run.** *Repaired during implementation.* The line read
`cargo run -p musa -- check stdlib/src/*.musa examples/*.musa`, and it could not test this prompt's deliverable for a
reason that is structural rather than incidental: **the audit is not in a release build.** `TRUST.md` says so and this
prompt's own Design repeats it — the re-checker doubles the cost of checking, so it runs behind `debug_assertions` and,
now, in test builds. `cargo run -p musa` executes neither, so the command would have compiled the corpus with the
re-checker switched off and reported green whatever the kernel thought. A gate pointed at a pass that is not running is
the exact failure the Design section names one paragraph earlier.

The replacement runs the two laws the Target asks for, in `musa-compiler`'s `document::laws`:
`the_kernel_rechecks_the_standard_library` reads all fourteen library files as one document and runs `recheck_program`
over it, and `the_kernel_rechecks_every_example` puts all fifty-five `examples/` fixtures through the whole pipeline,
where a `#[cfg(test)]` call in `elaborate/mod.rs` audits each document as it is elaborated — including the second
document a `make` or a staff region produces, which is why the law compiles rather than elaborating directly.

**And the command it replaces fails today, at HEAD, for three reasons none of which are this prompt's.** Measured on a
clean worktree at `7396cb2c` and byte-identically in this prompt's tree — 3 errors:

| File | What it says | Whose it is |
| --- | --- | --- |
| `stdlib/src/lib.musa` | `this file declares no piece` | the glob's. `lib.musa` is the package's module tree — twenty-four lines of `mod` — so it is neither a piece nor a library and `musa check` will always refuse it. The Check as written could never pass. |
| `examples/diatonic-sequences.musa` | reduction steps at 200001 of 200000 | the tonal budget class. Prompt [166](166-staff-rewrite.md)'s Check records that class as having "closed itself" — and it did, for `nextest`. It did not close for a whole-file check, which is a gap in 166's measurement rather than a defect here. |
| `examples/staff-page.musa` | `expanding this region with std::adapters::staff crossed a compilation limit` | prompt [166](166-staff-rewrite.md)'s, named in its Check's thirty-test table. |

The two `examples/` entries are recorded in the law itself, as `BUDGET_WALL` in `document/laws.rs`, with the argument
that exhaustion is not a disagreement: a fixture that ran out of steps was never judged, so the kernel was never asked.
They are still required to fail *only* that way, so a real refusal from either one fails the build.

Commit as `Close the re-checker over the whole core`.

## Stop

- No new typing rules, and no rule the kernel does not already have.
- No attempt to verify `convert` or `eval` themselves. That is a different technique and a different prompt, and
  overstating this one's reach is the failure mode TRUST.md exists to prevent.
