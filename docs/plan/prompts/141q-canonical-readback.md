---
id: 141q
slug: canonical-readback
status: pending
depends_on: [141b, 141e, 141j]
phase: 3
---

# Read Canonical Data Back Out of a Term

## Task

[`crate::registry::read_back`](../../../crates/musa-compiler/src/registry.rs) reads a normal form back into a host type
by looking for one `Shape::Lit` and downcasting it. That is the whole of the readback the language pass has, and it
answers exactly one question: *is this term a closed value of a base type?*

Prompt 142's `assert` needs a second question answered. `assert within_ranges([(c3, c5), (g3, g5)])` writes a
`List<(Pitch, Pitch)>`, and `assert voices(4)` writes a `Nat`. Neither is a base type: a `Nat` is the unary family the
prelude declares, a list is `List.Cons`/`List.Empty`, and a pair is `Pair.Both`. Their normal forms are *constructor
spines*, and `read_back` returns `NotALiteral` for every one of them.

The core already knows how to see a constructor spine as data — [`musa_core::Datum`] is the type, and §5.8's δ hands one
to every builtin rule. What it has no door for is the same reading from a **term**. `eval::canonical` and
`eval::constructed` are private and take a `Value`; `family::constructed`, which knows where a constructor's fields
begin, is `pub(crate)`. A consumer holding a normal form has no way in.

This prompt opens that door, and it opens it in `musa-core` rather than in the compiler for the reason `AGENTS.md`
states: the parameter count of `Pair.Both` is something the declaration already fixed, and a compiler that hardcoded
"two of `Pair.Both`'s four arguments are types" would be re-deriving a fact the core computed — the same shape of
mistake as an adapter re-parsing `3/8` out of a token's spelling.

## Read

- [`docs/rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §5.8, which states D1: what
  canonical data is, and why a δ-rule is a function over data rather than a callback into the evaluator. This prompt
  adds no new answer to that question — it exposes the answer §5.8 already gives, at a second input type.
- [`141b`](141b-base-types-and-builtins.md), which introduced [`musa_core::Datum`], its two arms, and the
  `Literal`/`Base` pair the compiler downcasts through.
- [`141e`](141e-compiler-registry.md), which wrote `read_back` and the `held::<T>` downcast this prompt restates rather
  than replaces.
- [`141ha`](141ha-machine-core.md), which declared `Pair` and argued why its halves stay positional — the reason a
  written product reaches this readback as a `Datum::Case` and not as a record.
- [`142`](142-surface-cutover.md)'s Target row for `assert`, which is the caller this exists for, and its Stop bullet
  forbidding a new language feature under a migration diff — which is why this is a prompt and not four lines inside
  that one.
- Peyton Jones ch. 4, on structured types: a constructor application *is* its data, and reading one back is a projection
  rather than an evaluation. That is why this door needs no context, no meter, and no type.

## Design

**One function, on `Term`.**

```rust
/// The canonical data a normal form denotes, or `None` when it denotes none.
pub fn data(term: &Term) -> Option<Datum>
```

Free rather than a method, and taking a normal form rather than a value, because that is what a consumer of
[`musa_core::check`] holds. It needs no `Cx`: a constructor spine carries its own [`Constant`], and a `Constant` carries
the group that declared it, so where the fields begin is already in the term.

**It is §5.8's `canonical`, at the other input type.** Both walk a head-and-spine, both skip a constructor's parameters,
both refuse a partial application, a λ, a record, and a universe. The difference is entirely that one forces a `Value`
and the other reads a `Term` that is already normal — so this one needs no `Meter`, and cannot fail. `None` means "not
data", exactly as it does for a blocked δ-spine.

The two implementations stay separate rather than one being written in terms of the other. Evaluating a term back into a
value to read it would be a normalization the caller has already paid for, and quoting a value into a term to read it
would be the same trade in the other direction. What they share is `family::constructed`'s question — *which
constructor, and how many of its arguments are parameters* — and this prompt is where that question gets its second
caller and stops being a δ-only helper.

**`read_back` becomes a case of it, not a second path.** `registry::read_back::<T>` keeps its signature and its callers,
and is restated over `data`: a base literal is `Datum::Lit`, and the downcast is unchanged. The compiler must not end
this prompt with two ways to look at a normal form.

**No record arm on `Datum`.** [`Datum`]'s own doc comment says the arm and the signature check that admits it arrive
together, and nothing here wants one: 142 lowers `(a, b)` to `Pair.Both`, so every product a program writes reaches this
readback as a `Case`. A record is still not data and still answers `None`.

**What the compiler adds is host-typed, and small.** `crate::registry` grows the four readings a claim's arguments need
— a `Nat` as a count, a `List` as a vector, a `Pair` as a tuple, and a base literal as its host type — each written once
over `Datum` rather than once per claim. They stay in `registry` because that is where the compiler's half of the
core/host boundary already lives.

## Target

- `musa_core::data`, exported from the crate root, doc-commented with its `None` cases enumerated: a partial
  application, a λ, a record, a record type, a universe, a Π, an identity, a variable, a definition, a family, a
  recursor, and a builtin.
- `family::constructed`'s question answered for a `Term` as well as a `Neutral`, without duplicating the parameter-count
  rule.
- `crate::registry::read_back` restated over `data`, with its behaviour and its callers unchanged.
- In `crate::registry`, the host-typed readings a structured argument needs: a whole number out of `Nat`, a `Vec` out of
  `List`, a pair out of `Pair`, each answering `None` rather than a diagnostic — the caller names what it wanted.
- Laws in `musa-core`: a literal reads back; a saturated constructor reads back with its parameters excluded; a
  constructor one field short does not; a λ, a record, and a universe do not; and — the one that ties the two readings
  together — a term and the value it evaluates to read back to the *same* `Datum`.
- Laws in `musa-compiler`: `Nat`, `List`, `Pair`, and a nested `List<(Pitch, Pitch)>` elaborated from real source and
  read back to the host shapes, so the four readings are proved against what the surface actually builds rather than
  against hand-written terms.
- Rows in [`docs/plan/code-map/`](../code-map/) for the new `musa-core` door and the compiler's readings.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-core -p musa-compiler
cargo clippy --all-targets -p musa-core -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Read canonical data back out of a term`.

## Stop

- No record arm on `Datum`, and no change to what a δ-rule is handed. This prompt reads; it does not widen what data is.
- No `assert` reading, no claim built, no argument checked against a `ParamType`. That is 142's row and it needs this
  door, not the other way around.
- No public `Value` → `Datum`. The one caller holds a term, and a second entry point with no caller is the surface this
  repo's standards forbid.
- No evaluation inside the readback. A term that is not already normal reads back as whatever it literally is, which is
  the honest answer for a projection.
