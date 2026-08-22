---
id: 150
slug: errors-below-refusals
status: in-progress
depends_on: [148]
phase: 3
---

# Put the Kernel's Errors Below the Elaborator's

## Task

[`142g`](142g-errors-below-refusals.md) asked for a split and gave a test for it: a variant belongs to `CoreError` when
a caller can reach it *with no program in hand*. Prompts 146–149 did the split. They did not use that test, and the
evidence says they were right not to — the test does not separate the two enums, and applying it now would move variants
the last four prompts deliberately placed.

So this prompt delivers what 142g's Task was actually after, against the code 146–149 left: **the rule, written where
the enums are, and a gate that holds both of them to it.** The braid is gone; what is missing is any statement of why it
is gone, and any check that keeps it gone.

## Read

- [`142g-errors-below-refusals.md`](142g-errors-below-refusals.md) — the argument, in full, and the test this prompt
  replaces.
- `crates/musa-calculus/src/kernel/error.rs` and `src/elaboration/refuse.rs` after 148's move and 149's additions.
- `crates/musa-calculus/src/elaboration/admit.rs`'s module doc — prompt 148's argument for why registry admissibility
  answers a `Refusal`.
- `crates/musa-calculus/tests/suite/elaboration_laws.rs`'s `each_refusal_is_reached_by_the_program_it_is_about` — the
  gate that exists, and the shape the missing one takes.
- [`crates/musa-calculus/TRUST.md`](../../../crates/musa-calculus/TRUST.md) — 149's reading of `Malformed` as a defect
  in this compiler rather than a fault in the program.

## Design

**Why 142g's test does not work.** Three ways, each checkable:

- It selects nothing. After 148 no file under `kernel/` names `Refusal` or `ElabError` on a code line, and
  `tests/suite/boundary_laws.rs` enforces that. Every variant 142g named — the conversion mismatch, the
  universe-arithmetic refusal, the registry's structural checks — is raised only from files under `elaboration/`.
- It selects everything. `Malformed::UnboundVariable` is reachable by handing the kernel a term, and so is
  `Refusal::UnknownName`. "No program in hand" describes how a *caller* got there, and both enums are reachable that
  way.
- It contradicts what the split already decided. Registry admissibility is `elaboration/admit.rs` because a host wrote a
  signature and gets a sentence about it; 148 argued that in the module doc. Moving those variants down would make that
  argument false and pull the file after them.

**The test that does separate them, which 146–149 used without writing down.** Two questions, asked in order:

1. **Who is the sentence addressed to?** A `Refusal` is addressed to someone who *wrote* something — an author writing
   `.musa`, or a host author writing a registration. A `CoreError::Malformed` is addressed to whoever maintains this
   compiler: nobody wrote the term, it was assembled wrong.
2. **When was it discovered?** The same host mistake can be either, and the boundary is whether the thing that was
   written is still in hand. A δ signature that D1 does not admit is caught at registration, where the registration can
   be named: `Refusal::HigherOrderDelta`. A δ *rule* that answers nothing at arguments it declared it accepts is caught
   mid-evaluation, long after §5.8's D2 promise was accepted, and there is nothing left to point at:
   `Malformed::BuiltinStuck`.

`CoreError::Exhausted` is neither and is already correct: §4 makes it the third outcome, not a verdict.
`CoreError::Refused` is a δ-rule's verdict on the author's own arguments, which is why it carries a sentence and a place
and why `ElabError: From<CoreError>` is the one lift.

**Under that test nothing moves, and the code already shows it.** Seven distinctions are drawn *twice* on purpose —
`NotAFunction`, `NotARecord`, `NoSuchField`, `NotAType`, `Uninferable`, `BeyondUniverses`, `UnreadableIndex` are each a
`Malformed` and a `Refusal`, and 149 added the pair `Malformed::Mistyped` beside `Refusal::Mismatch` for the same
reason. The repo's answer to a claim that lives on both sides of the line is to state it on both sides in each side's
vocabulary, not to move one copy down. This prompt writes that answer where the enums are, so it is a decision and not
an accident.

**Where prompt 153's unification failures go.** *These two terms have no solution* and *this constraint is still
blocked* are facts about two terms with no writer to address, discovered by the kernel. They are `CoreError`s. Said here
so 153 does not invent a third enum, and so it does not spend the decision again.

**The gate the two enums do not share.** `Refusal` has a coverage gate: `kind` matches every variant, `ALL_REFUSALS`
lists every tag, and a program reaches each one, so a variant nobody can reach fails the suite rather than shipping as a
message nobody has read. `Malformed` has none — seventeen variants, no gate. It gets the same one. A variant only a
compiler bug reaches cannot be reached by a program, so the gate takes a term, a registry, or a context built by hand; a
variant that even that cannot reach is listed with a one-line argument for why, the way `#[ignore]` is.

**No diagnostic changes, and that is checkable.** Every surface code, sentence, help line and span is byte-identical
afterwards. This is the whole safety argument: a refactor that also reworded a diagnostic could not be reviewed.

## Target

- `crates/musa-calculus/src/kernel/error.rs`: the rule on `CoreError`'s own doc — who the sentence is addressed to, when
  it was discovered — and the sentence assigning 153's unification failures here.
- `crates/musa-calculus/src/elaboration/refuse.rs`: the converse on `Refusal`'s doc, naming the seven doubled
  distinctions as the pattern rather than as duplication, and `From<CoreError>` documented as the one crossing.
- `crates/musa-calculus/tests/suite/`: a coverage gate over `Malformed` matching the one over `Refusal` — a `kind` match
  that is exhaustive by compilation, a tag list, and something that reaches each tag.
- `142g-errors-below-refusals.md`: its banner says this prompt revises its test, not merely its variant list.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

`--unreferenced=reject` with every snapshot unchanged is the diagnostic-identity check. One changed snapshot means a
message moved, which this prompt forbids.

And the check that makes the Task's claim true rather than asserted:

```sh
! grep -rn 'Refusal\|ElabError' crates/musa-calculus/src/kernel --include='*.rs' \
    | grep -v '///' | grep -v '//!'
```

## Stop

- **No variant moves between the two enums.** The Design says why, and a repair that re-opens it is re-opening 148 and
  149 with it.
- No reworded diagnostics, no new codes, no merged variants.
- No module move. `admit.rs` stays where 148 put it, for the reason 148 gave.
- No unification variants. 153 adds them; this prompt only says where they belong.
- No error-handling change in `musa-compiler`; the lift keeps the facade's type the same.

Commit as `Put the kernel's errors below the elaborator's`.
