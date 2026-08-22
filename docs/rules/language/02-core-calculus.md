# The one total source language

**Status: candidate.** The one total source language every surface construct elaborates into. This directory's hub —
most other pages here refine it.

The source language is a pure, strict, **total** calculus with **dependent types**: one function type whose result may
mention its argument, inductive families with parameters and indices, a universe hierarchy, pattern matching compiled to
case trees, structural recursion, metavariables solved by pattern unification, definitional equality decided by
normalization by evaluation, and exact resource checking. Its metaprogramming — hygienic quotation, splicing,
provenance, and syntax traversal — is the primary extension mechanism and is specified in `11-quotation.md`. Surface
conveniences elaborate into the calculus before evaluation. It is deliberately more expressive than the event-track term
calculus and deliberately less expressive than a general-purpose programming language, because it may not diverge.

It is **one** language, and it builds **both** core values: an event track and a machine (`../constitution.md` §9).
There is no second calculus for the studio, and nothing about audio lives outside it — only the audio *history* does,
because a history is coinductive and no value here is.

**One theory, and this document is its presentation.** The calculus described here is intensional Martin-Löf type theory
in the shape of Idris2's `Core/TT`: Π types, inductive families, case trees, metavariables, and a universe hierarchy,
and nothing beside them. `../constitution.md` §9 is where that was decided, at prompt 143, and
[`../../notes/research/language-design-closure/53-one-theory.md`](../../notes/research/language-design-closure/53-one-theory.md)
carries the evidence: the core had been carrying three partial mechanisms — an erased index sort, a non-dependent
eliminator, and first-order instantiation in place of unification — each admitted to avoid a dependent core and each
approximating one badly. Given families, all three disappear rather than being replaced.

Two earlier records stand unedited beside that one, because a reader of this document will meet their traces in the code
before the code catches up.
[`../../notes/research/language-design-closure/50-the-course-correction-audit.md`](../../notes/research/language-design-closure/50-the-course-correction-audit.md)
deleted a proof-assistant checklist that had been built around prompt 128's admission and that no committed program
used;
[`../../notes/research/language-design-closure/51-the-terseness-audit.md`](../../notes/research/language-design-closure/51-the-terseness-audit.md)
found that the deletion had gone one step too far and proposed the stratified index that §1.5 held until prompt 143
retired it. What separates this presentation from the checklist note 50 deleted is not the feature list but the *reason*
each feature is here: a family is here because seventeen compiler builtins spend themselves on one modulus without it,
unification is here because implicit arguments and index refinement are unavailable without it, and the apparatus that
has no such argument — tactics, proof search, hint databases, opt-outs from totality — stays refused and is listed as
refused in §1.3.

**What this document does not yet describe is the code.** The specification is written before the prompts that implement
it, which is the rule prompt 142c followed and the reason it is cited rather than repeated. Prompts 146–163 are that
implementation, and where a section below says a mechanism is specified but not yet built, it names the prompt that owes
it.

## 1. Syntax

There is **one syntactic category**. Types are terms, so the grammar below is the whole language and there is no
separate type grammar to keep in step with it.

The grammar below is the whole *pure* calculus. §5.8 extends it with base types, their literals, and the compiler-owned
builtins over them — a conservative extension, proved there rather than assumed here, and the reason a musical domain is
a registration rather than an amendment. Read the two together: nothing below changes, and three productions are added.

**The core term language is seven constructors**, and the surface language elaborates into them:

```text
t  ::= x                                    % a variable, a de Bruijn index into the local scope
     | n                                    % a name, carrying the role its context entry gives it
     | ?m[σ]                                % a metavariable at its scope (§2.1)
     | bind x. t                            % one binder former, at λ, Π, or let (below)
     | t t                                  % application
     | k                                    % a literal: a base-type payload, or a numeral at a counting family
     | Type ℓ                               % a universe at a level expression (§1, *The hierarchy*)
```

The binder former carries which binder it is and how its argument is filled:

```text
bind  ::= λ(x : A)  |  (x : A) →  |  let x : A = t
fill  ::= written | inferred
```

Seven, and the list is closed by an argument rather than by taste. Idris2's `Core/TT/Term.idr` carries twelve; the five
musa does not keep are the two quantity-carrying forms it has no linearity for, the laziness forms it has no laziness
for, and the erasure marker it has no erasure for. Everything else a term could be is a *context* fact rather than a
term fact: a name's reduction behaviour is what its `Definition` says — undeclared, a compiled case tree, a constructor,
a type constructor, a registered base type, or a compiler builtin — and never a constructor of the term language. A term
former holding a declaration is a context entry smuggled into a term, and §6 is where that boundary is stated as a rule.

The surface forms that look like term formers are elaborations into this grammar. A record type is a one-constructor
family and a projection is a generated function whose body is a one-branch case tree (§1.1); `match` is a case tree
(§6.2); a section is a λ (§1.3). None of them is a shape the kernel knows.

**The hierarchy.** `Type ℓ` for a **level expression** ℓ:

```text
ℓ ::= 0  |  u  |  ℓ + 1  |  max ℓ ℓ
```

`u` is a level parameter. Every level expression has a **normal form** `max(k, u₁+k₁, …, uₙ+kₙ)`: a constant and one
term per variable, deduplicated, each variable at the largest offset it was written with. `max` and `+1` build that form
as they go, so there is no separate normalization step and no un-normalized level to compare by mistake. Equality of
normal forms is equality of levels for every valuation of the variables, which makes level equality *decidable and
complete* — the incompleteness Lean lives with comes from `imax`, which is the next paragraph.

Levels are generalized **per declaration**, as in Lean: a declaration's level parameters are collected when it is
elaborated and instantiated at each use, so nothing is constrained from a distance and separate checking is unaffected.
There is no global constraint graph, and no typical ambiguity across declarations. There is also no `imax`. Lean needs
that former so `∀ x : A, B` lands in `Prop` when `B` does — impredicative `Prop` is what makes the level of a Π depend
on whether its codomain's level is zero. Musa has no `Prop` and no impredicativity, so a Π takes the plain `max` of its
parts, and the fourth former does not exist. A reader coming from Lean will look for it; this paragraph is why it is
absent.

**Three things travel under one word, and musa takes two of the three.**

|  | What it is | Who does it |
| --- | --- | --- |
| **Hierarchy** | `Type 0 : Type 1 : Type 2 : …`, predicative | everyone, musa included |
| **Cumulativity** | a *subtyping* rule: `A : Type i` implies `A : Type j` for `i ≤ j` | Coq; **not musa** |
| **Polymorphism** | a definition abstracts over levels: `id : {u} → (A : Type u) → A → A` | Agda, Lean, Coq ≥ 8.5, musa |

The hierarchy is **non-cumulative**: `A : Type i` does not make `A : Type i+1`. A definition that must work at several
levels is written level-polymorphically, which says the same thing and says it in the type. Cumulativity would be
subtyping — a directional `≼` where the calculus has a symmetric `≡` — and `../constitution.md` §9 refuses subtyping in
every form, on the argument that acceptance must be decided by one relation. Cumulativity and polymorphism are
alternative answers to the same ergonomic problem, and polymorphism is the one that does not make conversion
directional: under `≼` a metavariable's constraints have no most-general solution, which is Agda's own stated reason for
avoiding it. Agda and Lean 4 are both non-cumulative; the non-cumulative theory accepts strictly fewer terms than the
cumulative one and so cannot be unsound where that one is sound.

Conversion compares levels with `=`, never with `<`. Only the *formation* rules take a `max`.

**Nobody writes a level, and the defaulting rule says what happens when nothing determines one.** `Type` with no
argument elaborates to `Type ?u` at a fresh level unknown. An equation between two levels is solved where exactly one
level solves it — a variable standing alone takes what it met, and `max(k, u+j) = c` for a closed `c` above `k` gives
`u = c − j`. Every other equation is **postponed**, because `max ?u ?v = 3` has several solutions and picking one would
decide a program's meaning by the order its constraints arrived in. At the end of the declaration:

- a level unknown that occurs in the declaration's **type** is *generalized* — it becomes a level parameter, and each
  use site picks its own;
- every other level unknown is *defaulted to `0`*, because a level the type does not mention cannot be chosen by a use
  site, and refusing the program over an unknown no caller can observe would reject it for nothing;
- the postponed equations are then read once more, and one that is still false is refused.

**Where generalization applies.** At a *definition*'s boundary. An inductive family's own levels are inferred and then
defaulted by the same rule, so a family lands at one level rather than being generic over several; whether the standard
library needs a family that is level-polymorphic is a measurement rather than an assumption, and it is recorded here so
that the two halves of "per declaration" are not read as one.

The rule is stated here rather than left to fall out of solver order for §4's reason applied to levels: a program whose
acceptance depends on the order constraints arrived in is a program two implementations disagree about. It is
incomplete, deliberately — `max(?u+1, ?v+1) = 3` is refused although it has solutions — and Agda and Lean are incomplete
in the same place.

There is no `Type : Type`. A total language whose type theory is inconsistent is not total in any useful sense.

**One function type, two fillings.** `(x : A) → B` is the only function type, and `B` may mention `x`. A binder records
whether its argument is **written** by the caller or **inferred** by elaboration; that is a property of the binder, not
a second Π, and conversion does not read it. An inferred argument is solved by §2.1's unifier or reported at the call.

**Dependency is what Π means.** A result type may mention any earlier argument, and a family's index may be any term of
the index's type — a call, a projection, a value the program computed. `Pc(12)`, `Row(n)`, and `Bar(m)` are ordinary
applied type constructors, not a second sort of thing, and they are compared by §3's one conversion relation like
everything else. `Duration C`, `Position C`, and `Syntax c` are the same mechanism: an argument, kept in the term,
compared by conversion. The distinction the previous calculus drew between a *parameter that survives* and an *index
that is erased* does not exist here, because nothing is erased.

### 1.1 Inductive families

A declaration group introduces **inductive families**: type constructors with **parameters**, fixed across the whole
declaration, and **indices**, which each constructor chooses.

```text
data Tree<A> {
    Leaf,
    Node(left: Tree<A>, value: A, right: Tree<A>),
}

data Vec<A> : (n : Nat) -> Type {
    Nil                                    : Vec<A>(0),
    Cons(head: A, tail: Vec<A>(n))         : Vec<A>(n + 1),
}
```

A parameter is the same in every constructor's result; an index is not, and that difference is the whole of what a
family adds. `Nil` chooses `0` and `Cons` chooses `n + 1`, so matching a `Vec<A>(m)` against `Nil` teaches the checker
that `m` is `0` — which is what *refinement* means and what the previous non-dependent eliminator could not do.

**Elimination is dependent.** The generated eliminator's motive is a **family**, not a type: for `data N : Δ → Type` it
is `(δ : Δ) → N δ → Type ℓ`, so a method's result type and its induction hypotheses are the motive *applied to* the
indices that constructor chose and to the value being eliminated. `match` compiles through a case tree (§6.2) to this
eliminator, and coverage is decided on the tree.

**Unification decides which patterns are reachable.** Splitting a scrutinee of type `N a` against a constructor whose
result is `N b` unifies `a` with `b`. A solution refines the rest of the branch; a mismatch of two distinct constructors
makes the branch **impossible** and it is not written; a variable determined by the unification is a *forced* pattern,
elaborated rather than matched. §2.1 is the unifier, and it is the same one, not a second one.

**Strict positivity is checked on the declaration group**, so mutually recursive families are checked together. A
recursive occurrence may not appear to the left of an arrow at any depth: a negative occurrence admits a fixed point,
and a fixed point admits divergence. An occurrence **nested** inside another family's parameter —
`Body(items: List<StaffRead>)` — is admitted and carries **no induction hypothesis**: the eliminator's method takes such
a field and nothing more, because a hypothesis for it would be a synthesized functorial map rather than an application.
A fold *through* a container is written with the container's own fold, which is what the corpus already does.

**Indices must not make the checker search.** An index is an ordinary term, so two of them are compared by conversion
(§3) and solved by unification (§2.1), and both of those are decidable — but only the *pattern fragment* of unification
is unitary. Outside it, a constraint is **postponed**, never guessed, and elaboration that ends with one still unsolved
is an error naming the index it could not determine. That is the whole of the discipline: musa admits index refinement
and refuses index *search*.

**A family with no indices is an *enumeration*.** The word is not retired: §§5 and 6 use it where indices are irrelevant
— a counting enumeration (§5.10) is a family with no parameters, no indices, and two constructors — and nothing about
those sections changes because indices now exist.

**Records are one-constructor families.** `{ f₁ : A₁, …, fₙ : Aₙ }` declares a family with one constructor and `n`
fields, `r.f` is a generated function whose body is a one-branch case tree, and η holds at `quote` for a one-constructor
family exactly as it does for Π. Field types may mention the parameters in scope and, unlike the previous calculus, may
mention earlier fields' *values*, because a one-constructor family's constructor is an ordinary telescope. No committed
program has one that does; the restriction is lifted because keeping it would now be a special case rather than a
simplification.

The finite and exact musical domains named in `01-surface.md` enter as declared families and base types through §5.8's
conservative-extension obligation, not by assumption. The types the language carries for its own sake are:

```text
Unit  Bool  Nat  Ratio  Text
Duration C            % how much time: a nonnegative exact rational, in coordinate C
Position C            % when: an exact rational instant, in coordinate C
EventTrack C δ        % finite duration + finite multiset of occurrences of δ, in coordinate C
Primitive K δ δ       % one registered stepping unit: name, version, storable configuration
Machine K δ δ         % a finite description of a stepping process — not the history it produces
```

`C` ranges over `WrittenTime`, `PerformedTime`, and `PhysicalTime`; `K` over `AudioFrameStep`. Both are ordinary
declared families whose closed literals index the three base types above. `δ` ranges over types satisfying §1.2's
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

Recursion is by definition, checked (§2.4), and never by a term former. There is no `Type : Type`, no call-by-push-value
stratification, no coinduction, and no first-class signal. Each is a separate amendment under `../README.md`.

**No subtyping, in any form.** Not cumulativity, not subsumption, not a coercive rule for one type pair.
`../constitution.md` §9 refuses it and `../obligations.md` §17 states the test: acceptance is decided by one relation,
and a rule that lets a term of one type stand where another is written is a second one the checker cannot see. Where a
conversion is wanted it is a named total function, written at the site. The one coercive rule the implementation still
has — `Accepts`, which forgets a `Syntax` category — is deleted at prompt 159, where `Syntax : Cat → Type` becomes a
family and forgetting becomes an ordinary function.

**No apparatus.** The theory is admitted; the proof assistant is not, and the line is between what may be *declared* and
what the system will *search for*:

- **No tactic language, no hint database, no proof search, no `auto` implicit**, and no interactive hole as a workflow.
  An argument elaboration cannot determine is reported, not hunted for.
- **No opt-out from totality.** No `partial`, no `assert_total`, no assumed-terminating annotation, no `believe_me`.
- **No termination measures.** Structural descent over the case tree is the whole rule (§2.4).
- **No index search.** Unification solves the pattern fragment and postpones the rest (§2.1); it never guesses, and it
  never enumerates.
- **No guards and no fall-through in `match`** — unchanged from before, restated here because this section is where the
  refusals live.
- **No elaborator reflection.** Musa has one metaprogramming mechanism, typed quotation (`11-quotation.md`), and a macro
  is an ordinary total function over the `Syntax` family rather than a program that inspects the elaborator's state.

Three refusals the previous statement of this section carried are **retired**, and are listed here so a reader meeting
the old text is not confused. The identity type is now declarable (§1.4), because it is an ordinary family and refusing
one particular family would be a special case with nothing behind it. Implicit arguments are admitted (§2.1), as a
filling on the one binder rather than as a second Π. Families with indices are admitted (§1.1), which is what makes the
first two worth having. What was refused with them — the apparatus above — is refused still.

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

### 1.4 Identity: declarable, and not built in

`Equal` is an ordinary inductive family and `Refl` is its one constructor:

```text
data Equal<A> : (x : A) -> (y : A) -> Type {
    Refl : Equal<A>(x, x),
}
```

Nothing about it is core machinery. It is declared in the standard library like any other family, its eliminator is
generated like any other family's, and `rewrite` is surface sugar over that eliminator. The core has no identity former,
no `refl` term, no `J` rule, and no K: what makes an equality usable is §1.1's dependent motive, which every family has.

This is a repair of the previous statement of this section rather than a reversal of it. What that statement refused —
on the audit's evidence that no committed program constructs, eliminates, or converts an identity proof — was the
*apparatus*: a built-in identity type with its own conversion rule, the K question, and the machinery that comes with
treating equality proofs as a checker concern. That apparatus stays refused. What could not survive the amendment at
prompt 143 is the refusal of the *declaration*, because after §1.1 there is no mechanism left to refuse: `Equal` is four
lines of ordinary source, and a rule forbidding one particular family would be a special case the language could not
state a reason for.

Two things follow, and they are the reason this section is worth keeping rather than deleting. Equality that decides
*types* is definitional and lives in §3; equality that decides *values* is `==` and lives in the library. `Equal` is
neither: it is a proposition a program may state, and the only way to inhabit it is a term the author writes, because
there are no tactics (§1.3). A law that computation cannot discharge is therefore a law that does not get written, and
`05-verification.md` is where that boundary is drawn.

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
author re-enters checking mode, and it is why an annotation is a language feature rather than a hint. Here `ℓ` is a
level expression of §1, solved like any other unknown (§2.1) and generalized when the declaration closes.

`Switch` is also where `../obligations.md` §17's rule bites. It calls conversion, and conversion is the *only* thing it
may call: there is no second premise that accepts `A` where `B` was expected, no coercion inserted here or anywhere, and
no relation `≼`. A later prompt that adds one is amending the constitution, not extending the elaborator.

Introduction forms check; elimination forms infer. `λx. e` checks against `(x : A) → B`, so a lambda binder needs no
annotation when its type is known; an application infers its function, then checks its argument against the domain, then
instantiates the codomain with the argument — which is where dependency lives, and all it asks of this section. A record
literal checks field by field. A constructor checks against its family at known parameters, and its **indices are
matched against the expected type's**, which is where §1.1's refinement enters checking. A projection, a variable, and a
literal infer.

Annotations are written at public signatures and wherever information is not locally and deterministically available.
The elaborator never invents a term the author did not write except through §2.1's metavariables, and what those stand
for is always solved or reported — never defaulted.

### 2.1 Unification

**Checking may create a metavariable and postpone.** A metavariable `?m[σ]` stands for a term not yet known, carrying
the **scope** σ of local variables it may mention. It is created where the elaborator needs a term it cannot yet
determine — an omitted argument, an inferred binder's type, an index the expected type does not fix — and it is solved
by unification or reported.

The constraint queue is part of the judgment, not an implementation note. Elaboration produces, besides a term, a set of
constraints `A ≟ B`; a constraint is attempted when it is created and **postponed** when it is not yet decidable;
postponed constraints are retried when a metavariable they mention is solved. Elaboration of a declaration ends by
draining the queue, and a constraint still unsolved at the end is an **error naming what could not be determined**.
Nothing is defaulted, and nothing is left for a later declaration to settle.

**Solving is Miller's pattern fragment, and only that.** `?m x₁ … xₙ ≟ t` is solved uniquely when the `xᵢ` are distinct
local variables not otherwise constrained, and `t` mentions no variable outside them and does not mention `?m`. Inside
that fragment a solution is unique, so accepting it is not a guess. Outside it — a metavariable applied to a
non-variable, or a `max` of two unknown levels — the constraint is postponed, and if it never becomes a pattern it is
the error above. Higher-order unification proper, backtracking search over solutions, and defaulting rules that pick one
solution among several are each refused: they would make acceptance depend on the order constraints were solved in,
which is the property `../obligations.md` §17 exists to protect.

**Implicit arguments are a filling, not a second binder.** A binder marked `inferred` (§1) has its argument supplied by
the elaborator: an application against such a binder inserts a fresh metavariable. Insertion happens where the
elaborator meets the binder, so it is a property of the type and not of the call site's spelling, and conversion never
reads the filling — two Π types that differ only in filling are the same type.

**Argument order is not semantically significant.** An argument whose slot still mentions an unsolved metavariable, and
which cannot itself be inferred — a bare `fn (x) { … }`, a record literal, a `match` — is checked after the rest of the
spine has constrained its slot. With a real constraint queue this needs no special discipline: the argument's check
produces constraints like any other, and they are attempted when they can be. What the previous calculus achieved with a
fixed two-pass walk and no queue is a consequence here rather than a mechanism.

**What this replaces, and why the replacement is not optional.** The previous statement of this section specified
first-order matching of written arguments against declared parameter types, solved once at the call, with no queue and
nothing that waits — Idris2's `checkRtoL` without the fallback that makes it a unifier. It covered the corpus and
nothing more: every construct wanting a solution the call site does not exhibit — an implicit argument, an index unified
by a `match`, a metavariable outliving its call — was unavailable, and unavailable by omission rather than by decision.
`53-one-theory.md` measures that. This section specifies the mechanism; prompt 153 implements it, and prompt 154 the
implicit arguments that ride on it.

### 2.2 Refinement constructors and assertions

Literal checking is a partial *compiler* judgment `token ⇝ c : b`: malformed or out-of-range text produces a located
diagnostic, while accepted constants are mathematical booleans, bounded natural representations, reduced exact
rationals, or already-validated musical values. It is not a partial term operation.

Literal constructors enforce refinements such as nonnegative `Duration`, finite scale members, and row bijectivity.
These are constructor judgments returning a value or a located diagnostic, and the value they return may be **indexed**
(§1.1): `fn row(pcs: List<Pc(n)>) -> Result<Row(n), RowFault>` performs the same runtime check it always did and now
says in its type which modulus it checked against. That is this section's mechanism gaining a type, not a second one.

The refinement does not thereby become load-bearing in every operation that touches the value. An index is an ordinary
argument of an ordinary type constructor, so an operation that does not care about `n` is polymorphic in it and mentions
it only where it binds it. What an index gives is that two moduli cannot be mixed silently: `Row(12)` and `Row(24)` are
different types, and §3 says so by comparing their arguments like any other. Named predicates used by `assert` return
finite evidence that the assertion layer can report, and that is a different thing — evidence a program does hold, about
a value the checker could not settle.

**Nothing is erased.** The previous statement of this section rested on erasure: an index was arithmetic the checker did
and then discarded, so it could not be manipulated because there was nothing to manipulate. That is no longer true and
nothing replaces it, because nothing needed it. `../across-stages/04-identity-and-realization.md` digests performance
projections, bindings, seeds, and options — **never a core term** — so no stored artifact contains a type, and the
byte-identity that erasure was protecting was never at risk. The one rule that survives from it is stated where it
belongs: a value's *meaning* does not depend on a type argument, because a type argument is a type.

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
constructors of the same family. The elaborator sees the descent because it compiled the `match` (§6.2): the case tree
is where each hypothesis is bound, and a recursive call that reaches for something no pattern bound is refused, naming
the call.

**The descent is checked on the case tree, and that is a change of substrate rather than of rule.** Structural descent
used to fall out of the generated eliminator: a recursive call was an application of a method to a hypothesis the
eliminator had bound, so the argument was a subterm by construction and nothing had to be verified. With `match`
compiled to a case tree (§6.2) and elimination dependent (§1.1), the checker walks the tree instead: each `Split` binds
its constructor's fields as strict subterms of the scrutinee, and a call is admitted when its recursive argument is one
of those bindings, transitively, along the path to the `Answer` it sits in. Prompt 155 is where this is implemented, and
it is the one piece of termination checking musa has to write rather than inherit.

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

**There is one conversion relation, and one procedure decides it.**

```text
Γ ⊢ A ≡ B    is the congruence generated by  β, η, δ, ι
```

β and η at functions and at one-constructor families, δ for definitions, ι on eliminators applied to constructors. It is
decided by **normalization by evaluation**: each side is evaluated into a semantic domain of values with closures and
neutral terms, quoted back to a normal form, and the two normal forms are compared up to α. Reduction is never performed
on syntax, and there is no substitution function anywhere in the implementation.

```text
eval  : Env → Term  → Value          % closures for binders, neutrals for blocked eliminations
quote : Level → Value → Term         % reads a value back at a de Bruijn level, η-expanding where the type says to
Γ ⊢ A ≡ B   iff   quote(|Γ|, eval(ρ_Γ, A)) = quote(|Γ|, eval(ρ_Γ, B))
```

**That `iff` is exact, and it is the load-bearing sentence of this document.** Everything a type says survives
read-back, so two types are the same type exactly when their normal forms agree. Nothing is erased before quotation,
nothing is compared before quotation, and no separate decider is consulted alongside it. The obligation this puts on
every later prompt is stated as a rule in `../obligations.md` §17: **no rule may accept a program that conversion would
reject, and nothing that distinguishes two types may be hidden from read-back.**

The previous calculus could not state it that way. §1.5's erased index sort dropped indices at `quote`, which made the
`iff` say `Pc(12) ≡ Pc(24)`; the implementation therefore compared indexed forms structurally on *values*, before
quoting, and the specification and the code disagreed about what the rule was. That is the defect prompt 143's amendment
names, and it is why the amendment refuses subtyping in the same breath: a subsumption rule is the same shape of thing,
an acceptance decision made outside the one relation.

η at functions and at one-constructor families is performed by `quote` rather than by a conversion rule, which is why
two values with the same projections are convertible without a rule that inspects both at once. It is type-directed:
`quote` η-expands where the type it is reading back at says to, and for a one-constructor family that is a choice about
that family rather than a term former in the core (§1.1).

**`≡` is an equivalence relation.** Reflexivity is α on the read-back term; symmetry and transitivity follow from
comparing normal forms. There is no case in which two types nothing can read fail to be equal to themselves, because
there is no case in which conversion is handed something it cannot read: an ill-formed type is refused where it is
*formed*, not where it is compared.

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
than incidental. There are **no unary towers**, because of the numeral rule above. There are **no proof terms to
unfold** in the ordinary case: `Equal` is declarable (§1.4) but nothing in the checker constructs or inspects one, so
there is no need for the opacity annotations other systems use to seal proofs away from conversion. And there is
**nothing to decide beside the terms** — no index theory, no solver, no coercion — which is the difference between this
section and the one it replaces.

**Unification is not conversion, and the boundary is a rule.** §2.1 solves metavariables; §3 decides equality. Where
unification needs to know whether two closed terms agree it calls conversion, never the other way round: conversion
never invents a solution, never postpones, and never consults the constraint queue. A conversion that could solve a
metavariable would make type equality depend on elaboration state, and two identical programs could then differ in what
they accept.

What remains is **large elimination** — a case tree whose motive computes a type — and it is the one construct by which
evaluating a user's own definition can happen during conversion. Dependent elimination (§1.1) is what makes this
ordinary rather than exotic: a family's index may be computed, so checking a type may run a program. It is bounded by
§2.4's structural recursion and by the meter, and §4's numbers are measured **after** families land rather than
predicted before — prompt 165 owns that measurement.

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
with 320 nested evaluation levels. These are language-version constants, not timeouts or machine-memory observations;
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
because a program refused at one level count and accepted at another is a program two compilers disagree about. The
limit has been raised once, and the paragraph below is that bump's record.

**The limit was 256 and is 320, since prompt 155a.** The two clauses above derive the metric from how deeply a *term* is
written, and the counter charges recursion as well, because a recursive call is evaluated inside the enclosing
evaluation and holds its level until the steps beneath it finish. One number therefore decides two unrelated questions.
That has been true since the surface moved onto a core program and is not 155a's doing; what 155a changed is the price.
Measured by bisection: a recursive call cost about 2.5 levels through the generated recursor and costs about 3.0 through
a compiled case-tree body, and the deepest workload in the corpus — the standard library's staff adapter — moved from a
peak of at most 240 levels to a peak of at most 272. At 256 the corpus had sixteen levels of margin; at 272 it has none.

320 is bounded on both sides. Above 272, because that is what the corpus measures. At or below 362, because a limit has
to be reachable: the 200,000-step budget cannot afford to build a term deeper than 363 constructors, so a nesting limit
above that would never fire on the term-depth path these two clauses derive it from. 320 takes the middle, at 10 MiB of
stack under the obligation above rather than 8. It buys the corpus three times the margin it had and repairs nothing —
the limit still answers two questions with one number, and the deepest recursion a definition may perform is about a
hundred steps. Retiring that needs an evaluator whose control stack is explicit data rather than the host's frames, so
that recursion depth is bounded by the step budget it belongs to;
`../../notes/research/language-design-closure/54-the-nesting-limit.md` records the derivation and prompt 165a carries
the fix.

## 5. Metatheoretic obligations

**This section states obligations; it discharges none of them.** The list grew when the calculus became one theory, and
it grew in the places the theory actually reaches: unification has to be sound, coverage has to be complete under a
dependent motive, and the hierarchy has to be consistent. Two obligations the previous list carried are gone, because
the mechanisms are: there is no measure to be well founded and no index solver to be deterministic.
`../across-stages/05-metatheory.md` §1a says plainly which of these musa **argues** and which it **tests**, and the
answer is that none of them is mechanized. **The §5.7–§5.10 numbering is preserved** so that references from
`../across-stages/` and the prompt stack still land on the obligations they were pointing at.

| Obligation | What discharges it | Owed by |
| --- | --- | --- |
| **NbE soundness** — if `quote(eval(t)) = n` then `t ≡ n` | logical-relations argument over the value domain; differential test against a reference reducer | the correction's final phases |
| **NbE completeness** — convertible terms quote to α-equal normal forms | the same relation, at the identity substitution | the correction's final phases |
| **Decidability of conversion** — `Γ ⊢ A ≡ B` terminates and is an equivalence | normalization plus α-comparison; equivalence tested as a law | the correction's final phases |
| **Type preservation** — an elaborated term has the type its judgment gave it | induction over the elaboration rules | the correction's final phases |
| **Canonicity for closed storable types** — a closed term of a storable type evaluates to a constructor form | canonicity argument over the family declarations | the correction's final phases |
| **Strong normalization** — every accepted term normalizes | reducibility over the calculus, given §2.4 and §1.1's positivity | the correction's final phases |
| **Strict positivity ⟹ consistency** — the empty type has no closed inhabitant | positivity plus the predicative, non-cumulative hierarchy (§1) | the correction's final phases |
| **Coverage completeness** — an accepted `match` covers every constructor, and an unreachable arm is rejected | case-tree compilation (§6.2) | the correction's final phases |
| **Termination soundness** — an accepted recursive definition denotes a total function | the structural check of §2.4 | the correction's final phases |
| **Unification soundness** — a solution the unifier commits to makes the two sides convertible | induction over the unification rules, with the pattern-fragment restriction (§2.1) as the premise | prompt 158's re-checker, which re-checks every solved term |
| **Unification determinacy** — inside the pattern fragment a solution is unique, and outside it nothing is committed | Miller's property for the fragment; a postponement discipline that never defaults | prompt 153 |
| **Coverage under a dependent motive** — an accepted `match` covers every reachable constructor, and an impossible branch is one unification refutes | case-tree compilation (§6.2) with the index unification of §1.1 | prompts 155 and 156 |
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
markedly less of it. Bidirectional checking, pattern unification, NbE, case-tree compilation, and structural recursion
are all older than the proof assistants the previous calculus cited, and the term language is Idris2's with five
constructors removed. Musa's own contributions remain the storability check, the three-outcome budget law, the phase
environment of §5.9, and the refusals of §1.3 — each priced where it stands.
