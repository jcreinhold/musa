---
id: 151
slug: delete-the-index-stratum
status: in-progress
depends_on: [147]
phase: 3
---

# Delete the Index Stratum and Restore Read-Back Equality

## Task

Prompt 143 reversed 142c's amendment and retired §1.5 from the governing specification. The code still carries the
stratum it described: an `Indexed` shape, a `Form::Indexed` value, `kernel/index.rs` (453 lines of linear-form solver),
an erasure special case in `quote`, four refusals, one malformation, and the base-type and builtin decorations that feed
them. Delete all of it. Nothing is put in its place, and the governing documents already say so — this prompt is the
code catching up with `docs/rules/`, not a change to it.

## Read

- [`142c-index-amendment.md`](142c-index-amendment.md) and its reversal banner, [`143`](143-one-theory-amendment.md) —
  what was admitted, what retired it, and the two findings that did.
- [`142d-index-stratum.md`](142d-index-stratum.md) — the implementation this deletes, so the deletion can be checked
  against what was added rather than against a grep.
- `docs/rules/language/02-core-calculus.md` — §1.1 through §1.4, and the absence of a §1.5. The section list is the
  evidence that this prompt changes no governing document.
- `crates/musa-calculus/src/elaboration/convert.rs`, the `Form::Indexed` arms and `indexed_shown`.
- `crates/musa-calculus/src/kernel/quote.rs`, `Form::Indexed { ty, .. } => read_type(meter, reading, ty)` — the index
  dropped.
- `crates/musa-calculus/src/kernel/base.rs`, `kind: Term` — the machinery that already does what the stratum was added
  for — beside `indexed_by`, `measuring`, `reads_an_index`, `Operator` and `Builtin::indexing`, which are the
  decorations that go with it.
- `crates/musa-compiler/src/lower/types.rs`'s `indexed_type`, `lower/laws.rs`'s two index laws, `lower/refusals.rs`'s
  four arms, and `registry.rs`'s `measuring(exact_index)` and `indexes` — the whole of the compiler's stake.

## Design

**Nothing in the product uses it.** This is the finding that makes the deletion small, and it is not what prompt 142d
left behind on paper. `Pc12`, `PcSet12` and `Row12` are registered as *plain storable base types* and always were —
there is no `Pc(12)` in `stdlib/` or `examples/`, no `Pc` awaiting a numeral, and nothing to re-register. **No base type
anywhere calls `indexed_by`**, which `lower/laws.rs` states as a law without meaning to: its
`an_index_written_in_a_type_reaches_the_core_as_the_type_it_forms` writes `Nat(12)` and `Ratio(3/4)` and asserts both
are refused, *because neither head declares an index and none does*. The stratum's only reachable behaviour is its
refusals and its own test suite.

**The count is discharged by machinery already present.** 142c cited 17 of 121 builtins hardcoded to modulus 12. A
`Base` already carries `kind: Term`, so a base type may take parameters — `Nat : Type 0` and `Syntax : Type 0 → Type 0`
are both base types today. `Pc : Nat → Type 0` needs nothing new. The 17 builtins collapse at prompt 164, which is why
that prompt's dependency moves here.

**The solver's one unique buy is unused.** Index *arithmetic* — `Bar(p+q)`, a bar whose length is the sum of its parts —
is the only thing a Presburger solver does that application and NbE do not. **Zero committed `.musa` files use it**, and
142c's own Design already conceded that `Bar(m)` is checkable only when the durations are static, which is the case
application handles.

**This is the prompt that makes §3 true.** §3 says `A ≡ B iff quote(A) = quote(B)`, and the erasing wrapper made that
false about the implementation: `quote` dropped the index, so the equation as written said `Pc(12) ≡ Pc(24)`, and the
code kept the two apart only by comparing `Form::Indexed` structurally on *values*, before quoting — a conversion rule
read-back cannot decide. With no erasing wrapper the equation holds as written and `convert.rs` loses arms rather than
gaining any.

**Erasure needed no replacement.** §1.5's erasure protected byte-identity of stored artifacts. Identity digests event
tracks, bindings, seed and options — never a core term — so no artifact contains a type and nothing was protected.
Prompt 144 already wrote this down; this prompt is where it is relied on.

**The `T(i)` spelling stays, and lowers to an application.** `IndexedType` is a parser node, an AST wrapper, a
highlight class and a tree-sitter rule, and deleting it would spend this prompt on the grammar and the drift law for no
semantic gain. It is also the spelling prompt 156 will want for a family with a `Nat` index, which is where Stop below
sends anything that turns out to need arithmetic. So `Lowering::indexed_type` produces `Raw::app` instead of
`Raw::indexed`, and `Pc(12)` *is* `Pc 12` — which is Design's sentence above, applied at the one place the spelling
occurs. What changes for an author is only which sentence refuses a head that takes no argument.

**Four refusals and one malformation go with it.** `Refusal::UnreadableIndex`, `NotIndexed`, `MissingIndex` and
`NotAnIndexSort`, their four `Code` entries and their four arms in `lower/refusals.rs`, and
`Malformed::UnreadableIndex`. This is a diagnostic-surface deletion and the snapshots move; that is the deletion working
rather than a violation of anything, because every one of these sentences is about a stratum that no longer exists.

## Target

- `crates/musa-calculus/src/kernel/term.rs`, `value.rs`, `elaboration/raw.rs`: `Shape::Indexed`, `Form::Indexed` and
  `RawShape::Indexed` removed, with their constructors.
- `crates/musa-calculus/src/kernel/index.rs`: deleted.
- `crates/musa-calculus/src/kernel/{quote,eval,recheck}.rs` and `elaboration/convert.rs`: the erasure case, the
  structural arms and `indexed_shown` removed.
- `crates/musa-calculus/src/kernel/base.rs`: `indexed_by`, `declared_index`, `measuring`, `reads_an_index`, `Measures`,
  `Operator`, `Builtin::indexing` and `indexes` removed. `Accepts` **stays** — it reads a base type *applied* to a
  literal, which is §5.9's expansion phase and not §1.5's stratum.
- `crates/musa-calculus/src/elaboration/refuse.rs`, `kernel/error.rs`: the four refusals and the one malformation
  removed.
- `crates/musa-compiler/src/lower/types.rs`: `indexed_type` lowers to an application.
- `crates/musa-compiler/src/lower/refusals.rs`, `diagnostic` codes: the four arms and their codes removed.
- `crates/musa-compiler/src/registry.rs`: `measuring(exact_index)`, `exact_index` and `indexes` removed. `Pc12`,
  `PcSet12` and `Row12` are untouched — they are plain base types and 164 is what collapses them.
- `crates/musa-calculus/tests/suite/index_laws.rs`: deleted, with its `mod` line. `malformed_laws.rs` and
  `elaboration_laws.rs` lose the cases for the variants that went.
- `crates/musa-calculus/tests/suite/conversion_laws.rs`: a law stating that `Pc 12` and `Pc 24` are not convertible
  *and* that read-back distinguishes them — which is the equation §3 states, now decidable by the rule §3 names.
- `crates/musa-compiler/src/lower/laws.rs`: the two index laws replaced by one that `T(i)` reaches the core as the
  application it now is.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! test -f crates/musa-calculus/src/kernel/index.rs
! grep -rn 'Indexed' crates/musa-calculus/src
git diff --stat crates/musa-calculus    # must be net negative by ~600 lines
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

`--unreferenced=reject` will have snapshots to accept, unlike prompt 150: four diagnostics are deleted, and any snapshot
that held one moves. Accept only snapshots whose change is a deleted sentence; a snapshot whose *wording* changed is
this prompt doing something it did not say it would.

The re-checker prompt 149 built must accept everything this prompt elaborates, and its arm for an applied type
constructor is what replaces the `Indexed` arm it loses. This is prompt 149's standing obligation and prompt 158 audits
it.

Commit as `Delete the index stratum`.

## Stop

- No collapse of the seventeen builtins (164), no universe change (152), no families (155).
- No new solver of any kind. If an index turns out to need arithmetic, that is a family with a `Nat` index and prompt
  156's business — not a decision procedure beside conversion.
- No grammar change. `IndexedType` stays a node, `editors/tree-sitter-musa` is untouched, and the drift law stays green
  without being asked to.
- No reworded diagnostics. Sentences are deleted here, never rephrased.
