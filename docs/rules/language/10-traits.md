# Traits, methods, and namespaces

A trait is how Musa writes an operation whose body depends on the type it acts on: `==`, `<`, `+`, the collection folds,
and the one domain trait the standard library declares (`Transposable`, in `std::pitch`). The design rule, stated before
any mechanism because every mechanism answers to it:

> **Resolution is a lookup, not a search.** The complete algorithm is §4's three steps, and they fit in a paragraph
> because the corpus asked for exactly that much: one user trait with two concrete instances, four compiler-owned
> traits, and no instance anywhere that carries a constraint of its own. The previous calculus implemented recursive
> constraint discharge, postponement on unknown heads, and a termination measure for the recursion — a typeclass solver,
> in everything but name, with no committed program behind a single one of its features. The course correction
> ([`../../notes/research/language-design-closure/50-the-course-correction-audit.md`](../../notes/research/language-design-closure/50-the-course-correction-audit.md))
> deletes it and fixes this document as the flat replacement.

## 1. A trait is a record of methods

A `trait` declaration introduces a function from its type parameters to a record type, and an `impl` declaration
introduces a value of that record at particular arguments:

```musa
trait Eq<A> {
    fn equal(x: A, y: A) -> Bool;
}
```

```text
Eq        : (A : Type 0) → Type 0
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

A method — required or derived — may state a `where` clause over the trait's parameters and its own type parameters.
`Iterable.map` says `where Buildable<D, B>`: the method elaborates to a function taking the `Iterable` dictionary and a
`Buildable<D, B>` dictionary, and at a use site the second is found by §4's rules like any other. A method constraint is
an ordinary dictionary *parameter*; what no declaration in this language states is a dictionary *obligation that must be
synthesized from other instances* — that is §9's refused row, not a clause with a quieter spelling.

**Laws are prose.** A trait may state the equations its instances are expected to satisfy, and this specification and
the standard library state them for the traits they declare. Nothing checks them. A law that must be checked is written
as a `05-verification.md` law suite over the concrete instances, which is where the rest of Musa's checkable claims
live.

## 2. Coherence

**There is at most one instance per (trait, head type) in a whole program.** Not per module, not per import graph — per
program. A second impl for the same trait and head is an error naming both declarations, wherever they are and whether
or not any code uses both.

The property this buys is stated precisely, because "coherence" is used loosely elsewhere:

> For every use site of a method at a given trait and type arguments, the dictionary the elaborator supplies is unique
> up to conversion, and does not depend on the order in which the elaborator reached the site, on which modules the use
> site imported, or on where in the program the dictionary came from.

Uniqueness *up to conversion* is what the core gives and what the argument needs. A dictionary is a record value, and
`02-core-calculus.md` §3 decides record equality by η at `quote`: two records with the same projections at the same type
are convertible without a rule that inspects both at once.

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

## 4. Instance lookup, in full

Resolving a constraint `C<τ₁, …>` is three steps and no recursion:

1. **Look in the local context first.** A dictionary bound by an enclosing `where` clause — on the function, or on the
   trait method being elaborated — whose trait and arguments match is used, and lookup stops.
2. **Otherwise look up the head.** Take `τ₁`'s head constructor and find the one instance declared for `(C, that
   constructor)`. Its later parameters fall out of the instance (§1). If there is none, the constraint is refused,
   naming the type and the trait.
3. **Anything else is an error.** A constraint whose head is a type variable with no local dictionary is refused at the
   use site, naming the trait and the variable, and the repair is the `where` clause step 1 would have read.

There is no fourth step. No instance carries a `where` clause, so step 2 never produces new constraints; no constraint
waits on an unknown type, because a generic body is checked once against its signature and reads only its own local
dictionaries; and nothing is postponed or retried, because the unknown case is step 3, not a queue.

**Local beats global**, and the rule is still worth stating now that it is the whole interaction between the two steps.
It fixes *determinacy*: the elaboration of a generic function's body is decided by its own signature, so instantiating
`A := Tying` later cannot reroute a call that was already checked, and adding an instance elsewhere in the program
cannot change a body that was already checked.

An `impl` whose declaration would need a constraint — `impl<A> Eq<List<A>> where Eq<A>` is the canonical shape — is
refused at the declaration, and §9's table says why and what to write instead: the explicit ordinary function
`fn list_eq<A>(eq: Eq<A>) -> Eq<List<A>>`, or a macro that generates the concrete boilerplate. Composition the author
can read is the extensibility mechanism; synthesis the author cannot is what was deleted.

Lookup work is charged to the §4 meter of `02-core-calculus.md` like every other checking cost.

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
| `xs[i]` | `Index<C, I, A>.at` | `C → I → Option<A>`, or `C → I → A` for an index type that cannot be out of range | none today; a container with an index is its first instance |

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
- **`nat_fold`.** `Nat` is not a container; this is its eliminator, and `Iterable` is about containers.
- **`repeat` and `range`.** `repeat n { body }` is the notation-facing fold over musical material and keeps its
  spelling; `range` builds a list and is a library function once there is a library.
- **The track and machine builtins** — `transpose`, `stretch`, `retrograde`, `invert`, `shift`, `together`,
  `map_note_pitches`, `play`, and the nine machine constructors. These are the controlled constructors of
  `02-core-calculus.md` §5.7 and `../across-stages/03-machine-calculus.md` §2, and a trait method that constructed one
  would be a second path into a type whose whole point is that there is one.
- **The phase operations** of `02-core-calculus.md` §5.9. They live in a separate registry, in a scope ordinary source
  cannot name.

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

- **No method on a generic parameter.** `x : A` for a parameter `A` never acquires `.m` — unless the parameter carries a
  `where` dictionary with an `m`, which is §4 step 1 and is a projection, not a scan. Otherwise the elaborator would
  have to scan every trait in scope for one with a method called `m`, and adding a trait to a package would change what
  existing code means.
- **No auto-deref and no receiver coercion.** The receiver's type is the receiver's type.
- **No return-type-directed overloading.** `Duration::of(n)` is how a value of a named type is built, not an overloaded
  `of` chosen by what the surrounding expression wanted. The distinction that matters is between *fixing a type argument
  by checking* — `let out: List<Nat> = xs.collect();`, where `D` is determined by the annotation before the instance is
  looked up — and *choosing an instance because of a return type nobody has written down*, which makes checking depend
  on information nobody supplied. The first is ordinary bidirectional checking and is everywhere in this language; the
  second is refused.

## 7. `Eq`, and the equalities that are not here

Two things in this language are about two values being the same, and a reader must always be able to tell which one is
on the page.

- **Definitional equality** (`02-core-calculus.md` §3) is the checker's judgment, decided by NbE. It is how two *types*
  are compared, and no program writes it.
- **`Eq<A>.equal`**, written `x == y`, is a decision procedure answering `Bool`. It is a *computation*.

The third thing from the previous calculus — the propositional identity type `Id`, with `refl`, `J`, and the `DecEq`
trait that connected it to `==` — is deleted, for the reason `02-core-calculus.md` §1.4 states: no committed program
constructs or eliminates an equality proof. The law that section stated survives as prose and as law suites: `equal(x,
y)` answers `true` exactly when `x` and `y` are the same value, and the suites check it on the concrete instances.

**`Eq` and not `PartialEq`**, and this is a recorded deviation from the approved plan, which wrote `PartialEq`. Musa is
total and exact. Every instance is a decision procedure over finite exact data, so reflexivity is not a property some
instances lack; a partial equality is an artifact of languages that must accommodate floating-point NaN, and Musa's
floats live inside registered primitives at the DSP edge, outside the source language entirely (`02-core-calculus.md`
§3). Naming the trait `PartialEq` would advertise a hole this language does not have.

## 8. Iterating and building

Two traits carry the collection surface, and `01-surface.md` §1.6 spells them out:

- `Iterable<C, A>` requires `fold_from_start` and `fold_from_end` and derives `map`, `filter`, and `collect`.
- `Buildable<C, A>` requires `empty` and `push`.

The direction is in the name and not in the type, which is the rule `01-surface.md` §1 already fixed for the two list
folds: both have the same signature, so a reader comparing two calls compares only the word that differs.

The derived methods' constraints are method-level `where` clauses in §1's sense: `map` needs `Buildable` at its target,
so `map`'s elaborated form takes that dictionary as a parameter, and §4 finds it at the use site where the target is
concrete. No instance is ever synthesized from another; the dictionaries a body reads are the ones its caller had.

Three consequences worth stating. A container earns five operations by writing two, which is what derived methods are
for. `collect` needs its target fixed by checking, which is §6's distinction and not an exception to it. And the library
— `List`, its instances, and the builders — is ordinary source, so nothing here promises what a container looks like,
only how one is reached.

## 9. What is refused, and why

| Refused | Why |
| --- | --- |
| instance search with backtracking | a lookup that can fail and retry makes checking cost unpredictable and its result order-dependent; every mechanism in this document exists to make step 2 of §4 a table read |
| instance `where` clauses (`impl<A> Eq<List<A>> where Eq<A>`) | discharging one is recursive synthesis — the solver this document replaced; write `fn list_eq<A>(eq: Eq<A>) -> Eq<List<A>>` and pass the dictionary, or generate the concrete instances with a macro |
| supertraits (`trait Ord<A> where Eq<A>`) | a dictionary carrying another dictionary is the same synthesis one level down; the corpus has none, and a trait that wants another's methods states the constraint on the methods that use them |
| postponed constraints | a constraint whose head is unknown is step 3's error, not a queue entry; nothing about a program's meaning waits on checking order |
| overlapping instances | two instances that both match need a tie-break rule, and every such rule makes a program's meaning depend on which instances are visible |
| specialization | it is overlap with an ordering, and the ordering is invisible at the use site; §1's derived methods give the reuse without the choice |
| defaulting | an unresolved constraint silently resolved to a default type is a program the author did not write; §4 refuses instead, naming the type and the trait |
| overridable default method bodies | an impl that omits a method and one that supplies it would produce different dictionaries with the same declaration, and nothing at the use site says which happened; derived methods are the non-overridable form |
| return-type-directed overloading | it is search, keyed on information that may not exist yet (§6) |
| auto-deref and receiver coercion | the receiver's type would stop being the thing the author wrote |
| method resolution on a bare generic parameter | it requires scanning every trait in scope, so adding a trait would change what existing code means (§6) |
| dispatch on a parameter other than the first | it is a search over a product of heads; coherence already determines the later parameters (§1) |
| associated types | later parameters determined by coherence do the same work with no new declaration form; this is re-openable with a program the parameters cannot express |
| blanket instances (`impl<A> Eq<A>`) | an instance head must be a concrete type constructor, or coherence cannot be checked by looking at heads |
| user-defined operator symbols, and operator sections | a fixed operator set is what lets §5's table be a table; a section is a partially applied call, and a call must be complete (`02-core-calculus.md` §1.3) |
| `do`-notation and a `Monad` trait | one constructor propagating is evidence for one operation, not for abstracting over which constructor it is (`01-surface.md` §1) |
| comprehensions | sugar over `map` and `filter` (Peyton Jones 1987, `07-comprehensions.md`), and the thing being sugared has no user yet |
| checked trait laws | a law obligation needs the constraint to carry evidence; laws are prose here and law suites in `05-verification.md` |

Each row is a refusal with a stated reason, so overturning one is an ordinary amendment with ordinary evidence: a
program that cannot be written, measured the way `../obligations.md` §10 requires. What none of them is is an oversight.
