---
id: 141s
slug: numeral-representation
status: done
depends_on: [135, 141b]
phase: 3
---

# A Numeral Is One Node, Not a Tower

## Task

`Nat` is declared as `data Nat { Zero, Succ(Nat) }` ([`prelude.rs`](../../../crates/musa-compiler/src/prelude.rs)), so
[`lower.rs::whole`](../../../crates/musa-compiler/src/lower.rs) turns a written number into that many nested
applications. The doc comment there claims the numbers are "small by construction". They are not: `repeat 384` writes
384, a whole note over a 384-tick division writes 384, and the term is 384 applications deep before anything evaluates
it.

Two things follow, and both are observable today. `every_example_elaborates` refuses with
`ResourceLimit: … nested evaluation levels at 257 of 256`, and the depth is the number the composer wrote — so no limit
a version bump could name would fix it. And `resource_validation`'s `nat_fold(0, keep, 50000)` does not refuse at all;
it aborts, because a fifty-thousand-node left spine overflows the host stack while it is being *built*, before any
budget has a chance to speak.

Give the core a representation for numerals: the family and its constructors unchanged, and a literal that splits into
`Zero`/`Succ` only where an elimination forces it. Prompt 142's **Target** records this finding and its **Stop** forbids
adding a language feature under a migration diff, which is why it is a prompt of its own and why it comes first.

## Read

- [`02-core-calculus.md`](../../rules/language/02-core-calculus.md) §1.1 (declared constants and their roles), §2.4
  (definitions and the measure), §4.1 (the evaluation budget and its metrics), §5.8 (base types as a conservative
  extension — the precedent for a value the core does not build out of constructors).
- [Prompt 135, inductive families](135-inductive-families.md) — generated recursors and ι-reduction, which is the rule
  that has to learn to unfold.
- [`141b-base-types-and-builtins.md`](141b-base-types-and-builtins.md) — how the core already admits a shape it cannot
  take apart, and why that door is the wrong one here.
- Peyton Jones ch. 4 §4.1–§4.3 — a structured type's *representation* is a separate question from its constructors, and
  `case` is what has to agree with the representation. That separation is the whole of this prompt. Ch. 10 §10.1 makes
  the same point one level down: a program's representation is chosen for the evaluator that walks it.
- Lean 4's kernel, `~/Code/lean4/src/kernel/` — the same design in a shipped dependently typed kernel, read *after* the
  design above rather than as its source, and cited because a decision two systems reach independently is checkable in a
  way an assertion is not. `type_checker.cpp`'s `reduce_nat` is the collapse; `inductive.h`'s major-premise handling is
  the one-level unfold; `is_def_eq_offset` is conversion without a walk. `inductive.cpp`'s `nat_lit_to_constructor` is
  the exact shape of ι here: a literal major premise becomes `Nat.zero` or `Nat.succ (lit (v-1))`, one node.

## Design

### Why `Nat` does not become a base type

The cheap answer is to delete the family and register `Nat` as a base type with a `u64` literal, the way 141b did for
`EventTrack` and `Origin`. It is the wrong answer three times over. 141b's own rule refuses destructuring at a
base-typed column, so every `match n { Zero, Succ p }` in the corpus stops compiling.
[`rec.rs`](../../../crates/musa-calculus/src/rec.rs) derives its measure from a constructor field, so no function could
recurse on a number. And `Vec A n` indexes a family by a `Nat`, so a base-typed `Nat` would put the length outside the
language that reasons about it. A base type is for something the core has no rules about; numbers are something the core
has *every* rule about, and only their storage is at issue.

### A counting family, derived rather than nominated

`declare_data` recognizes a **counting family**: no parameters, no indices, exactly two constructors, one with no fields
(its *floor*) and one with exactly one field whose type is the family itself (its *step*). That is precisely the shape
for which "how many steps above the floor" is a complete description of a closed value, which is why the condition is
the same sentence as the soundness argument.

Deriving it beats nominating it. A `Registry` row would mean the host names the family — but a family only exists after
`declare_data` has run, so the nomination would have to happen afterwards, and a call order enforced by convention is
exactly the coupling the core does not have anywhere else. Deriving costs no parameter, no ordering, and no host
knowledge, and any user-declared family of that shape gets the representation for free.

### One canonical form, not two

`Shape::Numeral { family, count }`, with `count: u64`, and a matching value. The temptation is to let the numeral and
the `Succ`-spine both be normal forms and teach conversion to unfold one against the other. Do not: canonicity (148)
wants one normal form per value, and "unfold to compare" makes conversion at numerals linear again in the number the
composer wrote.

Instead, **normalization collapses toward the numeral**. Applying the step constructor to a numeral value yields the
numeral one higher; the floor constructor evaluates to the numeral zero. At a counting family there is then no
`Succ`/`Zero` spine in any *value*, ever — the spine survives only in raw and elaborated input, which is where an author
writes it. Conversion is `count == count`, in constant time.

The step applied to a *neutral* is still a neutral spine, and needs no special case: it is the ordinary rule, and a
neutral is not a closed numeral, so a numeral and a neutral are simply unequal. Partial application needs no case either
— the step constructor unapplied is the constant it always was, and the collapse lives in `apply`.

### Where the tower reappears

Exactly at an elimination, one level at a time. ι on the generated recursor reads the count: zero takes the base arm,
`k` takes the step arm applied to the numeral `k - 1` and to the hypothesis for it.
[`case.rs`](../../../crates/musa-calculus/src/case.rs) splits a column at a counting family the same way. One unfold,
one level, one step charged — so a fold over a numeral `n` costs what a fold over a tower of height `n` costs, and
*nothing else does*.

### Cost, and the limit that stays

Building a numeral is one step and one nesting level whatever the count, which is the entire point: §4.1's nesting
metric measures how deep the evaluator recurses, and a number is no longer deep. The step budget still bounds a fold
over a large numeral, and should — `nat_fold(0, keep, 50000)` is fifty thousand steps of real work and refusing it is
the correct answer. What changes is that it *refuses* rather than aborting. Prompt 7's budget-independence law must
still hold: the numeral's cost is charged the same way under every configuration.

A count that would exceed `u64` must not saturate, and does not need a refusal either: the step constructor stays an
ordinary blocked spine, which is a representation the core already has and already means the right thing. That is rule 5
of `module-design` — a normal result covers the edge case, so no error path is added for it — and the case is
unreachable regardless, since climbing there costs 2⁶⁴ steps and the step budget answers first.

The nesting limit is 256, but a hand-built tower never reaches it. Measured while implementing this prompt: a tower of
210 elaborates and normalizes, and a tower of 220 **aborts the process** — the host stack gives out first, in a debug
build, around 215 levels. So at the full budget the tower's boundary is not §4.1's metric at all; it is the host's, it
is lower, and it is not a refusal a test can assert. `Budget::scaled` only divides, and that is what makes the law
statable: divide the budget and the nesting limit moves *below* the stack cliff, so the exhaustion the calculus owes is
the one that actually happens. Both large-count laws below are therefore stated under a divided budget rather than the
full one. That the guard is calibrated above what a debug stack can survive is a real finding about `Budget::NESTING`
and not about numerals, so it is recorded rather than fixed here.

### A numeral crosses the δ boundary as a number

`Datum` gains a `Count { family, count }` arm, and the readback answers it. The alternative — read a numeral back as the
`count` nested [`Datum::Case`](../../../crates/musa-calculus/src/base.rs)s it denotes — is the same mistake one layer
out and it is worse there than in the term. Worse because a `Datum` tower costs a node per unit *and* recurses on the
host stack when it is **dropped**, so a host asking for a large `Nat` as data would abort where the term never could;
and because [`rules::nat`](../../../crates/musa-compiler/src/registry/rules.rs) exists only to count that tower back
down to the `u64` the core already had. `AGENTS.md`'s "hand a consumer what we already computed" names exactly this.
Note that before this prompt the tree was capped: `canonical` charges §4.1's nesting metric per level, so the deepest
readable `Datum` was 256 — a numeral that bypassed that charge would be a hole in the budget rather than a feature.

Lean has no analogue because it has no such boundary: its event track literal *is* the host datum. Musa's δ-rules read
[`Datum`](../../../crates/musa-calculus/src/base.rs) and never a value, which is roadmap §15.12's privacy boundary, so
the count has to cross it as a count.

### Where Lean 4 agrees, and where this deliberately does not

Three of the four decisions above are Lean's as well, which is worth writing down because the argument for each is then
not merely internal. Lean's `Nat` stays `inductive Nat where zero | succ`, with `Expr.lit (Literal.natVal n)` as a
*representation*; `reduce_nat` collapses `Nat.succ e` at a literal `e` into the literal one higher; the recursor unfolds
a literal major premise exactly one level. Four differences remain, and each is a decision rather than an omission:

1. **One canonical form, not two.** Lean lets `Nat.zero` and `lit 0` both be normal forms and reconciles them at every
   comparison site. A syntactic event track can afford that; NbE cannot, because a value with two shapes makes
   conversion ask the question twice, and prompt 148's canonicity obligation wants one normal form per value.
2. **Derived, not hard-wired.** Lean names `Nat.zero`, `Nat.succ`, and fourteen arithmetic operations as event track
   globals. `musa-calculus` names no family at all; the counting property is read off the declaration's shape, for the
   reason the section above gives.
3. **`u64`, not a bignum.** Lean's literal is arbitrary precision. Overflow here falls back to a blocked spine, and the
   step budget answers long before 2⁶⁴ steps could be climbed.
4. **No arithmetic in the core.** Lean's event track accelerates `add`/`mul`/`div`/`mod`/`beq`/`ble` and eight more,
   because the alternative for a proof event track is unary arithmetic. Musa's arithmetic is a registered δ-rule in the
   *compiler* (prompt 143), so the core keeps no privileged type. Cite Lean there when 143 weighs the same tradeoff.

### The raw layer names the family

`Raw::numeral(origin, family, count)`, with `family` a name, the way
[`Raw::var`](../../../crates/musa-calculus/src/raw.rs) already spells `Nat.Succ`. The compiler is what knows the numeral
family is called `Nat`; the core does not, and should not guess in infer mode or search for a unique counting family in
scope. Elaboration resolves the name and refuses `Refusal::NotANumeralFamily { name, reason }` when the family does not
count, with `reason` naming which of the four conditions failed. `lower.rs::whole` becomes one call and loses its loop.

### The governing document

`02-core-calculus.md` gains a subsection beside §5.8 stating the representation and its obligation: for every closed
count, the numeral is definitionally equal to the tower, so the extension is conservative and no program's meaning
moves. That obligation is a law here, not an assertion — see **Target**.

## Target

- `musa-calculus` recognizes a counting family at `declare_data` from its shape alone, with no host nomination and no
  ordering requirement.
- `Shape::Numeral { family, count }` and its value, threaded through `term`, `value`, `eval`, `case`, `unify`, `quote`,
  `recheck`, `show`, and `storable` — every walker gets a real arm, none gets a `todo!()`.
- Normalization collapses: the step applied to a numeral value is a numeral, the floor is the numeral zero, and no value
  at a counting family ever holds a `Succ`/`Zero` spine.
- ι and `case.rs` unfold one level per elimination, charging one step and one nesting level for that level and not for
  the count.
- `Raw::numeral`, and `Refusal::NotANumeralFamily { name, reason }` with its `musa explain` code and negative program.
- `show` prints `384`.
- `lower.rs::whole` builds one node, and its doc comment stops claiming the numbers are small.
- `Datum::Count`, answered by the readback and realized back to a numeral against the type the signature declares;
  `rules::nat` reads one arm and walks nothing.
- **The conservativity law**: for every count the tower can still be written at — 0, 1, and counts a couple of hundred
  below where the host stack ends it — the numeral and the hand-built tower are convertible, and a `match` on each
  computes the same answer. Past that the law is stated the other way round, because it is the finding: the tower
  **exhausts** where the numeral answers. A tower of depth `n` costs `n` evaluator frames — so "the numeral agrees with
  the tower at 5,000" is not a sentence about two terms, it is a sentence about one term and a term the calculus cannot
  evaluate. Stated under a divided budget, for the reason **Design** measures: at the full budget the tower dies on the
  host stack before §4.1's metric speaks, and a process abort is not a law.
- **The cost law**: elaborating and evaluating a numeral of count `n` charges a nesting depth independent of `n`, stated
  as a test over at least two counts three orders of magnitude apart, under the same divided budget — sixteen nesting
  levels admit a numeral of 5,000 and refuse a tower of 63, which is the whole claim in one pair of numbers.
- **The no-deep-tree law**: constructing and dropping a numeral of count 50,000 neither overflows the stack nor builds a
  term whose depth grows with the count — and neither does reading it back as data, which is the same claim at the δ
  boundary and the one a `Datum` tower would have broken.
- `02-core-calculus.md`'s new subsection, and the `docs/plan/code-map/` rows for the changed files.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus
cargo nextest run -p musa-compiler -E 'test(numeral) or test(whole)'
cargo clippy --all-targets -p musa-calculus -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

`musa-compiler`'s full suite and its clippy run are deliberately not gates here, for the same reason and by the same
measurement. This prompt lands in the middle of 142's migration: that suite is red for migration reasons this prompt
neither causes nor can fix, and `-D warnings` is red on `core.rs`'s superseded checking paths, which 142's own Target
deletes. `check_piece`, `check_arguments` and `check_template_voice` each occur once in the tree — their definition — at
the 142 checkpoint `1f071ea`, before this prompt existed, so the failure is 142's to close and gating on it would be
gating one prompt on another's unfinished work. `musa-calculus`'s suite and clippy are the ones that must be wholly
green, because `musa-calculus` is the crate this prompt changes; the compiler side is checked at the two named
behaviours, and 142's own Check is where the whole corpus answers.

Commit as `A numeral is one node, not a tower`.

## Stop

- **No arithmetic.** `+` on numerals, `nat_add`, and the builtin collapse are prompt 143. A numeral is a representation
  here and nothing more.
- **No conversions between numeric types.** A written number meaning a `Ratio` or a `Position` is 142's own finding and
  142's work.
- **No change to `Nat`'s declaration.** The family and its two constructors are exactly what they are today; if this
  prompt needs them to change, the design is wrong and this is a repair.
- **No change to the step budget or its default.** Three examples exhaust the 200,000-step reduction budget for reasons
  that are not this one, and prompt 144 owns that measurement.
- **No corpus migration.** Not one `.musa` file, fixture, or snapshot moves for this prompt except where a numeral's
  printed form appears in a `musa-calculus` law.
- **No second numeral family.** The compiler writes `Nat` and only `Nat`; that a user could declare another counting
  family is a consequence of deriving the property, not a feature to build a surface for.
