---
id: 127dcfah
slug: printed-literals
status: done
depends_on: [127dcfag]
phase: 3
---

# Give the Language the Two Operations a Printer Needs

## Task

A printer answers with `Text`, and the source language has exactly one text operation: `text_equal`. Nothing anywhere
*builds* a text. A printer can therefore write the literals it holds and pass along a text it was handed, and that is
all — which is enough for a fixture whose value is a text, and enough for nothing else. Prompt 127dcfb asks the staff
adapter to write `instrument "bb_clarinet"`, `key d major`, `time (4, 4)`, and `c5(3/8)` out of a `StaffDocument`, and
there is no operation in the language that turns the `4`, the `3/8`, the `c5`, or the key into the text that names it,
and none that joins the pieces once it has them.

This prompt closes that gap and nothing else: one operation that joins texts, and, for each base type a written region
names, the source literal that names a value of it.

## Read

- `docs/notes/research/language-design-closure/26-language-design-decision.md` §4 — `print_A`, `PrintLoss_A`, and the
  round-trip law the spellings below have to make reachable.
- Prompt [127dce](127dce-adapter-print.md) — where `print : A -> Result<Text, Text>` was fixed, and its sentence "There
  are no text operations: a printer can write the literals it holds and can pass along a text it was handed, and that is
  all". That was true when it was written and is what this prompt changes; the *reason* it gave for `doubled` staying
  editable is not, and survives in a sharper form.
- `docs/rules/language/02-core-calculus.md` §5.8 — the four builtin families, conditions D1–D4, Theorem 5, and the
  corollary that says a new operation over existing base types costs a registry entry and a discharge rather than a new
  induction. **This prompt adds no base type**, so the corollary applies unchanged and no governing document moves.
- `docs/rules/constitution.md` on plural, theory-owned presentations — the reason a compiler-owned `pitch_literal` is
  not a presentation. How a theory *displays* a pitch is a package's business; how the *language* writes one down is the
  lexer's, and this is the lexer read backwards.
- `crates/musa-compiler/src/pitch.rs` — `WrittenPitch::parse` and its `Display`, which are already inverse, and
  `PitchClass`'s the same. `crates/musa-compiler/src/origin.rs`'s `Display for Interval`, which falls back to `(d, s)`
  when no written name fits — the one place a spelling genuinely runs out. `crates/musa-compiler/src/score.rs`'s `Key`.
- Prompt [127dcfa](127dcfa-staff-expansion.md) and `examples/staff-page.musa` — what a staff region actually spells, so
  that the round-trip law is against the reader that already exists rather than against a second grammar.

## Design

**One join, not a concatenation.**

```text
text_join : List<Text> -> Text
```

A printer builds a sequence of pieces and joins it once. A binary `text_concat` folded over `n` pieces allocates `n`
intermediate texts to produce one answer and charges the §4 meter `O(n²)` bytes for an answer of size `O(n)`; joining
charges the sum, once, before it builds. There is no separator parameter: a caller that wants separators puts them in
the list, and a parameter every caller passes `""` is a knob rather than an interface.

**One spelling per base type a written region names**, each answering the source literal that names the value:

```text
nat_literal      : Nat      -> Text
ratio_literal    : Ratio    -> Option<Text>
pitch_literal    : Pitch    -> Text
key_literal      : Key      -> Text
interval_literal : Interval -> Option<Text>
```

The rule is a single rule and it is checkable: **reading back what one of these answers gives the value it was handed.**
That is what makes the family the reader's inverse rather than a presentation, and it is what the law suite asserts, per
operation, over a generated sample. A spelling that does not round-trip is wrong in the only sense the family has.

Two answer an `Option`, because the grammar's literals run out before the values do. `ratio_literal` has nothing to
write below zero: there is no negative numeric literal, and `ratio_sub(1/3, 3/2)` reaches one. `interval_literal` runs
out twice over — an interval outside the named size-and-quality grid has no spelling, and neither does a descending one,
which the reader writes with a `down` that no single literal token carries. D2 requires that partiality to sit in the
result type rather than in a stuck term or a fabricated pair. It is also what gives prompt 127dcfb's `PrintLoss` a
subject that is real rather than staged — a document a staff genuinely cannot write down, rather than one arranged to be
unwritable.

Which of the five are partial is settled by the law and not declared ahead of it: the implementation spells the value
and reads it back, and answers nothing where the reader will not take it. That is why `interval_literal` needs no second
copy of the reader's interval grid to consult.

**Nothing here reads a text.** No length, no index, no substring, no split, no ordering. A program can compare two texts
and can build one; it cannot take one apart. That asymmetry is deliberate and it is the same rule as the phase's: an
adapter may not read a source range, and a language that could take a text apart would let a package read the region it
was handed from the other side. Writing is the capability a printer needs; reading is the one nothing has asked for.

**The D1–D4 discharge**, in one paragraph each in the registry's own doc comments, since the registry is where §5.8 says
the premises live. D1: no new base type, and `Text` is already inert. D2: every operation is total on closed values of
its argument types, with the two partial cases in an `option`. D3: the answer is a function of the argument value alone.
D4: the answer's byte count is bounded by the sum of the argument's, charged to §4's constructed-bytes meter before
construction begins — which for `text_join` means preflighting the sum of its pieces, and for a spelling is a bound on
the value's own size that the ordinary charge on a built value already covers.

## Target

- The six operations in `crates/musa-compiler/src/core.rs`'s builtin-ownership registry, classified as δ-builtins with
  their declared signatures, and evaluated.
- Law tests: the family classification laws still pass with the six added (four families, disjoint and exhaustive, no
  arrow in a δ signature); the round-trip law per spelling over a generated sample; `text_join` over the empty list, one
  piece, and many; `interval_literal` answering nothing for an interval with no written name, with that interval
  exhibited rather than described.
- `stdlib/src/adapters/doubled.musa`'s level comment repaired: it stays *editable*, and the reason is that a `Music`
  value has no literal that names it — not that the language has no text operations, which after this prompt is false.
- `docs/book/src/reference/language.md`'s operation table extended with the six, and the paragraph under it saying what
  the family is for and why it does not read.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Give the language the two operations a printer needs`.

## Stop

- No text reading of any kind: no length, no index, no substring, no split, no ordering, no case folding.
- No new base type. Every operation here is over base types the language already has.
- No spelling for `Music`, `Score`, or any value that has no literal naming it. Writing a musical value down is what an
  adapter's `print` is for, and what the package that owns the value decides.
- No printer, no `print` change, and no staff work; prompt 127dcfb owns all three.
- No change to `docs/rules/`. §5.8's corollary covers this addition exactly; if a spelling turns out to need an
  amendment, that is a stop and not a repair.
