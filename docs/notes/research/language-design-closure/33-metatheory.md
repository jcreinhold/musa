# Metatheory of the inferred Musa language

## Purpose

This document proves the claims stated in the proof outline. Its one job is to answer:

> Do the rules in notes 29 and 30 define a deterministic, type-safe, terminating source language whose bounded syntax
> adapters and `Music` values reach Musa's accepted later stages without bypassing a check?

Yes—under the explicit implementation contracts in §1. The proofs do not cover package resolution, stable compiled
identity, or cache correctness.

## 1. Assumptions and theorem scope

Fix one build. Assume:

1. its package graph, source files, declaration tables, and compiler options are finite;
2. each table lookup is functional and every assigned id is fresh within the build;
3. nominal dependencies are acyclic except for the one permitted polynomial self-reference;
4. source values are immutable finite constructor trees and closures;
5. value declarations and lowering-only join dependencies are acyclic, and joins are erased before evaluation;
6. each compiler operation is first-order, total, deterministic, type preserving, and reducibility preserving;
7. scope ids, checked syntax wrappers, generated source ids, and resolved declaration ids can be made only by the
   compiler;
8. adapter definitions contain no adapter region; emitted adapter-call edges form a finite acyclic graph and an adapter
   may emit calls only to declared lower-rank adapters;
9. each admitted score map is first-order, total, deterministic, preserves admitted fact templates, and obeys the finite
   reference-unfolding rewrite in note 30;
10. the temporal kernel satisfies its governing typing and normalization rules; and
11. every later pass record is accepted by the governing presentation, anchor, evidence, and loss schemas.

Assumption 6 is checked once for each compiler operation. It does not allow the operation to receive a source function.
Assumption 9 is checked once for each `ScoreMapId`. Neither assumption says a source or `Music` computation succeeds;
the rules still return their stated `Result` values and diagnostics.

The mathematical source relation ignores resource limits. The limited implementation computes the same candidate step
and charge, then either commits it or returns the first limit diagnostic.

## 2. Declarations, resolution, and folds

### Lemma 2.1: accepted declarations have a finite order

Every accepted non-self nominal reference points to a smaller declaration rank.

**Proof.** Remove each permitted self-edge from the finite declaration graph. Acceptance requires the remaining graph to
be acyclic. Every finite acyclic graph has a topological order. Number declarations in that order. An import refers to a
dependency module already checked before the current module, and a local reference points to an earlier declaration, so
each non-self edge lowers the number. The removed edges are exactly the self occurrences admitted by the polynomial
grammar. ∎

### Definition 2.2: polynomial lifting

Let `X` be a relation on values of the recursive type `D`. For a permitted recursive field shape `P`, define
`Lift_P(X)`:

- `Lift_Self(X) = X`;
- a self-free field uses the ordinary reducibility relation of its type;
- a product satisfies the lift when every component does;
- a list satisfies it when every member does;
- `None` satisfies an option lift, and `Some(v)` satisfies it when `v` does; and
- `Ok(v)` or `Err(v)` satisfies a result lift when the selected payload does.

This definition is structural because the permitted field-shape grammar is finite.

### Lemma 2.3: each generated fold is well typed

For each recursive type `D<Ā>`, replacing every recursive `D<Ā>` occurrence in a constructor field by result type `R`
gives the argument type of that constructor's algebra. The generated `fold_D` has the scheme stated in note 29 §4.3.

**Proof.** Induct on the permitted field shape.

- `Self` becomes `R` by definition.
- A self-free field keeps its type.
- Products transform each component.
- `List`, `Option`, and `Result` transform their recursive payload positions with the corresponding container map.

The constructor case applies its algebra to exactly those transformed fields, so the result is `R`. Quantifying the data
parameters and `R` gives the generated rank-1 scheme. ∎

### Theorem 2.4: name resolution terminates and is functional

For a finite grouped module, resolution returns one resolved program or one finite ordered diagnostic set.

**Proof.** Header parsing traverses a finite list of groups. Every qualified import path follows a fixed edge in the
resolved build graph and performs finite table lookups. Every unqualified namespace lookup searches one finite table.
The resolver accepts exactly one result and reports zero or more than one. Record construction names its `TypeId`; short
field projection is accepted only when one visible `FieldId` has that spelling. Scope comparison is decidable equality
on finite authenticated ids.

The resolver visits each finite syntax node once, plus finite lookup work at the node. No step calls the resolver on a
larger source or changes the build graph. It therefore terminates. Functional tables and the fixed diagnostic order give
one result. ∎

## 3. Principal type inference

### Lemma 3.1: monotype unification terminates

Unification with an occurs check terminates on Musa monotypes.

**Proof.** Use the lexicographic measure:

```text
(number of unsolved variables, total size of pending equations).
```

Solving `α = τ` removes one unsolved variable after the occurs check and substitutes a finite type. Decomposing equal
type heads removes one outer constructor and replaces the equation by strictly smaller component equations. Unequal base
or rigid nominal heads fail. The occurs check fails on `α` inside `τ` instead of creating an infinite type. Each step
strictly decreases the standard unification measure, so the finite equation set is exhausted or fails. ∎

### Lemma 3.2: a successful unifier is most general

If unification returns `S` for equations `E`, every other solution `T` factors as `R ∘ S` on the variables of `E`.

**Proof.** This is the usual Robinson unification invariant. Variable elimination preserves all solutions and records
the most general substitution; decomposition preserves equivalence of the equation sets; rigid equal heads add no
choice. Rigid nominal ids change only the failure case: unequal ids have no unifier. Induction over the terminating run
gives the factorization. ∎

### Theorem 3.3: Algorithm W terminates

For a finite resolved expression, W returns a substitution and type or a finite diagnostic.

**Proof.** Induct on expression size. Every recursive call is on a proper subexpression. A `let` infers its finite right
side and body; it never calls itself. A match checks a finite scrutinee and finite arm list. Generated folds are
ordinary resolved values with finite schemes. At each node, W calls terminating unification on finite types by Lemma
3.1. Coverage is a separate finite pattern-matrix traversal that specializes a finite pattern at each step. Therefore
every branch terminates. ∎

### Theorem 3.4: W is sound

If `W(Σ, Γ, e) = (S, τ)`, then `Σ ; SΓ ⊢ e : τ`.

**Proof.** Induct on `e`.

- A variable receives a fresh instance of its recorded scheme, which is the `Var` rule.
- A literal uses its base rule.
- A function gives fresh monotypes to its parameters. The induction hypothesis types its body under the substituted
  environment, so `Fn` types the function.
- A call first types its function and arguments, then unifies the function type with the complete product arrow. The
  returned substitution makes the `Call` premises exact.
- A product, constructor, record, or field follows its corresponding premise componentwise. The resolved rigid id picks
  one scheme before W begins.
- A `let` types its right side by induction, generalizes exactly the variables not free in the substituted environment,
  and types the body under that scheme. This is `Let`.
- `if` unifies the test with `Bool` and both branch types with one fresh result.
- A match types the scrutinee, pattern bindings, and every arm, then unifies arm results. Coverage adds no equation. The
  `Match` rule follows.
- An annotation unifies the inferred type with the written monotype.
- An operation uses its fixed complete scheme and componentwise argument premises.

In each case, Lemma 3.2 substitutions make all premises agree. ∎

### Theorem 3.5: W is complete and principal

If `e` has any typing under an instance of `Γ`, W succeeds. Its result is more general than every other typing in the
factorization sense of note 32 Theorem 2.5.

**Proof.** Again induct on `e`. The typing rule fixes the same outer form W examines. Apply the induction hypothesis to
each premise. The other derivation's equalities solve the equation set W sends to unification, so Lemma 3.2 factors that
solution through W's most general unifier.

At `let`, the source is non-recursive and pure. Any type variable not free in the environment may be generalized; any
variable free in the environment may not. Thus `gen` is the greatest valid rank-1 scheme. Instantiation in later uses is
handled by the variable case. Constructors, records, and fields have fixed rigid schemes, so they introduce no hidden
choice. Match coverage can reject an ill-covered but otherwise typable term; for an accepted term it does not restrict
the result type. ∎

### Corollary 3.6: public module schemes are principal

**Proof.** Check top-level values in their finite dependency order from Theorem 2.1. Apply Theorem 3.5 at each
non-recursive boundary and generalize against the schemes already fixed. Hiding a constructor removes a name from the
exported table; it does not alter the inferred value derivation. ∎

## 4. Substitution and canonical values

### Lemma 4.1: weakening

If `Σ ; Γ ⊢ e : τ` and `Δ` adds fresh bindings not captured by `e`, then `Σ ; Γ, Δ ⊢ e : τ`.

**Proof.** Induct on the typing derivation. Variable lookup remains valid; binders are renamed apart; every other rule
reuses the induction hypotheses. ∎

### Lemma 4.2: type substitution

Applying a capture-avoiding type substitution to a typing derivation yields a typing derivation for the substituted
environment, term annotations, and result type.

**Proof.** Induct on the derivation. Type substitution acts homomorphically. It does not change rigid declaration ids.
At a generalized scheme, rename bound type variables away from the substitution domain before applying it. ∎

### Lemma 4.3: monomorphic term substitution

If `Σ ; Γ, x : τ ⊢ e : υ` and `Σ ; Γ ⊢ v : τ`, then `Σ ; Γ ⊢ e[v/x] : υ`.

**Proof.** Induct on the derivation of `e`. The variable case is either `x`, where the second premise supplies the
result, or another variable. At a binder, rename it away from `x` and use weakening. Constructors, patterns, and joins
substitute componentwise. No operation binds a source variable. ∎

### Lemma 4.4: one generalized value serves every instance

Suppose `Σ ; Γ ⊢ v : τ` and `ᾱ` is disjoint from `ftv(Γ)`. For any monotypes `ρ̄`, `Σ ; Γ ⊢ v : τ[ρ̄/ᾱ]`.

**Proof.** Apply Lemma 4.2 to the derivation. Since none of `ᾱ` occurs in `Γ`, the environment is unchanged. ∎

### Lemma 4.5: pattern matching gives typed bindings

If a closed value `v : τ` matches a linear well-typed pattern `p : τ ⇒ Γp` with substitution `θ`, then every `θ(x)` has
the type assigned to `x` in `Γp`.

**Proof.** Induct on `p`. Wildcard binds nothing. A variable binds `v : τ`. Product, record, and constructor patterns
use the canonical outer form of `v` and the induction hypothesis on each component. Linearity makes domains disjoint.
The boolean cases bind nothing and match only the corresponding boolean value. ∎

### Lemma 4.6: accepted coverage matches every closed value

**Proof.** The coverage checker specializes the pattern matrix by the finite constructors of the scrutinee type. Its
acceptance witness contains a covered branch for every constructor and recursively for every refutable field. Follow the
actual finite value's constructor through that witness. Each descent enters a proper field value or removes one pattern
column. It reaches an irrefutable row, which matches. For an abstract type, the only accepted covering row is a variable
or wildcard because no constructor is visible. ∎

## 5. Preservation, progress, and determinism

### Theorem 5.1: evaluation preserves types

If `Σ ; ∅ ⊢ e : τ` and `e -> e'`, then `Σ ; ∅ ⊢ e' : τ`.

**Proof.** Inspect the redex, then lift the result through its evaluation context.

- **Beta.** The function premise types its body under all parameters. Apply Lemma 4.3 once for each typed argument.
- **Let.** The typing rule gives `v : τ₁` and types the body with `x : gen(∅, τ₁)`. Each use of `x` has some instance of
  that scheme. Lemma 4.4 gives `v` at every such instance, and the scheme form of substitution replaces all uses.
- **Condition.** A closed boolean value is `true` or `false`; the chosen branch already has result type `τ`.
- **Field.** The resolved field scheme and record typing give the selected component exactly the projection type.
- **Annotation.** The premise already types `v` at the annotated type.
- **Match.** Lemma 4.5 types the substitution produced for the least matching arm. Term substitution preserves the arm
  type.
- **Fold.** Lemma 2.3 types every recursively folded proper child at result type `R`, then types the selected algebra
  application at `R`.
- **Operation.** Assumption 6 gives a value at the descriptor's return type.

For a context step, induction on the context grammar replaces one premise by another term of the same type. The outer
typing rule is unchanged. ∎

### Theorem 5.2: closed well-typed terms make progress

If `Σ ; ∅ ⊢ e : τ`, then `e` is a value or there is an `e'` with `e -> e'`.

**Proof.** Induct on the typing derivation.

- Literals, fully evaluated products, records, constructors, and functions are values.
- For a call, first progress the function, then each argument from left to right. When all are values, the canonical
  forms lemma says a source function type contains a function closure, so Beta applies. A compiler operation is not a
  source function type and uses its own rule.
- `let`, products, constructors, records, fields, conditions, annotations, folds, and operations similarly progress the
  leftmost non-value premise. Once their premises are values, the corresponding reduction applies.
- A closed record at a resolved field type contains that field.
- A closed `Bool` is `true` or `false`.
- For a match with a value scrutinee, Lemma 4.6 supplies at least one matching arm, and the finite ordered list has one
  least matching index.
- A recursive value has one constructor, so the matching fold algebra applies.
The explicit `(E : τ)` context is essential in the annotation case. ∎

### Lemma 5.3: unique decomposition

Every closed well-typed non-value is uniquely `E[r]`, where `E` is an evaluation context and `r` is one redex.

**Proof.** Structural induction on the term. Each compound form chooses the first non-value child in its stated
left-to-right order. If no child remains, its outer typing and canonical forms select exactly one redex. Evaluation
contexts do not overlap a value position. `Match-First` chooses the least successful arm; a functional operation
registry chooses one rule. Joins have already been erased. ∎

### Theorem 5.4: evaluation is deterministic

If `e -> e₁` and `e -> e₂`, then `e₁ = e₂`.

**Proof.** By Lemma 5.3, both steps use the same context and redex. Beta substitution, field lookup, selected match arm,
fold algebra, and operation function are functional. Therefore both reducts are equal. ∎

## 6. Ordered match lowering

### 6.1 A decision tree without failure

The coverage checker produces a witness tree with three nodes:

```text
Leaf(row number, variable bindings)
Bind(field path, name, subtree)
Switch(field path, one subtree for every visible constructor)
```

At a `Switch`, constructor specialization preserves the original row order inside each subtree. Acceptance requires a
subtree for every constructor. A wildcard or variable row appears in every compatible specialization. A `Leaf` names the
least surviving source row.

### Lemma 6.1: decision-tree compilation terminates

**Proof.** Each specialization removes one tested constructor pattern at a selected field path. Recursive field paths
come from finite source patterns, not arbitrary values, so their total pattern depth is finite. A `Leaf` removes the
remaining irrefutable row. Thus the finite sum of refutable pattern nodes strictly decreases. ∎

### Lemma 6.2: the decision tree chooses the first matching row

For every closed scrutinee value, evaluating the witness tree reaches the least source row whose pattern matches.

**Proof.** Induct on the witness tree. `Switch` selects the actual constructor and keeps exactly the rows compatible
with it, in source order. `Bind` records the component named by the source pattern. At `Leaf`, all remaining tests in
the chosen row are irrefutable. Any earlier source row was removed by a constructor mismatch on the path, so the leaf
row is the least match. Lemma 4.6 ensures a leaf exists. ∎

### 6.2 Sharing equal fallback subtrees

Before join introduction, the decision tree can contain structurally equal fallback subtrees. The compiler selects a
maximal repeated subtree, computes its free value variables in fixed source order, and replaces every occurrence by a
tail jump to one fresh join whose parameters are those variables. It repeats until no selected duplicate remains.

### Lemma 6.3: join erasure preserves type and result

Erase:

```text
join j(x̄) = body in continuation
```

to:

```text
let j = fn(x̄): body in continuation
```

and erase `jump j(v̄)` to `j(v̄)`.

For acyclic joins, erasure preserves typing and terminating ordinary values.

**Proof.** Induct on the join dependency order. The join typing rule and `Fn` assign the same product parameters and
result. A jump and complete call substitute the same already evaluated values into the same captured body. Since the
join cannot escape and every jump is tail, no source context can distinguish the administrative function value. The
induction hypothesis handles earlier joins in the body. ∎

### Theorem 6.4: match lowering is total, typed, and correct

**Proof.** Lemma 6.1 gives a finite complete decision tree. Each `Switch` is exhaustive at its scrutinee type, each
`Bind` uses the field type proved during pattern checking, and all leaves have the common arm result type. Thus it is
typed and contains no failure form. Lemma 6.2 gives the source arm result. Join introduction replaces only equal
subtrees with the transformation of Lemma 6.3, so it preserves that result and terminates over a finite tree. ∎

## 7. Strong normalization

### 7.1 Reducibility candidates

Fix an assignment `η` mapping each free type variable to a set of closed values satisfying these candidate conditions:

1. every member has the assigned type;
2. membership is unaffected by the path by which the value was evaluated; and
3. a value in the set is finite.

Define `R_η(τ)` on closed values:

- `Unit`, `Bool`, `Nat`, `Ratio`, and `Text` contain all closed values of that base type;
- a product or record is reducible when every component is reducible;
- a non-recursive nominal constructor is reducible when every field is reducible at its declared type;
- for a recursive nominal `D`, reducibility is the least relation generated by its constructors using `Lift_P(R_η(D))`
  from Definition 2.2 for recursive fields;
- a function value `f : A -> B` is reducible when, for every `a ∈ R_η(A)`, evaluation of `f(a)` terminates in a value in
  `R_η(B)`; and
- a type variable uses `η`.

An abstract nominal value uses the owning declaration's clause. A client need not see that clause to rely on the module
theorem.

Define term reducibility:

```text
T_η(τ, e)  exactly when  e ->* v for some v ∈ R_η(τ).
```

For a scheme `∀ᾱ.τ`, a closed value is reducible when it belongs to `R` for every candidate assignment to `ᾱ` and every
corresponding well-formed type instance.

The recursive nominal clause is well defined: values are finite trees, and membership for a recursive child is asked
only on a proper subvalue. Non-self nominal fields have smaller declaration rank by Lemma 2.1.

### Lemma 7.1: backward closure

If `e -> e'` and `T_η(τ, e')`, then `T_η(τ, e)`.

**Proof.** Prefix the finite reduction from `e'` by the one step from `e`. ∎

### Lemma 7.2: polynomial maps preserve reducibility

Suppose every proper recursive child of a `D` value folds to a reducible `R` value. Transforming one permitted field
shape for the generated algebra produces a reducible transformed field.

**Proof.** Induct on the field shape. `Self` uses the premise. A self-free field is already reducible. Products use the
component hypotheses. `List`, `Option`, and `Result` use finite container induction and preserve the selected
constructor. ∎

### Lemma 7.3: generated folds are reducible

If `d ∈ R_η(D<Ā>)` and every constructor algebra is reducible at its generated function type, then `fold_D(d, algebras)`
is in `T_η(R, -)`.

**Proof.** Induct on the finite constructor tree `d`. The operational rule recursively folds only proper recursive
children. The induction hypothesis makes each result reducible. Lemma 7.2 makes each transformed constructor field
reducible. The selected reducible algebra therefore terminates in a reducible result. Prefix the administrative fold
steps using Lemma 7.1. ∎

### Lemma 7.4: compiler operations are reducible

**Proof.** This is assumption 6. The first-order restriction is load-bearing: the operation receives no source function
that it could invoke repeatedly. ∎

### Theorem 7.5: fundamental reducibility lemma

If `Σ ; Γ ⊢ e : τ` and a closing substitution `θ` maps every variable scheme in `Γ` to a reducible value at each of its
instances, then `T_η(τ, θ(e))`.

**Proof.** Induct on the typing derivation.

- A variable follows from `θ`.
- Base literals are reducible values.
- Products, records, and constructors evaluate fields from left to right. The induction hypotheses terminate in
  reducible values; their completed value is reducible by definition.
- For `fn(x̄): e`, take arbitrary reducible arguments. Extend `θ` with them. The body induction hypothesis gives a
  terminating reducible result, so the closure is reducible at the arrow type.
- For a call, the induction hypotheses yield a reducible function and reducible complete argument product. The function
  clause gives the result.
- For `let`, the right side yields a reducible value. Generalization variables are absent from `Γ`; the proof is uniform
  in `η`, so the value satisfies every instance of the generalized scheme. Extend `θ` and apply the body hypothesis.
- A condition gets a reducible boolean, which is `true` or `false`, then uses the selected branch hypothesis.
- A field selects one reducible record component.
- For a match, the scrutinee becomes a reducible value. Lemma 4.6 selects an arm, Lemma 4.5 supplies reducible bindings
  by structural clauses, and the arm hypothesis applies.
- A fold uses Lemma 7.3.
- An operation uses Lemma 7.4.
- An annotation uses its inner hypothesis and one erase step.
Every administrative prefix is finite and covered by Lemma 7.1. ∎

### Theorem 7.6: closed well-typed source terms terminate

If `Σ ; ∅ ⊢ e : τ`, there is exactly one value `v` with `e ->* v`.

**Proof.** Apply Theorem 7.5 with the empty substitution. It supplies a finite reduction to a reducible value. Theorem
5.4 makes that value unique. ∎

### Corollary 7.7: the limited evaluator terminates deterministically

**Proof.** The mathematical reduction has finitely many deterministic steps by Theorem 7.6. Unique decomposition gives
one finite redex `r` and deterministic finite reduct `r'`. Their structural sizes make
`1 + semantic_size(r) + semantic_size(r')` computable. The limited evaluator either commits that unique step or returns
the unique first limit diagnostic. Therefore it also terminates. ∎

## 8. Hidden constructors

### Theorem 8.1: a client cannot forge or inspect a hidden constructor

Let constructor `C` be private to module `M`. In a resolved expression belonging to a client module:

1. original client syntax cannot resolve a constructor occurrence or pattern to `C`;
2. preserved client input syntax cannot resolve to `C`; and
3. generated syntax can resolve to `C` only when an adapter exported by `M` used a checked `definition_name` for `C`.

**Proof.** The client import table contains only the interface of `M`, and that interface omits `C`. Thus ordinary
qualified or short lookup cannot return its id. A preserved identifier retains client use-site scopes, so the same
lookup applies. Textual spelling cannot forge `M`'s opaque definition scope by assumption 7.

A `definition_name` checked inside `M` carries `M`'s definition scope and may therefore name `C`. If `M` exports that
adapter, the construction or inspection is an operation deliberately supplied by the owner, just as an exported ordinary
function may call a private constructor. Local-slot identifiers have expansion-local scopes and cannot equal `C`'s
binding. These exhaust the identifier origins. ∎

### Corollary 8.2: module sealing preserves abstraction

A client can observe an abstract value only through exported ordinary values, functions, adapters, and wildcard or
variable binding. It cannot branch on or construct the hidden representation on its own.

**Proof.** Theorem 8.1 excludes every direct constructor and constructor-pattern path. The remaining source forms do not
inspect a nominal representation. ∎

This is a source abstraction theorem for one checked build. It is not a promise that type ids or compiled values remain
stable across builds.

## 9. Syntax expansion

### Lemma 9.1: adapter definitions compile without expansion

Every adapter definition checks and terminates without invoking another adapter.

**Proof.** Assumption 8 excludes adapter regions from definitions. Ordinary subterms use Theorems 2.4, 3.3–3.5, 5.1–5.4,
and 7.6. Each transformer builder has one fixed typing rule and one deterministic finite reduction on finite arguments.
Structural induction on the adapter body proves typing; the reducibility proof adds one total first-order case per
builder. Algorithm W treats each saturated builder as one rigid known scheme, so its principal-type case is the same as
`op`. Emitted-call ranks concern the returned syntax, not this proof. ∎

### Lemma 9.2: one expansion step lowers the rank multiset

**Proof.** A call to adapter `A` removes one occurrence of `rank(A)`. Its finite result may contain only adapters in
`A`'s declared lower-rank list. Thus it replaces one multiset element by finitely many strictly smaller elements. By the
definition of the strict multiset extension of `<`, the measure decreases. ∎

### Theorem 9.3: expansion terminates

For fixed finite grouped syntax and fixed expansion inputs, expansion stops with ordinary expression syntax or a
diagnostic.

**Proof.** The strict multiset order over natural numbers is well founded. By Lemma 9.2, every successful replacement
strictly decreases it. The called adapter itself terminates by Lemma 9.1 and Theorem 7.6. A syntax or resource error
stops immediately. Therefore no infinite expansion sequence exists. ∎

### Theorem 9.4: expansion is deterministic

For equal grouped syntax, syntax imports, adapter definitions, compiler options, and limits, two expansion runs return
equal output syntax and records or the same diagnostic and charge trace.

**Proof.** Induct on the well-founded rank multiset. The strategy selects the same leftmost outermost call. The same
adapter function receives the same explicit `ExpansionContext` and finite `BlockSyntax`; source evaluation is
deterministic by Theorem 5.4 and limited evaluation by Corollary 7.7. Its node and scope ids are structural functions of
the context and literal slots. On success it yields equal finite syntax. The remaining multiset is smaller, so the
induction hypothesis applies. On error both runs stop at the same point. ∎

### Theorem 9.5: expansion is independent of type inference

Changing an expected type, ordinary value environment, or later inference substitution cannot change expansion when its
specified inputs remain equal.

**Proof.** None of those objects occurs in `expand_A`'s type, adapter capability set, compiler selection rule, or rank
measure. The expansion transition is a function only of the inputs named in Theorem 9.4. Functional extensionality gives
the same result. ∎

### Theorem 9.6: expansion is hygienic

Definition identifiers resolve from the adapter definition, preserved input identifiers retain their use-site binding,
and local-slot identifiers capture neither.

**Proof.** Induct on the finite syntax built by the transformer.

- `definition_name` attaches the checked definition scope.
- Inserting existing syntax copies the complete identifier including scopes.
- `local_name(ctx, node_path, binding_path, hint)` attaches `LocalScope(expansion_id(ctx), binding_path)`. The adapter
  checker gives distinct declared binders distinct binding paths; repeated uses of one binding path deliberately refer
  to one binder.
- Groups and other nodes apply the hypotheses componentwise.

Package code cannot construct, remove, or rewrite a raw scope id. Ordinary resolution compares the scoped identity, not
text alone. Therefore the three cases retain the stated binding. Nested expansion repeats the same argument at a lower
rank. ∎

### Theorem 9.7: every output node has finite source attribution

**Proof.** Induct on expansion steps. Existing nodes retain their prior `SourceInfo`, whose chain is finite by the
induction hypothesis. Every new node receives the exact context-derived `ExpansionId` and checked unique node path. The
record names the exact use site and optional parent. Parent records were created earlier in the finite sequence. Theorem
9.3 says only finitely many records are created. Following parents therefore ends at an original use site; it cannot
cycle forward to a later record. ∎

### Theorem 9.8: checked adapter output is source-safe

If expansion produces `ExprSyntax`, parsing and resolution accept it, and W infers `τ`, then its lowered evaluation
returns one value of `τ` or the deterministic limit diagnostic.

**Proof.** Checked wrappers guarantee one expression parse, but they do not grant a type. The result goes through the
same resolver as handwritten source (Theorem 2.4) and the same W (Theorems 3.3–3.5). Match lowering is type and result
preserving by Theorem 6.4. Evaluation preserves type and progresses by Theorems 5.1–5.2, is deterministic by Theorem
5.4, and terminates by Theorem 7.6. No adapter rule skips one of those phases. ∎

## 10. `Music` closure

### Definition 10.1: recipe validity

`ValidMusic(m)` holds when:

1. `m` is a finite `ScoreRecipe`;
2. each relative span has `0 ≤ start ≤ end`;
3. every `FactTemplate` is admitted by the current `ScoreFact` schema;
4. share names are unique in their lexical region; and
5. each `Use` names an enclosing earlier `Share`, so the binding graph is acyclic.

These checks are decidable by structural traversal with a finite lexical name stack.

### Lemma 10.2: source `Music` operations return finite recipes

Every successful well-typed `Music` operation returns a finite recipe. This is `FiniteMusic`, not necessarily
`ValidMusic`.

**Proof.** Each operation allocates one constructor around already finite argument values. It does not call itself or
store a source closure. `music_fact` checks the finite span and template. `music_share` rejects a local duplicate;
`music_use` creates one finite leaf whose binding is checked only in the complete recipe. Structural induction over a
finite sequence of operation calls proves finiteness. ∎

### Lemma 10.3: selective score-map rewriting terminates and preserves admission

If `music_map(id, m, anchor)` succeeds, it returns one finite share-free recipe whose facts are admitted. A mapped use
cannot change an unmapped use of the same binding.

**Proof.** First decide `ValidMusic(m)`. Traverse its finite lexical binding tree and replace each `Use` by its earlier
finite definition. The binding relation is acyclic, so induction on the number of enclosing bindings and then on recipe
size proves termination. Apply the admitted map from assumption 9 to each unfolded fact and append the map anchor to
that fact's origin. Sequence and overlay recurse componentwise. The output has no reference node and every fact remains
admitted. Since only the argument subtree is traversed, a sibling use outside it is unchanged. ∎

### Lemma 10.4: zero-origin closing preserves temporal typing

If `ValidMusic(m)` and the score scope and source root are valid, translating `m` at logical origin zero returns a
closed finite `Term<ScoreFact>` or a stated `MusicError`.

**Proof.** Induct on `m`, carrying a finite environment from share name to well-typed marked temporal binding.

- `Empty` translates to the typed empty term.
- `Fact(span, template, origin)` fills the valid score scope and finite origin beginning at the source root and original
  anchor, then appends the origin's admitted map steps. Admission gives a `ScoreFact`. The checked relative span yields
  a finite typed occurrence term.
- `Sequence(left, right)` uses both induction hypotheses. On two successes, the governed sequence constructor returns a
  finite term of the same payload type. It supplies relative placement of the right term.
- `Overlay(left, right)` is the same argument with governed unequal-length overlay. No padding premise is needed.
- `Share(name, definition, body)` closes the finite definition under the current environment, enters its typed marked
  binding, and closes the body under the extended environment.
- `Use(name, site)` finds the one earlier typed binding guaranteed by validity and emits the governed marked reference.
  The reference records binding root and generation site.

An invalid scope, root, payload, map, or defensive name check returns its stated finite error. No case gets stuck or
recurses on the original recipe. ∎

### Theorem 10.5: `close_music` is total and type safe

For finite `Music` value `m` and finite `MusicalContext κ`, close returns exactly one `Err(error)` or `Ok(t)`. On
success, `t` is finite, closed, and has type `Term<ScoreFact>`.

**Proof.** Decide `ValidMusic` and context validity by finite traversal. On failure, return the unique ordered first
error. On success, apply Lemma 10.4 at origin zero. Finally wrap the successful term in the governed delay by
`κ.placement`. The context check guarantees a nonnegative exact ratio, so delay preserves finiteness, closure, and
typing. Determinism of every check, admitted map, and temporal constructor gives one result. ∎

### Corollary 10.6: successful music closes to a finite timeline

**Proof.** Theorem 10.5 gives a closed finite well-typed temporal term. Governing kernel normalization returns one
finite `Timeline<ScoreFact>`. ∎

## 11. Typed stage composition

### Theorem 11.1: origin paths form a category for one registry

For exact anchored presentations accepted by one valid registry, finite origin paths compose associatively and empty
paths are identities.

**Proof.** A path is a finite typed list of steps whose adjacent exact endpoints match. Composition concatenates lists
at the equal middle endpoint and retains that endpoint. Finite-list concatenation is associative; concatenation with an
empty list changes nothing. If endpoints differ, no typed concatenation exists. ∎

### Theorem 11.2: successful typed passes compose

Suppose pass `P` returns a valid `PassResult<A, B>` and pass `Q` returns a valid `PassResult<B, C>`. For every path of
`Q`, take every exact `B` source anchor used by its first step and prepend all covering `P` paths that end there. If a
required anchor lacks a match, composition returns a diagnostic. Otherwise the resulting paths, `Q`'s output, the
retained intermediate presentation, and losses in pass order form a valid `PassResult<A, C>`.

**Proof.** The pass descriptors fix compatible source, target, evidence, and loss schemas. The exact endpoint premise
allows Theorem 11.1 on each required pair. Validity of `Q` gives at least one path for every addressable target anchor
in `C`. The construction processes every such path, never an arbitrary subset. Validity of `P` supplies complete
ancestry for each matched `B` anchor. Therefore every `C` anchor remains covered from `A`, including generated and
combined steps. Every intermediate anchor and step remains in the concatenated path, so the registry can validate it.
Exact-path deduplication and ordered finite-list concatenation give deterministic paths and losses. A version or anchor
mismatch returns a diagnostic rather than fabricating equality. ∎

The theorem applies whether the path goes source-to-notation, phrase-to-gesture, gesture-to-process, or through another
declared pair. It does not require one chain that every project follows.

## 12. Audit of the five paper programs

The programs in note 28 use the following source forms after expansion:

| Written form | Formal owner |
| --- | --- |
| `import ... as ...` | fixed qualified module rule |
| `import syntax ... as ...` | fixed header plus §9 expansion |
| `type` and `record` | nominal declarations and dependency order |
| `private` constructors | interface visibility and Theorem 8.1 |
| `pub fn`, `fn`, and `let` | product functions and non-recursive let generalization |
| record construction and `.field` | resolved `TypeId` and `FieldId` rules |
| constructor calls | rigid nominal constructor schemes |
| `match` and `if` | exhaustive matches, conditions, and Theorem 6.4 |
| list literals, `++`, `first`, and `second` | fixed prelude constructors and complete functions |
| `list.map`, `list.fold`, `result.map`, `and_then` | generated finite folds and explicit closures |
| `ratio_add`, `ratio_multiply`, comparisons, `nat_add`, `&&` | total exact base operations |
| staff blocks | rank-checked staff adapter, then the displayed ordinary expressions |
| studio block | rank-checked graph adapter, then the displayed ordinary expression |
| staff, theory, timing, gesture, and studio calls | complete ordinary package functions |
| close, temporal, interpretation, and preparation arrows | the typed operations in note 30 |

No program uses a recursive value binding, mutually recursive type, partial call, named call argument, default argument,
subtyping, overloaded field, implicit conversion, effect, dependent type, type-directed adapter, source-level process
loop, or hidden context dictionary.

The staff expansion in note 27 uses one self-recursive `StaffItem` whose recursive occurrences appear only in
`List<StaffItem>` and products. `Ending` is represented by the tuple `Anchor × Nat × List<StaffItem>`, so no mutual
declaration is hidden there. `Syntax` uses the same permitted `List<Syntax>` shape. These are the only new recursive
types needed by the trials.

### Theorem 12.1: the paper programs are covered by the metatheory

After replacing each displayed adapter block by its complete displayed expansion, every expression in note 28 belongs to
note 29's grammar. If its imported package schemes and stage descriptors are entered in a well-formed `Σ`, Theorems
2.4–11.2 apply.

**Proof.** The table above exhausts the concrete forms in the five programs. Each row maps to one parser sugar,
expression form, generated fold, adapter theorem, or stage contract. The full staff and studio expansions are finite
ordinary constructor and function expressions. There is no omitted branch or placeholder body. Therefore structural
induction over those displayed programs reaches only cases already proved. ∎

This is a paper-language result. It does not say the current parser accepts the new syntax.

## 13. Main theorem

### Theorem 13.1: safety and totality of the candidate language

Under the assumptions in §1, for every finite source module accepted by the candidate pipeline:

1. syntax expansion terminates, is deterministic, hygienic, source-attributed, and independent of inferred types;
2. resolution and rank-1 inference terminate, and inference returns principal types;
3. lowering preserves the first-match meaning of exhaustive source patterns and introduces no runtime match failure;
4. source evaluation preserves types, makes progress, is deterministic, and terminates;
5. clients cannot directly construct or inspect hidden representations;
6. adapter-produced expressions receive the same checks and evaluation theorem as handwritten expressions;
7. closing a `Music` value returns a stated error or a finite closed `Term<ScoreFact>`;
8. a successful close normalizes to a finite exact timeline under the accepted kernel laws; and
9. later typed pass records compose only at exact equal anchored representations.

**Proof.** Items 1–9 are Theorems 9.3–9.8, 2.4 and 3.3–3.5, 6.4, 5.1–5.4 and 7.6, 8.1, 9.8, 10.5, Corollary 10.6, and
Theorem 11.2 respectively. Their assumptions are exactly those fixed in §1. ∎

## 14. What the theorem deliberately leaves open

The theorem does not show that a musical package is faithful to a practice. It shows that the package's finite values
and total functions behave according to the language rules. Practitioner and domain review remain separate gates.

It also does not prove:

- package version solving or registry behavior;
- identity of compiled artifacts across builds;
- correctness of a persistent compiled-value cache;
- a stable binary interface;
- reversibility of a notation, analysis, gesture, or audio pass;
- termination of an unbounded audio run; or
- that every future syntax adapter satisfies its optional editing and printing laws.

Each editable adapter must prove or property-test its own semantic edit law. Each later pass must satisfy its own
schema, error, equality, origin, and loss contract. The main theorem supplies the language in which those checks run; it
does not make them true by declaration.

## 15. Verdict before independent review

The proof is complete under the stated contracts. The design earns this proof by refusing machinery the programs did not
need: partial calls, type-directed syntax, general recursion, open-ended `Music` context, and running audio in the
source evaluator.

This document is now the frozen proof target. It must receive one hostile independent review. One repair is allowed,
followed by one final review. Nothing moves to `docs/rules/` unless the final verdict is correct under the stated
contracts with no fatal, high, or medium finding.
