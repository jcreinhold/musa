---
id: 142e
slug: algebra-and-laws
status: pending
depends_on: [142a, 142db]
phase: 3
---

# Name the Musical Algebra

## Task

Note 52 names five structures a Musa author should be able to write — torsor, group action, orbit and stabilizer,
quotient with a chosen section, and the free construction — and shows the standard library writing each one by hand
under a different name. Give them their names: three ordinary flat traits, and `orbit` and `stabilizer` as ordinary
functions taking the modulus as the value it is. That is where `row12_symmetries`, `row12_forms`, and `row12_matrix`
stop being builtins.

## Read

- [`52-the-musical-algebra.md`](../../notes/research/language-design-closure/52-the-musical-algebra.md). §2 is the five
  structures with their current workarounds and §4 the table of what is already there; read it for the structures. **§3
  is not this prompt's mechanism** — see *Why there is no `law` clause* below.
- `docs/rules/language/10-traits.md` — the flat model, which this prompt uses and does not extend. §2's coherence in
  particular: at most one instance per `(trait, head)`, and `class.rs`'s `head_of` keys an indexed type on the type it
  indexes, so `Action<Ti(n), Pc(n)>` is *one* instance at a variable modulus and never one per modulus.
- `docs/rules/language/02-core-calculus.md` §1.5 as amended at 142c, and §3 as amended at 142db. Two facts decide what
  these traits can and cannot promise: an index is erased at quotation, and conversion is one relation whose index-sort
  case is decided by arithmetic.
- `docs/rules/language/05-verification.md` §4. Law suites are where an algebraic claim about these traits is checked,
  and they need nothing from the trait declaration to do it.
- `stdlib/src/pitch.musa`'s `Transposable`, `compose_intervals`, `inverse_interval`; `stdlib/src/transformational.musa`
  in full, including the comment on why finite-group statements hold only after spelling is forgotten;
  `stdlib/src/post_tonal/serial.musa`'s `D12 x C2` comment.
- `~/Code/Idris2/src/Idris/Elab/Interface.idr`. Idris 2 elaborates an interface to a record of methods, which is what
  `class.rs` already does; this prompt is not a deviation from it.
- Open Music Theory `102-set-class-and-prime-form.md`, `106-collections.md`, and `110-row-properties.md` — orbit,
  stabilizer, and canonical representative are the subject of those three chapters and are the functions this prompt
  makes ordinary.

## Design

**Three traits, and no more.** `Torsor<P, V>` with `-` and `+`; `Group<G>` with `unit`, `compose`, `inverse`;
`Action<G, X>` with `act`. Each is an ordinary flat trait under `10-traits.md`'s existing model — a record type, an impl
table keyed by `(trait, head)`, one-step lookup — so nothing about resolution changes. One instance covers every
modulus, because the index is erased and the table could not tell two moduli apart if they were written.

**Why there is no `law` clause.** Note 52 §3 proposes that a trait carry its laws and that the checker discharge them.
This prompt refuses that, and the reason is worth stating because §3 reads persuasive:

- A law quantified over an unbounded or parametric carrier cannot be *decided*; it can only be *proved*, and a proof
  needs a term that witnesses an equality. The identity type is deleted (§1.4), so that route is closed.
- The only proof-free route is exhaustion, and exhaustion needs a finite carrier. §1.5 erases indices and says in as
  many words that an index "cannot change a value", so `Pc(n)` cannot be enumerated *because of the n* — and a carrier
  that is enumerable without its index is one the trait did not need an index for.
- What remains is a mechanism with one true positive user, bought with a keyword, a discharge judgment, an enumerability
  judgment, and a budget. `10-traits.md` §9 refuses less for more.

Algebraic claims about these three traits are stated in `05-verification.md` §4's law suites over the concrete
instances, which is where Musa's checkable claims already live and which needs no new language surface.

**`Transposable` becomes `Action<Interval, A>`.** It was a group action written under another name, which is note 52
§2's whole observation, and keeping both would leave two vocabularies for one structure.

**`orbit` and `stabilizer` take the modulus as an argument.** They are ordinary total functions over a group and a set,
not builtins and not indexed operations, and the modulus reaches them as the number it is. That is what lets one
definition serve 12, 24, and 19 rather than seventeen builtins per modulus.

**The library's `Group` instance lives after the quotient.** `stdlib/src/transformational.musa` records that the PLR
cycle does not close on *spelled* triads — `P L P L P L` returns `dbb`. There is therefore no `Group<Triad>`; the
instance is on triad classes, and the library's existing insistence that forgetting spelling is something an author
performs is preserved. The comment saying so stays, and is now the reason a reader can see why the instance sits where
it does.

## Target

- `docs/rules/language/03-musical-domains.md`: the three traits as the domains' shared vocabulary.
- `docs/rules/language/10-traits.md`: nothing added. If §1's "Laws are prose" paragraph or §9's `checked trait laws` row
  needs a word to point at `05-verification.md` §4, that is the whole of this prompt's edit there.
- `stdlib/src/pitch.musa`: `Torsor<Pitch, Interval>` and `Torsor<Position, Duration>`; `Transposable` deleted in favour
  of `Action<Interval, A>`.
- `stdlib/src/post_tonal/`: `Group<Ti>`, `Action<Ti, Pc>` at an arbitrary modulus, and `orbit`/`stabilizer` as ordinary
  functions.
- `examples/`: the two fixtures the Check names.
- Law suites under `05-verification.md` §4 stating the group and action laws over the concrete instances.

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

## Stop

- **No `law` clause, no `law` keyword, and no discharge judgment.** See *Why there is no `law` clause*. A trait carries
  methods; an algebraic claim is a law suite.
- No enumerability judgment, and no registration of a base type's inhabitants. Nothing in this prompt needs to know how
  many values a type has.
- No `Functor`, `Monad`, `Category`, or higher-kinded parameter. The first-order two-parameter encoding covers every
  structure note 52 names, and a trait with no committed instance is not admitted.
- No superclasses, no constraint contexts, no recursive resolution. `10-traits.md`'s flat model is unchanged.
- No proof terms, and no identity type.
- No second instance table, and no instance keyed on an index. `head_of` keys an indexed type on the type it indexes,
  and a lookup that needed to tell `Pc(12)` from `Pc(24)` would be re-opening erasure.
- Do not port `std::tonal` or the adapters. That is 143 and 145.

Commit as `Name the musical algebra`.
