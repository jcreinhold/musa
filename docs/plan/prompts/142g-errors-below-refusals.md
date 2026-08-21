---
id: 142g
slug: errors-below-refusals
status: pending
depends_on: [142f]
phase: 3
---

# Put the Kernel's Errors Below the Elaborator's

## Task

`musa-calculus` states its own layering and does not obey it. [`error.rs`](../../../crates/musa-calculus/src/error.rs)
says of `CoreError::Refused` that it carries a sentence and a place "because this module is below that one:
[`crate::ElabError`]'s conversion is where the two meet, and is the only lift." That is the right architecture and it is
not the one in the code: `Refusal` has **70 variants** spanning two different jobs, and the files that decide conversion
and hold the base registry reach straight for it.

```sh
$ grep -n 'Refusal\|ElabError' crates/musa-calculus/src/{convert.rs,base.rs,term.rs} | head
crates/musa-calculus/src/convert.rs:47:use crate::refuse::{ElabError, Mismatch, PathStep, Refusal};
crates/musa-calculus/src/base.rs:94:use crate::refuse::Refusal;
crates/musa-calculus/src/term.rs:581:    pub fn level_of(term: &Self) -> Result<Level, crate::Refusal> {
```

One error type braids two claims that are not the same claim: _the kernel could not decide this conversion_ and _the
author wrote a name that is not in scope_. While they share an enum, every file that decides equality names a type
whose other sixty variants are about surface programs, and there is no direction left to enforce. Separate them, so
that the lift `error.rs` already describes is the only crossing.

**This is the prerequisite for 142h**, which cannot draw a module boundary the error type crosses in both directions.

## Read

- [`crates/musa-calculus/src/error.rs`](../../../crates/musa-calculus/src/error.rs) in full — the layering is written
  there already, in the doc comment on `CoreError::Refused`. This prompt makes it true rather than deciding it.
- [`crates/musa-calculus/src/refuse.rs`](../../../crates/musa-calculus/src/refuse.rs) — all 70 variants, read for which
  ones a caller could reach _without writing a program_: a `Mismatch` between two terms, a level past the two
  universes, a registry whose declarations do not agree with each other.
- [`crates/musa-calculus/src/lib.rs`](../../../crates/musa-calculus/src/lib.rs) — the facade's split between
  `Result<_, ElabError>` (elaboration) and `Result<_, CoreError>` (`normalize`, `convertible`). The signatures already
  say which side each operation is on; this prompt makes the _types they carry_ agree with them.
- `crates/musa-calculus/tests/suite/elaboration_laws.rs`'s
  `each_refusal_is_reached_by_the_program_it_is_about` — the coverage gate. Every variant needs a tag in `kind()`, an
  entry in `ALL_REFUSALS`, and a program that reaches it, and a variant that moves to `CoreError` must leave all three
  consistently rather than quietly.
- [`crates/musa-compiler/src/lower/refusals.rs`](../../../crates/musa-compiler/src/lower/refusals.rs) — the one place a
  refusal becomes a surface diagnostic. It is the reason this prompt is a refactor and not a diagnostics change: the
  author must see the same sentence with the same code afterwards.
- _Simple Made Easy_ on complecting. A sum type whose variants answer two different questions is the braid; splitting
  it is not a new abstraction, it is the removal of one.

## Design

**Which variants go down, and the test for it.** A variant belongs to `CoreError` when a caller can reach it _with no
program in hand_ — by handing the kernel two terms, a level, or a registry. That is: the conversion mismatch and its
`PathStep`/`Mismatch` payload, the universe-arithmetic refusal `term.rs` raises, and the registry's structural checks
(`TargetOutsideSignature`, `TargetNotABase`, `UnknownBase`, and their neighbours in `Registry::new`). Everything whose
sentence names something the author _wrote_ stays a `Refusal`.

**One lift, and it is the one already documented.** `ElabError: From<CoreError>` is the only crossing, and it is where
the place and the written spelling are attached. The kernel does not know how to spell a type the author's way, which
is why the lift is at the boundary rather than at each raise site.

**No diagnostic changes, and that is checkable.** Every surface code, sentence, help line, and span is byte-identical
afterwards. The coverage gate keeps its count honest: variants that move keep their tags, now supplied by the lift.
This is the whole safety argument for the prompt — a refactor that also reworded a diagnostic could not be reviewed.

**`Mismatch` is shared, not duplicated.** It describes two terms that did not agree, which is a kernel fact, so it
moves down whole and `Refusal` names the lifted `CoreError` rather than a second copy. Two mismatch types would be two
descriptions of one disagreement, obliged to agree by a law nobody could state — the same argument `lib.rs`'s "One
evaluator" invariant makes.

## Target

- `crates/musa-calculus/src/error.rs`: `CoreError` gains the kernel-raised cases and `Mismatch`/`PathStep` move beside
  it, each with the doc comment stating what a caller did to reach it.
- `crates/musa-calculus/src/refuse.rs`: those variants leave `Refusal`; `ElabError` gains `From<CoreError>` as the one
  lift, attaching origin and spelling.
- `crates/musa-calculus/src/convert.rs`, `base.rs`, `term.rs`: no mention of `Refusal` or `ElabError` remains — they
  return `CoreError`.
- `crates/musa-calculus/tests/suite/elaboration_laws.rs`: `ALL_REFUSALS` and `kind()` follow the move, and the coverage
  gate still reaches every case from a program.
- `crates/musa-compiler/src/lower/refusals.rs`: the lifted core errors reach the same `musa_score::diagnose::Code` they
  reach today.
- A law: the lift is total — every `CoreError` a caller can raise has a `Refusal` spelling, so no path reaches an
  author as a debug print.

## Check

```sh
cargo nextest run --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

And the check that makes this reviewable as a refactor: **every diagnostic is byte-identical**. Every `insta` snapshot,
`crates/musa-compiler/tests/fixtures/elaboration-compatibility.txt`, the `musa-events` pinned digests, and
`apps/musa-desktop/ui/fixtures/` are unchanged, and `git diff --stat` over `crates/musa-score/src/diagnose.rs` is
empty — no code was added, renamed, or removed.

## Stop

- **No new refusal, and none deleted.** Variants move between two types; the set of things that can go wrong is exactly
  what it is today.
- **No rewording.** Not a sentence, not a help line, not a code. Diagnostics are 144's subject.
- **No module move.** `kernel/` and `elaboration/` are 142h; this prompt leaves every file where it is.
- **No re-checker.** That is 142i, which this prompt exists to make possible.
- No change to any public signature in `lib.rs` — the facade already returns the right type on each side.
- No change to `Budget`, `ResourceError`, or exhaustion. Budget exhaustion is already its own outcome under §4 and is
  on the correct side.

Commit as `Put the kernel's errors below the elaborator's`.
