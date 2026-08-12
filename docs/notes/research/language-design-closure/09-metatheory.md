# Metatheory of the proposed source language

**Purpose:** prove the safety and termination claims needed to decide whether this language can govern Musa.

The proofs concern the research calculus in `04-source-calculus.md`. They assume the accepted temporal kernel, process
semantics, and derivation rules. They do not prove package resolution, stable compiled identity, or cache correctness.

## 1. Setting

One build starts with a finite, resolved import graph `G`. Package finding and version solving have already ended.
Repeated imports of one resolved package point to the same node in `G`.

The resolver produces:

- `Delta`, the private table of user data, constructors, and field types;
- `Sigma`, the public table visible after structures are sealed;
- `Gamma`, a value environment used while checking terms; and
- a closed core term `e`.

The compiler gives every data declaration a fresh name `mu` within this build. The names need not survive another build.
Two checked outputs are equal up to a consistent renaming of these fresh names.

The typing judgments are:

```text
Delta; Sigma; Gamma |- e => A
Delta; Sigma; Gamma |- e <= A
```

The first judgment finds a type. The second checks against a supplied type. When the distinction no longer matters,
`Delta; Sigma; Gamma |- e : A` means that one of these judgments has completed with type `A`.

The step judgment:

```text
Delta |- e --> e'
```

uses the left-to-right call-by-value rules from `04-source-calculus.md`. Write `-->*` for zero or more steps.

## 2. Assumptions about compiler operations

Ordinary compiler operations satisfy all of these conditions:

1. their types contain no function argument or result;
2. they are defined on every closed, well-typed argument tuple;
3. they return a closed value of the declared result type;
4. equal arguments give equal results;
5. they finish after finite work; and
6. their result size and work have a charged finite bound.

Expected failures appear inside `Option` or `Result`. A panic, hidden diagnostic, clock read, random read, or global
musical context violates the contract.

An operation signature has the form `op(A_1, ..., A_n) => B`. It is not a source function type. Operation names do not
appear in `Gamma`, and the source checker accepts them only in calls with exactly the declared arguments. Ordinary
source functions still have `A -> B` types and remain values.

All seven structural operations are core rules: natural, list, and option folds, `map`, `filter`, `range`, and `repeat`.
The existing `map_note_pitches` operation is the one admitted higher-order music transform. Source evaluation only
stores its total function in a finite recipe. Instantiation later visits the documented pitch positions within the
recipe's finite occurrence bound and calls the function once at each position. No other higher-order foreign operation
is admitted.

These assumptions are necessary. A foreign operation of type `(Unit -> Unit) -> Unit` could otherwise call its argument
forever and refute termination.

## 3. Finite checking

### Lemma 3.1. The declaration orders can be decided

Given finite source declarations, the compiler either finds dependency-first orders for data and values or reports a
cycle.

**Proof.** For data, inspect every field type, including nominal names nested inside products, `Option`, `List`, and
`Result`. Add an edge from each contained nominal type to the type that contains it. For values, inspect every resolved
reference in each definition body and add an edge from the referenced definition to its user. Both graphs have finitely
many nodes and edges.

A standard finite graph search either finds a cycle or returns a topological order. The compiler fixes source order as
the tie-breaker, so the result and diagnostics are repeatable. The search ends because it marks a finite number of
nodes. ∎

### Lemma 3.2. Ranks can be decided

If the data graph is acyclic, every nominal type has one unique rank under the rule in `04-source-calculus.md`.

**Proof.** Visit the data types in dependency-first order. Give a type with no nominal field rank 1. Otherwise give it
one plus the greatest rank already assigned to a nominal found in its fields. Every such nominal appears earlier in the
order, so the calculation is defined. The maximum of a finite non-empty set of natural numbers is unique. ∎

### Theorem 3.3. Name resolution and type checking are decidable

Given finite `G`, finite source declarations, and a finite operation table, resolution and bidirectional checking finish
with either one typed core output, up to fresh-name renaming, or a finite ordered error list.

**Proof.** The resolver performs finite table lookups over finite module and declaration paths. It rejects missing,
ambiguous, and duplicate names. It assigns fresh nominal names in source order. Lemma 3.1 decides both dependency
orders, and Lemma 3.2 assigns ranks.

Checking then visits definitions in dependency order. Each typing rule inspects a strict subterm, a finite argument
list, a finite quote-hole list, or a finite match-arm list. Structural type equality recurses over two finite type
trees. `None`, `[]`, `Ok`, and `Err` use a supplied expected type, so the checker never searches for a missing type
argument. The seven structural-operation schemes are instantiated from their displayed argument types, without search.
Flat-pattern coverage is a finite test over `Bool`, `Option`, `List`, `Result`, or the finite constructor list in
`Delta`; all other subject types need a catch-all arm. Signature matching compares two finite member tables and checks
each body once. Kernel quotation invokes the already decidable finite kernel checker on one finite term and its exact
finite hole table.

No rule guesses a type, unfolds a recursive declaration, solves an arithmetic formula, or searches an infinite space.
Every phase therefore ends. Each choice uses source order or an exact table key, so success is unique up to the names
freshly chosen for nominal types. ∎

This theorem is about checking a graph that has already been resolved. It says nothing about a registry or version
solver.

## 4. Structural lemmas

### Lemma 4.1. Weakening

If `Delta; Sigma; Gamma |- e : A` and `x` is not already in `Gamma`, adding an unused binding `x:B` preserves the
judgment.

**Proof.** Induct on the typing derivation. A name lookup either finds the same old binding or a binder introduced by
the same typing rule. All other premises use the induction hypotheses. `Delta` and `Sigma` do not change. ∎

### Lemma 4.2. Substitution preserves typing

If:

```text
Delta; Sigma; Gamma, x:A |- e : B
Delta; Sigma; Gamma |- v : A
```

and `v` is a value, then:

```text
Delta; Sigma; Gamma |- e[v/x] : B
```

**Proof.** Induct on the typing derivation for `e`.

For a variable, the variable is either `x`, in which case the second premise gives the result, or another variable,
whose lookup is unchanged. Literals and compiler operation names contain no variables.

For products, constructor calls, structural constructors, applications, annotations, and compiler calls, apply the
induction hypothesis to every immediate term premise and rebuild the same typing rule.

For a lambda, `let`, fold step, or match arm, first rename its bound variables away from `x` and the free variables of
`v`. Apply weakening to put both derivations under the same extended environment. The induction hypothesis then applies
to the body. In a match arm, the flat pattern gives distinct binders, so this renaming is finite and capture cannot
occur.

Substitution never changes a nominal name, constructor table, or module path. Expected types for `None`, `[]`, `Ok`, and
`Err` also stay fixed. Thus the rebuilt judgment has type `B`. ∎

### Lemma 4.3. Pattern substitution

Suppose a well-typed canonical value matches a well-typed flat pattern. Substituting the value's selected fields for the
pattern variables gives each variable the type assigned to it when the arm was checked.

**Proof.** Inspect the pattern. A wildcard binds nothing. A variable receives the subject type. A product receives its
component types. Each `Option`, list, `Result`, or nominal constructor pattern receives exactly the field types from the
typing rule and canonical value. Flat patterns have no nested case. ∎

### Lemma 4.4. Canonical forms

Let `v` be a closed, well-typed value.

- If `v: Bool`, then `v` is `true` or `false`.
- If `v: A -> B`, then `v` is a closed function value.
- If `v: Option<A>`, then `v` is `None` or `Some(w)`.
- If `v: List<A>`, then `v` is `[]` or `head :: tail`.
- If `v: Result<A,E>`, then `v` is `Ok(w)` or `Err(z)`.
- If `v: mu`, then `v` uses exactly one constructor listed for `mu` in `Delta`.

The analogous product and base-type statements also hold.

**Proof.** Inspect the last typing rule for `v`. Every value form has one introduction rule, and structural type
equality does not identify two different outer type constructors. A nominal constructor rule records its owner `mu` in
`Delta`, so a constructor from another nominal type cannot appear. ∎

## 5. Type safety

### Theorem 5.1. Evaluation preserves typing

If:

```text
Delta; Sigma; empty |- e : A
Delta |- e --> e'
```

then `Delta; Sigma; empty |- e' : A`.

**Proof.** Inspect the reduction.

- A call of a closed function substitutes a well-typed argument into a well-typed body. Lemma 4.2 gives the result.
- A `let` reduction is the same substitution case.
- A constructor, `Option`, list, `Result`, product, or literal match selects an arm whose pattern has the subject type.
  Lemma 4.3 types the field substitution, then Lemma 4.2 types the selected body.
- A natural, list, or option fold either returns its initial value or applies its typed step function to one canonical
  field and a typed accumulator. Its result type is unchanged.
- An ordinary compiler operation returns its declared type by the operation contract.
- `map` and `filter` enter private states only with value arguments. Each callback call preserves its stated result type
  by ordinary function application. A wait state records only a value of that type; the completed list therefore keeps
  its member type. `range` and `repeat` construct lists with the declared member type.
- Every controlled music operation returns a typed finite recipe. `map_note_pitches` stores a value of type
  `Pitch -> Pitch` and a `Music`; it does not run the callback at this stage. A checked quote stores its checked kernel
  term and typed `Music` holes.
- An annotation first steps its subject under the `(E:A)` context; once the subject is a value, erasure preserves its
  checked type.
- A step inside a left-to-right evaluation context preserves the subterm type by induction. Rebuilding the surrounding
  typing rule preserves the whole type.

These are all reduction forms. ∎

### Theorem 5.2. Closed terms make progress

If `Delta; Sigma; empty |- e : A`, then `e` is a value or there is an `e'` with `Delta |- e --> e'`.

**Proof.** Induct on the typing derivation.

Introduction forms are values once their fields are values; otherwise the leftmost non-value field steps by induction.
For an application, first step the function, then the argument. If both are values, Lemma 4.4 says the function has a
call rule. `let` behaves the same way.

For a match, first step its subject. If the subject is a value, Lemma 4.4 gives its outer form. The coverage rule
ensures that the corresponding constructor arm or a catch-all arm exists, so the match steps. A client may ignore or
bind an abstract value through a catch-all, but cannot write a constructor pattern for it. A compiled structure body
that can write constructor patterns retains the needed private entry in `Delta`.

A fold first evaluates its arguments. Canonical natural, list, and option forms select one fold rule. `map` and `filter`
first evaluate their arguments, then enter typed private states. A state with no input finishes. A state with input
creates one callback expression. A wait state steps that expression until it is a value; it then records the result and
continues. `filter_wait` instead chooses its unique rule from the canonical values `true` and `false`. `range` and
`repeat` with value arguments produce their finite list values. An ordinary compiler call with value arguments steps by
totality. A controlled music operation or checked quote with value arguments produces its finite recipe value. An
annotation steps its subject or erases a value annotation. No closed, well-typed case is stuck. ∎

### Lemma 5.3. A non-value has one evaluation-context decomposition

Every closed, well-typed non-value can be written in exactly one way as `E[r]`, where `r` is a basic redex and `E` may
be the empty context `hole`.

**Proof.** Induct on the term shape. For a form with several fields, the first non-value field is unique. For an
application the function position precedes the argument. For a match the subject is the only evaluated position before
arm selection.

The structural traversals are the only delicate cases. If either argument of `map` or `filter` is not a value, the
leftmost such argument gives the unique context. The entry rule cannot apply because it requires both values. If both
are values, the entry rule is the unique redex. A `map_wait` with a non-value pending expression has the unique wait
context; its completion rule requires a value. With a value pending expression, only the completion rule applies. A
`filter_wait` with a non-value decision has the unique wait context. With a value decision, canonical forms give exactly
`true` or `false`, and those two rules are disjoint. State formation already requires values in every other field. The
remaining term forms follow their one stated left-to-right order. ∎

### Theorem 5.4. Evaluation is deterministic

If `Delta |- e --> e_1` and `Delta |- e --> e_2`, then `e_1 = e_2`.

**Proof.** Lemma 5.3 gives one decomposition. Its basic redex has one rule. In particular, the value premises keep the
`map` and `filter` entry and completion rules disjoint from their evaluation contexts, and `true` and `false` select
different `filter_wait` rules. A match chooses the first matching arm in source order, which is unique even when a later
catch-all also matches. A compiler operation has one result by its contract. Rebuilding the unique context therefore
gives one next term. ∎

Preservation and progress together give the usual safety result: a closed, well-typed source term never reaches an
untyped or stuck term.

## 6. Termination

Safety does not yet say that evaluation ends. This section proves that stronger claim.

### 6.1 A well-founded type measure

For a type `A`, let:

- `r(A)` be the greatest rank of a nominal type found anywhere in `A`, or 0 if no nominal occurs;
- `s(A)` be the number of nodes in the written type; and
- `m(A) = (r(A), s(A))`, ordered first by rank and then by size.

### Lemma 6.1. Recursive type clauses decrease the measure

Every type mentioned recursively while defining good values has a smaller measure than the type being defined.

**Proof.** The component of a product, function, `Option`, `List`, or `Result` has no greater nominal rank and has
strictly smaller written size. A field of nominal `mu` contains only earlier nominals, so its greatest nominal rank is
strictly less than `rank(mu)`, regardless of its written size. These are all recursive clauses. The lexicographic order
on pairs of natural numbers has no infinite descending chain. ∎

The choice of rank 1 for a leaf nominal is essential. Rank 0 means “no nominal occurs.” Giving a leaf nominal rank 0
would make a field such as `List<Text>` larger rather than smaller in the second component.

### 6.2 Good values and reducible terms

Define `R_A(v)`, read “`v` is good at `A`,” by induction on `m(A)`:

- a base, bridge, or `Music` value is good when it is a closed canonical value of that type;
- a product is good when all fields are good;
- `Some(v)`, a list, `Ok(v)`, and `Err(e)` are good when their contents are good; their empty forms are good;
- a nominal constructor value is good when all its fields are good; and
- a function `f: A -> B` is good when, for every good `a: A`, evaluating `f(a)` ends at a good `b: B`.

A closed term `e: A` is *reducible* when it evaluates in finitely many steps to some `v` with `R_A(v)`.

Lemma 6.1 makes this definition valid. It does not assume the theorem it will prove.

### Lemma 6.2. The admitted operations preserve good values

Applying an ordinary compiler operation to good closed arguments ends at a good result. Each structural operation maps
good arguments to a good finite result. Applying a controlled music operation to good arguments ends at a good finite
`Music` recipe.

**Proof.** Ordinary operation types are first order. By the operation contract, the call ends at a closed, well-typed,
finite value. Induction on the result type shows that such a first-order value is good.

For a natural, list, or option fold, use its finite canonical input. For `map`, induct on the remaining input list in
`map_state`; goodness of the callback makes each `map_wait` expression end at a good output value. The value premise
then records that member. `filter` is the same argument with a canonical `Bool` decision. `range(n)` and `repeat(v,n)`
construct exactly `n` good members after finite preflight.

A controlled music operation constructs one finite recipe node from good values. In particular,
`map_note_pitches(function,music)` stores the good function and recipe; it does not call the function now. A checked
quote stores a finite checked term and finitely many good recipe holes. Both results are closed canonical `Music`
values. Section 11 separately proves that source construction preserves the stronger private recipe invariant. ∎

### Theorem 6.3. Fundamental reducibility lemma

Suppose `Delta; Sigma; Gamma |- e : A`. Replace every free variable in `Gamma` with a good closed value of its declared
type. The resulting closed term is reducible at `A`.

**Proof.** Induct on the typing derivation.

- A variable becomes the good value supplied for it. A literal is already good.
- For a product, constructor, `Option`, list, or `Result`, the induction hypotheses make all fields reducible. The
  call-by-value order reaches good field values, after which the constructed value is good by definition.
- For a lambda, take any good argument. Extend the closing substitution with that argument. The induction hypothesis for
  the body says the call ends at a good result. Hence the function value is good.
- For an application, the induction hypotheses give a good function and argument. The function clause of `R` gives a
  reducible result.
- For `let`, reduce the bound term to its good value and use that value in the induction hypothesis for the body.
- For a match, the subject induction hypothesis gives a good canonical value. Exhaustiveness chooses an arm. Its bound
  fields are good by the definition of the subject's good value. Apply the arm induction hypothesis with those fields.
- For a natural fold, induct on the finite natural value. For a list fold, induct on the finite list. For an option
  fold, inspect its one constructor. The initial value and step functions are good by their induction hypotheses, so
  every finite step yields a good accumulator.
- For `map`, induct on the finite remaining input of its private state. Each good callback ends at a good value before
  the wait rule records it, and the remaining list shortens. `filter` is the same argument with a canonical `Bool`
  decision. `range(n)` and `repeat(v,n)` make finite lists of exactly `n` good members.
- A controlled music operation or checked quote constructs a finite good recipe value from good arguments. It does not
  instantiate the recipe during source reduction.
- Compiler operations follow from Lemma 6.2.
- An annotation reduces its subject under `(E:A)`, then erases the value annotation. Its subject induction hypothesis
  therefore gives the same good value.

Every typing rule is covered. ∎

### Theorem 6.4. Every closed, well-typed source term terminates

If `Delta; Sigma; empty |- e : A`, there is a value `v` such that `Delta |- e -->* v`.

**Proof.** Apply Theorem 6.3 with the empty substitution. Reducibility includes finite evaluation to a value. ∎

This theorem ends at a source value, which may be a finite `Music` recipe or a finite protocol function. It does not
apply to repeated process steps or an unbounded live run.

## 7. Denotational agreement

### Theorem 7.1. Evaluation agrees with the set meaning

Let `rho` assign each free variable of `e` a value in the set denoted by its type. If `e` denotes `d` under `rho`, then
evaluation of the closed instance of `e` reaches a value whose denotation is `d`.

**Proof.** Induct on the typing derivation, using Theorem 6.4 to know that evaluation ends. Literals, products, and
constructors follow their set definitions. Function application follows the mathematical function denoted by the
closure. A match selects the same tagged union case in both meanings. Finite folds satisfy the same base and step
equations in both meanings. `map`, `filter`, `range`, and `repeat` traverse or construct the same finite lists as their
set functions. Controlled music operations construct the corresponding recipe node. Compiler operations agree by their
contracts. Determinism rules out another result. ∎

The theorem is soundness, not full abstraction. Two different source terms may denote the same function even though the
language has no way to decide that equality.

## 8. Sealing

### Lemma 8.1. Public resolution cannot introduce a private constructor

If a client term resolves using `Sigma`, every constructor name in its core output is public in `Sigma`.

**Proof.** Induct on source elaboration. Constructor syntax resolves through the current name table. Sealing removes a
structure's private constructors from `Sigma` before any client is checked. No other elaboration rule creates a
constructor node. Generated core inside the structure was checked earlier against its retained private table and is
marked with that structure as its origin. ∎

### Theorem 8.2. Sealed constructors cannot be forged or inspected

Let `M.T` be abstract in `M`'s public signature. No accepted client-originated core node can name a private constructor,
construct `M.T` with one, or use a constructor pattern to inspect `M.T`.

**Proof.** Construction or constructor-pattern inspection would require a private constructor name, which Lemma 8.1
excludes. Calls to exported functions remain allowed. Their compiled bodies retain the private data table and may safely
construct or inspect the value. A client may match `_` or a bare binder against `M.T`; those patterns discard or pass
through the whole value and reveal no constructor or field. ∎

The theorem depends on the compiler not exposing an unchecked value decoder. This calculus has none.

## 9. Deterministic resource charging

The metered evaluator carries a remaining budget and an ordered diagnostic log. Every evaluation event has one fixed
non-negative charge for the language version. It charges before performing the event.

### Theorem 9.1. Metered evaluation is repeatable

Equal checked terms, initial budgets, logs, operation tables, and language versions produce equal outcomes, remaining
budgets, charge traces, and diagnostics.

**Proof.** Induct on the metered evaluation trace. The next unmetered position is unique by Lemma 5.3. Its charge is a
fixed function of the same event and values. If the two budgets cannot pay, both runs report the same exhaustion before
the event. If they can pay, both subtract the same charge and take the same step by Theorem 5.4 and deterministic
operation contracts. Apply the induction hypothesis to the equal next states. ∎

### Theorem 9.2. Successful meter erasure

If metered evaluation of `e` succeeds with value `v`, unmetered evaluation of `e` reaches the same `v`.

**Proof.** Erase each charge event and budget component from the successful metered trace. Every remaining transition is
the corresponding unmetered transition. ∎

Different starting budgets may yield success and exhaustion for the same term. These theorems do not compare those
states. They also make no cache claim.

## 10. Retained programs and rejected partial calls

This section compares source behavior, not private evaluator states. The current evaluator may create a partial
`BuiltinValue` while processing even a complete source call. The refined language deliberately has no such value.

Let `Old_complete` contain the current well-typed source expressions in which every compiler-owned operation receives
all its declared arguments. The table in `04a-formal-rules.md` §12 maps these expressions to the refined core. The table
covers every current source form, all seven structural operations, all complete controlled music calls, and typed kernel
quotation. The migration bridge set contains every current base type.

Assume one comparison contract: a complete old compiler operation and its refined operation return related results on
related arguments. This is checked once for the finite operation table. It says nothing about either evaluator's private
steps or charges.

Define `v_old ~_A v_new`, read “the values agree at type `A`,” by induction on `A`. Base and bridge values are equal;
products, options, lists, results, nominal values, and finite recipes agree component by component; and two functions
agree when they send agreeing arguments to agreeing results. The function clause compares behavior, not closure layout.

### Theorem 10.1. Retained programs keep their type and result

Let `e` be a closed expression in `Old_complete`. If current checking gives `e` type `A`, its refined translation also
has type `A`. If current evaluation succeeds with `v_old`, refined evaluation finishes with a unique value `v_new` such
that `v_old ~_A v_new`.

**Proof.** We prove the typing and result claims together by induction on the current source typing derivation.

Literals and structural constructors translate directly, so the induction hypotheses give agreeing fields. A current
multi-argument source function becomes nested unary functions. Repeated function checking gives the same curried type,
and the induction hypothesis for its body proves the function clause of `~`. Current named and default arguments are put
in declared order before translation.

Matches use the same first matching pattern. Folds use the same finite base and step equations. For `map` and `filter`,
induct on the finite input list; related callbacks return related members or the same Boolean choice, so the refined
private traversal builds the same result in source order. `range` and `repeat` build the same finite lists.

A complete compiler call has the same argument types on both sides. The induction hypotheses give related argument
values, and the finite operation-table comparison contract gives related results. This applies to complete music calls.
`map_note_pitches` stores related callbacks and recipes without calling the callback. A checked quotation stores the
same checked temporal term, exact loci, and related hole recipes.

These cases cover the translation table. The refined term terminates by Theorem 6.4, and determinism gives its unique
result. The proof never needs to translate a current private `BuiltinValue`. ∎

### 10.2 Rejected programs

The refined checker rejects a compiler operation with missing arguments. This is an intentional source break, not a
metatheory gap. The repository contains partial calls only with fixed supplied values: fixed intervals for `transpose`,
a fixed ratio for `stretch`, a fixed pitch for `invert`, and bare `retrograde`. Named one-argument source functions can
replace those uses and make complete operation calls.

There is no theorem for an arbitrary dynamic partial call. For example, `transpose(interval)` would create a function
that remembers a run-time interval. Expressing that value without special compiler state would require an ordinary
source closure. None of the five musical cases requires one, so anonymous functions remain deferred. A later proposal
must justify them as a general language feature rather than smuggle them back through compiler operations.

A new language version also owns a new fixed resource schedule. No claim compares its charge trace with the old
partial-operation evaluator. A later parser may reserve words, and a later library migration may replace built-in
musical names with imports; those changes need their own implementation plan.

## 11. Closing `Music`

The source proof treats `Music` as abstract. This section discharges the adapter invariant instead of assuming the
successful conclusion.

### 11.1 The private recipe invariant

A private recipe is a finite directed acyclic graph built from these nodes:

```text
Atom(request, finite output bound)
Sequence(recipe list)
Overlay(recipe list)
Transform(name, finite scalar arguments, recipe)
MapPitches(total Pitch -> Pitch function, recipe)
Quote(checked kernel term, finite named recipe holes with exact loci)
Play(finite voicing, exact duration)
Reference(earlier recipe node)
```

Current note, rest, region, assertion, and contextual-use forms are `Atom` or finite combinations of these nodes. A
bounded source repeat becomes a finite `Sequence`. Every node records complete source origin. An edge points only to an
earlier completed node. Each node has a structural bound on the occurrences its successful instantiation can produce.

A recipe is valid when:

1. its graph is finite and acyclic;
2. captured values have their checked source types;
3. every atom adapter is total and deterministic and either reports a stated error or returns a finite admitted
   `ScoreFact` fragment within its bound;
4. every transform is a total finite kernel construction or payload map and preserves admitted payloads;
5. every quote has passed the kernel checker, its free names are exactly its distinct holes, and each locus is valid;
6. its occurrence bound is the sum for sequence and overlay, is preserved by transforms, and is computed structurally
   for a quote, including every use of every hole;
7. it reads only its explicit `MusicalContext` and never changes that context; and
8. its resource preflight occurs before occurrence-sized allocation.

The initial compiler bridges and every later bridge must prove these eight clauses for each recipe constructor it adds.

### Lemma 11.1. Source construction preserves valid recipes

Every closed, well-typed source expression of type `Music` evaluates to a valid finite recipe.

**Proof.** Induct on the source typing and evaluation that can produce `Music`. An atom constructor has a finite request
and bound by its checked bridge entry. Sequence and overlay combine finite acyclic graphs in source order, point only to
completed operands, and use the stated sum and maximum extent rules. A finite repeat makes finitely many references to
one completed body. Each ordinary transform adds one node and preserves the bound and admitted payload type by its
closed table contract. `map_note_pitches` adds one `MapPitches` node holding a good total source function; it does not
run the function. A checked quote has the exact finite term and hole table established by its typing rule. `Play` stores
a finite voicing and exact duration. A use or captured `Music` value refers only to a value already completed in the
acyclic definition graph. No case creates a back edge, unbounded collection, or hidden context read. ∎

### Lemma 11.2. Instantiation preserves the fragment invariant

Instantiating a valid recipe under a well-formed explicit `MusicalContext` finishes with either a stated error or a
finite well-formed `KernelFragment<ScoreFact>` within the recipe's bound.

**Proof.** Visit the finite recipe graph in dependency-first order and induct on its nodes. An atom finishes by its
adapter contract. Sequence and overlay use the accepted kernel constructors on the inductively obtained fragments; they
keep finite compatible binding tables, add occurrence bounds, and use sum or maximum extent as specified. Each ordinary
transform terminates and preserves admitted payloads by its table contract.

For `MapPitches`, first instantiate its source. The successful fragment has at most its finite occurrence bound. Visit
the documented pitch-bearing facts in canonical order. The stored source function is closed and well typed, so Theorem
6.4 makes each of the finitely many calls finish at a `Pitch`; preservation supplies the result type. Replacing only
those pitch fields preserves occurrence support and admitted `ScoreFact` formation.

For `Quote`, instantiate its finite holes by induction. Bind each successful fragment at its checked distinct name and
locus. The quote's free-name equality says no other name remains, and the checked kernel term plus admitted hole terms
gives a finite well-formed fragment. `Play` either reports its stated invalid-duration error or creates one finite fact
per tone in its finite voicing. A reference reuses an earlier completed result with a new origin step. Resource
exhaustion is a stated error before large allocation. Every case ends and respects the bound. ∎

### Lemma 11.3. Closing a valid fragment is total and sound

Closing a finite well-formed fragment finishes with either a stated closure error or a closed, well-typed
`Term<ScoreFact>`.

**Proof.** Keep only bindings reachable from the fragment root. Their dependency graph is a finite subgraph of the
acyclic recipe order. Wrap them in reverse dependency order, so every name is bound outside each use. Then run the
decidable kernel checker. A missing name, invalid payload, or invalid exact placement becomes its stated closure error.
On success, the checker establishes that the term is closed, well typed, finite, and carries admitted `ScoreFact`
payloads. ∎

### Theorem 11.4. A `Music` value closes or reports an error

For every closed, well-typed source expression `music: Music` and well-formed explicit `MusicalContext`, evaluation,
instantiation, and closing finish with either a stated error or a finite, closed, well-typed `Term<ScoreFact>`.

**Proof.** Lemma 11.1 gives a valid recipe. Lemma 11.2 gives an instantiation error or a finite valid fragment. In the
success case, Lemma 11.3 gives a closure error or the required term. ∎

The explicit context is load-bearing. A hidden current key, tuning, staff, or target would not be an input to the
theorem and would break repeatability.

The accepted temporal normalization theorem now applies: a successful closed term evaluates to a finite normalized
timeline. That is a theorem about the temporal kernel, not another source reduction.

## 12. Typed stage composition

The accepted cross-stage rules describe a derivation path as typed edges between anchored representations. Generated
facts retain their generation site and intermediate root. A well-formed rule table states the source and target type of
each edge and the losses it may record.

### Theorem 12.1. Adjacent valid passes compose

Suppose pass `P` returns a valid result from representation `A` to `B`, and pass `Q` consumes that exact stored `B`
result and returns a valid result from `B` to `C`. For every joined path, require the full ending anchor of `P`—its
`PresentationRef` and local anchor id—to equal the full starting anchor of `Q`. If both pass descriptors are well
formed, exact-anchor path concatenation returns a valid path from `A` to `C`. The composite loss list is the ordered
concatenation of `P`'s valid loss records followed by `Q`'s valid loss records.

**Proof.** The exact shared anchor is the premise of the governing path-joining rule. Concatenation keeps that anchor
and both steps, including any generated root and site. Source and target representation kinds agree through the same
stored anchor, so the joined path is typed. Every record in the concatenated loss list was already valid under its own
pass descriptor, and ordered list concatenation changes no record. Thus the composite paths and loss list are well
formed. No loss normalization or semantic-equality substitution is claimed. ∎

### Corollary 12.2. A finite source-to-preparation path composes

Any finite sequence of adjacent valid passes from source through a prepared process graph has a valid composite
derivation.

**Proof.** Induct on the number of passes and apply Theorem 12.1 at each adjacent boundary. ∎

For audio, the same statement applies to every finite history prefix. The process step theorem gives one next block and
state at a time. There may be no final block, so no theorem claims that an unbounded run normalizes.

## 13. Results and limits

Under the stated contracts, the proposed language has the required properties:

1. resolution and checking decide an answer;
2. substitution preserves typing;
3. evaluation preserves typing;
4. closed well-typed terms are values or can step;
5. evaluation is deterministic;
6. every source term terminates;
7. sealed constructors cannot be forged by clients;
8. retained current programs keep their type and result;
9. closing `Music` yields a finite typed temporal term or a stated error; and
10. valid typed stage passes compose.

The proof does not establish complete inference, equality of functions, a package solver, persistent type identity,
compiled caching, cultural adequacy, or a final value for live audio. None is needed for this language decision.
