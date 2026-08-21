---
id: 142e
slug: algebra-and-laws
status: pending
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
  the five structures with their current workarounds, §3 the law mechanism, §4 the table of what is already there.
- `docs/rules/language/10-traits.md` — the flat model, which this extends by one clause and does not otherwise touch.
- `docs/rules/language/05-verification.md` — `assert` and finite evidence, which is the unbounded-carrier case and
  already exists.
- `stdlib/src/pitch.musa`'s `Transposable`, `compose_intervals`, `inverse_interval`; `stdlib/src/transformational.musa`
  in full, including the comment on why finite-group statements hold only after spelling is forgotten;
  `stdlib/src/post_tonal/serial.musa`'s `D12 x C2` comment.
- Open Music Theory `102-set-class-and-prime-form.md`, `106-collections.md`, and `110-row-properties.md` — orbit,
  stabilizer, and canonical representative are the subject of those three chapters and are the functions this prompt
  makes ordinary.

## Design

**Three traits, and no more.** `Torsor<P, V>` with `-` and `+`; `Group<G>` with `unit`, `compose`, `inverse`;
`Action<G, X>` with `act`. Each is an ordinary flat trait under `10-traits.md`'s existing model — a record type, an impl
table keyed by `(trait, head)`, one-step lookup — so nothing about resolution changes. The generality is bought by the
index, not by a new trait mechanism: `Action<Ti(n), Pc(n)>` is one instance for every modulus.

**`law` is a trait clause, and its checking is decided by the carrier.**

```
trait Action<G, X> {
    fn act(operation: G, subject: X) -> X;
    law identity:    act(unit, x) == x;
    law composition: act(compose(g, h), x) == act(g, act(h, x));
}
```

An impl whose carriers are finite of statically known size — which the index is what makes knowable — has its laws
checked by enumeration at declaration, under the ordinary budget. `Action<Ti(12), Pc(12)>` is 24 × 24 × 12 = 6,912
evaluations. An impl over an unbounded carrier states its evidence with `assert` exactly as today, and the impl records
which of the two it got, so a reader can tell a checked law from an asserted one.

**The checker must refuse the laws that are false, and one of them is in the library.**
`stdlib/src/transformational.musa` records that the PLR cycle does not close on *spelled* triads — `P L P L P L` returns
`dbb`. Enumeration will find that counterexample, and the right outcome is a refusal naming it, not an accommodation.
The `Group` instance lives on triad classes, after the quotient, and the library's existing insistence that forgetting
spelling is something an author performs is preserved.

**A budget crossing is a refusal, not a hang, and not a silent pass.** A law over a carrier too large to enumerate says
so and asks for `assert`. It never reports the law as checked.

## Target

- `docs/rules/language/10-traits.md`: the `law` clause, the two discharge routes, and the rule that an impl says which
  it took.
- `docs/rules/language/03-musical-domains.md`: the three traits as the domains' shared vocabulary.
- `crates/musa-calculus/`: `law` in `class.rs`/`raw.rs`, and the enumeration discharge, metered.
- `stdlib/src/pitch.musa`: `Torsor<Pitch, Interval>` and `Torsor<Position, Duration>`; `Transposable` becomes
  `Action<Interval, A>` or is deleted in favour of it.
- `stdlib/src/post_tonal/`: `Group<Ti(n)>`, `Action<Ti(n), Pc(n)>`, and `orbit`/`stabilizer` as ordinary functions —
  which is where `row12_symmetries`, `row12_forms`, and `row12_matrix` stop being builtins.
- Law suites: a checked law, an asserted law, a refused law with its counterexample, and a budget crossing.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

And the musical check, which is the prompt's reason for existing: `orbit` under `Action<Ti(12), Pc(12)>` applied to the
committed set-class fixtures reproduces `102-set-class-and-prime-form.md`'s prime forms, and `stabilizer` applied to the
whole-tone and octatonic collections reproduces `106-collections.md`'s modes of limited transposition. Both as fixtures
in `examples/`, both cited by `make docs-check`'s theory-citation pass.

## Stop

- No `Functor`, `Monad`, `Category`, or higher-kinded parameter. The first-order two-parameter encoding covers every
  structure note 52 names, and a trait with no committed instance is not admitted.
- No superclasses, no constraint contexts, no recursive resolution. `10-traits.md`'s flat model is unchanged.
- No proof terms. A law is checked by enumeration or asserted with finite evidence, never proved.
- Do not port `std::tonal` or the adapters. That is 143 and 145.
