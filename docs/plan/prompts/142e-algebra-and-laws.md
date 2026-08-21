---
id: 142e
slug: algebra-and-laws
status: pending
depends_on: [142a, 142db]
phase: 3
---

# Name the Musical Algebra

## Task

Note 52 names five structures a Musa author should be able to write, and shows the standard library writing each one by
hand under a different name. Give the three that are structures rather than functions their names — `Torsor<P, V>`,
`Group<G>`, `Action<G, X>` — and instantiate each at the domains that exist today: intervals as a group, pitch and time
as its two torsors, pitch and spelled pitch class as its two carriers. `Transposable` stops being a trait with one
method and one idea, and becomes the action it always was.

That is the vocabulary. **The post-tonal half — `Group<Ti>`, `Action<Ti, Pc(n)>`, and `orbit`/`stabilizer` replacing
`row12_symmetries` and `row12_forms` — is prompt 143's**, for the reason stated below.

## Read

- [`52-the-musical-algebra.md`](../../notes/research/language-design-closure/52-the-musical-algebra.md). §2.1 and §2.2
  are the two structures this prompt names; §2.3 is the one it hands on. **§3 is not this prompt's mechanism** — see
  *Why there is no `law` clause*.
- `docs/rules/language/03-musical-domains.md` §1. It already proves what this prompt names: intervals form an abelian
  group componentwise, the action is faithful, and "if `p+i=p+j`, integer cancellation gives `i=j`" — which is the
  torsor property, stated as a lemma and never given an operation.
- `docs/rules/language/10-traits.md` — the flat model, which this prompt uses and does not extend. §2's coherence in
  particular, and §9's refusal of checked trait laws.
- `docs/rules/language/05-verification.md` §4, whose law 7 is already *"Pitch action: identity, composition, and
  cancellation"*. This prompt gives that law the names its statement has been missing.
- `stdlib/src/pitch.musa` in full, and `stdlib/src/core.musa` for where a shared vocabulary module sits.
- `crates/musa-compiler/src/phase/ownership.rs`'s pitch and time entries — `position_shift`, `position_between`,
  `interval_add`, `interval_inverse`, `pitch_transposed`, `pitchclass_transposed`. Read them for what is there, and for
  the one thing that is not.
- `~/Code/Idris2/src/Idris/Elab/Interface.idr`. Idris 2 elaborates an interface to a record of methods, which is what
  `class.rs` already does; this prompt is not a deviation from it.
- Open Music Theory `016-intervals.md` for the generic/specific naming the interval group is over.

## Design

**Three traits, and no more.** `Torsor<P, V>` with `difference` and `shift`; `Group<G>` with `unit`, `compose`,
`inverse`; `Action<G, X>` with `act`. Each is an ordinary flat trait under `10-traits.md`'s existing model — a record
type, an impl table keyed by `(trait, head)`, one-step lookup — so nothing about resolution changes.

**The instances are the point, and they exist today.** `Group<Interval>` is `P1`, `interval_add`, `interval_inverse`.
`Action<Interval, Pitch>` and `Action<Interval, NoteName>` are the two halves `Transposable` had. `Torsor<Position,
Duration>` is `position_between` and `position_shift`. That is one group, two carriers, and — with the paragraph below —
two torsors, which is what makes a two-parameter trait worth declaring rather than a shape asserted with one instance.

**One builtin is added, and it discharges a claim the governing document already makes.** There is no
`pitch_between(a, b) -> Interval`. `03-musical-domains.md` §1 *proves* it exists and is unique — the cancellation lemma
is exactly that — and the registry never gave it an operation, so the corpus's canonical torsor has an action and no
difference. Add it, in `crates/musa-compiler`, beside `pitch_transposed` and hidden by the same fixed-width coordinate
representation. Prompt 143's survey is over 132 entries after this, not 131, and the entry it audits is one whose
counterpart it will keep.

**Why the post-tonal half is 143's.** `Group<Ti>` and `Action<Ti, Pc(n)>` need three things this prompt cannot make: the
carrier `Pc(n)`, which does not exist — `Pc12` is a base type at one modulus and 143 is the prompt that collapses it; a
decidable equality on that carrier, since `orbit` is a set of images and `stabilizer` is a count of elements that fix
one, and `Eq` is declared by 143 along with the other operator traits; and the retirement of `row12_symmetries`,
`row12_forms`, and `row12_matrix`, which is 143's Target in as many words. 143's own opening already says the seventeen
modulus-12 builtins "collapse onto the index of 142d, and the operations onto the traits of 142e" — so the traits are
this prompt and the collapse is that one. A version of this prompt that tried to do both would be writing `orbit`
against a carrier fixed at twelve, which is the thing note 52 §5's falsifier exists to catch.

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
§2.2's whole observation, and keeping both would leave two vocabularies for one structure. `up` and `down` keep their
spellings; what changes is the trait the surface resolves them through.

**The traits get their own module.** `stdlib/src/algebra.musa`, imported by `pitch` and later by `post_tonal`, because a
vocabulary two subtrees share belongs to neither of them.

## Target

- `docs/rules/language/03-musical-domains.md`: the three traits as the domains' shared vocabulary, and §1's lemmas
  restated as the laws of the instances they license.
- `docs/rules/language/10-traits.md`: nothing added. If §1's "Laws are prose" paragraph or §9's `checked trait laws` row
  needs a word to point at `05-verification.md` §4, that is the whole of this prompt's edit there.
- `stdlib/src/algebra.musa`: the three trait declarations, and its `mod` line in `lib.musa`.
- `stdlib/src/pitch.musa`: `Group<Interval>`, `Action<Interval, Pitch>`, `Action<Interval, NoteName>`, `Torsor<Pitch,
  Interval>`; `Transposable` deleted.
- `stdlib/src/core.musa` or `algebra.musa`, whichever the module tree wants: `Torsor<Position, Duration>`.
- `crates/musa-compiler`: `pitch_between`, with its ownership entry and its registry law.
- `examples/`: one fixture writing the group, both torsors, and both actions, and showing that a difference and a shift
  undo each other.
- `docs/rules/language/05-verification.md` §4: law 7 restated over the named traits, and the group and torsor laws
  beside it.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

And the musical check, which is the prompt's reason for existing: for every pair of written pitches in the fixture,
`shift(a, difference(a, b))` is `b` — the cancellation lemma of `03-musical-domains.md` §1, executable for the first
time — and the hexatonic cycle of `examples/neo-riemannian.musa` still returns the spelling it returns today, because
naming a structure must not quietly quotient anything. Both in `examples/`, and cited by `make docs-check`'s
theory-citation pass.

## Stop

- **No `law` clause, no `law` keyword, and no discharge judgment.** See *Why there is no `law` clause*. A trait carries
  methods; an algebraic claim is a law suite.
- **No `Pc(n)`, no `Ti`, no `orbit`, and no `stabilizer`.** That is 143, and the reason is stated above.
- **No `Eq` or `Ord` declaration.** 143 declares the operator traits; a second declaration here would be the coherence
  violation §2 refuses.
- No builtin but `pitch_between`, and none removed. The collapse is 143.
- No enumerability judgment, and no registration of a base type's inhabitants.
- No `Functor`, `Monad`, `Category`, or higher-kinded parameter. The first-order two-parameter encoding covers every
  structure note 52 names, and a trait with no committed instance is not admitted.
- No superclasses, no constraint contexts, no recursive resolution. `10-traits.md`'s flat model is unchanged.
- No proof terms, and no identity type.
- No second instance table, and no instance keyed on an index.
- Do not port `std::tonal` or the adapters. That is 143 and 145.

Commit as `Name the musical algebra`.
