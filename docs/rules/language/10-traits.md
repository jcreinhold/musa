# Traits, methods, and namespaces

This document fixes Musa's one mechanism for writing an operation once and using it at several types. It is deliberately
boring: a trait is a record of methods, an impl is a value of that record, and choosing which impl a use gets is a
**lookup** rather than a search. Everything a search would buy — overlap, specialization, defaulting,
return-type-directed choice — is refused here by name, with the reason, so that later evidence has something specific to
argue against.

The grammar is `01-surface.md` §1.4–§1.6. The core these forms elaborate into is `02-core-calculus.md`, whose dependent
records with η are what the coherence argument in §2 rests on. Prompt 137 builds the mechanism, prompt 143 collapses the
builtin registry onto it, and prompt 148 audits what is claimed here.

## 1. A trait is a record of methods

A `trait` declaration introduces a function from its type parameters to a record type, and an `impl` declaration
introduces a value of that record at particular arguments:

```musa
trait Eq<A> {
    fn equal(x: A, y: A) -> Bool;
}
```

```text
Eq        : (A : Type ℓ) → Type ℓ
Eq A      = { equal : A → A → Bool }
Eq@Tying  : Eq Tying                              % from `impl Eq<Tying> { … }`
```

The elaborated name `Eq@Tying` is not spellable in source; it is how this document refers to the dictionary a lookup
finds. A trait's **required** methods — declared with `;` — are the record's fields. Its **derived** methods — declared
with a block — are ordinary functions that take the dictionary and are defined once at the trait. An impl supplies the
required methods and may not replace a derived one.

That split is doing real work. It is why §9's refusal of specialization is a *mechanism* rather than a rule to enforce:
there is no overridable definition to specialize, because a definition an impl can write is a field and a definition it
cannot write is not in the dictionary at all. It is also what lets `Iterable` (§8) give a container five operations for
two.

**A trait has one or more parameters, and the first is the head.** Later parameters are *determined* by the instance
rather than searched for: coherence (§2) says there is at most one instance per trait and head, so once the head is
known the whole instance is known, and with it every other parameter. `Iterable<C, A>` is therefore a lookup on `C` in
which `A` falls out, not a two-dimensional search. A trait every one of whose parameters is unused by its head is
rejected at the declaration, because nothing could ever select it.

A trait may state a `where` clause of its own. `trait Ord<A> where Eq<A>` means the `Ord` dictionary carries an `Eq`
dictionary as an extra field, and `impl Ord<Ratio>` must be able to produce `Eq<Ratio>`. This is projection, not search:
reaching `Eq` from `Ord` is one field read.

**Laws are prose.** A trait may state the equations its instances are expected to satisfy, and this specification and
the standard library state them for the traits they declare. Nothing checks them. A law that must be checked is written
as a `05-verification.md` law suite over the concrete instances, which is where the rest of Musa's checkable claims
live; a `where`-clause proof obligation would need the constraint to carry evidence, and that is a separate amendment
with its own evidence.

## 2. Coherence

**There is at most one instance per (trait, head type) in a whole program.** Not per module, not per import graph — per
program. A second impl for the same trait and head is an error naming both declarations, wherever they are and whether
or not any code uses both.

The property this buys is stated precisely, because "coherence" is used loosely elsewhere:

> For every use site of a method at a given trait and type arguments, the dictionary the elaborator supplies is unique
> up to conversion, and does not depend on the order in which the elaborator reached its constraints, on which modules
> the use site imported, or on where in the program the constraint was discharged.

Uniqueness *up to conversion* is what the core gives and what the argument needs. A dictionary is a record value, and
`02-core-calculus.md` §3 decides record equality by η at `quote`: two records with the same projections at the same type
are convertible without a rule that inspects both at once. So two elaborations that reach the same instance produce
convertible dictionaries even when they build them by different routes — one directly, one through a `where` parameter
that was later instantiated. Without η the two would be distinguishable terms and "the same instance" would be a claim
about elaboration history rather than about values.

Coherence is what makes the rest of this document short. Ambiguity cannot arise, so there is nothing to break ties with;
instance choice cannot depend on scope, so importing a module cannot change what existing code means; and a use site
does not have to say which instance it meant, because there is only one.

## 3. The orphan rule

An `impl C<T, …>` may be declared in exactly two places: the package that declares the trait `C`, or the package that
declares the head type `T`'s constructor. An impl in any third package is refused at the declaration, naming the two
packages that could hold it.

This is what makes §2 checkable rather than aspirational. Without it, two packages that never heard of each other could
each declare `impl Eq<Foo>`, and the conflict would appear only in the program that depended on both — so a coherent
program could be made incoherent by adding a dependency, and there would be no author to report it to. With it, every
possible impl for a given (trait, head) is written in one of two packages, and each of those packages can see all of
them.

The cost is real and is the ordinary one: to use a trait from package `p` at a type from package `q`, one of the two
must know about the other, or a local one-case enum wraps the type. Musa pays it rather than paying the alternative,
which is a program whose meaning depends on its dependency graph.

`Storable` (`02-core-calculus.md` §1.2) is the one exception in the whole system, and it is an exception in the safe
direction: its instances are *generated* from declaration groups and may not be written by hand at all, so the set of
instances is smaller than any author could produce, never larger. A signature may require `Storable A`; nothing may
supply it.

## 4. Instance lookup, and why it terminates

Resolving a constraint `C<τ₁, …>` is three steps and no backtracking:

1. **Look in the local context first.** A dictionary bound by an enclosing `where` clause whose trait and head match is
   used, and lookup stops.
2. **Otherwise look up the head.** Take `τ₁`'s head constructor and find the one instance declared for `(C, that
   constructor)`. If there is none, the constraint is refused, naming the type and the trait.
3. **Discharge the instance's own `where` clause** by recursion, with the instance's type arguments substituted.

If `τ₁`'s head is not yet known — it is a metavariable — the constraint is **postponed** exactly as
`02-core-calculus.md` §2.1 postpones any blocked constraint, and retried when the metavariable is solved. A constraint
still blocked at the end of the declaration is the ordinary *unsolved metavariable* error, reported at the term that
created it. Guessing an instance for an unknown head would be search with one candidate, which is still search.

**Local beats global**, and it is worth saying what that rule is for. Under coherence the local dictionary and the
global instance for the same head are the same instance, so the rule never changes what a program means — if it ever
did, coherence would already have been violated. What it fixes is *determinacy*: the elaboration of a generic function's
body is decided by its own signature, so instantiating `A := Tying` later cannot reroute a call that was already
elaborated, and adding an instance elsewhere in the program cannot change a body that was already checked.

**Termination is checked at the declaration.** Define the size of a type as the number of type constructors and type
variables it contains, counting repeats. An instance is admissible only when, for every constraint `D<σ₁, …>` in its
`where` clause:

- `size(σ₁) < size(head)`, and
- no type variable occurs more times in `σ₁` than it does in the instance head.

`impl<A> Eq<List<A>> where Eq<A>` passes: `A` is smaller than `List<A>` and occurs once on each side. A context that
does not decrease is refused **at the instance**, naming the constraint that failed to shrink, and never at a use site —
because a use site is the wrong place to learn that a library cannot answer. The measure makes step 3 above a finite
descent, so lookup terminates on every input rather than on the inputs anyone happened to try.

Lookup work is charged to the §4 meter of `02-core-calculus.md` like every other elaboration cost; a program that
constructs a deep instance context is refused by budget rather than by a hidden depth limit.

## 5. Operators, and the builtins they replace

An operator is surface syntax for a trait method. `01-surface.md` §1.5 fixes the two rules that keep it from becoming
overloading — an operator resolves only at a known head or under a `where`, and an operation that can fail keeps its
failing shape — and this is the table.

| Surface | Trait and method | Signature | Builtins it replaces |
| --- | --- | --- | --- |
| `x == y` | `Eq<A>.equal` | `A → A → Bool` | `text_equal`, `ratio_equal`, `duration_equal`, `position_equal` |
| `x < y` | `Ord<A>.less` | `A → A → Bool` | `ratio_less`, `duration_less`, `position_less` |
| `x + y` | `Add<A>.add` | `A → A → A` | `nat_add`, `ratio_add`, `duration_add`, `interval_add` |
| `x - y` | `Sub<A>.sub` | `A → A → Result<A, E>` where the difference can leave the type | `nat_sub`, `ratio_sub` |
| `x * y` | `Mul<A>.mul` | `A → A → A` | `nat_mul`, `ratio_mul` |
| `x / y` | `Div<A>.div` | `A → A → Result<A, E>` | `ratio_div` |
| `xs[i]` | `Index<C, I, A>.at` | `C → I → Option<A>`, or `C → I → A` for an index type that cannot be out of range | none today; prompt 141's containers are its first instances |

The traits are **homogeneous**, and the operations that are not stay named functions: `position_shift(p, d)` adds a
duration to a position, `position_between(a, b)` answers the duration between two positions, and `duration_scale(d, r)`
scales a duration by a rational. None of these is an operator, and the reason is the one `02-core-calculus.md` §1.1
gives for separating `Position` from `Duration` at all. A `+` that accepted a beat where a number of beats was meant
would hand back the single arithmetic error the two types exist to catch.

`Sub` and `Div` return `Result` because natural subtraction and rational division can fail, and the trait's signature is
where that is recorded once instead of at every instance. An instance whose operation is total may still be declared —
the failing shape is a property of the trait, and a total instance answers `Ok` — but the *trait* does not get a total
signature just because one instance would fit it, because that would make the operator's type depend on which instance
was found.

Methods and namespaces absorb most of the rest of the registry. The shape is uniform:

| Registry family | What it becomes | Example |
| --- | --- | --- |
| `<type>_of`, `<type>_on` constructors | an inherent function in the type's namespace (§6) | `duration_of(r)` → `Duration::of(r)` |
| `<type>_<field>` accessors | an inherent method on the receiver | `chord_root(c)` → `c.root()` |
| `<type>_<transform>` operations | an inherent method on the receiver | `row12_retrograde(r)` → `r.retrograde()` |
| the two list folds, `map`, `filter` | `Iterable` methods (§8) | `list_fold_from_end(z, f, xs)` → `xs.fold_from_end(z, f)` |
| `option_fold` | `match`, since `Option` is an ordinary enum | — |

What does **not** move, and why:

- **The literal judgments** — `nat_literal`, `ratio_literal`, `pitch_literal`, `key_literal`, `interval_literal`. These
  are compiler judgments on tokens (`02-core-calculus.md` §2.2), not operations on values, and there is nothing for a
  trait to dispatch on.
- **`nat_fold`.** `Nat` is not a container; this is its recursor, and `Iterable` is about containers.
- **`repeat` and `range`.** `repeat n { body }` is the notation-facing fold over musical material and keeps its
  spelling; `range` builds a list and is a library function once there is a library.
- **The track and machine builtins** — `transpose`, `stretch`, `retrograde`, `invert`, `shift`, `together`,
  `map_note_pitches`, `play`, and the nine machine constructors. These are the controlled constructors of
  `02-core-calculus.md` §5.7 and `../across-stages/03-machine-calculus.md` §2, and a trait method that constructed one
  would be a second path into a type whose whole point is that there is one.
- **The fourteen phase operations** of `02-core-calculus.md` §5.9. They live in a separate registry, in a scope ordinary
  source cannot name.

**Prompt 143 owns the count.** This table is the design it executes and the argument for each move; what shrank, what
did not, and by how much is that prompt's measurement to report, not a number promised here.

## 6. Inherent methods and type namespaces

`impl T { … }` — an impl block whose head names a type rather than a trait — declares **inherent** items in `T`'s
namespace. The orphan rule applies unchanged: an inherent impl lives in the package that declares `T`.

`T::x` reaches into that namespace, and it holds three kinds of thing: an enum's constructors, inherent functions, and —
through `Trait::method(x)` — a trait method named explicitly. Qualification always resolves, which is what makes the
strictness of method syntax affordable: any use that lookup refuses can be written out.

**Method syntax resolves by exact receiver.** `x.m(y)` takes the head of `x`'s already-known concrete type and finds `m`
among that type's inherent items and the methods of traits with a dictionary in scope for that head. Exactly one
candidate resolves; none is an error naming the type and the method; two is an error naming both, and under coherence
two can only mean an inherent item and a trait method sharing a name, which is refused at the inherent declaration
rather than at the use.

The three things this rules out are the three that turn a lookup into a search:

- **No method on a generic parameter.** `x : A` for a parameter `A` never acquires `.m`. Otherwise the elaborator would
  have to scan every trait in scope for one with a method called `m`, and adding a trait to a package would change what
  existing code means.
- **No auto-deref and no receiver coercion.** The receiver's type is the receiver's type.
- **No return-type-directed overloading.** `Duration::of(n)` is how a value of a named type is built, not an overloaded
  `of` chosen by what the surrounding expression wanted. The distinction that matters is between *fixing a type argument
  by checking* — `let out: List<Nat> = xs.collect();`, where `D` is determined by the annotation before the instance is
  looked up — and *choosing an instance because of a return type nobody has written down*, which makes elaboration
  depend on the order constraints were reached. The first is ordinary bidirectional checking and is everywhere in this
  language; the second is refused.

## 7. `Eq`, `Id`, and `DecEq`

Three things in this language are about two values being the same, and a reader must always be able to tell which one is
on the page.

- **`Id A x y`** is the propositional identity type of `02-core-calculus.md` §1.4, introduced by `refl` and eliminated
  by `J`. It is a *type*: a value of it is evidence. It is spelled with the word `Id` and never with `=`.
- **`Eq<A>.equal`**, written `x == y`, is a decision procedure answering `Bool`. It is a *computation*.
- **`DecEq<A>`** connects them:

```musa
enum Decision<P> { Yes(P), No(P -> Empty) }

trait DecEq<A> where Eq<A> {
    fn decide(x: A, y: A) -> Decision<Id<A, x, y>>;
}
```

`DecEq` is the dependent strengthening: it answers not whether two values are equal but *why*, with evidence either way,
and it is what index reasoning consumes — the length arithmetic of prompt 141's vector, and the constructor comparisons
a dependent `match` needs when an index is not syntactically apparent.

**`Eq` and not `PartialEq`**, and this is a recorded deviation from the approved plan, which wrote `PartialEq`. Musa is
total and exact. Every instance is a decision procedure over finite exact data, so reflexivity is not a property some
instances lack; a partial equality is an artifact of languages that must accommodate floating-point NaN, and Musa's
floats live inside registered primitives at the DSP edge, outside the source language entirely (`02-core-calculus.md`
§3). Naming the trait `PartialEq` would advertise a hole this language does not have.

The law relating the two is stated and, for a `DecEq` instance, provable rather than assumed: `equal(x, y)` answers
`true` exactly when `Id A x y` is inhabited. Where an instance supplies only `Eq`, the law is prose and the law suites
check it on the concrete instances. Where it supplies `DecEq`, the law falls out of `decide`, and Hedberg's theorem —
decidable equality implies uniqueness of identity proofs — is the reason admitting K in §1.4 of the core costs nothing
for the types Musa actually declares. `citations.md` §13.2 carries the reference.

## 8. Iterating and building

Two traits carry the collection surface, and `01-surface.md` §1.6 spells them out:

- `Iterable<C, A>` requires `fold_from_start` and `fold_from_end` and derives `map`, `filter`, and `collect`.
- `Buildable<C, A>` requires `empty` and `push`.

The direction is in the name and not in the type, which is the rule `01-surface.md` §1 already fixed for the two list
folds: both have the same signature, so a reader comparing two calls compares only the word that differs.

Three consequences worth stating. A container earns five operations by writing two, which is what derived methods are
for. `collect` needs its target fixed by checking, which is §6's distinction and not an exception to it. And the library
— `List`, its instances, the builders, and the length-indexed vector — is prompt 141's, so nothing here promises what a
container looks like, only how one is reached.

## 9. What is refused, and why

| Refused | Why |
| --- | --- |
| instance search with backtracking | a lookup that can fail and retry makes elaboration cost unpredictable and its result order-dependent; every mechanism in this document exists to make step 2 of §4 a table read |
| overlapping instances | two instances that both match need a tie-break rule, and every such rule makes a program's meaning depend on which instances are visible |
| specialization | it is overlap with an ordering, and the ordering is invisible at the use site; §1's derived methods give the reuse without the choice |
| defaulting | an unresolved constraint silently resolved to a default type is a program the author did not write; §4 refuses instead, naming the type and the trait |
| overridable default method bodies | an impl that omits a method and one that supplies it would produce different dictionaries with the same declaration, and nothing at the use site says which happened; derived methods are the non-overridable form |
| return-type-directed overloading | it is search, keyed on information that may not exist yet, and it makes elaboration order-dependent (§6) |
| auto-deref and receiver coercion | the receiver's type would stop being the thing the author wrote |
| method resolution on a generic parameter | it requires scanning every trait in scope, so adding a trait would change what existing code means (§6) |
| dispatch on a parameter other than the first | it is a search over a product of heads; coherence already determines the later parameters (§1) |
| associated types | later parameters determined by coherence do the same work with no new declaration form; this is re-openable with a program the parameters cannot express |
| blanket instances (`impl<A> Eq<A>`) | an instance head must be a type constructor applied to distinct type variables, or coherence cannot be checked by looking at heads |
| a user-written `Storable` instance | `Storable` is a structural fact about a type, not a claim a user may assert (`02-core-calculus.md` §1.2) |
| user-defined operator symbols, and operator sections | a fixed operator set is what lets §5's table be a table; a section is a partially applied call, and a call must be complete (`02-core-calculus.md` §1.3) |
| `do`-notation and a `Monad` trait | one constructor propagating is evidence for one operation, not for abstracting over which constructor it is (`01-surface.md` §1) |
| comprehensions | sugar over `map` and `filter` (Peyton Jones 1987, `07-comprehensions.md`), and the thing being sugared has no user yet |
| checked trait laws | a law obligation needs the constraint to carry evidence; laws are prose here and law suites in `05-verification.md` |

Each row is a refusal with a stated reason, so overturning one is an ordinary amendment with ordinary evidence: a
program that cannot be written, measured the way `../obligations.md` §10 requires. What none of them is is an oversight.
