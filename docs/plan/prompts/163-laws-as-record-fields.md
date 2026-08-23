---
id: 163
slug: laws-as-record-fields
status: pending
depends_on: [156, 157, 164]
phase: 3
---

# Let a Structure Carry Its Laws

## Task

`stdlib/src/algebra.musa` says, in its own words, that *"a trait declaration in this language carries methods and never
obligations… an obligation needs evidence, and evidence needs a term witnessing an equality."* Prompt 156 made `Equal`
declarable and prompt 157 made a structure an ordinary record, so the evidence exists. What is still missing is the
thing the evidence would be attached to: **nothing in the corpus constructs a `Group`, an `Action`, or a `Torsor`.** The
three records are declared in `algebra.musa` and are named nowhere else, and their carriers — `Interval`, `Pitch`,
`NoteName`, `PitchClass` — are compiler base types with δ-rules and no eliminator. Prompt 164 is where both change:
`Pc(n)` becomes a declared indexed family and `Group<Ti>`, `Action<Pc(n), Ti>`, and `Torsor<Pc(n), Ic(n)>` become
instances. This prompt is where those instances gain their equations.

`depends_on` therefore names a *later* number, which the anatomy in [`README.md`](README.md) does not otherwise expect.
Renumbering this prompt to sit after 164 is the tidier bookkeeping and the worse change: `02-core-calculus.md` names
"prompts 146–163" as the implementation of the dependent core, and a governing document should not be edited to keep a
plan file's number in order. The dependency is the fact; the number is a label.

## Read

- `stdlib/src/algebra.musa` — the three structures and the sentence above.
- `stdlib/src/indexed.musa` — `Equal` and `Refl` as prompt 156 left them, and the paragraph refusing tactics, proof
  search, and hints.
- [`164`](164-builtin-collapse.md) — the prompt that supplies the carriers and the instances. Its Target names
  `Group<Ti>`, `Action<Pc(n), Ti>`, `Torsor<Pc(n), Ic(n)>`, and `orbit`/`stabilizer` over a declared `Pc(n)`.
- `docs/rules/language/05-verification.md` §4 — the law suites that are currently the only place a law is written down,
  and §4.7 in particular, which states the pitch-action laws this prompt would move.
- `docs/rules/language/01-surface.md` §1's grammar, `type` and `fn-type` — and `docs/rules/language/02-core-calculus.md`
  §1 ("`(x : A) → B` is the only function type, and `B` may mention `x`"). The asymmetry between those two lines is the
  second finding below.
- `stdlib/src/transformational.musa` — in particular the note that PLR chains do not close in the spelled domain, which
  is a law with a *precondition*, and the interesting case for this prompt.

## Design

**A law is a field, and its inhabitant is a proof term.**

```
record Group(G : Type) {
  unit    : G,
  compose : G -> G -> G,
  inverse : G -> G,
  left_unit : (x : G) -> Equal (compose unit x) x,
  ...
}
```

Constructing a `Group` now requires supplying the equations, which is the point: an instance that does not satisfy them
cannot be built.

**And here is the honest cost, stated up front.** Musa refuses tactics, proof search, and a hint database. So every law
field must be inhabited by a term the author *writes*, and for the concrete carriers in the stdlib those are `refl`
after both sides normalize — which works exactly when the operation is δ-reducible on canonical data, and does not when
the carrier is abstract. **Where it does not work, the field is not added.** This prompt's deliverable is the laws that
can be discharged by computation, not a demonstration that all of them can.

### Three things measured before this prompt was reordered

The reordering above is not a guess. Three probes were run against the compiler as prompt 162a left it, and they decide
what this prompt can and cannot deliver.

**One: the mechanism works, and it is exact.** A record field whose type mentions an earlier field is checked against
the field the literal supplied, and a false law is refused at the literal rather than anywhere later:

```musa
record Doubler {
    twice: Nat -> Nat;
    fixes_zero: Equal<Nat>(twice(Zero), Zero);
}
let counting: Doubler = Doubler { twice = fn (n) { n }, fixes_zero = Refl(Zero) };   // ok
let lying: Doubler = Doubler { twice = fn (n) { Succ(n) }, fixes_zero = Refl(Zero) }; // expected `1`, found `0`
```

Closed equalities over the base types compute too: `Equal<Interval>(interval_add(P5, interval_inverse(P5)), P1)` is
inhabited by `Refl(P1)` today. So dependent fields and `Equal` compose exactly as the Design above hoped.

**Two: a *quantified* law cannot be spelled as a field, because the surface has no named function type.** §1's grammar
gives `fn-type := type "->" type | "(" (type ("," type)*)? ")" "->" type`. A parenthesized parameter list is a list of
*types*, never of named binders, so the codomain has nothing to mention and
`left_unit: (x: G) -> Equal<G>(compose(unit(x), x), x)` does not parse — the parser asks for `)` after `x`.
`fn(x: G) -> …` is not a type either. A `fn` *declaration* is the only place in the surface where an author names a Π
binder, which is why `row_top`'s own type, `(size: Nat) -> Row<A>(Succ(size)) -> A`, cannot be written down: that
function cannot be annotated, stored in a field, or returned. `02-core-calculus.md` §1 has the form and calls it the
only function type; the surface never grew a spelling for it.

So this prompt has a decision to make with 164's instances in hand, and it is a decision and not a detail:

- **Extend §1's `fn-type` with a named parameter** — `"(" IDENT ":" type ("," IDENT ":" type)* ")" "->" type`, the
  spelling `02-core-calculus.md` §1 already uses and the one a `fn` declaration's parameter list already parses. That is
  an amendment to a governing document, made under `docs/rules/README.md`'s procedure, on the evidence of `row_top` and
  of the law fields below. It is in this prompt's scope because a law field is unwritable without it and because the gap
  is a hole in the surface rather than a feature this prompt wants.
- **Or add no fields at all**, and say in `algebra.musa` and in `05-verification.md` §4 that a structure carries no
  obligations because a law over its carrier has no spelling, which is a worse answer and an honest one.

Take the first only if the second turns out to lose something real — that is, only if there is at least one law field
that is *inhabitable* once it is spellable. Which brings the third finding.

**Three: a law is inhabitable only over a carrier the language declares.** `Interval`, `Pitch`, `NoteName`,
`PitchClass`, and `Triad` are compiler base types. Their operations are δ-rules that compute on canonical data and there
is no case analysis to split an abstract element with, so `(x: Interval) -> Equal<Interval>(interval_add(x, P1), x)` has
no inhabitant a Musa author can write — verified: `Refl(x)` is refused with *expected `interval_add #0 P1`, found `#0`*.
There is no finite `data` carrier in `stdlib/` to induct over instead. This is exactly why the prompt now waits for 164:
`Pc(n)` arrives there as a declared indexed family, and a law over a declared family is provable the way every other
total function in this language is — by matching.

**Which laws those are, expected before the work starts.** The pitch-class group at a fixed modulus, the interval torsor
over written pitch, and transposition as an action are the candidates, and each is a candidate only in the form 164
leaves it in. The transformational laws with preconditions — PLR closing only after spelling is forgotten — become laws
over `triad_classes`, which is the type the precondition was already carved into; note that `triad_classes` takes a
`Triad`, so that law is quantified over a base type and is subject to finding three until 164 says otherwise. Anything
not discharged is recorded as *not discharged*, with the reason, in the module.

**`05-verification.md` §4 does not go away.** A property test over generated values and a proof term are different
evidence: one covers the concrete carriers the corpus actually uses, the other covers all of them. Where a law is
proved, the suite keeps the test as a check on the *proof* being about the operation the code calls.

**This is where "not a proof assistant" gets its real test.** If discharging these laws turns out to want tactics, that
is evidence for the refusal, not against it: it means the boundary is in the right place and the answer is fewer law
fields, not more apparatus. Record which way it went.

## Target

- `stdlib/src/algebra.musa`: law fields on `Group`, `Action`, `Torsor`, with the undischarged ones named and explained.
  If the count of dischargeable fields is zero, the module says so in the same place and this prompt's deliverable is
  that sentence with its three measurements behind it.
- `docs/rules/language/01-surface.md` §1: the named function type, if and only if a law field turns out to be
  inhabitable — amended under `docs/rules/README.md`'s procedure, with `row_top`'s unwritable type as the second piece
  of evidence, and with the parser, formatter, tree-sitter grammar, and lowering following it.
- `stdlib/src/transformational.musa`: the PLR laws over `triad_classes`, to the extent 164's carriers make them
  provable.
- `docs/rules/language/05-verification.md` §4: which laws are proved, which are tested, and why each is where it is.
- `crates/musa-compiler/tests/suite/`: the law suites kept, pointed at the proved operations.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo run -p musa -- check stdlib/src/*.musa
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Let a structure carry its laws`.

## Stop

- No tactic language, no `auto`, no proof search, no hint database. If a law needs one, it does not get a field.
- No new equality reasoning in the core. `Equal` and `rewrite` as 156 left them.
- No new carrier invented for a law to be provable over. The carriers are the ones 164 leaves; declaring a second `Pc12`
  beside the compiler's own to have something to induct on is the layering mistake root `AGENTS.md` names.
