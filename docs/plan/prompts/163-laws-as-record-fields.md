---
id: 163
slug: laws-as-record-fields
status: done
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
`Pc(n)` becomes a declared indexed family and `Group<Ti>` and `Action<Pc(n), Ti>` become instances. This prompt is where
those instances gain their equations — to the extent the fourth finding below leaves any.

`depends_on` therefore names a *later* number, which the anatomy in [`README.md`](README.md) does not otherwise expect.
Renumbering this prompt to sit after 164 is the tidier bookkeeping and the worse change: `02-core-calculus.md` names
"prompts 146–163" as the implementation of the dependent core, and a governing document should not be edited to keep a
plan file's number in order. The dependency is the fact; the number is a label.

## Read

- `stdlib/src/algebra.musa` — the three structures and the sentence above.
- `stdlib/src/indexed.musa` — `Equal` and `Refl` as prompt 156 left them, and the paragraph refusing tactics, proof
  search, and hints.
- [`164`](164-builtin-collapse.md) — the prompt that supplies the carriers and the instances. Its Target names
  `Group<Ti>`, `Action<Pc(n), Ti>`, and `orbit`/`stabilizer` over a declared `Pc(n)`. **It names no `Torsor`**, and an
  earlier statement of this prompt said it did: there is no `Ic(n)` in `stdlib/` and none was planned, so a
  `Torsor<Pc(n), Ic(n)>` field has no instance to be a field of. `Torsor` keeps its one field and gains no law here.
- [`note 61`](../../notes/research/language-design-closure/61-the-registry-survey.md) — which builtins 164 kept, and
  why. The δ-rules over `Nat` are on the kept list, which is what the fourth finding below turns on.
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
There was no finite `data` carrier in `stdlib/` to induct over instead. This is why the prompt waits for 164: `Pc(n)`
arrives there as a declared indexed family, and the expectation recorded here was that a law over a declared family is
provable the way every other total function in this language is — by matching. **That expectation is measured false, and
the fourth finding is why.**

**Four: a declared carrier is not enough, because a δ-rule does not unfold on a neutral.** Measured against 164's
commit, with `musa check` on four probe files:

- The *mechanism* is sound and induction really does work. `cong` — `fn cong<A, B>(f: A -> B, x: A, y: A, same:
  Equal<A>(x, y)) -> Equal<B>(f(x), f(y))`, whose body is one `match` on `Refl` — type-checks, and with it
  `fn right_unit(x: Nat) -> Equal<Nat>(plus(x, Zero), x)` is provable by induction on `x`, for a `plus` written by
  matching. Nothing about proof terms, records, or `Equal` is the obstacle.
- The obstacle is `+`. `nat_add` is a builtin δ-rule
  ([note 61](../../notes/research/language-design-closure/61-the-registry-survey.md) keeps it, because it hides the
  representable range), and a δ-rule computes on *canonical* data only. So `fn add_zero(x: Nat) -> Equal<Nat>(x + 0, x)`
  is refused with *expected `nat_add #0 0`, found `#0`*, and splitting `x` does not help: the `Succ` arm is refused with
  *expected `nat_add (Nat.Succ #2) 0`*. `Nat` has constructors to match on and no arithmetic that steps under them.
- Every law over 164's carriers passes through that arithmetic. `Action<Cyclic(n), Ti>`'s unit law is refused at
  `Nat.add (number_of #2 #1) 0`; `Group<Ti>`'s left-unit law at `folded (size_of #4 #3) (Nat.add 0 #1) (Nat.add 0 #1)`;
  `Action<Pc(n), Ti>`'s at `place_in #2 #1 (number_moved …)`. The carriers are declared; their operations are not.
- `Ti`'s group laws are also *false* up to `Equal`, and that is not an accident of the encoding. `Ti` is deliberately
  not indexed by the cycle (`cyclic.musa` says why: `T_3` is `T_3` in every division), so `Transpose(15)` and
  `Transpose(3)` are one operation at twelve and two terms everywhere. The relation the laws hold up to is
  `same_operation`, which is a `Bool` and not a proposition.
- Closed instances still compute, exactly as finding one said: `Equal<Ti>(ti_compose(12, chromatic, Transpose(0),
  Transpose(3)), Transpose(3))` is inhabited by `Refl(Transpose(3))` today, and so is the corresponding fact about
  `class_moved`. What does not exist is the *quantified* form, which is the only form a law field has.

So the decision the second finding poses resolves against the amendment, and for a reason that is not the one the
decision anticipated: the blocker is not that a law field has no spelling. It is that no law field has an inhabitant, so
a spelling would have nothing to hold. `01-surface.md` §1 is **not** amended here, `row_top`'s type stays unwritable,
and the evidence for the named function type has to come from somewhere that is not this prompt.

**Which laws those are, expected before the work starts.** The pitch-class group at a fixed modulus, the interval torsor
over written pitch, and transposition as an action are the candidates, and each is a candidate only in the form 164
leaves it in. The transformational laws with preconditions — PLR closing only after spelling is forgotten — become laws
over `triad_classes`, which is the type the precondition was already carved into; note that `triad_classes` takes a
`Triad`, so that law is quantified over a base type and is subject to finding three, which 164 did not change — `Triad`
is still a base type and still has no eliminator. Anything not discharged is recorded as *not discharged*, with the
reason, in the module, and after the fourth finding that is all of them.

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
  of evidence, and with the parser, formatter, tree-sitter grammar, and lowering following it. **The fourth finding
  resolves this to no**: nothing is amended, and the reason is recorded where the amendment would have gone.
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
cargo run -p musa -- check stdlib/src/algebra.musa stdlib/src/transformational.musa
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

The `musa check` line named `stdlib/src/*.musa` and could not pass, for a reason that predates this prompt and is not
about laws: the glob picks up `stdlib/src/lib.musa`, which is a **module file** — a root of `mod` declarations and
nothing else. `parser/engine.rs` accepts that shape and says a file declaring a module tree owes no piece;
`elaborate/mod.rs` dispatches on `PieceDecl` and then `LibraryDecl` and has no third arm, so the file is refused with
*this file declares no piece*. The pre-164 version of the same file is refused identically, so it is a standing gap
between the parser's three document shapes and the elaborator's two, and closing it is compiler work this prompt's Stop
has no room for. The line now names the two modules this prompt edits; the whole bundle, module tree included, is
compiled by `the_standard_library_elaborates` in the workspace run above, so nothing is checked less than before.

Commit as `Let a structure carry its laws`.

## Stop

- No tactic language, no `auto`, no proof search, no hint database. If a law needs one, it does not get a field.
- No new equality reasoning in the core. `Equal` and `rewrite` as 156 left them.
- No new carrier invented for a law to be provable over. The carriers are the ones 164 leaves; declaring a second `Pc12`
  beside the compiler's own to have something to induct on is the layering mistake root `AGENTS.md` names.
