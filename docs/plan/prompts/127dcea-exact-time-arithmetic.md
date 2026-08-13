---
id: 127dcea
slug: exact-time-arithmetic
status: pending
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
- `crates/musa-compiler/src/core.rs`: `Type` at line 432, where `Duration` is untagged and there is no `Position`; the
  `Builtin` enum and `BUILTIN_OWNERSHIP` around line 2300, where `interval_add` is the only arithmetic-shaped entry in
  the whole registry.
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

The operations, each a δ-builtin whose signature contains no arrow:

```text
ratio_add, ratio_sub, ratio_mul     : ratio, ratio -> ratio
ratio_div                           : ratio, ratio -> Result<ratio, text>
ratio_compare                       : ratio, ratio -> Ordering
nat_add, nat_mul                    : nat, nat -> nat
nat_sub                             : nat, nat -> option<nat>
duration_add                        : Duration[C], Duration[C] -> Duration[C]
duration_scale                      : Duration[C], ratio -> Result<Duration[C], text>
duration_compare                    : Duration[C], Duration[C] -> Ordering
duration_ratio                      : Duration[C] -> ratio
duration_of                         : ratio -> Result<Duration[C], text>
position_start                      : Position[C]
position_shift                      : Position[C], Duration[C] -> Position[C]
position_between                    : Position[C], Position[C] -> Result<Duration[C], text>
position_compare                    : Position[C], Position[C] -> Ordering
```

There is deliberately no `position_add`. Adding two positions is the one arithmetic error a tagged rational exists to
catch, and the way to forbid it is to have no name for it — not a refinement checked at a constructor.

Partiality is in the result type in every case, which is D2: `ratio_div` by zero, a negative `duration_of`, a negative
`duration_scale`, a `position_between` whose second argument precedes its first, and a `nat_sub` that would go below
zero all return `Result` or `option`. None of them is a diagnostic, a stuck term, or a panic. `Ordering` is the existing
three-way spelling if the compiler has one and a sum of units if it does not; do not add a fourth comparison convention.

**Two track operations, and no more.** `follow(a, b)` and `track_duration(t)`, both already proved by §5.7 Lemma 1. They
exist because line 191 reserves track construction to builtins, so a package that lays items end to end has no other way
to do it. Everything above them — bar offsets, tie spans, tuplet scaling, pickups — is ordinary package code folding
over ordinary data with the operations above, which is what keeps a staff concept out of the compiler.

**The premises are carried, not asserted.** Each new operation gets its `BUILTIN_OWNERSHIP` entry naming its family and
signature, and the existing law suite is extended so that the D1–D4 checks §5.8's last paragraph describes actually run
over them: classified exactly once, no arrow in a δ signature, every base type reachable from a δ signature inert and
undestructurable, and evaluation over a documented sample returning a value of the declared type without panicking or
reporting a Rust-level absence at a non-`option` result.

**Migrating today's untagged `Duration`.** It becomes `Duration[WrittenTime]` at every existing use — note literals,
`stretch`, `shift`, the elaborator, the kernel text. This is the part with reach, and it is why the trials do not do it
in passing. A written duration that turns out to be physical is a bug this tagging exists to find; if the migration
surfaces one, fix it and say so in the commit rather than widening a type to make it go away.

## Target

- `Duration[C]` coordinate-tagged and `Position[C]` added in `crates/musa-compiler/src/core.rs`, with every existing
  duration use migrated to `WrittenTime`.
- The fourteen δ-builtins above and the two track builtins, each with its `BUILTIN_OWNERSHIP` entry.
- The D1–D4 law suite extended to cover them, including a test that no name spells position-plus-position.
- Tests: each operation's value law; each partial operation's refusal (`ratio_div` by zero, negative `duration_of`,
  reversed `position_between`, `nat_sub` below zero); that `Duration[WrittenTime]` and `Duration[PhysicalTime]` do not
  unify; that `follow` gives `Σᵢdᵢ` and `together` gives `maxᵢdᵢ` on the same operands; and `track_duration` agreeing
  with both.
- `docs/book/src/reference/` naming the operations, and `stdlib/reference.md` regenerated if the generator covers them.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-kernel -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-kernel -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Give the source language exact time and the arithmetic to compute with it`.

## Stop

- No infix operators and no binary-expression node; prompt 127e owns surface changes.
- No `EventTrack` type in the source language and no track constructor beyond `follow` and `track_duration`; prompt 127c
  owns tracks and prompt 127e owns the cutover.
- No deletion of contextual `Music` and no corpus migration.
- No coordinate kind, no coordinate variables, and no third coordinate.
- No staff or studio package; prompts 127dcf–127dcg carry those.
- No change to `docs/rules/`.
