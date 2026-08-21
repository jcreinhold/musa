---
id: 127dcea
slug: exact-time-arithmetic
status: done
depends_on: [127dce]
phase: 3
---

# Give the Source Language Exact Time and the Arithmetic to Compute With It

## Task

Implement the exact-time algebra the governing calculus already states: coordinate-tagged `Duration[C]`, a new
`Position[C]`, the δ-builtins that compute with them and with `ratio` and `nat`, and the two track operations a package
needs to lay music end to end and ask how long it is. Discharge each new operation against §5.8's D1–D4 rather than
assuming it.

## Read

- `docs/rules/language/02-core-calculus.md` §1, lines 29 and 37–40 — `ratio`, `Duration[C]`, `Position[C]`, and
  `EventTrack[C, δ]` are already in the governing type grammar, and `C` already ranges over `WrittenTime` and
  `PhysicalTime`. This prompt implements that grammar; it does not decide it.
- The same file's commentary at lines 55–68, which fixes the algebra completely: `ratio` is exact and never a float;
  `Duration[C]` rather than a bare rational because a bare rational carries neither the coordinate nor the
  nonnegativity; positions are the abelian group `(ℚ, +, 0)` and durations the ordered monoid `(ℚ≥0, +, 0)`; a position
  plus a duration is a position, two durations add, **two positions do not add at all**, and their difference is a
  duration only when it is nonnegative, "so that difference returns `Result`".
- The same file's line 119 — ordinary signed arithmetic is `ratio`, "with the refinements at its constructors" — and
  line 191, which reserves `EventTrack` construction to compiler-owned builtins.
- §5.8 in full: the four disjoint builtin families, D1 inertness / D2 totality / D3 purity / D4 finiteness, Theorem 5,
  and its last paragraph — the implementation *carries* the premises in the ownership registry and its law suite rather
  than trusting them.
- §5.7, Lemma 1 — `follow` has duration `Σᵢdᵢ` and `together` duration `maxᵢdᵢ`, both with the same occurrence-count
  sum. `together` is implemented today and `follow` is not.
- `crates/musa-compiler/src/core/mod.rs`: `Type` at line 432, where `Duration` is untagged and there is no `Position`;
  the `Builtin` enum and `BUILTIN_OWNERSHIP` around line 2300, where `interval_add` is the only arithmetic-shaped entry
  in the whole registry.
- `crates/musa-compiler/src/time.rs` and the note, stretch, and shift paths that consume today's untagged duration.
- Prompt 127ca, which made the ownership registry the single place a compiler-owned name is declared, and prompt 127c,
  which owns event tracks proper.

## Design

**This is implementation of a governing document, not an amendment.** Every decision below is already written in
`docs/rules/language/02-core-calculus.md`; the prompt's work is to make the compiler say it. If implementation evidence
contradicts the specification, that is stop condition 4 and the amendment procedure, not a quiet divergence.

**The coordinate is a closed nominal index, not a new kind.** `C` ranges over exactly `WrittenTime` and `PhysicalTime`,
so `Duration[WrittenTime]` and `Duration[PhysicalTime]` are two base types that happen to share a spelling, and each
operation is registered once per coordinate. A genuine coordinate kind with coordinate-polymorphic builtins is real
machinery for a two-element domain, and Theorem 5 applies verbatim to a finite family of inert leaves without it.
Promoting the index to a kind later is additive, because the tags are already written down and nothing depends on their
being unquantifiable.

**The arithmetic is named operations, not operators.** The grammar has no binary-expression node, and `Minus` and
`Slash` are already spoken for by duration and pitch literals. Adding infix arithmetic is a surface change and belongs
to prompt 127e's one break, where the grammar, tree-sitter grammar, formatter, corpus, and book move together. A named
δ-builtin needs none of that and is what §5.8 is written about.

The operations, each a δ-builtin whose signature contains no arrow. The coordinate `C` is written `<WrittenTime>` or
`<PhysicalTime>` in source, because the surface applies a type with angle brackets — `Option<τ>`, `Machine<K, A, B>` —
and §1's `[C]` is the paper's notation, not this language's.

```text
ratio_add, ratio_sub, ratio_mul, ratio_div : Ratio, Ratio -> Result<Ratio, Text>
ratio_less, ratio_equal                    : Ratio, Ratio -> Bool
nat_add, nat_mul                           : Nat, Nat -> Result<Nat, Text>
nat_sub                                    : Nat, Nat -> Option<Nat>
duration_of                                : Ratio -> Result<Duration<C>, Text>
duration_ratio                             : Duration<C> -> Ratio
duration_add                               : Duration<C>, Duration<C> -> Result<Duration<C>, Text>
duration_scale                             : Duration<C>, Ratio -> Result<Duration<C>, Text>
duration_less, duration_equal              : Duration<C>, Duration<C> -> Bool
position_of                                : Ratio -> Position<C>
position_ratio                             : Position<C> -> Ratio
position_shift                             : Position<C>, Duration<C> -> Result<Position<C>, Text>
position_between                           : Position<C>, Position<C> -> Result<Duration<C>, Text>
position_less, position_equal              : Position<C>, Position<C> -> Bool
```

There is deliberately no `position_add`. Adding two positions is the one arithmetic error a tagged rational exists to
catch, and the way to forbid it is to have no name for it — not a refinement checked at a constructor. The law that says
so reads the registry rather than trying the spelling, because a spelling test would pass the day someone added the
operation under another name.

**Every arithmetic operation answers with a `Result`, and that is D2 rather than caution.** Exact values are reduced
`i64` rationals, so two representable operands can have an unrepresentable sum: making addition total would mean a panic
or a stuck term, which D2 forbids outright. So representability joins the refinements — a nonnegative duration, a
nonzero divisor, a difference whose second operand is not earlier — and each is a *different* answer, which is why the
error half is `Text` carrying the operation's own sentence rather than a bare absence. `nat_sub` is the one exception
and uses `Option`: going below zero is the only way it can fail, so there is nothing to distinguish it from.
`position_of` and the projections are total, because a position is signed and a projection loses nothing.

**Comparison is `_less` and `_equal`, not a three-way `Ordering`.** There is no three-way type in the language, and
adding one would mean a new inert base with literals nobody can write and a `match` that cannot name its cases. Two
Boolean answers are total, need no new type, and every other ordering question derives from them.

**No track operation.** `follow` and `track_duration` would have to be built on the contextual `Music` value that prompt
127e deletes, and the packages this prompt unblocks do not need them: a staff or studio package folds over its *own*
recursive data with the operations above, and whether the result becomes a `Music` or an `EventTrack` is the cutover's
question. Track construction stays with prompts 127c and 127e, where the track type is settled.

**The premises are carried, not asserted.** Each new operation gets its `BUILTIN_OWNERSHIP` entry naming its family and
signature, and the existing law suite is extended so that the D1–D4 checks §5.8's last paragraph describes actually run
over them: classified exactly once, no arrow in a δ signature, every base type reachable from a δ signature inert and
undestructurable, and evaluation over a documented sample returning a value of the declared type without panicking or
reporting a Rust-level absence at a non-`option` result.

**Migrating today's untagged `Duration`.** It becomes `Duration<WrittenTime>` at every existing use — note literals,
`stretch`, `shift`, the elaborator, the events text. This is the part with reach, and it is why the trials do not do it
in passing. A written duration that turns out to be physical is a bug this tagging exists to find; if the migration
surfaces one, fix it and say so in the commit rather than widening a type to make it go away.

**The bare word names no type.** `Duration` written without a coordinate is refused, with the spelling that works in the
help line. Admitting it as an alias for written time would make the vocabulary offered, the vocabulary read, and the
vocabulary printed three vocabularies — a composer would write `Duration` and be answered about `Duration<WrittenTime>`
— and `musa-compiler`'s `every_offered_type_name_is_read_and_written_the_same_way` is the law that says so. The one
exception is the legacy `motif` parameter list, whose types are a fixed word list rather than the type grammar, and
which prompt 127e deletes.

**Only written time is registered.** Every operation is registered at `WrittenTime`, because that is the only coordinate
the source language can construct a value of; §5.7 fixes it as the score side's. `PhysicalTime` exists in the type so
that the day a physical duration reaches the source it arrives as a *different type* rather than as the same one with a
different meaning. Registering operations for a coordinate nothing can make would be names nothing could call.

## Target

- `Duration<C>` coordinate-tagged and `Position<C>` added in `crates/musa-compiler/src/core/mod.rs`, with every existing
  duration use migrated to `WrittenTime`, and the coordinate carried in the exact encoding.
- The twenty-one δ-builtins above, each with its `BUILTIN_OWNERSHIP` entry.
- The D1–D4 law suite extended to cover them — the sampling law reaching every one of them from the seeds — and a law
  that no compiler-owned operation takes two positions and answers with one.
- Tests: each operation's value law; each partial operation's refusal (`ratio_div` by zero, negative `duration_of` and
  `duration_scale`, reversed `position_between`, `nat_sub` below zero); comparison without subtraction; and that
  `Duration<WrittenTime>`, `Duration<PhysicalTime>`, and `Position<WrittenTime>` are three types that do not unify.
- `docs/book/src/reference/language.md` naming the operations and the two time types, and the generated fixtures
  regenerated where a signature moved.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-events -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-events -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Give the source language exact time and the arithmetic to compute with it`.

## Stop

- No infix operators and no binary-expression node; prompt 127e owns surface changes.
- No `EventTrack` type in the source language and no track constructor at all; prompt 127c owns tracks and prompt 127e
  owns the cutover.
- No deletion of contextual `Music` and no corpus migration.
- No coordinate kind, no coordinate variables, and no third coordinate.
- No staff or studio package; prompts 127dcf–127dcg carry those.
- No change to `docs/rules/`.
