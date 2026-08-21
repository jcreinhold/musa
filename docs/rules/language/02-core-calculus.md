# The one total source language

**Status: candidate.** The one total source language every surface construct elaborates into. This directory's hub —
most other pages here refine it.

The source language is a pure, strict, **total** calculus with lightweight dependency: two universes, one function type
whose result may mention its argument, records with named fields, parameterized enumerations, types refined by an
**index** from a decidable arithmetic domain (§1.5), pattern matching, structural recursion, definitional equality
decided by normalization by evaluation, and exact resource checking. Its metaprogramming — hygienic quotation, splicing,
provenance, and syntax traversal — is the primary extension mechanism and is specified in `11-quotation.md`. Surface
conveniences elaborate into the calculus before evaluation. It is deliberately more expressive than the event-track term
calculus and deliberately less expressive than a general-purpose programming language, because it may not diverge.

It is **one** language, and it builds **both** core values: an event track and a machine (`../constitution.md` §9).
There is no second calculus for the studio, and nothing about audio lives outside it — only the audio *history* does,
because a history is coinductive and no value here is.

**The course correction.** Prompt 128 admitted a dependent foundation on the evidence of
`stdlib/src/adapters/staff.musa`, and prompts 129–142 implemented the standard proof-assistant checklist around that
evidence: inductive families with indices, an identity type, universe-level metavariables, pattern unification with
postponement, a well-founded termination rule, and a trait engine with recursive constraint discharge. The audit of
every committed `.musa` program —
[`../../notes/research/language-design-closure/50-the-course-correction-audit.md`](../../notes/research/language-design-closure/50-the-course-correction-audit.md)
— found that none of that machinery has a user: no program declares an indexed family, writes an identity proof,
ascribes a universe level, states an instance constraint, or presents a non-structural measure. This document therefore
describes the surviving design: the same totality, the same exactness, the same macros, and the dependency real programs
use, with everything else deleted rather than wrapped. Where a section below refuses a feature the checklist would have
included, the refusal is priced here and is re-opened only by a committed program that needs it.

**And one step of the correction was taken back, by measuring what the corpus writes instead.** The audit's rule — every
mechanism names the committed program that requires it — proves nothing about a feature a self-written standard library
never had, because absence of use is evidence of the workaround, not of the absence of demand.
[`../../notes/research/language-design-closure/51-the-terseness-audit.md`](../../notes/research/language-design-closure/51-the-terseness-audit.md)
asked the question the audit did not, exhibited the workaround — seventeen compiler builtins spent on a single modulus,
a `fallback` parameter in a public signature, a bar summed at run time — and §1.5 is what deletes it. It is Xi and
Pfenning's stratified index and not a step back toward indexed families: the index language is separate, the solver
decides it, and every piece of machinery the audit removed stays removed.

## 1. Syntax

There is **one syntactic category**. Types are terms, so the grammar below is the whole language and there is no
separate type grammar to keep in step with it.

The grammar below is the whole *pure* calculus. §5.8 extends it with base types, their literals, and the compiler-owned
builtins over them — a conservative extension, proved there rather than assumed here, and the reason a musical domain is
a registration rather than an amendment. Read the two together: nothing below changes, and three productions are added.

```text
e  ::= x                                    % variable
     | Type 0  | Type 1                     % the two universes
     | (x : e) → e        | λx. e | e e     % function: formation, introduction, elimination
     | { f₁ : e, …, fₙ : e }                % record type; field types may mention the parameters in scope
     | { f₁ = e, …, fₙ = e } | e.f          % record introduction and projection
     | N e…                                 % an enumeration at its parameters
     | c e…                                 % a constructor of an enumeration
     | elim_N …                             % the generated eliminator of an enumeration
     | let x : e = e in e                   % non-recursive local binding
```

That is the whole grammar. There is no metavariable former, no identity former, and no level arithmetic: the two
features the previous calculus listed here exist nowhere in this one, and what replaced the reasoning they supported is
stated in §1.4 and §2.1.

**Two universes, fixed.** `Type 0 : Type 1`, and there is no `Type : Type`, no `Type 2`, and no level a program can
write or a checker can solve. Ordinary types and type constructors live at `Type 0`; `Type 1` exists so that a type of
types — a trait's dictionary type, an enumeration's parameters — has somewhere to stand. A declaration that would need a
third level is refused. The hierarchy exists to keep the checker from proving everything: a total language whose type
theory is inconsistent is not total in any useful sense.

**One function type, and every binder is explicit.** There is no implicit-binder form and no plicity annotation for a
rule to ignore. A generic function's type parameters are ordinary leading parameters; at a call site they may be
*omitted*, which is an elaboration rule of §2.1 and not a binder property. Nothing downstream — traits, `Syntax<Cat>`,
`Duration C` — needs a second binder form, and now nothing anywhere has one.

**Dependency where programs use it, in two forms.** A result type may mention an earlier explicit parameter, and a type
may carry **index arguments** from the decidable domain of §1.5 — `Pc(12)`, `Row(n)`, `Bar(p + q)`. `Duration C`,
`Position C`, and `Syntax<Cat>` are neither: their argument is an ordinary parameter, kept in the term and compared by
conversion, which is what §1.5's closing paragraph separates them on. Records are named products: a field's type may
mention the type parameters in scope at the declaration, and no field's type mentions a sibling field's *value*, because
no committed program has one that does. That last restriction is the difference between records and a telescope, and it
is what keeps projection a lookup rather than an instantiation.

### 1.1 Enumerations

A declaration group introduces enumerations with **parameters**, fixed across the whole declaration:

```text
data Tree<A> {
    Leaf,
    Node(left: Tree<A>, value: A, right: Tree<A>),
}
```

There are no per-constructor indices. A constructor's fields are ordinary types written under the parameters, and a
constructor at known parameters checks or infers like any function value. The index was the previous calculus's one
machine for making a `match` narrow a *type*; no committed program narrows a type by matching, and `11-quotation.md` §1
states why that is the whole of what quotation asks of the type system.

**This is not the index of §1.5, and the two must not be confused.** §1.5's index is an arithmetic argument the *type*
carries, decided by a solver and erased before evaluation; a constructor never chooses one, no `match` reads one, and
nothing in this section changes to admit it. What stays deleted here is the machinery that made a constructor's choice
of index a fact a pattern could learn: index unification, the dependent motive, and the forced and inaccessible patterns
that go with them.

**Strict positivity is checked on the declaration group**, so mutually recursive enumerations are checked together. A
recursive occurrence may not appear to the left of an arrow at any depth: a negative occurrence admits a fixed point,
and a fixed point admits divergence. An occurrence **nested** inside another enumeration's parameter —
`Body(items: List<StaffRead>)` — is admitted and carries **no induction hypothesis**: the eliminator's method takes such
a field and nothing more, because a hypothesis for it would be a synthesized functorial map rather than an application.
A fold *through* a container is written with the container's own fold, which is what the corpus already does.

**The core's elimination form is the generated eliminator `elim_N`**, non-dependent: its motive is a constant function
of the scrutinee. Surface `match` compiles through a case tree to nested eliminator applications, which is where
coverage is decided (§6.2). No rule unifies an index, because there is none to unify.

The finite and exact musical domains named in `01-surface.md` enter as declared enumerations and base types through
§5.8's conservative-extension obligation, not by assumption. The types the language carries for its own sake are:

```text
Unit  Bool  Nat  Ratio  Text
Duration C            % how much time: a nonnegative exact rational, in coordinate C
Position C            % when: an exact rational instant, in coordinate C
EventTrack C δ        % finite duration + finite multiset of occurrences of δ, in coordinate C
Primitive K δ δ       % one registered stepping unit: name, version, storable configuration
Machine K δ δ         % a finite description of a stepping process — not the history it produces
```

`C` ranges over `WrittenTime`, `PerformedTime`, and `PhysicalTime`; `K` over `AudioFrameStep`. Both are ordinary
declared enumerations whose closed literals index the three base types above. `δ` ranges over types satisfying §1.2's
storability check.

Five of these are decisions rather than conveniences:

- **`Ratio`, not a float.** Musical time is exact. A float appears at the performance and DSP edge and nowhere earlier,
  so no source-language type can hold one.
- **`Duration C` rather than `Ratio`.** A bare rational carries neither the coordinate nor the nonnegativity.
  `Duration WrittenTime` and `Duration PhysicalTime` are not convertible, so adding a written beat to a number of
  seconds is a type error rather than a number. Its constructors are where nonnegativity is checked, which is why they
  return `Result`.
- **`Position C` separate from `Duration C`.** *When* something happens and *how much time* it takes are different
  quantities with different algebras: `../events/00-purpose.md` states positions as the abelian group `(ℚ, +, 0)` and
  durations as the ordered monoid `(ℚ≥0, +, 0)`. A position plus a duration is a position; two durations add; two
  positions do not add at all, and their difference is a duration only when it is nonnegative, so that difference
  returns `Result`. One type for both would let beat 3 and three beats be added, which is the one arithmetic error a
  tagged rational exists to catch.
- **`Primitive K δ δ` separate from `Machine K δ δ`.** A primitive is one registered unit whose private state and step
  function belong to its owner; a machine is the finite composite built from primitives and the structural forms. Source
  code can build the second and can neither inspect nor forge the first's state.
- **`Machine K δ δ` is a description.** The audio history it produces is coinductive and is not a value at all
  (`../constitution.md` §4). Every `K` names what one step means, which is what stops a host block from becoming the
  unit of meaning.

`declaration κ`, `structure`, `piece`, `part`, `voice`, and `Term[A]` are not value types; nor is an audio history.
`EventTrack`, `Primitive`, and `Machine` are abstract in the sense that user code has constructors and controlled
transforms but no representation eliminator: there is no way to read a machine's private state, and no way to read a
track's occurrence list as a list.

**Two words, two meanings, and they are not interchangeable.** A **primitive** is a registered unit whose implementation
this language does not own — a `Primitive K δ δ` supplying its own `State`, `start`, and `step`, or a foreign operation
supplied by a host. A **builtin** is an operation the compiler owns and can reason about: the four families of §5.8,
which is where their conditions are checked. The boundary is information, not privilege — the compiler proves things
about builtins because it can see inside them, and states contracts for primitives because it cannot. Base types are
called *base types*, never primitives; `../events/` uses "primitive" in its ordinary English sense of *irreducible*,
where no registered unit is in scope.

### 1.2 Storable data, as a structural check

Every type is a **value type**. A type is *also* **storable data** when it contains no function at any depth and has a
versioned finite exact encoding. Storability is a structural fact the compiler computes over a declaration — base types
are storable; a record or enumeration is storable exactly when every field it stores is; an arrow type is never
storable, and neither is any container holding one, including at a depth the surface never writes out; an abstract
compiler-owned type is storable only when its owner guarantees the hidden representation holds no closure and supplies
the exact encoding.

It is not a constraint, not a trait, and not a name an author can write. There is no `Storable` in the source, no
instance table for it, and nothing to resolve: where the language needs the fact — the machine calculus's port types
(§2.3) and the expansion phase's boundary (§5.9) — the checker computes it directly from the type's structure. A
constraint exists to be asserted and discharged; a fact that must never be asserted by hand has no business being one,
and making it one was the only reason a class of type variable ever appeared in this calculus.

### 1.3 What the language does not have

There is no `fix`, no recursive binding, no `partial`, no while loop, no exception, no mutation, no I/O, no reflection,
no dynamic cast, and no effect handler. Functions may be higher-order. **A call must be complete, and a section is
written**: an application supplies every declared parameter, and an under-applied call is a type error rather than a
value — unless the slots it leaves are written `_`, in which case it is a **section** and denotes the function that
still needs them. `f(a, _)` is the function of one argument; `f(a)` on a two-parameter `f` is the same type error it
always was.

The rule is about *silence*, and the `_` is what removes the silence rather than the rule. What must never happen is
`f(x)` quietly becoming a function-valued result when `f(x, y)` was meant, and a written `_` names every slot at every
call, so a reader can still count the parameters without looking up the declaration. This does not need §1.2: a section
is a function value, storability is a structural fact the checker computes over a type (§1.2), and where a function
value may not stand §1.2 already refuses it. The earlier statement of this rule cited §1.2 for a rule §1.2 does not
imply — see note 51 §6, and prompt 142a, which repaired it.

Sections are surface, not a core form: `f(a, _)` elaborates to `fn (x) { f(a, x) }` before the core sees anything, and
several `_`s bind left to right, so `g(_, b, _)` is `fn (x, y) { g(x, b, y) }`. Composition follows from the same
enrichment and is an ordinary declaration in `std::core` rather than an operator.

Recursion is by definition, checked (§2.4), and never by a term former. There is no `Type : Type`, no cumulativity, no
universe polymorphism, no call-by-push-value stratification, no coinduction, and no first-class signal. Each is a
separate amendment under `../README.md`.

The course correction adds to this list, and each addition is priced in note 50's audit:

- **No identity type.** `Id`, `refl`, `J`, and K are gone. Definitional equality (§3) is what type checking needs and
  `==` is what programs need; no committed program proves an equality theorem. The K question the previous calculus
  priced in §1.4 dissolves with the type it was asked about.
- **No implicit arguments.** A type parameter is explicit at the definition and may be omitted at the call, where §2.1's
  one rule solves it or refuses. There is no second binder form, no insertion, and nothing for conversion not to read.
- **No indexed families, which is a narrower refusal than it was.** An *enumeration* has parameters only (§1.1): a
  constructor never chooses an index, so there is no index unification, no dependent motive, and no forced pattern. A
  *type* may carry an index, by §1.5, decided by a solver and erased before evaluation. The refusal is about what a
  constructor may determine and what a `match` may learn, and it is unchanged; it was never about whether `Row(24)` may
  be a type.
- **No metavariables in the term language, and no postponed constraints.** A generic argument the call site does not
  determine is an error there, not a hole that waits.
- **No termination measures.** Structural descent is the whole rule (§2.4).
- **No recursive instance resolution.** Traits are flat dictionaries with one-step lookup (`10-traits.md` §4).
- **No guards and no fall-through in `match`** — unchanged from before, restated here because this section is where the
  refusals live.

**There is also no signed integer type**, and the reason is elimination rather than a preference about numbers. `Nat` is
in the language to be the *inductive* numeric type: `zero | succ` is well founded, so its eliminator terminates by
construction. ℤ has no such structure and no least element to descend to, so an integer eliminator would either be a
`Nat` eliminator on the magnitude with a sign carried alongside — `Nat` plus bookkeeping — or an unbounded loop, which
totality forbids. An `Int` could therefore only be a base type with no eliminator of its own, and the governing design
rule then applies: removing it makes nothing impossible.

Nothing musical is left unsayable by that. Ordinary signed arithmetic is `Ratio`, which is signed, with the refinements
at its constructors. The signed *musical* quantities are domains: an `Interval` is a signed pair of written diatonic
steps and semitones (`03-musical-domains.md`, after *Open Music Theory* `016-intervals.md`), and a `Degree` is a signed
ordinal relative to a scale. Everything the language counts with `Nat` — repeat and occurrence counts, list lengths,
range bounds, the cost table of §4 — is nonnegative.

The musical falsifier is what settles it. Admitting `Int` would make `transpose(-3)` the obvious spelling, and that
number cannot say whether the composer wrote a descending minor third or a descending augmented second. The two are
different notes on the page and different chords underneath. Keeping the general signed integer out is what keeps
`Interval` load-bearing rather than decorative — roadmap §2's separation of written pitch from MIDI number, applied to
the number itself.

### 1.4 Identity: not admitted

This section previously stated the identity type and the K decision. Both are deleted, and the section is kept so
references to it land on the reason. The audit found no committed program that constructs, eliminates, or converts an
identity proof; the trial that motivated admitting K had already found that no program unifies an index at all. The
correct response to an unused equality type in a language that is not a proof assistant is deletion, not a cheaper
version of it. Equality here is definitional (§3, deciding types) or computational (`==`, deciding values), and those
two are each documented where they live. Re-opening is an ordinary amendment, and the evidence it needs is stated in
advance: a committed program that states an equality between two values and eliminates it.

### 1.5 Index refinement

A type may carry **index arguments**. An index is a value of a fixed decidable arithmetic domain, written in parentheses
after the type name so that it is distinguishable at a glance from a parameter in angle brackets:

```text
Pc(12)            % a pitch class in a 12-fold division
Row(n)            % a bijection onto Pc(n)
Bar(3/4)          % a bar whose contents sum to three quarters
Voicing(4)        % four voices, not a list that happens to have four
```

`Pc<A>` would be a type built from another type. `Pc(12)` is a type built from a *number*, and the whole of this section
is what that number is allowed to be, who decides when two of them agree, and what it is forbidden to do.

**The rule, in one sentence.** A type may carry index arguments drawn from a fixed decidable domain; indices are erased
before evaluation, never matched on, and two indexed types are the same type when the solver proves their indices equal.

#### The domain

Three sorts, all decidable, and all already in the language: `Nat`, exact `Ratio`, and **finite literal enums** — which
is the sort `Syntax<Cat>`'s category has always been. The expression language over them is:

```text
i ::= n                 % a literal of the sort
    | x                 % an index variable
    | i + i  |  i - i   % addition and subtraction
    | k * i             % multiplication by a literal
    | i = i  |  i < i   % comparison, where a condition is wanted
```

That is Presburger arithmetic, so equality and entailment in it are decidable, and the fragment the corpus generates —
equality of two linear forms — is decidable by normalizing both and comparing. **Nothing else enters.** An expression
outside the grammar is a refusal that names the expression, not a constraint that is postponed, approximated, or
assumed. Multiplication of two variables, division, an arbitrary function call, a `match`, a projection, and a variable
of any other sort are each that refusal.

An **index variable** is an ordinary parameter of index sort. It is introduced by appearing in a signature and solved at
the call by §2.1's first-order matching, from the written arguments' types — the same binder and the same rule a type
parameter gets, because an index that needed its own binder form would be the second Π §1 just finished deleting:

```text
fn row(pcs: List<Pc(n)>) -> Result<Row(n), RowFault>
fn follow(a: Bar(p), b: Bar(q)) -> Bar(p + q)
```

`n` in the first is read off the argument, and `p + q` in the second is computed rather than matched. An index variable
no written argument determines is the same refusal §2.1 already gives for a type parameter, naming the index.

#### What this is not

This is the confusion that costs the most, so it is stated before anything else about the mechanism.

**It is not inductive-family indices.** Enumerations have parameters only (§1.1), and that is unchanged. A constructor
never chooses an index; there is no index unification, no dependent motive, no forced or inaccessible pattern, and no
`J`. `family/` does not grow by one line. An indexed type is a **declared or base type applied to index expressions**,
and its refinements are the constructor judgments §2.2 already describes, now with a type to say what they enforced.

**It is not a dependent type**, and the difference is what an index may *say*, not where it is written down. A dependent
type may mention any term: a call, a `match`, a projection, a value the program computed. An index may mention only the
grammar above — literals, the four operators, and **variables of index sort that the signature binds**. `n` in
`fn row(pcs: List<Pc(n)>) -> Result<Row(n), RowFault>` is such a variable, bound the way §2.1 binds a type parameter and
solved the way §2.1 solves one, which is why an index needs no second binder form and gets none. An index position
holding anything else — `Row(f(x))`, `Row(match … )` — is the refusal below, named at the expression.

That restriction is what keeps the two decision procedures apart, and the code says so structurally: `index.rs` holds
the expression, its normal form, and `decide`, and **does not import `value.rs`**. It is handed an index expression and
answers; it never evaluates, never forces, and never sees a term. Reading an index position into that expression — or
refusing it — happens at the one place conversion meets one, which is also the only place that needs to know the
grammar. If `index.rs` ever needs a `Value`, the stratification has been broken.

**It is not a constraint, and not a trait.** There is no instance table, no dictionary, nothing to resolve, and nothing
an author can hand-write an instance for. §1.2 makes the same argument about storability, for the same reason.

#### Erasure is total

An index is erased **at quotation**. `quote` drops index arguments, so a read-back term carries none, and everything
downstream of elaboration is byte-identical to what it was before this section existed: a compiled term, an event track,
the `% musa-events-3` interchange format, `../across-stages/43-semantic-identity.md`'s digests, and every snapshot in
the suite. This is not a property to be tested for and hoped about; it is where the erasure happens, so that
byte-identity is the check.

The consequence worth stating plainly: **an index changes which programs are accepted and nothing else.** It cannot
change a value, a sound, a rendering, or a file. A design that let one would have made the index a runtime quantity, and
a runtime quantity is a term.

#### Conversion asks the solver

`Γ ⊢ T(a) ≡ T(b)` holds exactly when the solver decides `a = b` in the index domain. This is the **index-sort case of
§3's one relation**, not a second equality standing beside it: `≡` is the congruence generated by β, η, δ, ι together
with this theory, and what differs between the two halves is the procedure that decides them, not the judgment they
decide. §3's normalization by evaluation is not involved here and cannot be — it decides *terms*, and an index is not
one — so the conversion checker meets an index the way it meets any opaque payload: it asks, and takes the answer.

An index expression outside the grammar makes the **type** ill formed and is refused where the type is written, so a
comparison is never handed one. That is what makes the solver total on what reaches it, and it is what makes `≡`
reflexive; refusing at the comparison instead would answer "different" for two indices nothing can read, and a type
would fail to be the same type as itself.

This separation is the whole design, and the reason to state it as a rule rather than as an implementation note. The
calculus this one replaced braided two questions into one procedure: *are these terms equal* and *are these indices
equal* were both settled by unification, and the second is where the machinery and the cost lived. Two independent
procedures deciding one relation, neither of which knows the other exists, is not a smaller version of that; it is a
different shape, and it is why the term core does not grow to buy this.

A mismatch is reported in the domain's own words. "a `Row(12)` where a `Row(24)` was expected" is the diagnostic; the
solver's internal linear form never appears in a message.

#### What is honestly checkable

An index says something statically only when something static determines it, and this section says which case each type
is in rather than implying the strong one everywhere.

| Type | The index is | Checked |
| --- | --- | --- |
| `Pc(n)`, `Ic(n)` | a parameter fixed where the value is constructed | statically |
| `Row(n)`, `Icv(n)` | derived arithmetically from the carrier's | statically |
| `Voicing(k)` | a literal at the declaration | statically |
| `Bar(m)` | the sum of the contents' durations | statically **when the durations are**; otherwise a checked constructor and a runtime refusal, exactly as today |

The `Bar(m)` row is the honest one and the reason this table exists. `follow(a: Bar(p), b: Bar(q)) -> Bar(p + q)`
settles a bar written out note by note, which is the common case in notated music and in every expansion an adapter
produces. A bar folded out of a list whose length the checker does not know has no static sum, and gets the constructor
and the diagnostic it has now. Promising more would be promising a length-indexed list, and a length-indexed list is a
dependent type.

#### The refusals

Each of these is a refusal that names what it refused, and each is re-opened only by a committed program that needs it,
under `../README.md`:

- **An index expression outside the grammar above** — two variables multiplied, a division, a call, a `match`, a
  projection, or a variable of any sort but `Nat`, `Ratio`, and a finite literal enum. The refusal names the expression,
  and it is a refusal rather than an approximation: an index the solver cannot read is not assumed, not postponed, and
  not compared syntactically as a fallback.
- **Matching on an index.** There is no pattern that inspects one, because there is no value to inspect: it is erased.
- **An index in an eliminator's motive.** §1.1's eliminator is non-dependent and stays so.
- **An existential index** — a value carrying an index the type does not name. `Row(n)` for *some* `n` is not a type.
- **An index-level function.** The grammar is closed; a user cannot extend it, and neither can the standard library.
- **A proof term.** Nothing witnesses an index equality, because the solver decides it and produces no evidence a
  program could hold. The identity type stays deleted (§1.4).
- **An index over a type**, which would be a universe by another name (§1.3).

**The compiler-owned indexed types are a different mechanism, and this section does not absorb them.** `Duration C`,
`Position C`, `EventTrack C δ`, `Syntax<Cat>`, `Primitive K δ δ`, and `Machine K δ δ` look indexed and are not: their
arguments are **parameters**, present in the elaborated term and compared by §3's ordinary conversion. That is what
makes `Duration WrittenTime` and `Duration PhysicalTime` inconvertible, what lets `Syntax<Cat>` carry a directional
forgetting rule (`11-quotation.md` §1) that a symmetric conversion could not state, and what puts two of `Machine`'s
three arguments at *types* rather than at any arithmetic domain. An erased index could do none of the three, because
erasure is exactly the promise that the argument stops mattering after elaboration.

So the two live side by side and the distinction is which question the argument answers. A parameter says *what this is
a type of*, and survives. An index says *how many* or *how much*, is decided by arithmetic, and is erased. Prompt 142d
records this at the registration rather than leaving a reader to infer it from a kind.

## 2. Static semantics: bidirectional checking

There is no principal type and no global inference. A term is either **checked** against a type that is already known,
or its type is **inferred** and flows outward:

```text
Γ ⊢ e ⇐ A ⇝ t          % check: A is given; t is the elaborated core term
Γ ⊢ e ⇒ A ⇝ t          % infer: A is produced
```

Two rules switch between the modes, and they are the only two:

```text
Γ ⊢ e ⇒ A ⇝ t    Γ ⊢ A ≡ B                     Γ ⊢ A ⇐ Type ℓ ⇝ A′    Γ ⊢ e ⇐ A′ ⇝ t
────────────────────────────── Switch          ─────────────────────────────────────── Annot
Γ ⊢ e ⇐ B ⇝ t                                  Γ ⊢ (e : A) ⇒ A′ ⇝ t
```

`Switch` is where conversion (§3) is called, and it is the only place a type equality is decided. `Annot` is how an
author re-enters checking mode, and it is why an annotation is a language feature rather than a hint. Here `ℓ` is one of
the two levels of §1, and the rule knows which one by the declaration form it is checking.

Introduction forms check; elimination forms infer. `λx. e` checks against `(x : A) → B`, so a lambda binder needs no
annotation when its type is known; an application infers its function, then checks its argument against the domain, then
substitutes the argument into the codomain — which is where the lightweight dependency of §1 lives, and all it asks of
this section. A record literal checks field by field. A constructor checks against its enumeration at known parameters.
A projection, a variable, and a literal infer.

Annotations are written at public signatures and wherever information is not locally and deterministically available.
The elaborator never invents a term the author did not write except the one case §2.1 states, and that case invents a
*type argument*, never a value.

### 2.1 Generic instantiation is first-order and local

A call may omit leading type parameters. Each omitted parameter is solved **once, at the call**, by first-order matching
of the explicitly written arguments' inferred types against the declared parameter types, left to right:

```text
fn map<A, B>(function: A -> B, values: List<A>) -> List<B>
map(f, pitches)      % A := Pitch, B := NoteName, read off f's inferred type and the list's
```

Matching is syntactic over type constructors and variables: `List<A>` against `List<Pitch>` binds `A`; a variable
already bound is checked against its binding. A parameter no explicit argument determines is an **error at the call,
naming the parameter**, and the author writes the argument. Nothing is guessed: the set of solutions is empty or a
singleton by construction, because first-order matching against a known type is the most general unifier computed
without search.

**The spine is walked in two passes, and argument order is not semantically significant.** An argument whose parameter
type still mentions an unsolved parameter, and which no rule can infer — a bare `fn (x) { … }`, a record literal, a
`match` — is *deferred*: a placeholder stands in its slot, the rest of the spine is walked, and the argument is then
checked against the type that walk gave its slot. The deferred arguments are revisited in the order they were written,
after the position the call stands in has been matched, since that position is the last thing that can decide a
parameter.

This is not postponement. Every argument is elaborated exactly once; there is no constraint queue, no wakeup discipline,
no fixpoint, and no retry. Both the deferred set and the order of the second pass are fixed by the written argument
order, so the answer cannot depend on which branch ran first, and a parameter still undetermined at the end is the
refusal above rather than a new failure mode. What it buys is that `fold(combine, seed, values)` and
`fold(seed, values, combine)` type the same, so a library no longer has to order its parameters around the checker. Two
arguments do *not* defer: one whose slot is already settled, and a `fn` that annotates its own binder — checking the
second is what makes its annotation and the slot agree, so it is one of the arguments a deferred one is waiting for. The
design is Idris2's `checkRtoL` (`TTImp/Elab/App.idr`) without its fallback to left-to-right, which Musa does not need
because a Musa spine has no ambiguous name resolution to fall back from.

This is the whole inference story. It covers the corpus — the container operations, the folds, the phase traversals —
because in all of them every type parameter is read off an explicit argument or off the position the call stands in. It
deliberately does not cover a parameter that appears in *neither*: `compose(fn (p) { … }, fn (q) { … })` at
`Pitch -> Pitch` leaves the middle type unnamed, and the refusal names it rather than choosing one.

### 2.2 Refinement constructors and assertions

Literal checking is a partial *compiler* judgment `token ⇝ c : b`: malformed or out-of-range text produces a located
diagnostic, while accepted constants are mathematical booleans, bounded natural representations, reduced exact
rationals, or already-validated musical values. It is not a partial term operation.

Literal constructors enforce refinements such as nonnegative `Duration`, finite scale members, and row bijectivity.
These are constructor judgments returning a value or a located diagnostic, and the value they return may be **indexed**
by §1.5: `fn row(pcs: List<Pc(n)>) -> Result<Row(n), RowFault>` performs the same runtime check it always did and now
says in its type which modulus it checked against. That is this section's mechanism gaining a type, not a second one.

The refinement does not thereby become load-bearing in every operation that touches the value, and the reason is
erasure. An index is not evidence a program holds, constructs, or passes; it is arithmetic the checker does and then
discards, so an operation that does not care about `n` mentions `n` nowhere. The earlier statement of this section
refused type-level refinements on the grounds that no committed program manipulates such evidence before evaluation,
which is still true and is now the argument *for* the stratified form rather than against any form: there is no evidence
to manipulate. Named predicates used by `assert` return finite evidence that the assertion layer can report, and that is
a different thing — evidence a program does hold, about a value the checker could not settle.

### 2.3 Track and machine construction

Only compiler-owned builtins may construct an `EventTrack` or a `Machine`. `map_note_pitches` accepts a total
`Pitch → Pitch` and visits a documented subset of musical payload positions; it does not reveal them as a list. An event
track quote has a dedicated typing rule and cannot be encoded by string operations.

Machine construction is typed by the seven rules of `../across-stages/03-machine-calculus.md` §2 — `machine(p)`,
`identity`, `connect`, `beside`, `feedback`, `copy`, `drop`, `swap` — and every port type in them must pass §1.2's
storability check, computed structurally at registration. Those rules are stated there rather than repeated here,
because the machine's *step* semantics is stated there too and a typing rule separated from its step is a rule nobody
can check.

### 2.4 Termination is structural

A recursive definition is admitted when every recursive call it can make descends structurally: the call's recursive
argument is a variable bound by an enclosing `match` pattern to a **strict subterm** of the value being matched, through
constructors of the same enumeration. The elaborator sees the descent because it compiled the `match` (§6.2): the case
tree is where each hypothesis is bound, and a recursive call that reaches for something no pattern bound is refused,
naming the call.

That is the whole rule. There is no measure language, no well-founded-order argument, and no decreasingness proof to
write, because the thirteen recursive definitions in the corpus all descend structurally and the audit found no
fourteenth kind. A definition whose recursion does not fit the rule is rewritten over a fold — which is what the corpus
already does for everything but the thirteen — or the language is amended for it, with the program as evidence.

Nothing is opaque to the checker. There is no `partial`, no `fix`, no assumed-terminating annotation, and no way to ask
the checker to take a definition on trust. A definition whose termination the checker cannot see is **rejected**, and
the diagnostic names the call it could not establish.

The soundness obligation this creates is stated in §5: an accepted definition denotes a total function.

Top-level signatures are collected before bodies are elaborated, so a later declaration may be referenced. From each
body the checker records an edge to every free named declaration. **Non-recursive** declarations must form an acyclic
graph; a *recursive* declaration is the case this section admits, admitted through the structural check rather than
through the graph.

## 3. Dynamic semantics: evaluation, and conversion by NbE

**There is one conversion relation, and it is modulo the index theory.**

```text
Γ ⊢ A ≡ B    is the congruence generated by  β, η, δ, ι  ∪  T
```

β, η at functions and at records, δ for definitions, and ι on eliminators applied to constructors are the term half; `T`
is §1.5's index theory. Both halves are decided by one procedure: **normalization by evaluation** evaluates each side
into a semantic domain of values with closures and neutral terms and quotes it back to a normal form, comparing up to α,
and where it meets two index arguments it asks `T` instead of normalizing — because an index is not a term and cannot be
evaluated. Reduction is never performed on syntax.

This is *conversion modulo a theory* in Strub's sense, and the shape is what buys the metatheory: `T` is decidable, so
`≡` is decidable, so type checking is. Stating it as **one relation with two decision procedures** rather than as two
relations is deliberate. The procedures are and stay separate — `index.rs` does not import `value.rs`, and neither calls
the other — but what they jointly decide is a single judgment, and a reader who holds two equalities in mind will look
for a rule that relates them and find none, because there is nothing to relate.

**`≡` is an equivalence relation**, and each property is discharged in the half it belongs to. Reflexivity: α on the
read-back term for the term part, and `T`'s own reflexivity for the index part — which holds only because an index
outside §1.5's grammar is refused where the type is *formed*, so a comparison never meets one. Symmetry and transitivity
likewise hold in both halves. A conversion checker whose relation is not reflexive would let a term fail to check
against its own type, which is why the siting of that refusal is a rule and not an implementation preference.

```text
eval  : Env → Term  → Value          % closures for binders, neutrals for blocked eliminations
quote : Level → Value → Term         % reads a value back at a de Bruijn level, η-expanding at functions and records
Γ ⊢ A ≡ B   iff   quote(|Γ|, eval(ρ_Γ, A)) = quote(|Γ|, eval(ρ_Γ, B))
```

η at functions and at records is performed by `quote` rather than by a conversion rule, which is why two record values
with the same projections are convertible without a rule that inspects both at once — the property `10-traits.md`'s
one-instance-per-head rule rests on when two projections of the same dictionary meet.

**The index-sort case, stated once more because it is where the two procedures meet.** Where conversion meets two
indexed types at the same head, it asks §1.5's solver whether the index arguments are equal and takes the answer. The
two procedures are independent — this one settles terms by NbE, that one settles indices by arithmetic, and neither
calls the other — and that independence is what keeps the term core from growing to buy the stratum. `quote` drops
indices, so a normal form never carries one, which is what makes erasure a fact about where the code sits rather than a
property a test has to keep watching.

**The strategy is part of the specification.** Conversion is decidable either way, but the four rules below are what
make it *cheap*, and a checker that lost one would still be correct and would grind. They are rules, not notes:

- **Rigid heads first.** Two neutrals with different rigid heads are different, and neither side is unfolded to find
  that out.
- **One side at a time.** When a head must be unfolded, one side is opened and the comparison retried; both are never
  opened at once.
- **A numeral is compared as a number.** A closed value of a counting family is never expanded into a tower of its step
  constructor.
- **The meter is the backstop.** A budget crossing is a refusal (§4), never a hang and never a silent pass.

**What conversion does *not* have to do** is the other half of why it is cheap, and each absence is load-bearing rather
than incidental. There is **no index unification inside the checker**, because §1.1 admits no family indices — that is
where the complexity of a general indexed family lives. There are **no unary towers**, because of the numeral rule
above. There are **no proof terms to unfold**, because there are none at all; a language with them needs a way to seal
them, and that is what opacity annotations in other systems are for.

What remains is **large elimination** — a recursor whose motive is a type — and it is the one construct by which
evaluating a user's own definition can happen during conversion. It is bounded by §2.4's structural recursion and by the
meter. Naming it is what makes the three absences above a claim a reader can check rather than an accident of the
current implementation.

**The dependency chain runs one way, and it is the reason totality is kept rather than a matter of taste:**

```text
termination  ⟹  normalization  ⟹  decidable conversion  ⟹  decidable type checking
```

Conversion evaluates. If evaluation could diverge, conversion could diverge, and type checking would hang rather than
answer — a compiler that does not return and an editor that stops answering while the composer types. §2.4 is what holds
the left end of that chain up.

**Evaluation of an accepted program** is deterministic call by value, left to right for arguments and source order for
finite folds. A closure is a term with an immutable finite environment. Track constructors produce ordinary track
values; machine constructors produce ordinary machine values. **Neither runs anything.** Building a machine does not
step it, and building a track does not schedule it.

Exact values remain integers or reduced rationals. There is no floating-point type in the source language; floats appear
only inside a registered primitive's private state and at the device edge. Ordering of maps, declarations, diagnostic
witnesses, and provenance steps is source-stable, never hash-iteration order.

The evaluator that decides conversion and the evaluator that runs an accepted program are **one evaluator**. A second
one would be a second semantics, and the two would be obliged to agree by a law nobody could state, since one of them
works on open terms.

## 4. Resource acceptance

Normalization does not bound a terminating program to useful project size, and it does not bound *checking* either. Musa
therefore maintains one deterministic meter over checking, conversion, and evaluation. A finite aggregate operation
charges its known count and logical result shape before entering its loop or allocating its result; nested work is
charged when its enclosing operation is reached. A value is charged once, where it is constructed: an expression that
names, selects, or returns a value already built charges it no nodes and no bytes, and a constructor is charged its own
node and one field per part it holds rather than those parts again. Charging a value once per mention instead would make
a project's cost the product of its data size and its program size, which is not a measure of what the project builds.
All values remain private until the whole declaration graph succeeds, so exhaustion publishes neither a partial value
nor a partial score. The meter covers:

- instantiated definition count and closure environment size;
- eliminator work, including products induced by nested recursion;
- **conversion work: evaluation steps and quoted nodes charged during a `Switch`**;
- generic-instantiation work at call sites;
- generated occurrence count and core-term binding count;
- constructed machine node count and wiring depth;
- template/module instantiation count and static dependency depth;
- quotation size after typed substitution;
- nested evaluation levels — how far inside itself an evaluation currently is.

The three-outcome law. A check or an evaluation ends in exactly one of:

```text
accepted(t)  |  refused(Diagnostic)  |  exhausted(ResourceError)
```

**A budget may end a check or an evaluation, and may never silently accept, silently reject, or change an accepted
program's value.** Formally: if `run(b₁, e) ⇓ accepted(v₁)` and `run(b₂, e) ⇓ accepted(v₂)` then `v₁ = v₂`, for every
pair of budgets; and if `run(b₁, e) ⇓ refused(d)` then no budget yields `accepted` for `e`. Exhaustion is named as a
third outcome rather than folded into refusal, because a rejection that depends on a resource limit would make
acceptance a property of the machine.

A project whose next charge exceeds the deterministic budget is exhausted at that operation. The diagnostic names the
operation, metric, attempted amount, and limit. The current defaults are 200,000 reduction steps, 100,000 constructed
value nodes, 1,048,576 logical value bytes, 2,048 instantiated prelude entries, and 1,000,000 estimated occurrences,
with 256 nested evaluation levels. These are language-version constants, not timeouts or machine-memory observations;
the course correction re-derives them against the simplified checker and records the derivation in its final report.
Interactive cancellation remains an external compiler operation, not a language effect.

### 4.1 Nesting, and the room to reach the limit

Nesting is the one metric that goes back down. Every other counter measures what a run has spent and never returns; this
one measures how far in the run currently is, and a level is released when the work at that level finishes. A sequence
of a million siblings is one level deep, not a million. The limit is charged wherever an evaluation can stand inside
another one — evaluating an expression, eliminating a finite data value, descending into a syntax value, and quoting a
value back during conversion — so what the counter bounds is exactly what the machine spends stack on.

The metric exists because §5.9's traversal descends *through* the transformer's own branches: one level of a region's
nesting costs a whole chain of evaluator frames rather than one, so a deep enough region could exhaust a host stack.
NbE's `quote` adds a second such descent, over a value rather than a region. Ending the process there is not an outcome
this language may have. Musa is total and refuses by budget; a compiler that aborts instead has replaced a diagnostic
with a crash, and none of the three outcomes above describes what happened.

A limit is only a refusal if the machine survives long enough to print it. The implementation therefore owes a second,
non-normative obligation: **the compiler must run checking and evaluation with at least `nesting limit × frame ceiling`
bytes of stack**, where the frame ceiling is a stated measured bound on what one level costs. This is not a second guard
and does not decide acceptance — it is what makes the guard's decision reachable. Room without a limit only moves the
cliff; a limit without room only promises a diagnostic the process may not live to print. How a host provides the room
is the host's business, and because the budget does not move with it, every host accepts and refuses exactly the same
programs.

Shrinking the frame ceiling is welcome and changes nothing normative. Raising the *limit* is a cost-table version bump,
because a program refused at 256 levels and accepted at 512 is a program two compilers disagree about.

## 5. Metatheoretic obligations

**This section states obligations; it discharges none of them.** The course correction owes the audit, and it is smaller
than the one it replaces for the same reason the calculus is: there is no identity type to be consistent, no index
unification to be sound, no measure to be well founded, and no constraint solver to be deterministic. **The §5.7–§5.10
numbering is preserved** so that references from `../across-stages/` and the prompt stack still land on the obligations
they were pointing at.

| Obligation | What discharges it | Owed by |
| --- | --- | --- |
| **NbE soundness** — if `quote(eval(t)) = n` then `t ≡ n` | logical-relations argument over the value domain; differential test against a reference reducer | the correction's final phases |
| **NbE completeness** — convertible terms quote to α-equal normal forms | the same relation, at the identity substitution | the correction's final phases |
| **Decidability of conversion** — `Γ ⊢ A ≡ B` terminates and is an equivalence | normalization plus α-comparison; equivalence tested as a law | the correction's final phases |
| **Type preservation** — an elaborated term has the type its judgment gave it | induction over the elaboration rules | the correction's final phases |
| **Canonicity for closed storable types** — a closed term of a storable type evaluates to a constructor form | canonicity argument over the enumeration declarations | the correction's final phases |
| **Strong normalization** — every accepted term normalizes | reducibility over the calculus, given §2.4 and §1.1's positivity | the correction's final phases |
| **Strict positivity ⟹ consistency** — the empty type has no closed inhabitant | positivity plus the two-level hierarchy | the correction's final phases |
| **Coverage completeness** — an accepted `match` covers every constructor, and an unreachable arm is rejected | case-tree compilation (§6.2) | the correction's final phases |
| **Termination soundness** — an accepted recursive definition denotes a total function | the structural check of §2.4 | the correction's final phases |
| **Generic instantiation determinacy** — a call's omitted parameters are the unique first-order match, or the call is refused | the singleton property of first-order matching against known types | the correction's final phases |
| **Storability faithfulness** — the structural check accepts exactly the types with no function at any depth | induction over the type's structure; a compile-fail suite for functions in stored positions | the correction's final phases |
| **Track-construction safety** (§5.7) | re-derived against the surviving calculus | the correction's final phases |
| **Musical domains are a conservative extension** (§5.8) | re-derived against the surviving calculus | the correction's final phases |
| **The expansion phase, including law 11** (§5.9) | re-derived against the surviving calculus | the correction's final phases |
| **Numerals are a conservative extension** (§5.10) | convertibility with the tower at every count, and an elimination that agrees at each | the correction's final phases |
| **Budget independence** (§4) | the three-outcome law, tested across budget pairs | the correction's final phases |

### 5.7 Track-construction safety

**Obligation.** A track value is not built directly by user code; it is built by a private fragment `F = (t, B, d, n)`
where `t : Term[ScoreFact]`, `B` is a finite acyclic environment of term bindings, `d ∈ ℚ≥0` is the exact duration in
`WrittenTime`, and `n ∈ ℕ` is the exact occurrence count. Its well-formedness requires: every free term name of `t` is
in `dom(B)`; binding edges in `B` point to earlier completed bindings; every literal in `t` is a valid finite
`EventTrack WrittenTime ScoreFact` and every mark is a total payload map; `d` and `n` agree structurally with `t` after
its bindings are instantiated; and all facts have the requested scope, exact nonnegative placement, and a complete
`Origin`.

What must be established against the surviving calculus: that `follow` and `together` compose well-formed fragments into
well-formed fragments with duration `Σᵢdᵢ` and `maxᵢdᵢ` respectively and count `Σᵢnᵢ`; that replacing a fragment by a
marked reference to its completed binding preserves duration and count; and that building a checked track expression at
a valid placement returns a finite well-formed fragment whose closure is a closed checked `Term[ScoreFact]` with exactly
`n` occurrences. The induction runs on the declaration graph with §2.4's structural descent at the recursive nodes.

**What this obligation does not cover.** Machines. A machine's safety is M1–M8 of
`../across-stages/03-machine-calculus.md` §7, stated over a step relation this section has no vocabulary for, and the
two arguments share no lemma. Keeping them apart is deliberate: an argument that covered both would have had to talk
about a "value" general enough to be either, and that generality is the contextual-`Music` mistake in a new place.

### 5.8 Musical domains as a conservative extension

**Obligation.** Every compiler-owned builtin belongs to exactly one of four families, and that the families are disjoint
and exhaustive is a checked law:

- **δ-builtins** — every argument type and the result type is a base type or a finite constructor (`Option`, `List`,
  record) over base types, with no arrow anywhere in the signature;
- **structural eliminators** — the generated eliminators of §1.1's enumerations and the derived traversals over them;
- **track builtins** — the constructors and controlled transforms of §5.7; and
- **machine builtins** — the constructors of `../across-stages/03-machine-calculus.md` §2.

A δ-builtin must satisfy four conditions: **D1 inertness** — its base types have no eliminator, so no reduction rule
inspects a closed value of one and the only pattern that may match it is a literal or a catch-all; **D2 declared
partiality** — for every tuple of closed values of the declared argument types it either yields a closed value of the
declared result type or *declares* that it cannot, and never a stuck term, a panic, or a silent absence; **D3 purity** —
the result is a function of the argument values alone, with no ambient context, evaluation-order dependence,
hash-iteration order, or diagnostic emission; and **D4 finiteness** — the result's constructed-node count is bounded by
a function of the argument sizes, charged to the §4 meter before construction begins.

**The two ways D2 admits of declaring it.** A builtin declares partiality either **in the result type**, as an `Option`,
or **through §4's refusal outcome**, by answering with the sentence to say about the program. Which one is right is a
question about the *composer*, not about the implementation: `pc12_spelled` answers `Option` because a pitch class with
no spelling in a collection is a musical fact a piece may branch on, and `ratio_div` refuses at a zero divisor because
no branch repairs dividing by zero — the only repair is in the source. Wrapping the second kind in a value hands the
composer a failure they can do nothing with, and every caller then threads it; §4 exists so that the checker can say it
instead. What D2 forbids is unchanged and is the whole point: an application of a builtin to closed arguments of its
declared types never gets stuck, never panics, and never answers a bare absence the evaluator has to read as its own
defect. A refusal is not a diagnostic *emitted during* reduction — D3 still forbids that — it is one of the three
outcomes §4 already gives elaboration, reached instead of a value and carrying its own sentence.

**The theorem to establish.** Adding a base type with no eliminator, together with any finite set of δ-builtins over the
extended base set satisfying D1–D4, preserves every obligation in §5's matrix. A base type with no eliminator is inert
in conversion: it contributes no ι-rule, so two closed values of it are convertible iff they are the same constant.

**Corollary.** A later musical domain needs no new proof — it needs a base type with no eliminator, builtin signatures
containing no arrow, and a discharge of D1–D4.

The obligation concerns the type system only. That a German sixth spells its top note as an augmented sixth, that `ii`
is minor in a major collection, and that a harmonic-minor `III7` is honestly absent rather than rounded to a named chord
are claims about music, checked by the law suites in `crates/musa-compiler/tests/` and defined in
`03-musical-domains.md`. Neither statement substitutes for the other.

The implementation carries the premises rather than trusting them. The builtin-ownership registry records each
operation's family and declared signature, and its law suite checks that every builtin is classified exactly once, that
no δ-builtin signature contains an arrow, that every base type reachable from a δ signature is inert and admits no
destructuring pattern, and that evaluating each δ-builtin over a finite sample of its argument domains returns a value
of the declared type without panicking, diagnosing, or reporting a Rust-level absence at a non-`Option` result type.

A new base type is admissible only with a stated reason no existing domain can carry the distinction, a D1–D4 discharge
with its registry entries, and a row in `03-musical-domains.md` giving its definition, source, and a counterexample it
rules out.

### 5.9 The expansion phase

A syntax adapter runs before name resolution and checking: it is handed the region a composer wrote and answers with the
syntax that stands there instead. The adapter module is written in this same calculus and checked by this same checker,
under a **phase environment** that adds three things and takes nothing away — the phase-local types (`Syntax`,
`NodePath`, and `Syntax<Cat>` the parameterized base type); a separate registry of compiler-owned phase operations; and
the `Reading::Expansion` scope in which those names mean anything at all.

**Law 11 is unchanged and is the load-bearing one.** Ordinary source is read in a scope where none of those names
resolve, and no value of a phase-local type ever crosses the phase boundary: what leaves expansion is storable data, in
§1.2's structural sense.

The phase registry is a **second registry, not a fifth family**: nothing in it is a δ-builtin, a structural eliminator,
a track builtin, or a machine builtin, so §5.8's four families remain the four families of the source core, and the
disjointness law it names is unaffected.

**Obligations.** Sealed association and local decrease — a step minted for a proper child runs the algebra it was sealed
to, at any context, however many times, wherever it occurs. Normalization of the traversal, given §2.4's structural
descent, over higher-order contexts, capture, duplication, delayed use, and nested traversal. Path uniqueness and
derivation coverage. The budget charges for minting, running, and descent.

Prompt 127da's design law — "the fold is the only way into a syntax value" — was superseded before this amendment and
stays superseded. `Syntax` remains opaque, paths remain compiler-derived, `SourceInfo` remains unreadable, and
provenance remains derived rather than written.

### 5.10 Numerals as a conservative extension

A **counting enumeration** is a declared enumeration with no parameters and exactly two constructors: one with no fields
— its *floor* — and one whose single field is a recursive occurrence of the enumeration itself — its *step*. That
sentence is both the recognition rule and the reason the extension is sound, which is why it is one sentence and not
two. The property is derived from the declaration's own shape rather than nominated by the host, so `Nat` is not
privileged: any enumeration of that shape gets what follows, and one that loses the shape loses it.

A closed value of a counting enumeration is represented as **one node holding a count**, not as a tower of step
applications. The representation is definitionally the tower it stands for, and that is the obligation:

> **Conservativity.** For every count `n` and every counting enumeration `N`, the numeral `n : N` and the `n`-fold step
> applied to the floor are convertible, and every elimination computes the same answer at each. No program's meaning
> moves, and no type gains or loses an inhabitant.

Three consequences fix the design rather than merely describing it.

**One canonical form, not two.** Normalization collapses *toward* the numeral: the floor evaluates to the numeral zero,
and the step applied to a numeral evaluates to a numeral one higher. A value at a counting enumeration therefore never
holds a floor-or-step spine, and conversion at one is a comparison of counts rather than a walk. Admitting both forms
would make conversion — which §3 requires to be decidable *and* an equivalence — reconcile two representations at every
site that inspects a value, which is the cost NbE exists to avoid.

**The tower reappears one level per elimination, and no further.** ι-reduction on a numeral answers the floor's method
at zero and the step's method at `n`, with the field bound to the numeral `n-1`. A `match` does the same through §6.2's
case trees. So a fold over a count of `n` costs the `n` steps of real work it names, and *writing* the number costs one
step and one nesting level whatever it is.

**A count crosses the storable-data boundary as a count.** §1.2's storability admits a counting enumeration, and what
crosses is the number, not the tower. This is not an optimization: §4.1 charges one nesting level per level of a term,
so a tower deep enough to be worth writing is a value the boundary could not carry at all.

A count that would exceed the representation's range does not saturate and is not refused. The step constructor stays an
ordinary blocked spine there — a form the calculus already has, already types, and already means the right thing — so no
new failure mode is added for a case that costs more steps to reach than the budget admits.

**What this obligation does not cover.** Arithmetic. `+` on numerals is a δ-rule in the compiler's registry under §5.8's
first family, and it is bound by that section's obligations rather than by this one. §5.10 is a claim about
representation only: that a number written as a number is the number written as a tower.

## 6. Implementation boundary

`Term`, `Value`, `Closure`, evaluator environments, and resource proofs remain private to their crates. `musa-calculus`
exposes checking, conversion, and normalization over `Term` and exposes no `Value`: the semantic domain contains
closures over the evaluator's own representation, so publishing it would make every later change to evaluation a
breaking change for `musa-compiler`. A registered primitive's `State`, `start`, and `step` are private to the crate that
registers it. Passes may expose narrow internal queries, but no single-implementor public trait or pass-through facade
is added. The stable public result remains the compilation/snapshot contract.

### 6.1 This language and the event-track term calculus are two stages, not two cores

Musa has two calculi, and the boundary between them is **staging**, not an accident. This is the one place the word
"core" is used in two senses, so: there is one *source language* (this document) and two *core values* (a track and a
machine). The term calculus below is the residual syntax of the first of those values, not a third thing.

The classical arrangement for a functional compiler is surface → *enriched* calculus → *ordinary* calculus, where the
enriched layer is the ordinary one plus constructs whose semantics *is* their transformation away (Peyton Jones 1987,
§3.1). Musa is deliberately not that arrangement:

- This language has functions, records, enumerations, and eliminators. `../events/10-term-calculus.md` has none of them
  — six forms, a reference, and no abstraction at all.
- So the term calculus is not this language with the sugar removed. There is no simplifying transformation between them.

What connects them is **evaluation, applied twice**:

```text
source ──check──▶ core term ──evaluate (this document)──▶ Term[ScoreFact] ──evaluate (events)──▶ EventTrack WrittenTime ScoreFact
                    functions      eliminates functions      let + constructors     eliminates sharing
```

Three consequences:

1. **`Term[ScoreFact]` is a stage boundary, not an internal representation.** It is the reason `.musa.events` can be an
   interchange format at all: a residual program in a language with no functions is checkable, normalizable, and
   hashable by a consumer that knows nothing about this calculus.
2. **Totality is proved twice, separately.** §5's normalization obligation is about *this* language; T4 is about the
   term calculus. Neither implies the other. A machine's M1 is a third, independent totality claim — one *step*, not one
   evaluation — and it shares no lemma with either.
3. **Nothing may leak backwards.** A core term cannot mention a closure, and this language cannot observe a track's
   occurrence list. Where that discipline is enforced is §5.7, §1.2's storability check, and the event track's
   payload-opacity rule; this section is the statement of *why* they exist.

### 6.2 Patterns are compiled to case trees

Patterns nest. A pattern position may hold another pattern, and a `match` may scrutinize several subjects. Surface
`match` is compiled to a **case tree** and then to nested applications of the generated eliminators, which is where
coverage is decided: an accepted `match` covers every constructor, and an arm no constructor can reach is rejected as
unreachable rather than silently kept. Coverage is constructor coverage — with no indices there is no index unification,
and a reachable arm is one whose pattern some value of the scrutinee's type matches.

There are still no guards, no conditional equations, and no pattern on the left of a definition. A guard reintroduces
the fall-through between equations that a case tree exists to eliminate, and coverage in the presence of guards is
either unsound or requires deciding arbitrary boolean equivalence.

The compilation is Peyton Jones 1987 chapters 4–6's: the variable, constructor, empty, and mixture rules, with the
`FAIL`/fat-bar mechanism **not** adopted, because it exists to express failure between equations and Musa's arms do not
fall through. What a dependent core would have needed it for — narrowing a later binding's type through a constructor's
indices — is a feature this calculus does not have, deleted with the indices themselves, so the compiler is the plain
one. The subsystem earns its place on the evidence that motivated it: the `staff.musa` reader that destructured eight
fields to read one, and the dispatch table written as a chain. Paying once here rather than at every author is the same
trade §1 makes about every surface convenience.

## 7. Provenance on elaborated terms

**Every core term records the surface node it was elaborated from.** This is a property of the representation, not a
debugging convenience, and it is normative for three reasons:

1. **A checker reports failures in terms the author never wrote.** A conversion failure is between two normal forms
   produced by `quote`, and a normal form is not source. Without an origin on the terms involved, the best available
   diagnostic is two unfamiliar expressions and no place to point. With one, the failure lands on the expression whose
   elaboration created the mismatch.
2. **Quotation needs origin to be a fact rather than a reconstruction.** `11-quotation.md` derives a quoted node's
   identity as `Derived { origin, quotation, path }`. If `origin` had to be reconstructed by matching a term against the
   syntax it might have come from, the derivation would be a heuristic and the identity law would be unprovable. The
   whole reason hand-allocated role integers leave adapter code is that this field already exists.
3. **Origin paths are a governing obligation** (`../across-stages/`), and a stage that dropped provenance and re-derived
   it later would be re-deriving something it had.

An elaborated term's origin is preserved by substitution, by instantiation, and by `quote`: the normal form of a term
carries the origins of the terms it was built from, and where `quote` η-expands, the expansion carries the origin of the
term it expanded.

Origins are **not** part of conversion. Two terms with different origins and the same normal form are convertible;
otherwise provenance would change what a program means, and a compiler that type-checked differently after a file was
moved would be the result.

**Two things a core term carries that conversion does not look at**, and they are one rule rather than two exceptions:
an origin and a binder's written name. Each exists so that a later stage can say something an author will recognize —
where a term came from, what they called a variable — and neither may decide what a program means. `(x : A) → B` and
`(y : A) → B` are the same type, quotation writes whichever name the type it is quoting at happens to hold, and a
conversion that compared names would answer `false` for two spellings of one function type.

The theoretical provenance of this calculus — every construction, and the chapter or paper it comes from — is
[`citations.md`](citations.md) §13. The short form: nothing here is novel, and after the course correction there is
markedly less of it. Bidirectional checking, first-order unification, NbE, case-tree compilation, and structural
recursion are all older than the proof assistants the previous calculus cited. Musa's own contributions remain the
storability check, the three-outcome budget law, the phase environment of §5.9, and the refusals of §1.3 — each priced
where it stands.
