---
id: 144
slug: diagnostics-and-performance
status: pending
depends_on: [143]
phase: 3
---

# Make the New Failures Legible and the New Checker Fast Enough

## Task

A dependent checker fails in ways Musa has never failed before — a conversion mismatch between two normal forms, an
incomplete match, a recursion the termination checker cannot see, an unsolved metavariable, an ambiguous instance — and
it does work Musa has never done before, because conversion evaluates. Bring the diagnostics up to the standard the rest
of the compiler holds, and bring P1 and P2 back inside `06-performance.md`'s 10% gate.

## Read

- `docs/rules/language/06-performance.md` §"Measurements and honest seams", the P1–P5 table, the recorded baselines, and
  §"Gates and budgets" — the 10% relative gate, and the end-to-end authority in `docs/rules/desktop/06-performance.md`:
  B1 is ≤120 ms after debounce and B2 is ≤400 ms with the previous engraving visible. Those two are what a musician
  actually experiences, so they are the numbers that decide whether this prompt is done.
- `crates/musa-compiler/src/diagnose.rs` and `crates/musa/src/main.rs`'s `cmd_explain` — the existing diagnostic
  vocabulary and the rule that every code has an explainable rule behind it.
- Every `Code` variant added by prompts 134, 135, 137, 139, and 140, and the message each currently produces. Those were
  written to be correct; this prompt makes them good.
- The `rust-performance` skill's workflow in full — name the workload, reuse the nearest credible bench, find what
  actually costs, make the smallest matching change, re-measure, and report the numbers with the command.
- `crates/musa-compiler/src/bench.rs` and `core_budget.rs` — where measurement already happens and where the budget is
  charged.
- `docs/rules/desktop/` on states and voice: a diagnostic is interface text, and the desktop specification already says
  what Musa sounds like when it refuses.

## Design

**The conversion error is the one that decides whether this language is usable.** Two normal forms printed in full is a
correct message nobody can read. The policy prompt 134 stated — normalize far enough to be honest, unfold no further
than necessary, show the surface node each side came from — gets implemented properly here: find the first point of
disagreement, print the two sides around it, name the definition that was unfolded to get there, and offer the fully
unfolded forms behind `musa explain` rather than in the message. Test it on a deliberately deep mismatch, because a
message that is fine on a two-line example and useless on a real one has not been tested.

**Coverage and termination failures should name the program, not the theory.** An incomplete match names the missing
constructor pattern and offers it. A termination failure names the recursive call and the argument that did not
decrease, and — since there is no `partial` to suggest — says what a measure would look like for that shape. A
non-positive occurrence names the constructor and the position. None of these should require the author to know what a
recursor is.

**An unsolved metavariable is a question, not a failure.** It says what could not be determined, where the choice was
left open, and what annotation would settle it. This is the message that will appear most often while people learn the
new language, so it is worth more care than its frequency in the test suite suggests.

**Ambiguity names both candidates.** Prompt 137 made ambiguity an error rather than a default; the error is only better
than a default if it says what the two possibilities were and how to pick one.

**Then measure, in that order.** Diagnostics first because a fast compiler with unreadable errors is worse than a slow
one with good errors, and because improving messages sometimes changes what code runs. The workloads are
`06-performance.md`'s existing four — open-shape, higher-order-shape, declaration-heavy, audio-bridge — so the numbers
are comparable to the recorded baseline rather than to a benchmark invented to make this prompt look good.

**Where a dependent checker actually costs, and where it does not.** Conversion evaluates, so the suspects are
normalization during checking, metavariable solving, and instance resolution — not parsing and not term construction.
Profile before changing anything. The likely interventions, in order of how much they cost to get wrong: caching normal
forms of definitions, resolving known-concrete instances at elaboration time (prompt 143 may already have done this),
and avoiding re-normalization when checking against a type already in normal form. Each is measured independently, and a
change that does not move the number is reverted rather than kept because it seemed principled.

**Update the recorded baseline, honestly.** `06-performance.md` records both the pre-migration baseline and the current
numbers. If something is genuinely slower and the 10% gate is exceeded, the prompt's output is either a mitigation or an
argued amendment to the budget — with the musician-facing B1/B2 numbers as the check that the argument is acceptable —
and never a quietly raised threshold.

## Target

- Every diagnostic from prompts 134, 135, 137, 139, and 140 rewritten to the standard above, each with `musa explain`
  text and a test that pins the message on a realistic program rather than a minimal one.
- Profiles of the new checker on the four recorded workloads, the interventions chosen, and the re-measured numbers.
- `docs/rules/language/06-performance.md` updated with post-migration P1/P2 rows and, if the gate was exceeded, the
  argument and its resolution.
- `docs/rules/desktop/06-performance.md`'s B1/B2 confirmed still met, measured rather than assumed.
- `docs/plan/code-map/` rows.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-compiler -- p1_compile p2_elaborate
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Make the new failures legible and the new checker fast enough`.

## Stop

- No behaviour change: same programs accepted, same programs rejected, same values, same rendered output. A diagnostic
  prompt that changes what compiles has changed the language.
- No new language feature offered as a fix for a bad message.
- No optimization without a measurement, and no `#[inline(always)]`, SIMD, parallelism, or custom allocator on
  intuition. The `rust-performance` skill's rule is the rule.
- No raised budget threshold in place of a fix, and no benchmark chosen to flatter the result.
- No adapter rewrite. Prompts 145 and 146, which is also where the real-program evidence for these diagnostics comes
  from.
