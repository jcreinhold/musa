---
id: 127b
slug: inferred-source-core
status: done
depends_on: [127ad]
phase: 3
---

# Close the Inferred Source Core

> **Governed by the event-track and machine core installed by prompts 127a–127e and 150–153.** Last of the five prompts
> that replace the source checker and evaluator; the chain is 127aa, 127ab, 127ac, 127ad, 127b. Prompts 127c and 127d
> depend on this one, so it is the point at which the source language is a single, closed, checkable thing again.

## Task

Give the evaluator its typed configurations and versioned resource semantics, close the foreign-primitive and privacy
boundaries, and audit the four preceding prompts as one language: the small inferred core is now the only checker and
evaluator in `musa-compiler`.

## Read

- `docs/rules/language/02-core-calculus.md` §§1–4 — the type grammar, the storable-data rule, the typing rules, and the
  resource meter.
- `docs/rules/language/00-semantics.md` §2, which states the evaluation judgments this prompt implements:
  `run(budget, e) ⇓ done(v) | failed(ResourceError)`, and the rule that a budget may stop an evaluation but never change
  an accepted one.
- Research `05-selected-calculus.md` §2 and `06-proof-outline.md` §2.
- `crates/musa-compiler/src/core_budget.rs` — the existing deterministic `WorkMeter`, its five limits, and `Exhaustion`.
- The completion notes of prompts 127aa, 127ab, 127ac, and 127ad.

## Design

Replace the ad-hoc meter calls with typed evaluator configurations: `run(budget, e)` reduces to `done(v)` or
`failed(ResourceError)`. Charge a fixed versioned integer cost for the unique next reduction. Never read wall time,
allocator behaviour, or any other machine-dependent quantity — the same source and compiler version fail at the same
operation on every machine, which is what makes acceptance a property of the language rather than of the host.

The cost table is versioned data, not scattered constants. Changing a cost is a version bump with a stated reason, and
the version is part of what a cache key records.

A budget can stop an evaluation but cannot change an accepted one: if two runs both reach `done`, they reach the same
value. State this as a law and test it by running the corpus at several budgets and comparing every accepted result.

Foreign source primitives are first-order, data-only, and total, with ordinary `Result` failures. A foreign primitive
receives no function argument and no closure, and its arguments and result are storable data. Reject a registration that
violates this at build time, not at the call.

Audit the privacy boundary: core terms, environments, inferred schemes, nominal ids, and evaluator values stay private
to `musa-compiler`. Expose only the compiler facts an actual caller needs. Delete any public item this chain left
without a caller.

Then audit the chain as one language: one checker, one evaluator, no surviving second path. A remaining alternate path
is a finding to report and remove here, not to carry into 127c.

## Target

- Typed evaluator configurations `run`, `done`, `failed(ResourceError)` replacing the ad-hoc meter calls, over a
  versioned integer cost table.
- The budget-independence law, stated and tested across several budgets over the whole corpus.
- Foreign primitives constrained to first-order, data-only, total operations with `Result` failures, checked at
  registration.
- A privacy audit removing any public mirror of the private core AST, environment, scheme, nominal id, or evaluator
  value.
- Principal-type and termination law tests for the accepted fragment, and the compile-fail suite covering hidden
  functions, incomplete calls, recursive terms, non-exhaustive matches, and bad data instantiations.
- Updated language facts, hover text, and diagnostics with inferred types shown in plain form.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
find examples stdlib -name '*.musa' -print0 | xargs -0 -n1 cargo run -q -p musa -- check
```

Commit as `Replace the source evaluator with the inferred core`.

## Stop

- No `EventTrack` rename, machine type, scheduler, DSP change, type-directed macro, general recursion, overloading,
  subtyping, higher-rank type, or dependent type.
- No compatibility mode for partial calls or default parameters.
- No public Rust mirror of the private core AST or evaluator value.
- No interrupt, timeout, or partial-result mechanism: resource accounting is an acceptance rule, not a scheduler.
