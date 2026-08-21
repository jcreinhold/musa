---
id: 141d
slug: finite-constructor-builtins
status: done
depends_on: [141b, 141c]
phase: 3
---

# Let a δ-Rule Speak the Finite Constructors

## Task

§5.8's D1 says a δ-builtin's argument and result types are "a base type **or a finite constructor over base types**".
Prompt 141b implemented the first half: a [`Rule`] is `fn(&[&Literal]) -> Option<Literal>`, and a `Literal` is a closed
value of a *base* type. The second half has no mechanism at all, and the table that needs it is the one prompt 142
registers on its first step.

Counted from `crates/musa-compiler/src/core.rs`'s `BUILTIN_OWNERSHIP`: of 92 δ-builtins, **54 answer a base type, 17
answer an `Option`, 12 a `Result`, and 9 a `List`** — 38 of them, 41%, answer a value of a *declared family*. Six more
take a `List` as an argument. Prompt 141 proved `Option`, `Result`, and `List` are ordinary declarations with ordinary
constructors, which is exactly why their values are not literals: `Some(scale)` is a constructor applied to a field, and
no arrangement of `Literal` is one.

Nor can a rule build one by hand. `Constant` has no public constructor and no public accessor anywhere in
`musa-calculus`, so a `fn` pointer cannot name `Some`, `Ok`, or `Cons` even if it wanted to — and it could not capture
the group they come from, because capturing is what D3 forbids. Give δ-rules the finite constructors, so that prompt 142
can register the table it has.

## Read

- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §5.8's D1 in full, and D2 and
  D3 beside it. D1's disjunction is the whole basis of this prompt: the *or* clause was always there, and 141b
  implemented one side of it. Then §5.6, on why a value of a declared family is eliminated by its recursor and
  constructed by its constructors, with nothing else in between.
- [`141b`](141b-base-types-and-builtins.md), for `Rule`, `Literal`, `Payload`, and the argument that a literal's payload
  is opaque — which is what a `Datum` must not undo — and [`141c`](141c-structural-eliminators.md), for the precedent
  this prompt follows: a rule that could not say what it meant got a shape that could, and got its own handle rather
  than a captured one.
- [`141`](141-collections.md), which proved `List`, `Option`, and `Vec A n` are writable as declarations. That proof is
  what makes this a gap rather than a design choice: their values are constructor applications *because* prompt 141 was
  right.
- `crates/musa-compiler/src/core.rs`'s `Shape`, the `delta` registration function, and `BUILTIN_OWNERSHIP` — the shape
  language the host already writes its signatures in (`Base`, `Option`, `List`, `Product`, `Result`), and the 92 entries
  the counts above come from. `Shape::Product` is spelled and used by nothing, which this prompt's Design answers
  explicitly rather than building for.
- `crates/musa-calculus/src/family.rs`'s `Constant`, `Group`, `Declared`, `Constructor`, and `Found::named` — every one
  of them public as a *type* and none of them constructible or readable from outside the crate. That is the wall, and
  whether to open it is this prompt's central decision. Then `element`, which reads a family application off a value
  type, and `iota`'s `ready`, which is where "a constructor's parameters come before its fields" is load-bearing.
- `crates/musa-calculus/src/case.rs`'s split, around `built` — the declaration context, then the parameters, then each
  field as it is assumed, which is the environment a stored field type is read in. Realization walks the same telescope
  in the same order, which is why it needs no new machinery and no index.
- `crates/musa-calculus/src/eval.rs`'s `delta` and `structural` — the two firing conditions, and the one this prompt
  generalizes.
- Peyton Jones ch. 3 §3.2 and ch. 6, on the enriched calculus's constants: a δ-rule is a rewrite over *constructed*
  data, not only over atoms, and ch. 4 §4.1 on the constructor as the introduction form a rule must be able to write.

## Design

**The gap is D1's *or*, and it runs in both directions.** A rule that answers `Option Scale` has to *build* a
constructor application; a rule that takes `List Pc12` has to *read* one. 141b's `Rule` can do neither, and the two
halves are one mechanism because a δ-builtin composed of them is still first-order — `Option` and `List` have no arrow
in them, so D1's premise, §5.8's theorem, and D3's purity are all untouched by admitting them. What changes is only what
a rule may say.

**Canonical data is the firing condition, and it is the core's notion rather than the host's.** δ fires today when every
argument has reduced to a literal. It fires after this prompt when every argument has reduced to *canonical data*: a
literal, or a constructor of a declared family applied to canonical data. That is D1's phrase turned into a predicate,
it is decidable by looking, and it degenerates to the old condition exactly when no declared family is involved — so no
existing registration changes behaviour. A record, a λ, a neutral, or a partially applied constructor is not data, and
leaves the spine blocked, which is what a builtin over an open term must do.

**A rule reads data and answers data, and data is one type.**

```rust
pub enum Datum {
    Lit(Literal),
    Case { constructor: Name, fields: Vec<Datum> },
}

pub type Rule = fn(&[Datum]) -> Option<Datum>;
```

One type rather than a borrowing `Datum` in and an owning `Answer` out, and the reason is not symmetry for its own sake.
A borrowed view cannot be built: the core has to *force* each argument before it can see whether it is canonical, and a
forced value is a new value the borrow would have to outlive — pushing them into a side vector while holding references
into it is the shape of an arena, which is a lot of machinery for two `Arc` bumps per literal. δ already clones every
argument literal today. And a rule that answers one of its arguments unchanged — `option_or`, `result_or_else` — writes
that as `arguments[0].clone()` rather than rebuilding a tree in the other type.

Constructor names are qualified, the spelling [`Constant::name`] already prints and a diagnostic already shows:
`Option.Some`, not `Some`. That is what makes reading and writing the same vocabulary, so passing an argument back is
the identity it looks like.

**Reading needs no type; writing is type-directed.** A value is canonical or it is not, and looking says which, so the
`Datum` a rule receives is built from the argument values alone. Realizing an answer is the other way round: `Some(x)`
does not say what `A` is, and a constructor value carries its parameters before its fields — `iota` reads them by
position — so a value built without them is a value the re-checker would refuse and `List.elim` would take apart wrong.
The result type supplies them. It is the builtin's own signature applied to the argument values, so realization walks
the answer and the type together: at `F p⃗` the answer names one of `F`'s constructors, the parameters are `p⃗`, and each
field is realized at that field's declared type instantiated at `p⃗` and the fields before it. That is exactly what
`case.rs`'s split already does in the other direction, and it is why no registration-time constructor index is needed:
the type in hand at each node resolves the name, so nesting and repeated families cost nothing and can be ambiguous
about nothing. Only an answer that is a `Case` pays for the signature walk, so the 54 rules that answer a base type pay
nothing.

**The arrow refusal stays where it is.** [`Refusal::HigherOrderDelta`] is the diagnostic an author of a table actually
hits, and it says the specific thing. So the positive check reports it when the offending head is a Π, and reports the
new refusal for everything else that is not data — a record type, a universe, a bare variable. D1's arrow-free clause
becomes a consequence *and* keeps its own name.

**The alternative was to open `Constant`, and it is refused.** The obvious other design gives `Group` a public
constructor lookup and lets a rule build a `Term` — 141c's `Rewrite` shape, one prompt later. It is refused twice over.
It puts the core's declaration representation into all 38 host rules, each of which would walk its own signature to find
the group it needs before it can say `Some`; and it makes `Constant` constructible from outside, which is the one thing
keeping "a constant is what a declaration put in scope" true. A name plus fields is what a rule actually means, so that
is what it should be able to write. The cost is one small tree per call where 141b had a borrowed slice, and prompt 144
is where that gets measured rather than asserted.

**D1 becomes a registration check instead of a comment.** `Registry::new` today refuses an arrow in a δ signature and
nothing else, which admitted the record types and the bare type variables D1 never meant. State it positively: every
argument and result type of a δ-builtin is a registered base type, or a declared family applied to argument types that
are themselves this, at any depth. A signature that is neither is `Refusal::NotFiniteData`, and that refusal is what
makes "no arrow anywhere" a consequence rather than a separate rule. A literal in an index position — `Vec Nat 3` — is
refused with everything else, because no entry in the table has one and a check that admits what nothing writes is a
check nobody has read.

**A product is not built for.** `Shape::Product` is spellable in the compiler's shape language and used by no entry in
`BUILTIN_OWNERSHIP`. `Datum` therefore has no record arm, a record argument is not canonical data, and a δ signature
naming a record type is refused by the check above. When a product is wanted the arm and the loosened check arrive
together, with a caller; adding them now would be surface with nothing behind it.

**A host defect stays a host defect.** An answer naming a constructor its result type does not declare, or applying one
to the wrong number of fields, or standing at a type that is not a family at all, is `Malformed` and not a `Refusal` —
141b's rule, unchanged: D2 promises a value for every closed argument tuple, and a rule that cannot say what it meant is
a bug in the table rather than in a program. One variant for all three, because all three are the same sentence: the
data does not fit the type the builtin declared it answers at.

**Proved by a program.** As in 141b and 141c: the worked registry in the test suite gains a declared family and δ-rules
over it, so the mechanism is exercised without `musa-compiler` changing. Prompt 142 is still the one cutover.

## Target

- `crates/musa-calculus/src/base.rs`: `Datum` and the widened `Rule`, doc-commented with their invariants before the
  implementation.
- `crates/musa-calculus/src/eval.rs`: the firing condition generalized from *literal* to *canonical data*, the `Datum`
  built from the argument values, and the answer realized at the builtin's own result type.
- `crates/musa-calculus/src/family.rs`: the constructor a saturated spine is built by, and realization at a family type.
- `Registry::new`: D1 stated positively, replacing the arrow-only check for δ-builtins.
- One new `Refusal` variant with its `musa explain` code in `crates/musa-compiler/src/diagnose.rs`, and one new
  `Malformed` variant for an answer that does not fit the type it answers at.
- `crates/musa-calculus/tests/suite/base_laws.rs`: a declared family in the worked registry, and the laws — a rule reads
  a constructed argument, a rule answers a constructed value, a partially applied constructor leaves the spine blocked,
  the degenerate case still behaves exactly as 141b's laws say, and the registration refusal.
- `docs/plan/code-map/` rows for `musa-calculus`, extending 141b's row rather than adding a third.
- No change to `musa-compiler`'s checker, no `stdlib/` or `examples/` change, and no builtin moved out of
  `BUILTIN_OWNERSHIP`.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo clippy --all-targets -p musa-calculus -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Let a δ-rule speak the finite constructors`.

## Stop

- No public constructor and no public accessor on `Constant`, `Group`, `Declared`, or `Constructor`. The whole point of
  `Answer` is that a rule names a constructor rather than holding one.
- No change to `Rewrite` or to how a structural eliminator fires. 141c decided that, and a traversal's target is a
  literal for its own reason.
- No ambient context in a rule and no interior mutability in the registry. D3 is still enforced by the type.
- No `List`, `Option`, `Result`, or `Nat` in `musa-calculus/src/`. The core learns that a family has constructors, never
  which families exist.
- No wiring, no move of `BUILTIN_OWNERSHIP`, and no `musa-compiler` checker change. Prompt 142 owns the cutover.
- No amendment to §5.8. This prompt implements the half of D1 that was skipped; a disagreement is a finding to record.
- No record arm on `Datum` and no product in a δ signature until a caller wants one.
- No second `Datum` type for the answer direction, and no borrowing view. One owned type is the decision; a rule that
  wants to hand an argument straight back writes a clone.
