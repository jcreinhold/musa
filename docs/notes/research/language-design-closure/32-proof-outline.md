# Proof outline

## Purpose

This note states the theorems before proving them. It fixes the assumptions, names the lemmas, and shows the dependency
order. The final proof may add detail, but it may not quietly strengthen a premise or weaken a conclusion.

## 1. Fixed objects and notation

Fix:

- one finite, well-formed build graph `B`;
- its functional declaration table `Σ`;
- one finite source module and its local environment `Γ`;
- the source and core rules in note 29;
- the stage rules in note 30; and
- the compiler-operation, score-map, temporal, and derivation contracts listed in note 31 §8.

Write:

```text
resolve_B(s) = r or diagnostic
W(Σ, Γ, e) = (S, τ) or diagnostic
Σ ; Γ ⊢ e : τ
e -> e'
e ->* v
expand_B(s) = s' or diagnostic
close_music(m, κ) = Ok(t) or Err(error)
```

`S` is a type substitution. `->` is one unmetered mathematical source step. The implemented limited evaluator either
performs that same step and charges it or returns a phase-limit diagnostic before committing it.

## 2. Static well-formedness

### Lemma 2.1: nominal dependency order

Every accepted nominal declaration has a finite rank. A non-recursive field mentions only smaller-ranked nominal types.
A recursive field mentions its own type only in a permitted polynomial position.

**Method:** topologically sort the declaration graph after removing permitted self-edges; inspect the polynomial
grammar.

### Lemma 2.2: generated folds are well typed

For every accepted recursive type `D`, the generated `fold_D` has the scheme constructed in note 29 §4.3, and every
recursive result in an algebra replaces one proper `D` subvalue.

**Method:** induction on the polynomial field grammar.

### Theorem 2.3: resolution is decidable and functional

For finite `B`, source `s`, and authenticated scope ids, `resolve_B(s)` finishes with one resolved program or one finite
ordered diagnostic set. Every resolved occurrence has one id.

**Method:** structural traversal plus finite functional table lookup. Qualified paths follow fixed graph edges;
ambiguous short fields are errors before inference.

### Lemma 2.4: unification is decidable and returns a most general unifier

First-order unification of Musa monotypes, with an occurs check and rigid nominal heads, finishes. On success it returns
a most general unifier.

**Method:** the standard disagreement-pair measure: unresolved variables plus total constructor size. Rigid unequal
`TypeId`s fail.

### Theorem 2.5: Algorithm W is sound, complete, and principal

If `W(Σ, Γ, e) = (S, τ)`, then `Σ ; SΓ ⊢ e : τ`. If another substitution `S'` and type `τ'` type the same expression
under `S'Γ`, then a substitution `R` exists such that `τ' = Rτ` and `S'Γ` agrees with `R(SΓ)` on the free variables of
`e`.

If the expression is typable, W succeeds. W always terminates with a result or diagnostic.

**Method:** induction on expression syntax using Lemma 2.4. Match coverage and field resolution are finite independent
checks. Non-recursive `let` uses standard generalization.

### Corollary 2.6: inferred public interfaces are principal

For an acyclic module, checking declarations in dependency order gives each public value one principal scheme. Hiding
constructors removes names from the exported table without changing those schemes.

## 3. Type safety

### Lemma 3.1: weakening

Adding fresh unused monomorphic or scheme bindings to `Γ` preserves typing.

### Lemma 3.2: type substitution

Applying one well-formed type substitution to a typing derivation preserves the derivation.

### Lemma 3.3: term substitution

If `Σ ; Γ, x : τ ⊢ e : υ` and `Σ ; Γ ⊢ v : τ`, then `Σ ; Γ ⊢ e[v/x] : υ`.

### Lemma 3.4: generalized-value instantiation

If `Σ ; Γ ⊢ v : τ`, `ᾱ` is disjoint from `ftv(Γ)`, and `S` replaces only `ᾱ`, then `Σ ; Γ ⊢ v : Sτ`. Therefore one
let-bound value can replace uses at every instance of `gen(Γ, τ)`.

**Method for Lemmas 3.1–3.4:** induction on typing derivations; constructor and operation ids are rigid and unaffected.

### Lemma 3.5: pattern matching preserves bindings

If `Σ ; ∅ ⊢ v : τ`, `Σ ; Γ ⊢ p : τ ⇒ Γp`, and `matches(p, v) = Some(θ)`, then each binding in `θ` has the type given by
`Γp`.

**Method:** induction on the pattern.

### Lemma 3.6: exhaustive patterns match

If the coverage checker accepts an ordered matrix for `τ`, then every closed value of `τ` matches at least one row.

**Method:** the usual constructor-specialization proof, by induction on the finite value and accepted coverage witness.

### Lemma 3.7: operation preservation

For each operation descriptor `op : τ̄ ⇒ τ`, if the arguments are closed values of `τ̄`, then `δ_op` returns one closed
value of `τ`.

**Method:** operation-registry contract; each implementation must test this premise.

### Theorem 3.8: preservation

If `Σ ; ∅ ⊢ e : τ` and `e -> e'`, then `Σ ; ∅ ⊢ e' : τ`.

**Method:** induction on the step. Beta and `let` use Lemmas 3.3–3.4; match uses Lemma 3.5; operation uses Lemma 3.7;
fold uses Lemma 2.2.

### Theorem 3.9: progress

If `Σ ; ∅ ⊢ e : τ`, then `e` is a value or one `e'` exists with `e -> e'`.

**Method:** induction on typing. Match uses Lemma 3.6; operation uses totality; fold uses the canonical form of its
scrutinee. The annotation context and erase rule handle both annotation cases.

### Theorem 3.10: deterministic evaluation

If `e -> e₁` and `e -> e₂`, then `e₁ = e₂`.

**Method:** prove unique decomposition into one evaluation context and one redex. `Match-First` chooses one least arm;
operation functions are functional. Joins have already been erased and are not cases of this relation.

## 4. Lowering correctness

### Lemma 4.1: join erasure preserves types and results

Replace a non-recursive join by an ordinary local function and each tail jump by one complete call. The translated term
has the same type and ordinary result. Evaluation begins only after this erasure, so raw joins are not terms of the
progress or termination theorems.

### Lemma 4.2: one compiled pattern row is correct

Given an on-failure join for the remaining rows, compiling one pattern either evaluates its arm with exactly the
bindings from `matches` or jumps to that join. It does not evaluate both.

### Theorem 4.3: ordered match lowering is total and correct

For an accepted exhaustive ordered match, lowering terminates, emits no runtime failure form, preserves typing, and
returns the value of the first matching source arm.

**Method:** induction on pattern-matrix specialization. Lemma 3.6 closes the residual final row; Lemmas 4.1–4.2 handle
sharing and semantics.

## 5. Source termination

### Definition 5.1: reducible values

For a reducibility assignment `η` to free type variables, define `R[τ]η(v)`:

- base values have their stated type;
- products, records, and non-recursive constructors have reducible fields;
- a recursive constructor has reducible ordinary fields and reducible proper recursive children;
- a function maps every reducible argument product to a terminating reducible result; and
- an abstract nominal value is judged by its owning declaration, even when clients cannot see the constructor.

A term is reducible at `τ` when it evaluates in finitely many steps to a value in `R[τ]η`.

### Lemma 5.2: reducibility is closed under backward steps

If `e -> e'` and `e'` is reducible at `τ`, then `e` is reducible at `τ`.

### Lemma 5.3: generated folds preserve reducibility

If the recursive value and every algebra are reducible at their generated types, `fold_D` terminates in a reducible
result.

**Method:** induction on the finite recursive constructor tree; the polynomial lifting lemma handles products and
standard containers.

### Lemma 5.4: operations preserve reducibility

Each admitted first-order compiler operation maps reducible argument values to a reducible result.

**Method:** operation-registry contract. First-order input prevents an operation from repeatedly invoking a source
closure.

### Theorem 5.5: fundamental reducibility lemma

If `Σ ; Γ ⊢ e : τ` and substitution `θ` maps every variable in `Γ` to a reducible value at every required scheme
instance, then `θ(e)` is reducible at `τ`.

**Method:** induction on typing. The function, let, constructor, match, fold, and operation cases use the preceding
lemmas. Generalization quantifies over arbitrary reducibility assignments for fresh type variables.

### Theorem 5.6: every closed well-typed source term terminates

If `Σ ; ∅ ⊢ e : τ`, then one value `v` exists with `e ->* v`.

**Method:** Theorem 5.5 with the empty substitution. Theorem 3.10 makes `v` unique.

### Corollary 5.7: limited evaluation terminates deterministically

For fixed compiler limits, evaluation either returns the same value as the mathematical evaluator with the exact charge
trace, or returns the unique first phase-limit diagnostic. It never commits a step whose charge would exceed the limit.

## 6. Module privacy

### Theorem 6.1: clients cannot name a hidden constructor

In a resolved client node originating from client source or preserved input syntax, every constructor id is public in
the imported interface. A hidden constructor id can appear in generated syntax only when the owning module's exported
adapter used a checked `definition_name`. Such an adapter is an owner-defined constructor operation, not client forgery.

No client pattern can inspect a hidden constructor unless an owner-exported adapter deliberately emits that pattern.

**Method:** induction on resolution paths and hygiene origins. Private ids are absent from imported tables; opaque scope
ids prevent textual forgery.

### Corollary 6.2: sealing is representation-independent

Replacing a private representation while preserving public type ids for the current build's checked dependents and
public operation behavior cannot be observed by direct construction or matching. This is a one-build abstraction claim,
not a stable ABI promise.

## 7. Expansion

### Lemma 7.1: adapter definitions compile without expansion

Every adapter definition contains no adapter region. Its ordinary subterms use the source theorem. Its six transformer
builders have fixed types and deterministic finite reductions, so the whole adapter compiles to a total deterministic
function. Emitted-call rank is checked later and plays no role in compiling the definition.

### Lemma 7.2: the expansion measure decreases

One successful expansion replaces one adapter rank by a finite multiset of strictly smaller ranks. The finite multiset
measure decreases.

### Theorem 7.3: expansion terminates and is deterministic

For fixed grouped syntax, syntax imports, adapter definitions, compiler options, and limits, expansion returns one
ordinary expression syntax tree or one diagnostic after finitely many steps.

**Method:** well-founded induction on the multiset measure; Theorems 3.10 and 5.6 make each adapter call total and
deterministic. The leftmost-outermost strategy and deterministic charge check choose one next step.

### Theorem 7.4: expansion is independent of inference

Changing expected types, ordinary value environments, or inferred substitutions while keeping expansion inputs fixed
does not change the expansion result.

**Method:** those objects are absent from every function and capability used by expansion.

### Theorem 7.5: expansion preserves hygiene

Definition, preserved input, and local-slot identifiers resolve according to their definition, use, and local scopes.
Expansion cannot capture or forge another scope.

**Method:** structural induction on the finite builder result plus authenticated opaque scope ids and the exact
`(ExpansionContext, binding path)` rule.

### Theorem 7.6: every generated node has finite source attribution

Every output node either keeps original source info or names one unique expansion record and child. Following parent
records ends at an original use site after finitely many links.

**Method:** induction on expansion steps; `ExpansionId` is the exact adapter-region path, node paths are unique by the
adapter checker, and Theorem 7.3 makes the record list finite.

### Theorem 7.7: checked expansion is source-safe

If expansion returns expression syntax, parsing and resolution succeed, and W assigns type `τ`, then evaluating the
lowered result returns one value of `τ` or the fixed limit diagnostic. Adapter output bypasses no ordinary static rule.

**Method:** Theorems 2.3, 2.5, 3.8–3.10, 4.3, and 5.6.

## 8. `Music` closure

### Lemma 8.1: recipe construction is finite

Every well-typed `Music` operation returns a finite `ScoreRecipe` or a stated build error. `ValidMusic` separately
decides lexical share/use validity and payload admission.

### Lemma 8.2: admitted maps preserve fact templates

Every `ScoreMapId` maps one admitted `FactTemplate` to one admitted template. `music_map` validates and unfolds the
finite selected recipe, maps each resulting fact, returns a share-free finite recipe, and terminates deterministically.

### Theorem 8.3: closing `Music` is total and type safe

For finite `m` and `κ`, `close_music(m, κ)` terminates with one `MusicError` or `Ok(t)`. If it returns `Ok(t)`, then `t`
is finite, closed, and has type `Term<ScoreFact>`.

**Method:** validate, then use structural induction on the recipe with a finite lexical environment for `Share` and
`Use`. Sequence, overlay, binding, and marked reference use their governed temporal typing rules. Pending payload maps
cannot occur; Lemma 8.2 proves their earlier rewrite.

### Corollary 8.4: temporal evaluation is finite

If close returns `Ok(t)`, governed temporal normalization returns one finite `Timeline<ScoreFact>`.

## 9. Stage composition

### Theorem 9.1: exact-anchor composition is well defined

Two valid pass paths compose exactly when the first target is the second source as the same presentation version and
anchor. Composition retains the middle anchor, concatenates path steps and ordered losses, is associative, and has empty
paths as identities.

**Method:** typed finite-list concatenation at equal endpoints.

### Theorem 9.2: typed stage passes compose

If pass `P : A -> Result<PassResult<A, B>, E>` and pass `Q : B -> Result<PassResult<B, C>, F>` both return valid
results, compose every `Q` target path with all `P` paths required by its exact `B` source anchors. The result covers
every target anchor in `C`. If one required anchor or version differs, composition returns a diagnostic rather than a
partial origin record.

## 10. Paper-program coverage

### Theorem 10.1: the five paper programs use only proved forms

After the displayed adapter expansions, every expression in note 28 is an instance of note 29's grammar and uses only
the listed module, data, record, function, match, fold, operation, and stage rules.

**Method:** a literal construct table in the final proof. This is a paper audit, not a claim that the current compiler
accepts the programs.

## 11. Dependency order

The final proof follows this order:

```text
declaration order
-> unification and W
-> substitution
-> preservation, progress, determinism
-> match lowering and join erasure
-> reducibility and termination
-> module privacy
-> adapter rank and expansion
-> Music closure
-> exact stage composition
-> paper-program audit
```

No theorem in this list depends on package identity or cache correctness.
