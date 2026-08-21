---
id: 142da
slug: index-reflexivity
status: pending
depends_on: [142d]
phase: 3
---

# Refuse an Unreadable Index Where It Is Written, So Conversion Is Reflexive

## Task

Conversion is not reflexive today. `Row(mystery n)` is not the same type as itself, and three tests in
`crates/musa-calculus/tests/suite/index_laws.rs` assert exactly that. A term can therefore fail to check against its own
type, which is a break in the conversion relation rather than a strict choice about indices. Move the refusal of an
out-of-grammar index from the *comparison* to the *type*, which is where `02-core-calculus.md` §1.5 already sites it, so
that a type carrying an index the solver cannot read is never built and reflexivity holds by construction.

## Read

- `docs/rules/language/02-core-calculus.md` §1.5, twice and for two different sentences. Under *What this is not*: "An
  index position holding anything else — `Row(f(x))`, `Row(match … )` — is the refusal below, **named at the
  expression**." Under *The refusals*, first bullet: an index outside the grammar "is not assumed, not postponed, and
  not compared syntactically as a fallback." The specification already sites the refusal at the written expression. 142d
  sited it at the comparison, and this prompt closes that drift — it is not an amendment, and §1.5 needs no edit.
- `crates/musa-calculus/src/convert.rs`, `Conversion::indices` — the current site. Its comment argues that refusing here
  is right because a syntactic fallback "would have decided an index question by a rule that is not arithmetic." That
  argument is wrong twice over, and the prompt turns on seeing why: syntactic agreement *implies* semantic agreement, so
  a fallback would be incomplete rather than unsound; and the choice was never fallback-or-refuse, because a third
  option — refuse earlier — makes the comparison total.
- `crates/musa-calculus/tests/suite/index_laws.rs`, the three tests this prompt replaces:
  `an_index_outside_the_grammar_is_refused_rather_than_compared_syntactically`,
  `two_variables_multiplied_leave_the_fragment`, and `a_literal_of_an_unmeasured_base_type_is_not_an_index`. Each
  asserts that some type is **not** convertible with itself. They were written at 142d, they faithfully test what 142d
  built, and what they record is the defect.
- `crates/musa-compiler/src/lower/types.rs`, around the `Raw::refine` call — the one place the surface builds an indexed
  type — and `crates/musa-calculus/src/declare.rs` where a declared signature carries one.
- Xi and Pfenning (1999), "Dependent types in practical programming", §3–4. In DML an index expression outside the
  constraint domain makes the *type* ill-formed; it is rejected at type formation, and constraint solving is never
  handed something it cannot express. That is the discipline this prompt restores, and it is why the solver in a DML
  implementation is total on what reaches it.

## Design

**The refusal moves to where the type is built, and nothing else moves.** Elaborating an indexed type reads its index
into §1.5's grammar and refuses at the written expression when it cannot. The reading is the one that already exists —
the index is elaborated and evaluated exactly as it is today, and `index_of` is asked for its linear form — so this is a
change of *site*, not a second reader. Two readers would be the drift this prompt is closing, one step down.

Evaluating before reading is deliberate and is what keeps `def twelve = 12; Row(twelve)` working: δ is part of how an
index expression is read, so a definition standing for a literal is in the grammar, and only what is still outside it
after evaluation is refused.

**`Conversion::indices` then has no out-of-grammar arm.** Both sides are in the fragment by construction, so
`index::decide` is total on what reaches it and the `let (Some(mine), Some(theirs)) = … else { disagree }` fallthrough
goes away. With it goes the only path by which two identical types could be found different.

**An unsolved hole is unchanged.** An index still mentioning a hole is §2.1's question, asked of the ordinary walk, and
an index variable that no written argument determines is §2.1's existing refusal. Neither is this prompt's, and neither
becomes a postponement: the type is formed once its index is solved, and §2.1 already refuses the case where it is not.

**No syntactic fallback is added, and §1.5's first refusal is honoured rather than weakened.** There is nothing to fall
back *for* once the ill-formed type cannot be built. That is the difference between this fix and the lenient one §1.5
refuses, and it is worth stating in the code where the old comment stood.

**The three tests are replaced, not deleted.** What each of them was reaching for is a real refusal, and it still fires
— one step earlier, at the expression, with a better message. Each becomes a type-formation refusal asserted at
elaboration, naming the expression it refused. Beside them goes the law they were the negation of: every well-formed
type is convertible with itself.

## Target

- `crates/musa-compiler/src/lower/types.rs` and `crates/musa-calculus/src/declare.rs`: forming an indexed type reads its
  index into §1.5's grammar and refuses at the written expression, naming it.
- `crates/musa-calculus/`: whatever narrow surface that reading needs to be callable at type formation; `convert.rs`'s
  `indices` loses its out-of-grammar arm and its defending comment, and gains a note on why the comparison is total.
- `crates/musa-calculus/tests/suite/index_laws.rs`: the three non-reflexivity tests replaced by their type-formation
  counterparts, plus `conversion_is_reflexive_at_every_well_formed_type` over the same registry.
- A diagnostic fixture for `Row(mystery n)` showing the refusal at the expression rather than at a comparison.

## Check

```sh
cargo nextest run -p musa-calculus -p musa-compiler
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

The prompt's own check is that no test in the workspace asserts a type is inconvertible with itself:

```sh
! grep -rn 'against itself' crates/musa-calculus/tests/
```

## Stop

- **No syntactic fallback in conversion.** §1.5's first refusal stands; this prompt makes it unnecessary to weaken, not
  optional.
- No change to §1.5's grammar. What is in the fragment and what is not is 142c's decision and is untouched.
- No amendment to any document under `docs/rules/`. §1.5 already specifies this behaviour; if implementation evidence
  says otherwise, that is a repair to *this* prompt and a stop, not an edit to §1.5.
- No postponement, no approximation, no assumption of an unread index — the three §1.5 names.
- No identity type, no proof term, no evidence that an index equality held.
- Do not restate conversion, rename `Shape::Refine`, or fix the strategy. That is 142db.

Commit as `Refuse an unreadable index where it is written`.
