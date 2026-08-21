---
id: 127dca
slug: text-patterns-match
status: done
depends_on: [127dc]
phase: 3
---

# Make a Text Pattern Match the Text It Names

## Task

A `match` arm written with a text literal never matches, so the arm below it runs instead. Fix the evaluator's literal
comparison, state the law that a literal pattern matches exactly the value it spells, and prove it for every literal
pattern the checker admits.

## Read

- `docs/rules/language/02-core-calculus.md` §5.5 on `match` and its patterns, and §5.7 on evaluation — a pattern's
  meaning is decided by the value it names, not by which base type it happens to be.
- `crates/musa-compiler/src/phase/mod.rs`: `Checker::pattern_literal` (which literal patterns exist),
  `literal_values_equal` (the runtime comparison), `literal_key` (the exhaustiveness key), and `matches_pattern`.
- Prompt 127ab, which added `Text` and its literals, and prompt 127b's evaluator configurations.
- Prompt 127dc, and prompt 127dd's Design — an adapter reads the token kind and the token text it is handed, and both
  arrive as `Text`. Until a text pattern matches, no adapter can tell a slash from a pitch.

## Design

The defect is one omission and not a design question. `pattern_literal` admits seven literal patterns — `true`/`false`
at `Bool`, an integer at `Nat` or `Duration`, a string at `Text`, a rational at `Ratio` or `Duration`, a pitch literal
at `Pitch`, and an interval literal at `Interval`. `literal_values_equal` compares six of the resulting value kinds and
answers `false` for `Value::Text`, so `match word { "c5" -> a, _ -> b }` evaluates to `b` for every `word`, silently.

Nothing else in the pipeline is confused: `literal_key` already gives text its own exact key, so exhaustiveness and
duplicate-arm reporting are right today. Only the comparison is wrong.

Two things follow, and the second is the reason this is a prompt rather than a one-line fix:

1. Compare text by its exact bytes, the same equality `literal_key` already implies. Two texts match exactly when they
   are one text.
2. Make the comparison total over the values a literal pattern can hold, so that the next base type to grow a literal
   pattern cannot repeat this. A comparison whose fallback is "not equal" turns a missing case into a wrong answer
   instead of a compile error; whatever shape the repair takes, adding a literal pattern kind without teaching the
   comparison about it must stop being possible silently.

State the law where the other evaluation laws are stated, and check it for each of the seven admitted patterns rather
than only for text: a law about one type would leave the same hole one type over.

This is a correctness fix in ordinary source, not a phase-local addition. A composer writing a text match today gets the
wrong arm with no diagnostic, which is the worst failure a total language can have — the file compiles and means
something else.

## Target

- `literal_values_equal` compares text, and cannot silently omit a value kind a literal pattern admits.
- A law suite that runs every literal pattern kind the checker accepts — bool, nat, duration-from-integer, text, ratio,
  duration-from-rational, pitch, interval — through a `match` and reads the answer back, both for the arm that should
  match and for one that should not.
- One `examples/` fixture whose meaning depends on a text match, so the regression is a piece and not only a unit test.
- The budget-independence law and the existing exhaustiveness laws still hold: this changes which arm runs, never which
  files are accepted.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-compiler -p musa-project -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Match a text pattern against the text it names`.

## Stop

- No new literal pattern kinds, and no pattern syntax the checker does not already admit.
- No text operations of any kind — no concatenation, comparison builtin, length, or splitting. A pattern that matches is
  all this prompt delivers; whether the phase language needs more than that is prompt 127dd's evidence to produce.
- No change to exhaustiveness, coverage, or duplicate-arm reporting, which are already right.
- No change to `docs/rules/`.
