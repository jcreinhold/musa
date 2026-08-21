---
id: 142e
slug: algebra-and-laws
status: in-progress
depends_on: [142a, 142d]
phase: 3
---

# Name the Musical Algebra, and Check Its Laws by Exhaustion

## Task

Note 52 names five structures a Musa author should be able to write — torsor, group action, orbit and stabilizer,
quotient with a chosen section, and the free construction — and shows the standard library writing each one by hand
under a different name. Give them their names, and give a trait the ability to carry laws that the checker discharges
**by enumeration over a finite indexed carrier**, which is how Musa gets rigorously checked algebra with no proof
machinery and no identity type.

## Read

- [`52-the-musical-algebra.md`](../../notes/research/language-design-closure/52-the-musical-algebra.md) in full. §2 is
  the five structures with their current workarounds, §3 the law mechanism, §4 the table of what is already there. §3
  was written before the course correction fixed erasure, and this prompt's Design says where it has to be read
  differently — read it for the structures, not for the mechanism.
- `docs/rules/language/02-core-calculus.md` §1.5 **as amended at 142c**, and `crates/musa-calculus/src/index.rs` as 142d
  built it. Two sentences there decide this prompt's whole mechanism: an index is *erased at quotation*, and it "cannot
  change a value". A carrier's inhabitants are values, so nothing reads them out of an index.
- `docs/rules/language/10-traits.md` — the flat model, which this extends by one clause and does not otherwise touch.
  §2's coherence in particular: at most one instance per `(trait, head)`, and `class.rs`'s `head_of` keys a refinement
  on the type it refines, so `Action<Ti(n), Pc(n)>` is *one* instance at a variable modulus and never one per modulus.
- `docs/rules/language/05-verification.md` §2 and §4. §2's `assert` is a fixed family of compiler-owned predicates over
  a passage of music and is **not** the mechanism for an algebraic law; §4's law suites are. The second discharge route
  below is §4's, not §2's.
- `stdlib/src/pitch.musa`'s `Transposable`, `compose_intervals`, `inverse_interval`; `stdlib/src/transformational.musa`
  in full, including the comment on why finite-group statements hold only after spelling is forgotten;
  `stdlib/src/post_tonal/serial.musa`'s `D12 x C2` comment.
- `~/Code/Idris2/src/Idris/Elab/Interface.idr`, and `src/Parser/Lexer/Source.idr`'s keyword list. Idris 2 elaborates an
  interface to a record of methods, which is what `class.rs` already does and is not a deviation. It has **no** law
  mechanism: `law` is not a keyword, and the idiom for enforcing one is a proof field over propositional `=`, discharged
  with `Refl` and `%hint` search. Musa has deleted all three — the identity type (§1.4, note 51 §7), proof terms (this
  prompt's Stop), and instance search (`10-traits.md` §9's first row) — so a law here can be prose or decided by
  exhaustion, and there is no third option. Record the deviation as forced rather than chosen.
- Runciman, Naylor & Lindblad, *SmallCheck and Lazy SmallCheck* (Haskell Symposium 2008), and Claessen & Hughes,
  *QuickCheck* (ICFP 2000). Exhaustive enumeration over a bounded value space is the mechanism, and the boundary those
  papers draw is the one the two routes encode: exhaustion at a **fixed finite carrier** is a proof for that carrier and
  not a schema for all `n`.
- Open Music Theory `102-set-class-and-prime-form.md`, `106-collections.md`, and `110-row-properties.md` — orbit,
  stabilizer, and canonical representative are the subject of those three chapters and are the functions this prompt
  makes ordinary.

## Design

**Three traits, and no more.** `Torsor<P, V>` with `-` and `+`; `Group<G>` with `unit`, `compose`, `inverse`;
`Action<G, X>` with `act`. Each is an ordinary flat trait under `10-traits.md`'s existing model — a record type, an impl
table keyed by `(trait, head)`, one-step lookup — so nothing about resolution changes. One instance covers every
modulus, because the index is erased and the table could not tell two moduli apart if they were written.

**`law` is a trait clause, and its binders are written.**

```musa
trait Action<G, X> {
    fn act(operation: G, subject: X) -> X;
    law identity(x: X): act(unit(), x) == x;
    law composition(g: G, h: G, x: X): act(compose(g, h), x) == act(g, act(h, x));
}
```

The binder list is written and not inferred from the equation's free names. Which variables a law quantifies over is
exactly what decides how it is discharged, and reading them off the body would be the scan `10-traits.md` §9 refuses one
more time. The equation is an ordinary expression of type `Bool`, so `==` is `Eq<A>.equal` and the law form introduces
no new judgment — a law is a closed `Bool`-valued function of its binders, and nothing about checking it is new except
what supplies the arguments.

**Two discharge routes, decided by the binders' types, and the instance records which it got.**

- **Enumerated.** Every binder's type is *enumerable*, so the checker builds the inhabitants, evaluates the equation at
  every tuple under the ordinary meter, and refuses at the first assignment that answers `false` — naming the assignment
  and the law, not the enumeration.
- **Stated.** Otherwise. The law is recorded on the instance as stated, and `05-verification.md` §4's law suites
  discharge it over the concrete instances, which is where Musa's checkable claims already live. The elaborated instance
  carries which route each of its laws took, so a reader can tell a checked law from a stated one and the documentation
  surface can say so.

**Enumerable is a property of a declaration, and it is not what an index makes knowable.** This is where note 52 §3 has
to be read against §1.5 rather than with it. §1.5 erases an index at quotation and says in as many words that an index
"cannot change a value"; a carrier's inhabitants are values; so `Pc(12)` cannot be enumerated *because of the 12*. What
the checker can enumerate is:

- a data or enum type all of whose constructors' field types are enumerable — nullary constructors included;
- a record type all of whose field types are enumerable;
- a base type whose registration supplies its inhabitants, which is 142d's `Measures` pattern used a second time and for
  the same reason: the finite representation is the host's, and the calculus asks rather than knows.

`Pc12`, `NoteName`, `Interval`, and `Triad` are the four registrations, and they are the four the committed refusal
below needs. A carrier that is none of these — `Pc(n)` at a variable modulus, `Pitch`, `List<A>` — takes the stated
route, which is not a weaker answer but a differently sourced one.

**The checker must refuse the laws that are false, and one of them is in the library.**
`stdlib/src/transformational.musa` records that the PLR cycle does not close on *spelled* triads — `P L P L P L` returns
`dbb`. `Triad` is enumerable, so `Group<Triad>` takes the enumerated route, and enumeration will find that
counterexample. The right outcome is a refusal naming it, not an accommodation. The `Group` instance the library keeps
lives on triad classes, after the quotient, and the library's existing insistence that forgetting spelling is something
an author performs is preserved.

**A budget crossing is a refusal, not a hang, and not a silent pass.** A law over a carrier too large to enumerate says
so and asks for the stated route. It never reports the law as checked.

## Target

- `docs/rules/language/10-traits.md`: the `law` clause, the two discharge routes, what makes a carrier enumerable, and
  the rule that an impl says which route each law took. §1's "Laws are prose" paragraph and §9's `checked trait laws`
  row are the two places that say the opposite today, and both are this prompt's to amend.
- `docs/rules/language/03-musical-domains.md`: the three traits as the domains' shared vocabulary.
- `crates/musa-calculus/`: `law` in `class.rs`/`raw.rs`, the enumerability judgment, and the enumeration discharge, all
  metered.
- `crates/musa-compiler/`: the four enumerable registrations, and the surface reading of a `law` clause.
- `stdlib/src/pitch.musa`: `Torsor<Pitch, Interval>` and `Torsor<Position, Duration>`; `Transposable` becomes
  `Action<Interval, A>` or is deleted in favour of it.
- `stdlib/src/post_tonal/`: `Group<Ti>`, `Action<Ti, Pc>` at an arbitrary modulus, and `orbit`/`stabilizer` as ordinary
  functions taking the modulus as the value it is — which is where `row12_symmetries`, `row12_forms`, and `row12_matrix`
  stop being builtins.
- Law suites: a checked law, a stated law, a refused law with its counterexample, and a budget crossing.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

And the musical check, which is the prompt's reason for existing: `orbit` under `Action<Ti, Pc>` at modulus 12, applied
to the committed set-class fixtures, reproduces `102-set-class-and-prime-form.md`'s prime forms, and `stabilizer`
applied to the whole-tone and octatonic collections reproduces `106-collections.md`'s modes of limited transposition.
Both as fixtures in `examples/`, both cited by `make docs-check`'s theory-citation pass.

The mechanism check is the counterexample: `Group<Triad>` on spelled triads is refused at the declaration, and the
message names `dbb`.

## Stop

- No `Functor`, `Monad`, `Category`, or higher-kinded parameter. The first-order two-parameter encoding covers every
  structure note 52 names, and a trait with no committed instance is not admitted.
- No superclasses, no constraint contexts, no recursive resolution. `10-traits.md`'s flat model is unchanged.
- No proof terms. A law is checked by enumeration or stated for a law suite, never proved.
- No second instance table, and no instance keyed on an index. `head_of` keys a refinement on the type it refines, and a
  law mechanism that needed to tell `Pc(12)` from `Pc(24)` at a lookup would be re-opening erasure.
- No index-level enumeration. A carrier says how many it has at its declaration or its registration, never through the
  index — `02-core-calculus.md` §1.5's closed grammar has no such function and this prompt does not add one.
- Do not port `std::tonal` or the adapters. That is 143 and 145.

Commit as `Name the musical algebra, and check its laws by exhaustion`.
