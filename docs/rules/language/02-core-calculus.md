# The one total source language

The source language is a pure, strict, **total** dependently typed calculus: a predicative universe hierarchy, one
dependent function type, dependent records with named fields, inductive families with parameters and indices, an
identity type, definitional equality decided by normalization by evaluation, and a checked well-founded termination
rule. Surface conveniences elaborate into it before evaluation. It is deliberately more expressive than the event-track
term calculus and deliberately less expressive than a general-purpose programming language, because it may not diverge.

It is **one** language, and it builds **both** core values: an event track and a machine (`../constitution.md` §9).
There is no second calculus for the studio, and nothing about audio lives outside it — only the audio *history* does,
because a history is coinductive and no value here is.

Prompt 127a amended this document in four places: inference replaced the monomorphic discipline (§1), storable data
replaced the ad-hoc payload rule (§1.1), machines became value types (§1), and the contextual `music` type was deleted
(§5.7).

**Prompt 128's amendment replaced the type discipline itself**, and this document is its contract. Rank-1 Hindley–Milner
inference, principal types, and the `a`/`d` variable classes are gone; bidirectional elaboration with metavariables,
dependent types, and a `Storable` constraint replace them. Totality is kept and *widened*: structural decrease becomes a
well-founded measure the checker verifies, with no `partial` and no escape hatch. The evidence, the refused
alternatives, and the answers to the argument that recommended against all of this are in
[`../../notes/research/language-design-closure/42-dependent-core-decision.md`](../../notes/research/language-design-closure/42-dependent-core-decision.md).
Where that record and this document disagree, one of them is defective; the record is the argument and this document is
the contract.

## 1. Syntax

There is **one syntactic category**. Types are terms, so the grammar below is the whole language and there is no
separate type grammar to keep in step with it.

```text
l  ::= 0 | succ l | max l l | ℓ            % universe levels; ℓ is a level metavariable

e  ::= x                                    % variable
     | Type l                               % universe at level l
     | (x : e) → e        | λx. e | e e     % dependent function: formation, introduction, elimination
     | { f₁ : e, …, fₙ : e }                % dependent record type; later fields may mention earlier binders
     | { f₁ = e, …, fₙ = e } | e.f          % record introduction and projection
     | N e… e…                              % an inductive family at its parameters and indices
     | c e…                                 % a constructor of a family
     | elim_N …                             % the generated dependent recursor of a family
     | Id e e e                             % identity: `Id A x y`
     | refl e                               % its sole constructor
     | J …                                  % its dependent eliminator; K is derived, not primitive (§1.4)
     | let x : e = e in e                   % non-recursive local binding
     | ?α[σ]                                % a metavariable under an explicit substitution (elaboration only, §2)
```

**Universes are predicative and not cumulative.** `Type l : Type (succ l)`, and there is no `Type : Type`. Levels are a
core sort with `0`, `succ`, and `max`; the elaborator solves level metavariables against that constraint set, and the
surface never writes a level. Cumulativity is refused: it buys convenience at the price of a subtyping relation inside
conversion, and conversion already carries dependent records and family elimination. A hierarchy is what keeps the
checker from proving everything, and a total language whose type theory is inconsistent is not total in any useful sense
— under `Type : Type` a closed term of the empty type exists, so §5's canonicity obligation would be discharged by a
lie.

**One function type, and plicity is not in the core.** Implicit arguments are an *elaboration* notion: the surface marks
a binder implicit, the elaborator inserts a metavariable at each use, and what reaches the core is an ordinary `(x : A)
→ B`. Nothing downstream — traits, `Syntax<Cat>`, `Vec A n` — needs a second binder form, and keeping plicity out of the
core keeps conversion from having to know about it.

**Dependent records are primitive, not Σ sugar**, with η. Two reasons, both concrete. Trait dictionaries are records
(`10-traits.md`), and a coherence argument is far easier to state when two dictionaries for the same instance are
convertible by η than by a chain of projections. And a Σ-encoded record loses its field names, so every error message
about a missing field would name `fst` and `snd` instead of the field the author wrote. Σ remains derivable — a
two-field record — and is not separately primitive.

### 1.1 Inductive families

A declaration group introduces families with **parameters**, which are fixed across the whole declaration, and
**indices**, which may vary per constructor:

```text
data Vec (A : Type l) : (n : Nat) → Type l {
    Nil  : Vec A zero,
    Cons : (n : Nat) → A → Vec A n → Vec A (succ n),
}
```

The distinction is load-bearing and is not cosmetic: a parameter may be generalized once for the whole family, an index
is what a constructor *chooses*, and it is the index that unification interrogates when a `match` narrows a type.
`Syntax<Cat>` (`11-quotation.md`) is an indexed family and is the program the distinction exists for.

**Strict positivity is checked on the declaration group**, so mutually recursive families are checked together. A
recursive occurrence may not appear to the left of an arrow at any depth, which is the same refusal the old §5.6 made
for the same reason: a negative occurrence admits a fixed point and a fixed point admits divergence, and this language
may not diverge.

**The core's elimination form is the generated dependent recursor `elim_N`**, and nothing else eliminates a family.
Surface `match` compiles through a case tree to nested recursors, which is where coverage is decided (§6.2).

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
declared enumerations used as parameters, not values smuggled into types — though they *could* now be values in types,
and nothing about them changes because of it. `δ` ranges over types satisfying `Storable` (§1.2).

Five of these are decisions rather than conveniences, and prompt 128 changed none of them:

- **`Ratio`, not a float.** Musical time is exact. A float appears at the performance and DSP edge and nowhere earlier,
  so no source-language type can hold one.
- **`Duration C` rather than `Ratio`.** A bare rational carries neither the coordinate nor the nonnegativity.
  `Duration WrittenTime` and `Duration PhysicalTime` are not convertible, so adding a written beat to a number of
  seconds is a type error rather than a number. Its constructors are where nonnegativity is checked, which is why they
  return `Result`.
- **`Position C` separate from `Duration C`.** *When* something happens and *how much time* it takes are different
  quantities with different algebras: `../kernel/00-purpose.md` states positions as the abelian group `(ℚ, +, 0)` and
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
called *base types*, never primitives; `../kernel/` uses "primitive" in its ordinary English sense of *irreducible*,
where no registered unit is in scope.

### 1.2 Storable data, as a constraint

Every type is a **value type**. A type is *also* **storable data** when it contains no function at any depth and has a
versioned finite exact encoding. Under prompt 128's amendment this is written as a constraint, `Storable A`, rather than
as a second class of type variable.

**Why a constraint and not a variable class.** The old `d` variable class was a side condition on unification: `d` could
not be replaced by a function or a container holding one. That is the right *rule* stated in the wrong *place*. As a
constraint it composes, so `Storable A` and `Storable B` yield `Storable (A × B)` by an ordinary instance rather than by
a case in the unifier; it appears in the signatures an author already reads instead of in a variable's spelling; and its
failures are ordinary instance errors that name the offending field, where a `d` unification failure could only name a
type variable. Bidirectional elaboration has no principal types to preserve, so nothing is lost with the class.

**Its instances are generated and never written by hand.** `Storable` is a *structural fact about a type*, not a claim a
user may assert:

- base types, `Unit`, `Bool`, `Nat`, `Ratio`, `Text`, `Duration C`, `Position C` are storable;
- a record or family is storable exactly when every field type it stores is, checked once per declaration group with
  mutually recursive families grouped together;
- `EventTrack C A` is storable when `A` is; `Primitive K A B` and `Machine K A B` are storable when `A` and `B` are and
  every stored configuration value is;
- an arrow type is **never** storable, and neither is any container holding one, including at a depth the surface never
  writes out — which is why the check is structural rather than a surface-syntax rule;
- an abstract compiler-owned type is storable only when its owner guarantees that its hidden representation contains no
  closure and supplies the exact encoding.

There is no user-written `impl Storable`, no `deriving` that a user may attach to a type that does not qualify, and no
way to name `Storable` in an instance head. The elaborator generates the instance or refuses the type; a signature may
only *require* the constraint. This is the one place where the trait system's "instances are declarations" rule
(`10-traits.md`) has a deliberate exception, and it is an exception in the safe direction: the set of instances is
smaller than a user could write, never larger. The check terminates because the declaration graph is finite.

`SyntaxStep C A` (§5.9) is the type that is never storable for a stronger reason than an arrow's: a step *holds* the
algebra of the recursor that minted it, which is four closures and a child of the region being read. It has no
`Storable` instance, no stored field may hold one at any depth, and it never crosses the expansion phase's boundary — so
the completed phase result stays storable data, as §5.9's law 11 requires. `Syntax<Cat>` inherits the same treatment
when prompt 138 defines it.

Only storable data may be:

- an event-track payload;
- a machine's input or output port type;
- a machine's feedback value;
- a registered primitive's configuration; or
- an argument to a foreign primitive.

These conditions are also the well-formedness rules of the types that carry them. `../kernel/12-payload-admission.md` is
the kernel-facing statement of the same predicate, and it does not change: the kernel asks a question about a type, and
`Storable` is now how the language answers it.

### 1.3 What the language does not have

There is no `fix`, no recursive binding, no `partial`, no while loop, no exception, no mutation, no I/O, no reflection,
no dynamic cast, and no effect handler. Functions may be higher-order. **A call must be complete**: an application
supplies every declared parameter, and an under-applied call is a type error rather than a value. Partial application
would make an ordinary argument list a place where a function silently becomes a function-valued result, which is
exactly the value that may not be stored (§1.2).

Recursion is by definition, checked (§2.4), and never by a term former. There is no `Type : Type`, no cumulativity, no
universe polymorphism beyond level metavariables, no call-by-push-value stratification, no coinduction, and no
first-class signal. Each is a separate amendment under `../README.md`.

**There is also no signed integer type**, and the reason is elimination rather than a preference about numbers. `Nat` is
in the language to be the *inductive* numeric type: `zero | succ` is well founded, so its recursor terminates by
construction. ℤ has no such structure and no least element to descend to, so an integer recursor would either be a `Nat`
recursor on the magnitude with a sign carried alongside — `Nat` plus bookkeeping — or an unbounded loop, which totality
forbids. An `Int` could therefore only be a base type with no eliminator of its own, and the governing design rule then
applies: removing it makes nothing impossible.

Nothing musical is left unsayable by that. Ordinary signed arithmetic is `Ratio`, which is signed, with the refinements
at its constructors. The signed *musical* quantities are domains: an `Interval` is a signed pair of written diatonic
steps and semitones (`00-semantics.md` §3, after *Open Music Theory* `016-intervals.md`), and a `Degree` is a signed
ordinal relative to a scale (`03-musical-domains.md` §2). Everything the language counts with `Nat` — repeat and
occurrence counts, list lengths, range bounds, the cost table of §4 — is nonnegative.

The musical falsifier is what settles it. Admitting `Int` would make `transpose(-3)` the obvious spelling, and that
number cannot say whether the composer wrote a descending minor third or a descending augmented second. The two are
different notes on the page and different chords underneath. Keeping the general signed integer out is what keeps
`Interval` load-bearing rather than decorative — roadmap §2's separation of written pitch from MIDI number, applied to
the number itself.

### 1.4 Identity, and the K decision

`Id A x y` is the identity type, `refl` its sole constructor, and `J` its dependent eliminator. **Uniqueness of identity
proofs is not admitted as an axiom.** K is a theorem where a program needs it, derived from decidable equality, and not
a rule of the core.

Prompt 129 wrote the other decision, and prompt 132 was nominated in this section as the place it could be cheaply
undone. The trial's answer is that **no program unifies an index at all.** `Syntax<Cat>` appears only at closed index
constructors, so a `match` on a syntax value never learns anything about its index; `Vec A n` has no user in ten
programs, because the two fixed-arity things the staff adapter has read better as enums with named cases. K was
therefore admitted for a use nothing exercises.

The reason it can go without loss is `10-traits.md` §7's, which was written as a remark and is the argument: for every
type these programs declare, K is a *theorem*. Each is a finite inductive family over base types with decidable
equality, and Hedberg's theorem gives uniqueness of identity proofs from decidable equality — so `DecEq A` supplies
exactly what an index-unifying `match` on `Syntax<c>` would have needed, at `DecEq Cat`, over two closed cases.

Two consequences follow and prompt 135 is held to both. The coverage checker may not use a unification rule that
requires K until a program requires one, which costs nothing today and is checkable at the rule rather than at its uses.
And what admitting K globally would have foreclosed stays open rather than closed: univalence, higher inductive types,
and internal parametricity in the cubical sense are inconsistent with K as an axiom and are merely absent without it.
None is on any roadmap, which is why prompt 129 was willing to pay; the trial found the payment unnecessary, and note 39
§8.2's warning against importing adjacent dependent foundations by analogy is better served by not importing the axiom
either.

**Re-opening is an ordinary amendment**, with the evidence it needs stated in advance: an indexed family whose index
type has no `DecEq`, and a `match` on it that needs the deletion rule. Prompt 141's `Vec A n` is the first candidate and
probably is not one, since `Nat` has `DecEq`.

## 2. Static semantics: bidirectional elaboration

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
author re-enters checking mode, and it is why an annotation is a language feature rather than a hint.

Introduction forms check; elimination forms infer. `λx. e` checks against `(x : A) → B`, so a lambda binder needs no
annotation when its type is known; an application infers its function, then checks its argument against the domain, then
instantiates the codomain. A record literal checks field by field, with each field's type instantiated by the fields
already elaborated. A constructor checks against its family at known parameters and indices. A projection, a variable,
and a literal infer.

Annotations are still written at public signatures and wherever separate checking needs one. What changed is that a
signature may mention a value: `(n : Nat) → Vec A n → Vec A (succ n)` is an ordinary signature.

### 2.1 Metavariables and pattern unification

A metavariable `?α[σ]` stands for an unknown term under an explicit substitution recording the context it was created
in. Metavariables are created in exactly three places, and listing them is what keeps elaboration predictable:

1. at an implicit application, one per inserted implicit argument;
2. at an unannotated binder whose type the checking type does not supply; and
3. at a level position the surface did not write.

They are solved by **pattern-fragment (Miller) unification**: a constraint `?α x₁ … xₙ ≡ t` is solved immediately when
`x₁ … xₙ` are *distinct bound variables* and every free variable of `t` is among them and the context of `?α`, giving
`?α := λx₁ … xₙ. t`. That restriction is exactly what makes the solution unique and the problem decidable; outside it,
higher-order unification is undecidable and a solver that guessed would make a program's meaning depend on the order in
which the checker reached its constraints.

A constraint outside the pattern fragment, or blocked on an unsolved metavariable, is **postponed** rather than guessed,
and is retried when one of the metavariables it is blocked on is solved. A metavariable still unsolved at the end of the
declaration it was created in is an error reported at the term that left it unsolved, never defaulted and never
generalized. Generalization happens only where an author wrote a binder.

### 2.2 Refinement constructors and assertions

Literal checking is a partial *compiler* judgment `token ⇝ c : b`: malformed or out-of-range text produces a located
diagnostic, while accepted constants are mathematical booleans, bounded natural representations, reduced exact
rationals, or already-validated musical values. It is not a partial term operation.

Literal constructors enforce refinements such as nonnegative `Duration`, finite scale members, and row bijectivity.
These are constructor judgments returning a value or a located diagnostic. They *could* now be dependent — the language
has an identity type and indexed families — and they deliberately are not: a refinement moved into an index becomes
load-bearing in every operation and every proof that touches the value, which is note 39 §8.1's third reason and is not
overturned. A refinement enters an index only where a program has shown that the evidence must be manipulated *before*
evaluation. Named predicates used by `assert` return finite evidence that the assertion layer can report.

### 2.3 Track and machine construction

Only compiler-owned builtins may construct an `EventTrack` or a `Machine`. `map_note_pitches` accepts a total
`Pitch → Pitch` and visits a documented subset of musical payload positions; it does not reveal them as a list. A kernel
quote has a dedicated typing rule and cannot be encoded by string operations.

Machine construction is typed by the seven rules of `../across-stages/03-machine-calculus.md` §2 — `machine(p)`,
`identity`, `connect`, `beside`, `feedback`, `copy`, `drop`, `swap` — and every port type in them must satisfy
`Storable` (§1.2). Those rules are stated there rather than repeated here, because the machine's *step* semantics is
stated there too and a typing rule separated from its step is a rule nobody can check.

### 2.4 Termination is a checked measure

Every recursive definition presents a **measure** into a well-founded order, and the checker verifies that every
recursive call decreases it. Structural decrease is the special case where the measure is the subterm order of one
argument and the elaborator supplies it without the author writing anything — which is what makes the common case free
and the old rule a subset of this one rather than a casualty of it.

Nothing is opaque to the checker. There is no `partial`, no `fix`, no assumed-terminating annotation, and no way to ask
the checker to take a definition on trust. A definition whose termination the checker cannot see is **rejected**, and
the diagnostic names the call whose decrease it could not establish and the measure it was checking against.

The soundness obligation this creates is stated in §5 and owed by prompt 135: an accepted definition denotes a total
function. Until that is discharged, the termination checker is a filter believed sound rather than a filter known to be.

Top-level signatures are collected before bodies are elaborated, so a later declaration may be referenced. From each
body the checker records an edge to every free named declaration. **Non-recursive** declarations must form an acyclic
graph, exactly as before; a *recursive* declaration is the case §2.4 admits, and it is admitted through the measure
rather than through the graph. This is the one place where the amendment widened what the declaration graph accepts, and
the widening is exactly the size of the termination checker.

## 3. Dynamic semantics: evaluation, and conversion by NbE

**Definitional equality** is β, η at Π and at records, δ for definitions, and ι on recursors applied to constructors. It
is decided by **normalization by evaluation**: both sides are evaluated into a semantic domain of values with closures
and neutral terms, and quoted back to a normal form, which are then compared up to α. Reduction is never performed on
syntax.

```text
eval  : Env → Term  → Value          % closures for binders, neutrals for blocked eliminations
quote : Level → Value → Term         % reads a value back at a de Bruijn level, η-expanding at Π and records
Γ ⊢ A ≡ B   iff   quote(|Γ|, eval(ρ_Γ, A)) = quote(|Γ|, eval(ρ_Γ, B))
```

η at Π and at records is performed by `quote` rather than by a conversion rule, which is why two record values with the
same projections are convertible without a rule that inspects both at once — the property the trait-coherence argument
of `10-traits.md` rests on.

**The dependency chain runs one way, and it is the reason prompt 128 kept totality rather than a matter of taste:**

```text
termination  ⟹  normalization  ⟹  decidable conversion  ⟹  decidable type checking
```

Conversion evaluates. If evaluation could diverge, conversion could diverge, and type checking would hang rather than
answer — a compiler that does not return and an editor that stops answering while the composer types. Under the old core
divergence would have been a bad runtime; under this one it is a bad checker. §2.4 is what holds the left end of that
chain up.

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

Normalization does not bound a terminating program to useful project size, and under a dependent checker it no longer
bounds *checking* either. Musa therefore maintains one deterministic meter over checking, conversion, and evaluation. A
finite aggregate operation charges its known count and logical result shape before entering its loop or allocating its
result; nested work is charged when its enclosing operation is reached. A value is charged once, where it is
constructed: an expression that names, selects, or returns a value already built charges it no nodes and no bytes, and a
constructor is charged its own node and one field per part it holds rather than those parts again. Charging a value once
per mention instead would make a project's cost the product of its data size and its program size, which is not a
measure of what the project builds. All values remain private until the whole declaration graph succeeds, so exhaustion
publishes neither a partial value nor a partial score. The meter covers:

- instantiated definition count and closure environment size;
- recursor work, including products induced by nested recursion;
- **conversion work: evaluation steps and quoted nodes charged during a `Switch`**;
- **metavariable count, and postponed-constraint retries**;
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
with 256 nested evaluation levels. **Conversion and metavariable metrics have no defaults yet**: prompt 144 measures the
new checker and sets them, and until it does, the checker charges them and reports them without a limit. These are
language-version constants, not timeouts or machine-memory observations. Interactive cancellation remains an external
compiler operation, not a language effect.

### 4.1 Nesting, and the room to reach the limit

Nesting is the one metric that goes back down. Every other counter measures what a run has spent and never returns; this
one measures how far in the run currently is, and a level is released when the work at that level finishes. A sequence
of a million siblings is one level deep, not a million. The limit is charged wherever an evaluation can stand inside
another one — evaluating an expression, eliminating a finite data value, descending into a syntax value, and now
**quoting a value back during conversion** — so what the counter bounds is exactly what the machine spends stack on.

The metric exists because §5.9's recursor descends *through* the transformer's own branches: one level of a region's
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

**This section states obligations; it discharges none of them.** Prompt 148 owes the audit, and prompts 133–137 owe the
mechanisms and their executable evidence. That is a change from what this section used to be: §§5.1–5.6 previously
carried worked proofs of preservation, progress, determinism, and strong normalization for a rank-1 simply typed
fragment. Those theorems were about a language that no longer exists, and carrying them forward as if they still applied
would be the most expensive kind of stale document. They are deleted rather than quietly reworded. **The §5.7–§5.9
numbering is preserved** so that references from `../across-stages/`, `03-musical-domains.md`, and the prompt stack
still land on the obligations they were pointing at.

| Obligation | What discharges it | Owed by |
| --- | --- | --- |
| **NbE soundness** — if `quote(eval(t)) = n` then `t ≡ n` | logical-relations argument over the value domain; differential test against a reference reducer | 133 states, 148 audits |
| **NbE completeness** — convertible terms quote to α-equal normal forms | the same relation, at the identity substitution | 133 states, 148 audits |
| **Decidability of conversion** — `Γ ⊢ A ≡ B` terminates and is an equivalence | normalization plus α-comparison; equivalence tested as a law | 133, 148 |
| **Type preservation** — an elaborated term has the type its judgment gave it | induction over the elaboration rules; the checker's own reconstruction check at every declaration boundary | 134, 148 |
| **Canonicity for closed storable types** — a closed term of a storable type evaluates to a constructor form | canonicity argument over the family declarations | 135, 148 |
| **Strong normalization** — every accepted term normalizes | reducibility over the dependent core, given §2.4 | 135, 148 |
| **Strict positivity ⟹ consistency** — the empty type has no closed inhabitant | positivity check plus the predicative hierarchy | 135, 148 |
| **Coverage completeness** — an accepted `match` covers every reachable case, and an unreachable arm is rejected | case-tree compilation with index unification (§6.2) | 135, 148 |
| **Termination soundness** — an accepted recursive definition denotes a total function | the well-founded measure check of §2.4 | 135, 148 |
| **Metavariable solution uniqueness** — a pattern-fragment solution is most general, and elaboration order does not change an accepted program | the fragment restriction of §2.1; a permutation test over constraint order | 134, 148 |
| **`Storable` faithfulness** — an instance exists exactly when the structural condition holds, and no user may add one | generated instances only (§1.2); a compile-fail suite for hand-written instances | 137, 148 |
| **Track-construction safety** (§5.7) | re-derived against the dependent core | 148 |
| **Musical domains are a conservative extension** (§5.8) | re-derived against the dependent core | 148 |
| **The expansion phase, including law 11** (§5.9) | re-derived against the dependent core and prompt 140's syntax patterns | 147, 148 |
| **Budget independence** (§4) | the three-outcome law, tested across budget pairs | 144, 148 |

Three of these were previously proved and are now *owed again*, and the reason is worth stating rather than leaving to
inference: a proof about a simply typed calculus with rank-1 inference does not transfer to a dependent one by
inspection. Conversion is now part of typing, so preservation and normalization are entangled where they used to be
separable, and the reducibility argument that used to induct on types must now induct on something well founded in a
theory where types contain terms.

### 5.7 Track-construction safety

**Obligation.** A track value is not built directly by user code; it is built by a private fragment `F = (t, B, d, n)`
where `t : Term[ScoreFact]`, `B` is a finite acyclic environment of term bindings, `d ∈ ℚ≥0` is the exact duration in
`WrittenTime`, and `n ∈ ℕ` is the exact occurrence count. Its well-formedness requires: every free term name of `t` is
in `dom(B)`; binding edges in `B` point to earlier completed bindings; every literal in `t` is a valid finite
`EventTrack WrittenTime ScoreFact` and every mark is a total payload map; `d` and `n` agree structurally with `t` after
its bindings are instantiated; and all facts have the requested scope, exact nonnegative placement, and a complete
`Origin`.

What must be re-established against the dependent core: that `follow` and `together` compose well-formed fragments into
well-formed fragments with duration `Σᵢdᵢ` and `maxᵢdᵢ` respectively and count `Σᵢnᵢ`; that replacing a fragment by a
marked reference to its completed binding preserves duration and count; and that building a checked track expression at
a valid placement returns a finite well-formed fragment whose closure is a closed checked `Term[ScoreFact]` with exactly
`n` occurrences.

**What changed and why the proof is owed again.** The old argument inducted on "the declaration dependency rank and the
finite syntax size of `m`". The dependency graph now admits recursive definitions (§2.4), so the induction must run on
the termination measure instead, and that is not a rewording — it is the place where §2.4's soundness obligation is
actually consumed. Prompt 148 owes it.

**What this obligation does not cover.** Machines. A machine's safety is M1–M8 of
`../across-stages/03-machine-calculus.md` §7, stated over a step relation this section has no vocabulary for, and the
two arguments share no lemma. Keeping them apart is deliberate: an argument that covered both would have had to talk
about a "value" general enough to be either, and that generality is the contextual-`Music` mistake in a new place.

### 5.8 Musical domains as a conservative extension

**Obligation.** Every compiler-owned builtin belongs to exactly one of four families, and that the families are disjoint
and exhaustive is a checked law:

- **δ-builtins** — every argument type and the result type is a base type or a finite constructor (`Option`, `List`,
  record) over base types, with no arrow anywhere in the signature;
- **structural eliminators** — the generated recursors and the derived traversals over them;
- **track builtins** — the constructors and controlled transforms of §5.7; and
- **machine builtins** — the constructors of `../across-stages/03-machine-calculus.md` §2.

A δ-builtin must satisfy four conditions: **D1 inertness** — its base types have no eliminator, so no reduction rule
inspects a closed value of one and the only pattern that may match it is a literal or a catch-all; **D2 totality** — for
every tuple of closed values of the declared argument types it yields a closed value of the declared result type, with
partiality expressed *in the result type* as an `Option` rather than as a stuck term, a panic, or a diagnostic; **D3
purity** — the result is a function of the argument values alone, with no ambient context, evaluation-order dependence,
hash-iteration order, or diagnostic emission; and **D4 finiteness** — the result's constructed-node count is bounded by
a function of the argument sizes, charged to the §4 meter before construction begins.

**The theorem to re-derive.** Adding a base type with no eliminator, together with any finite set of δ-builtins over the
extended base set satisfying D1–D4, preserves every obligation in §5's matrix. The old proof took `R_b(t) ⟺ t : b ∧ t ∈
SN` and observed that the other cases quantify over the base-type set without inspecting it. Under a dependent core the
same shape of argument should go through — a base type with no eliminator is inert in conversion for the same reason it
was inert in reduction — but "should" is not a proof, and D1's meaning has to be restated for conversion rather than for
reduction: an inert base type contributes no ι-rule, so two closed values of it are convertible iff they are the same
constant.

**Corollary, if it holds.** A later musical domain needs no new proof — it needs a base type with no eliminator, builtin
signatures containing no arrow, and a discharge of D1–D4.

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

A syntax adapter runs before name resolution and elaboration: it is handed the region a composer wrote and answers with
the syntax that stands there instead. The adapter module is written in this same calculus and checked by this same
checker, under a **phase environment** that adds three things and takes nothing away — the phase-local types (`Syntax`,
`NodePath`, `BindingPath`, `SyntaxStep C A`, and from prompt 138 the indexed `Syntax<Cat>`); a separate registry of
compiler-owned phase operations; and the `Reading::Expansion` scope in which those names mean anything at all.

**Law 11 is unchanged and is the load-bearing one.** Ordinary source is read in a scope where none of those names
resolve, and no value of a phase-local type ever crosses the phase boundary: what leaves expansion is storable data.

The phase registry is a **second registry, not a fifth family**: nothing in it is a δ-builtin, a structural eliminator,
a track builtin, or a machine builtin, so §5.8's four families remain the four families of the source core, and the
disjointness law it names is unaffected.

**Obligations to re-derive.** Sealed association and local decrease — a step minted for a proper child runs the algebra
it was sealed to, at any context, however many times, wherever it occurs. Normalization of the recursor, given
definition acyclicity, over higher-order contexts, capture, duplication, delayed use, and nested traversal. Path
uniqueness and derivation coverage. The budget charges for minting, running, and descent.

**What changed, and why these are owed again rather than carried.** Two things. Definition acyclicity was the premise
that made the recursor's normalization induction well founded; §2.4 replaced acyclicity with a termination measure for
recursive definitions, so the premise has to be restated in terms of the measure. And prompt 140 adds syntax patterns,
which destructure a quotation through the same case-tree compiler as every other pattern — so the recursor is no longer
the only way into a syntax value, and the freeze has to say which obligations are now discharged by §6.2's coverage
argument instead. Prompt 147 owes the phase-boundary freeze; prompt 148 owes the core half.

Prompt 127da's design law — "the fold is the only way into a syntax value" — was superseded before this amendment and
stays superseded. `Syntax` remains opaque, paths remain compiler-derived, `SourceInfo` remains unreadable, and
provenance remains derived rather than written.

## 6. Implementation boundary

`Term`, `Value`, `Closure`, evaluator environments, conversion state, metavariable contexts, theory representations,
instantiation tables, and resource proofs remain private to their crates. `musa-core` exposes elaboration, conversion,
and normalization over `Term` and exposes no `Value`: the semantic domain contains closures over the evaluator's own
representation, so publishing it would make every later change to evaluation a breaking change for `musa-compiler`. A
registered primitive's `State`, `start`, and `step` are private to the crate that registers it. Passes may expose narrow
internal queries, but no single-implementor public trait or pass-through facade is added. The stable public result
remains the compilation/snapshot contract.

### 6.1 This language and the event-track term calculus are two stages, not two cores

Musa has two calculi, and the boundary between them is **staging**, not an accident. This is the one place the word
"core" is used in two senses, so: there is one *source language* (this document) and two *core values* (a track and a
machine). The term calculus below is the residual syntax of the first of those values, not a third thing.

The classical arrangement for a functional compiler is surface → *enriched* calculus → *ordinary* calculus, where the
enriched layer is the ordinary one plus constructs whose semantics *is* their transformation away (Peyton Jones 1987,
§3.1). Musa is deliberately not that arrangement:

- This language has dependent functions, records, families, and recursors. `../kernel/10-term-calculus.md` has none of
  them — six forms, a reference, and no abstraction at all.
- So the term calculus is not this language with the sugar removed. There is no simplifying transformation between them.

What connects them is **evaluation, applied twice**:

```text
source ──elaborate──▶ core term ──evaluate (this document)──▶ Term[ScoreFact] ──evaluate (kernel)──▶ EventTrack WrittenTime ScoreFact
                      functions        eliminates functions      let + constructors     eliminates sharing
```

Three consequences:

1. **`Term[ScoreFact]` is a stage boundary, not an internal representation.** It is the reason `.musa.kernel` can be an
   interchange format at all: a residual program in a language with no functions is checkable, normalizable, and
   hashable by a consumer that knows nothing about this calculus — and now, importantly, by a consumer that knows
   nothing about dependent types either. The kernel did not become dependent because the source language did.
2. **Totality is proved twice, separately.** §5's normalization obligation is about *this* language; T4 is about the
   term calculus. Neither implies the other. A machine's M1 is a third, independent totality claim — one *step*, not one
   evaluation — and it shares no lemma with either.
3. **Nothing may leak backwards.** A core term cannot mention a closure, and this language cannot observe a track's
   occurrence list. Where that discipline is enforced is §5.7, §1.2's `Storable` constraint, and the kernel's
   payload-opacity rule; this section is the statement of *why* they exist.

### 6.2 Patterns are compiled to case trees

Patterns nest. A pattern position may hold another pattern, a `match` may scrutinize several subjects, and an arm may
refine the type of a later binding through the indices its constructor chose. Surface `match` is compiled to a **case
tree** and then to nested applications of the generated dependent recursors, which is where coverage is decided: an
accepted `match` covers every case the index constraints leave reachable, and an arm that no constraint can reach is
rejected as unreachable rather than silently kept.

There are still no guards, no conditional equations, and no pattern on the left of a definition. A guard reintroduces
the fall-through between equations that a case tree exists to eliminate, and coverage in the presence of guards is
either unsound or requires deciding arbitrary boolean equivalence.

**This replaces the previous rule, and the replacement is argued rather than assumed.** §6.2 previously fixed patterns
at depth one — every sub-position a binder, never another pattern — on the ground that nested patterns require a
pattern-match compiler whose whole purpose is to compile a surface convenience into eliminators the language already
writes directly, and that under the governing design rule the subsystem did not earn its place. That argument was
correct for a one-level evaluator and is wrong here, for two reasons that did not exist when it was written:

1. **A dependent match cannot be flat.** The point of an index is that matching one constructor tells you something
   about another position. `Vec A (succ n)` and `Syntax<Expr>` are the programs the feature is for, and a match that
   cannot look through two constructors cannot express what they are indexed for. The compiler is not compiling away a
   convenience; it is the only place index unification can happen.
2. **The cost was being paid anyway, by the wrong people.** Forcing the author to write the nesting by hand is precisely
   the `staff.musa` failure one level down: a reader that destructures eight fields to read one, and a dispatch table
   written as a chain. Prompt 128's amendment was granted on that measurement. Paying once here rather than at every
   author is the same trade §1 makes about every surface convenience.

The subsystem is therefore taken on deliberately, and it is cited: Peyton Jones 1987 chapters 4–6 for the case-tree
compilation and the variable/constructor/empty/mixture rules, with the `FAIL`/fat-bar mechanism **not** adopted, because
it exists to express failure between equations and Musa's arms do not fall through. Coverage and index unification are
`citations.md` §13's rows.

Prompt 172's conformance row that reads "patterns are still depth one" is falsified by this section and is repaired when
that prompt is reached, under the prompt README's §6 procedure.

## 7. Provenance on elaborated terms

**Every core term records the surface node it was elaborated from.** This is a property of the representation, not a
debugging convenience, and it is normative for three reasons:

1. **A dependent checker reports failures in terms the author never wrote.** A conversion failure is between two normal
   forms produced by `quote`, and a normal form is not source. Without an origin on the terms involved, the best
   available diagnostic is two unfamiliar expressions and no place to point. With one, the failure lands on the
   expression whose elaboration created the constraint.
2. **Quotation needs origin to be a fact rather than a reconstruction.** `11-quotation.md` derives a quoted node's
   identity as `Derived { origin, quotation, path }`. If `origin` had to be reconstructed by matching a term against the
   syntax it might have come from, the derivation would be a heuristic and the identity law would be unprovable. The
   whole reason hand-allocated role integers leave adapter code is that this field already exists.
3. **Origin paths are a governing obligation** (`../across-stages/`), and a stage that dropped provenance and re-derived
   it later would be re-deriving something it had.

An elaborated term's origin is preserved by substitution, by instantiation, and by `quote`: the normal form of a term
carries the origins of the terms it was built from, and where `quote` η-expands, the expansion carries the origin of the
term it expanded. A metavariable's solution carries the origin of the term that solved it, which is what lets an
"unsolved metavariable" diagnostic name the site that left it unsolved rather than the site that created it.

Origins are **not** part of conversion. Two terms with different origins and the same normal form are convertible;
otherwise provenance would change what a program means, and a compiler that type-checked differently after a file was
moved would be the result.

The theoretical provenance of this calculus — every construction, and the chapter or paper it comes from — is
[`citations.md`](citations.md) §13. The short form: nothing here is novel. Universes, Π, dependent records, families,
`Id`, Hedberg's theorem, NbE, bidirectional elaboration, and pattern unification are all standard, and Musa's own
contributions are the `Storable` constraint, the three-outcome budget law, the phase environment of §5.9, and the
refusals — no cumulativity, no `partial`, no CBPV, no signed integer, no K as an axiom — each of which is priced
somewhere in that section.
