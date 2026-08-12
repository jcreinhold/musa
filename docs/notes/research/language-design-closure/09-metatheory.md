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

Finite natural, list, and option folds are core reduction rules. The existing `map_note_pitches` operation is the one
admitted higher-order music transform. Its input `Music` value carries a finite occurrence bound. It visits only the
documented pitch positions within that bound and calls its supplied total source function once at each visited position.
No other higher-order foreign operation is admitted.

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
list, or a finite match-arm list. Structural type equality recurses over two finite type trees. `None`, `[]`, `Ok`, and
`Err` use a supplied expected type, so the checker never searches for a missing type argument. Flat-pattern coverage is
a finite test over `Bool`, `Option`, `List`, `Result`, or the finite constructor list in `Delta`; all other subject
types need a catch-all arm. Signature matching compares two finite member tables and checks each body once.

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
- `map_note_pitches` returns `Music` by its admitted traversal contract; each callback call preserves `Pitch` by the
  ordinary function application case.
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
ensures that the corresponding constructor arm or a catch-all arm exists, so the match steps. A client cannot write a
match on an abstract type; a compiled structure body that can write the match retains the needed private entry in
`Delta`.

A fold first evaluates its arguments. Canonical natural, list, and option forms select one fold rule. An ordinary
compiler call with value arguments steps by totality. The admitted music traversal steps over its finite private
representation. No closed, well-typed case is stuck. ∎

### Lemma 5.3. A non-value has one evaluation-context decomposition

Every closed, well-typed non-value is either one basic redex or can be written in exactly one way as `E[r]`, where `E`
selects the next left-to-right call-by-value position and `r` is a basic redex.

**Proof.** Induct on the term shape. For a form with several fields, the first non-value field is unique. For an
application the function position precedes the argument. For a match the subject is the only evaluated position before
arm selection. Each remaining form has the one order stated by the semantics. ∎

### Theorem 5.4. Evaluation is deterministic

If `Delta |- e --> e_1` and `Delta |- e --> e_2`, then `e_1 = e_2`.

**Proof.** If `e` is a basic redex, its outer form selects one rule. A match chooses the first matching arm in source
order, which is unique even when a later catch-all also matches. A compiler operation has one result by its contract. If
`e` is not a basic redex, Lemma 5.3 selects one context and one inner redex. Apply the induction hypothesis to the inner
step and rebuild the same context. ∎

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

Applying an ordinary compiler operation to good closed arguments ends at a good result. Applying `map_note_pitches` to a
good pitch function and good `Music` value ends at a good `Music` value.

**Proof.** Ordinary operation types are first order. By the operation contract, the call ends at a closed, well-typed,
finite value. Induction on the result type shows that such a first-order value is good.

For `map_note_pitches`, the input recipe has a finite occurrence bound. Induct on the number of visited pitch positions.
At zero positions the unchanged finite recipe is good. At the next position, goodness of the supplied function makes its
call end at a good `Pitch`; the traversal replaces that one pitch and continues with a smaller remaining count. Thus the
whole traversal ends at a finite `Music` value. ∎

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
- Compiler operations follow from Lemma 6.2.
- An annotation has the same meaning as its checked term.

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
equations in both meanings. Compiler operations agree by their contracts. Determinism rules out another result. ∎

The theorem is soundness, not full abstraction. Two different source terms may denote the same function even though the
language has no way to decide that equality.

## 8. Sealing

### Lemma 8.1. Public resolution cannot introduce a private constructor

If a client term resolves using `Sigma`, every constructor name in its core output is public in `Sigma`.

**Proof.** Induct on source elaboration. Constructor syntax resolves through the current name table. Sealing removes a
structure's private constructors from `Sigma` before any client is checked. No other elaboration rule creates a
constructor node. Generated core inside the structure was checked earlier against its retained private table and is
marked with that structure as its origin. ∎

### Theorem 8.2. Sealed constructors cannot be forged

Let `M.T` be abstract in `M`'s public signature. No accepted client-originated core node can construct or directly match
a value of `M.T`.

**Proof.** Construction would require a private constructor name, which Lemma 8.1 excludes. Direct nominal matching also
requires the private constructor set to type and check coverage, but the public environment exposes only the abstract
name `M.T`; the match rule therefore rejects it. Calls to exported functions remain allowed. Their compiled bodies
retain the private data table and may safely construct or inspect the value. ∎

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

## 10. The old expression fragment

Let `Old` be the already accepted expression core before the additions in this proposal. Embed its types, terms, values,
finite folds, `Music` constructors, and admitted operations without changing them.

### Theorem 10.1. The new calculus conservatively extends `Old`

For every closed old term `e`:

1. if `Old` gives `e` type `A`, the new calculus gives the embedded term the embedded type `A`;
2. every old step is the same new step; and
3. old and new evaluation reach the same value.

**Proof.** Induct on the old typing derivation for the first claim. Every old rule appears unchanged. New rules have new
outer forms and are never needed.

For the second claim, inspect the old reduction rule. The new semantics retains it with the same evaluation order.
Finite folds keep their structural rules. Old first-order operations retain their entries and contracts.
`map_note_pitches` retains its bounded traversal rather than entering the open ordinary-operation family.

For the third claim, repeatedly use the second claim. Both evaluations end, and determinism gives the same value. ∎

This is a core-language theorem. A later parser change may reserve new words, and a later library migration may replace
built-in musical names with imports. Those source-compatibility questions need their own implementation plan.

## 11. Closing `Music`

The source proof treats `Music` as an abstract finite value. The next result needs an adapter contract because source
typing cannot inspect the private recipe representation.

Assume:

1. `instantiate(music, context)` is total, deterministic, and type preserving;
2. every successful result is a finite, well-formed `KernelFragment<ScoreFact>`;
3. `close(fragment)` is total and deterministic; and
4. every successful close result is a closed, well-typed temporal `Term<ScoreFact>`.

### Theorem 11.1. A `Music` value closes or reports an error

For every closed `music: Music` and well-formed explicit `NotationContext`, instantiation followed by closing finishes
with either a stated error or a finite, closed, well-typed `Term<ScoreFact>`.

**Proof.** Totality of `instantiate` gives two cases. An error is a stated final result. On success, its contract gives
a finite, well-formed fragment. Totality of `close` again gives two cases. An error is stated; a success is the finite,
closed, well-typed temporal term promised by the fourth assumption. ∎

The explicit context is load-bearing. A hidden current key, tuning, staff, or target would not be an input to the
theorem and would break repeatability.

The accepted temporal normalization theorem now applies: a successful closed term evaluates to a finite normalized
timeline. That is a theorem about the temporal kernel, not another source reduction.

## 12. Typed stage composition

The accepted cross-stage rules describe a derivation path as typed edges between anchored representations. Generated
facts retain their generation site and intermediate root. A well-formed rule table states the source and target type of
each edge and the losses it may record.

### Theorem 12.1. Adjacent valid passes compose

Suppose pass `P` returns a valid result from representation `A` to `B`, and pass `Q` returns a valid result from that
same `B` value to `C`. If both rule tables are well formed, the accepted composition operation returns a valid path from
`A` to `C` and the normalized combined loss record.

**Proof.** Every end anchor of a path from `P` that `Q` uses is the start anchor and semantic identity recorded by `Q`.
The governing composition rule joins at that anchor while retaining the intermediate step, including any generated root
and site. Source and target types agree by the premise, so the joined path is typed. The governing loss operation
combines and normalizes the two valid loss lists. Its closure law gives another valid list. Thus the composite is well
formed. ∎

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
8. the source core conservatively extends the existing expression fragment;
9. closing `Music` yields a finite typed temporal term or a stated error; and
10. valid typed stage passes compose.

The proof does not establish complete inference, equality of functions, a package solver, persistent type identity,
compiled caching, cultural adequacy, or a final value for live audio. None is needed for this language decision.
